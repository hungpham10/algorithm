use std::io::{Error, ErrorKind};
use algorithm::{encrypt, decrypt};

pub async fn run(master_key: &str, action: &str, payload: &str) -> std::io::Result<()> {
    if master_key.len() != 32 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Master key must be 32 bytes, current length: {}", master_key.len()),
        ));
    }

    match action {
        "encrypt" => {
            // payload truyền vào là một chuỗi thô cần mã hóa (ví dụ: "my_token_secret")
            let token_plain = payload.to_string();

            // 1. Mã hóa ra mảng bytes bằng hàm trong module algorithm
            let encrypted_bytes = encrypt(master_key.as_bytes(), &token_plain)?;

            // 2. Encode mảng bytes đó thành chuỗi Hex và in ra
            // Bạn lấy chuỗi này để nhét vào hàm UNHEX('...') của SQL
            println!("{}", hex::encode(encrypted_bytes));
            Ok(())
        }
        "decrypt" => {
            // payload truyền vào lúc này phải là chuỗi Hex lấy từ DB lên (hoặc kết quả của hàm HEX(token))
            // 1. Decode chuỗi Hex đó ngược lại thành mảng bytes thô
            let encrypted_bytes = hex::decode(payload).map_err(|error| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("Decode chuỗi Hex thất bại: {error}"),
                )
            })?;

            // 2. Giải mã mảng bytes về lại chuỗi gốc ban đầu
            println!("{}", decrypt(master_key.as_bytes(), &encrypted_bytes)?);
            Ok(())
        }
        _ => Err(Error::new(
            ErrorKind::InvalidInput,
            "Unknown action, only 'encrypt' or 'decrypt'",
        )),
    }
}
