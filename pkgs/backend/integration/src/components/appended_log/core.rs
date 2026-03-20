use std::io::{BufReader, Error, ErrorKind, Read, Seek, SeekFrom, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, RwLock};

use ssh2::{OpenFlags, OpenType, Session};
use url::Url;

#[derive(Clone)]
pub struct AppendedLog {
    session: Session,
    storage: String,
    lock: Arc<RwLock<()>>,
}

impl AppendedLog {
    pub fn new(dsn: &String) -> Result<Self, Error> {
        let parsed = Url::parse(dsn.as_str())
            .map_err(|e| Error::new(ErrorKind::InvalidInput, format!("URL is invalid: {}", e)))?;

        if parsed.scheme() != "sftp" && parsed.scheme() != "ssh" {
            Err(Error::new(
                ErrorKind::InvalidInput,
                "Only support `sftp` or `ssh`",
            ))
        } else {
            let host = parsed
                .host_str()
                .ok_or(Error::new(ErrorKind::InvalidInput, "Missing `host` in url"))?;
            let port = parsed.port().unwrap_or(22);
            let username = parsed.username();
            let password = parsed.password().unwrap_or("");
            let storage = parsed.path().trim_start_matches('/').to_string();

            let tcp = TcpStream::connect(format!("{}:{}", host, port))?;

            let mut session = Session::new()?;

            session.set_tcp_stream(tcp);
            session
                .handshake()
                .map_err(|e| Error::new(ErrorKind::Other, e))?;
            session.userauth_password(username, password).map_err(|e| {
                Error::new(ErrorKind::PermissionDenied, format!("Auth failed: {}", e))
            })?;

            if !session.authenticated() {
                Err(Error::new(ErrorKind::PermissionDenied, "Auth failed"))
            } else {
                Ok(Self {
                    session,
                    storage,
                    lock: Arc::new(RwLock::new(())),
                })
            }
        }
    }

    pub fn list_partitions(&self) -> Result<Vec<String>, Error> {
        let _guard = self
            .lock
            .read()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Lock failed: {}", e)))?;

        Ok(self
            .session
            .sftp()
            .map_err(|e| Error::new(ErrorKind::Other, e))?
            .readdir(Path::new("/"))
            .map_err(|e| Error::new(ErrorKind::Other, e))?
            .into_iter()
            .map(|(path, _)| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            })
            .filter(|s| !s.is_empty())
            .collect())
    }

    pub fn rotate_new_partition(&self) -> Result<(), Error> {
        let _guard = self
            .lock
            .write()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Lock failed: {}", e)))?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let new_path = format!("{}.{}", self.storage, timestamp);

        self.session
            .sftp()
            .map_err(|e| Error::new(ErrorKind::Other, format!("SFTP error: {}", e)))?
            .rename(Path::new(&self.storage), Path::new(&new_path), None)
            .map_err(|e| Error::new(ErrorKind::Other, format!("Rotate failed: {}", e)))?;
        Ok(())
    }

    pub fn get_latest_offset(&self) -> Result<u64, Error> {
        let _guard = self
            .lock
            .read()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Lock failed: {}", e)))?;

        Ok(self
            .session
            .sftp()
            .map_err(|e| Error::new(ErrorKind::Other, format!("SFTP session failed: {}", e)))?
            .open(Path::new(&self.storage))
            .map_err(|e| Error::new(ErrorKind::Other, format!("Open failed: {}", e)))?
            .stat()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Stat failed: {}", e)))?
            .size
            .unwrap_or(0))
    }

    pub fn write_log_stream(&self, data: &[u8]) -> Result<(), Error> {
        let _guard = self
            .lock
            .write()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Lock failed: {}", e)))?;

        let mut file = self
            .session
            .sftp()
            .map_err(|e| Error::new(ErrorKind::Other, format!("SFTP session failed: {}", e)))?
            .open_mode(
                Path::new(&self.storage),
                OpenFlags::CREATE | OpenFlags::APPEND | OpenFlags::WRITE,
                0o644,
                OpenType::File,
            )
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        let len = data.len() as u32;
        file.write_all(&len.to_le_bytes())?;

        file.write_all(data)?;
        file.flush()?;
        Ok(())
    }

    pub fn read_log_stream(
        &self,
        start_offset: u64, // Bắt đầu từ offset cụ thể
    ) -> Result<impl Iterator<Item = (Vec<u8>, u64)> + '_, Error> {
        let _guard = self
            .lock
            .read()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Lock failed: {}", e)))?;

        let mut file = self
            .session
            .sftp()
            .map_err(|e| Error::new(ErrorKind::Other, format!("SFTP error: {}", e)))?
            .open(Path::new(&self.storage))
            .map_err(|e| Error::new(ErrorKind::Other, format!("Open failed: {}", e)))?;

        // Di chuyển tới vị trí bắt đầu
        file.seek(SeekFrom::Start(start_offset))?;

        // Dùng BufReader để tối ưu việc đọc từ network
        let mut reader = BufReader::new(file);
        let mut current_pos = start_offset;

        let iter = std::iter::from_fn(move || {
            // 1. Đọc 4 bytes độ dài
            let mut len_buf = [0u8; 4];
            if reader.read_exact(&mut len_buf).is_err() {
                return None; // Hết file hoặc lỗi
            }
            let len = u32::from_le_bytes(len_buf) as usize;

            // 2. Đọc khối dữ liệu theo độ dài vừa lấy
            let mut data_buf = vec![0u8; len];
            if reader.read_exact(&mut data_buf).is_err() {
                return None;
            }

            // 3. Cập nhật vị trí offset mới (4 bytes header + len data)
            current_pos += 4 + len as u64;

            Some((data_buf, current_pos))
        });

        Ok(iter)
    }

    pub fn read_block_at(&self, offset: u64) -> Result<(Vec<u8>, u64), Error> {
        let _guard = self
            .lock
            .read()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Lock failed: {}", e)))?;

        let mut file = self
            .session
            .sftp()
            .map_err(|e| Error::new(ErrorKind::Other, format!("SFTP error: {}", e)))?
            .open(Path::new(&self.storage))
            .map_err(|e| Error::new(ErrorKind::Other, format!("Open failed: {}", e)))?;

        file.seek(SeekFrom::Start(offset))?;

        let mut len_buf = [0u8; 4];
        file.read_exact(&mut len_buf).map_err(|e| {
            Error::new(
                e.kind(),
                format!("Failed to read block header at offset {}: {}", offset, e),
            )
        })?;
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut data_buf = vec![0u8; len];
        file.read_exact(&mut data_buf).map_err(|e| {
            Error::new(
                e.kind(),
                format!(
                    "Failed to read block data (len {}) at offset {}: {}",
                    len, offset, e
                ),
            )
        })?;

        let next_offset = offset + 4 + len as u64;
        Ok((data_buf, next_offset))
    }

    pub fn clear_old_logs(&self, before_timestamp: u64) -> Result<(), Error> {
        let _guard = self
            .lock
            .write()
            .map_err(|e| Error::new(ErrorKind::Other, format!("Lock failed: {}", e)))?;

        let sftp = self
            .session
            .sftp()
            .map_err(|e| Error::new(ErrorKind::Other, format!("SFTP error: {}", e)))?;

        let entries = sftp
            .readdir(Path::new("/"))
            .map_err(|e| Error::new(ErrorKind::Other, format!("Readdir failed: {}", e)))?;

        for (path, _stat) in entries {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with(&format!("{}.", self.storage)) {
                    let parts: Vec<&str> = file_name.split('.').collect();

                    if let Some(ts_str) = parts.last() {
                        if let Ok(file_ts) = ts_str.parse::<u64>() {
                            if file_ts < before_timestamp {
                                sftp.unlink(&path).map_err(|error| {
                                    Error::new(
                                        ErrorKind::Other,
                                        format!("Delete failed: {}", error),
                                    )
                                })?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const TEST_DSN1: &str =
        "sftp://LB4.hung0913208:XXh_01664957141@edge13.ftpgrid.com:22/test_unittest1.log";
    const TEST_DSN2: &str =
        "sftp://LB4.hung0913208:XXh_01664957141@edge13.ftpgrid.com:22/test_unittest2.log";
    const TEST_DSN3: &str =
        "sftp://LB4.hung0913208:XXh_01664957141@edge13.ftpgrid.com:22/test_unittest3.log";
    const TEST_DSN4: &str =
        "sftp://LB4.hung0913208:XXh_01664957141@edge13.ftpgrid.com:22/test_unittest4.log";

    #[test]
    fn test_full_lsm_flow() {
        let logger = AppendedLog::new(&TEST_DSN1.to_string()).expect("Kết nối thất bại");
        let mock_data = b"Unit test data content\n";

        logger
            .write_log_stream(mock_data)
            .expect("Ghi log thất bại");

        let files = logger.list_partitions().expect("Liệt kê thất bại");
        assert!(
            files.iter().any(|f| f.contains("test_unittest1.log")),
            "File log phải tồn tại sau khi append"
        );

        logger.rotate_new_partition().expect("Rotate thất bại");

        let files_after = logger
            .list_partitions()
            .expect("Liệt kê sau rotate thất bại");
        assert!(
            files_after
                .iter()
                .any(|f| f.starts_with("test_unittest1.log.")),
            "Phải tìm thấy file archive có chứa timestamp"
        );
    }

    #[test]
    fn test_streaming_read_integrity() {
        let logger = AppendedLog::new(&TEST_DSN2.to_string()).expect("Kết nối thất bại");

        // 1. Chuẩn bị các "message" riêng biệt
        let messages = vec![
            b"Message-1".to_vec(),
            b"Message-2-Longer".to_vec(),
            b"Msg-3".to_vec(),
        ];

        // Xóa file cũ/Rotate để test môi trường sạch
        let _ = logger.rotate_new_partition();

        // 2. Ghi từng message vào log (mỗi message sẽ có 4 bytes header độ dài)
        for msg in &messages {
            logger.write_log_stream(msg).expect("Ghi log thất bại");
        }

        // 3. Đọc lại bằng stream từ đầu (offset 0)
        let stream = logger.read_log_stream(0).expect("Mở stream thất bại");

        let mut recovered_messages = Vec::new();
        let mut last_offset = 0;

        for (data, next_offset) in stream {
            // Kiểm tra offset trả về phải luôn tăng tiến
            assert!(next_offset > last_offset);
            last_offset = next_offset;

            recovered_messages.push(data);
        }

        // 4. Kiểm chứng
        assert_eq!(
            recovered_messages.len(),
            messages.len(),
            "Số lượng message không khớp"
        );

        for (original, recovered) in messages.iter().zip(recovered_messages.iter()) {
            assert_eq!(original, recovered, "Nội dung message bị sai lệch");
        }

        // 5. Kiểm tra tính năng seek: Đọc từ message thứ 2 (dùng offset của message 1)
        // Tính toán offset message 2: 4 (len msg1) + 9 (nội dung msg1) = 13
        let msg1_len = (4 + messages[0].len()) as u64;
        let mut partial_stream = logger
            .read_log_stream(msg1_len)
            .expect("Mở stream từ offset thất bại");

        let (first_data, _) = partial_stream.next().expect("Phải có message thứ 2");
        assert_eq!(
            first_data, messages[1],
            "Đọc từ offset không khớp message mong đợi"
        );

        println!(
            "Stream test passed: {} messages recovered. Last offset: {}",
            recovered_messages.len(),
            last_offset
        );
    }

    #[test]
    fn test_append_multiple_times() {
        let logger = AppendedLog::new(&TEST_DSN3.to_string()).expect("Kết nối thất bại");

        // Xóa trắng file active bằng cách rotate để đảm bảo môi trường sạch
        let _ = logger.rotate_new_partition();

        let messages = vec!["Line 1\n", "Line 2\n", "Line 3\n"];

        // 1. Ghi 3 lần liên tiếp
        for msg in &messages {
            logger.write_log_stream(msg.as_bytes()).unwrap();
        }

        // 2. Mở stream từ đầu (offset 0)
        // Lưu ý: Hàm read_log_stream mới chỉ nhận 1 tham số là start_offset
        let stream = logger.read_log_stream(0).expect("Mở stream thất bại");

        // 3. Thu thập dữ liệu từ stream (trả về tuple (data, next_offset))
        let recovered_data: Vec<Vec<u8>> = stream.map(|(data, _offset)| data).collect();

        // 4. Kiểm chứng
        assert_eq!(
            recovered_data.len(),
            3,
            "Phải nhận được đúng 3 khối dữ liệu"
        );

        // Ghép các khối lại để kiểm tra nội dung chuỗi nếu cần
        let full_content = recovered_data.concat();
        let content_str = String::from_utf8_lossy(&full_content);

        assert!(content_str.contains("Line 1"));
        assert!(content_str.contains("Line 2"));
        assert!(content_str.contains("Line 3"));

        // Kiểm tra từng dòng log cụ thể
        for (i, data) in recovered_data.iter().enumerate() {
            let line = String::from_utf8_lossy(data);
            assert_eq!(line, messages[i], "Nội dung dòng thứ {} không khớp", i + 1);
        }

        println!(
            "Append multiple times test passed. Total bytes (including headers): {}",
            (4 * 3) + full_content.len()
        );
    }

    #[test]
    fn test_concurrent_writes() {
        use std::thread;

        let logger = AppendedLog::new(&TEST_DSN4.to_string()).expect("Kết nối thất bại");

        let _ = logger.rotate_new_partition();

        let messages: Vec<String> = (0..20)
            .map(|i| format!("Message from thread {}\n", i))
            .collect();

        // Spawn nhiều luồng ghi song song
        let mut handles = Vec::new();
        for msg in messages.clone() {
            let logger_clone = logger.clone();
            handles.push(thread::spawn(move || {
                logger_clone.write_log_stream(msg.as_bytes()).unwrap();
            }));
        }

        // Join tất cả luồng
        for h in handles {
            h.join().unwrap();
        }

        // Đọc lại toàn bộ log
        let stream = logger.read_log_stream(0).expect("Mở stream thất bại");
        let recovered: Vec<String> = stream
            .map(|(data, _)| String::from_utf8_lossy(&data).to_string())
            .collect();

        // Kiểm tra xem tất cả messages đều xuất hiện
        for msg in &messages {
            assert!(
                recovered.contains(msg),
                "Thiếu message `{}`, nhận `{:?}`",
                msg,
                recovered
            );
        }

        println!(
            "Concurrent write test passed. {} messages recovered.",
            recovered.len()
        );
    }

    #[test]
    #[should_panic]
    fn test_invalid_url() {
        AppendedLog::new(&("not_a_url".to_string())).unwrap();
    }

    #[test]
    fn test_cleanup_logs_older_than_one_hour() {
        for dsn in vec![
            TEST_DSN1.to_string(),
            TEST_DSN2.to_string(),
            TEST_DSN3.to_string(),
            TEST_DSN4.to_string(),
        ] {
            let logger = AppendedLog::new(&dsn).expect("Kết nối thất bại");
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let one_minute_ago = now - 60;
            logger
                .clear_old_logs(one_minute_ago)
                .expect("Lỗi khi xóa log cũ");

            let files = logger.list_partitions().unwrap();
            let exists_old = files.iter().any(|f| {
                if let Some(ts_str) = f.split('.').last() {
                    ts_str.parse::<u64>().unwrap_or(0) <= one_minute_ago
                } else {
                    true
                }
            });

            assert!(exists_old, "File cũ phải còn xoá hết");
        }
    }
}
