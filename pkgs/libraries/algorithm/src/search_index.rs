//! Search module — KMP + DFS substring ("LIKE") search trên RadixTree + Storage.
//!
//! ## Idea
//! Duy trì **shortcuts** (in-memory map) giúp tìm nhanh các node có chứa ký tự
//! đầu tiên của pattern. Với mỗi candidate, chạy **KMP** matching trên prefix
//! của node; nếu prefix ngắn hơn pattern thì **DFS** xuống children.
//!
//! ## Shortcut structure
//! ```text
//! shortcuts[shard][byte] = HashSet<node_id>
//! ```
//! - `shard` — shard index (0..sharding)
//! - `byte` — ký tự bất kỳ (0..255)
//! - `HashSet<node_id>` — các node có chứa byte đó trong prefix
//!
//! Shortcuts chỉ là **index nhanh** để tìm candidate node, không lưu vị trí.
//! Vị trí được scan trực tiếp từ prefix của node khi search.
//!
//! Shortcuts được cập nhật:
//! - Khi **insert** node mới → `update_shortcuts()`
//! - Khi **split** node → callback `OnSplitCallback` transfer entries từ parent sang leg

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::radixtree::{self, EMPTY, RadixTree};
use crate::storage::Storage;

// ==================== Error ====================

#[derive(Debug)]
pub enum SearchError {
    #[allow(dead_code)]
    NotFound,
    Storage(String),
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::NotFound => write!(f, "not found"),
            SearchError::Storage(msg) => write!(f, "storage error: {msg}"),
        }
    }
}

impl std::error::Error for SearchError {}

impl From<radixtree::RadixError> for SearchError {
    fn from(e: radixtree::RadixError) -> Self {
        match e {
            radixtree::RadixError::NotFound => SearchError::NotFound,
            _ => SearchError::Storage(e.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, SearchError>;

// ==================== Shortcut Data ====================

/// shortcuts[shard][byte] = HashSet<node_id>
/// Chỉ lưu node nào có chứa byte đó, không lưu vị trí.
/// Vị trí được scan trực tiếp từ prefix khi search.
type ShortcutData = Vec<HashMap<u8, HashSet<usize>>>;

// ==================== SearchIndex ====================

/// SearchIndex — cho phép tìm kiếm substring (LIKE) trên RadixTree.
pub struct SearchIndex {
    tree: RadixTree,
    shortcuts: Arc<Mutex<ShortcutData>>,
    entries: Vec<(i32, String)>,
}

impl SearchIndex {
    // ── Constructor ──

    /// Tạo `SearchIndex` mới với `Storage` cụ thể.
    pub fn new<S: Storage + 'static>(sharding: usize, storage: S) -> Self {
        let sharding = sharding.max(1);
        let shortcuts = Arc::new(Mutex::new(
            (0..sharding)
                .map(|_| HashMap::<u8, HashSet<usize>>::new())
                .collect::<Vec<_>>(),
        ));

        let mut tree = RadixTree::new(sharding, storage);

        // Register split callback
        // 1. Thêm shortcuts cho từng byte trong leg prefix
        // 2. Xoá parent khỏi byte nào không còn trong parent prefix sau split
        let cb_shortcuts = shortcuts.clone();
        tree.with_callback(Arc::new(
            move |parent_id, leg_id, old_prefix, breakpoint| {
                let mut sc = match cb_shortcuts.lock() {
                    Ok(s) => s,
                    Err(_) => return Err(radixtree::RadixError::Callback),
                };

                let sharding = sc.len();

                for (_, byte) in old_prefix.iter().enumerate().skip(breakpoint) {
                    let si = radixtree::shard_of(*byte, sharding);
                    let byte_set = sc[si].entry(*byte).or_default();

                    // Nếu byte này không còn trong prefix mới của parent → xoá parent
                    let parent_still_has = old_prefix[..breakpoint].contains(byte);
                    if !parent_still_has {
                        byte_set.remove(&parent_id);
                    }

                    // Leg node chắc chắn có byte này
                    byte_set.insert(leg_id);

                    if byte_set.is_empty() {
                        sc[si].remove(byte);
                    }
                }

                Ok(())
            },
        ));

        Self {
            tree,
            shortcuts,
            entries: Vec::new(),
        }
    }

    /// Convenience: `SearchIndex` in-memory.
    pub fn in_memory(sharding: usize) -> Self {
        Self::new(sharding, crate::storage::InMemoryStorage::default())
    }

    // ── Insert ──

    /// Thêm một entry vào index.
    ///
    /// - `key` — key để search (vd: tên cửa hàng dạng byte)
    /// - `entry_id` — ID của entry (vd: store_id)
    /// - `name` — tên hiển thị
    pub async fn insert(&mut self, key: &[u8], entry_id: i32, name: &str) -> Result<()> {
        if key.is_empty() {
            return Err(SearchError::NotFound);
        }

        // record trong RadixTree là 1-indexed (EMPTY = 0)
        let record_idx = self.entries.len() + 1;
        self.entries.push((entry_id, name.to_string()));

        let (new_node_id, breakpoint) = self.tree.insert(key, record_idx).await?;

        // Nếu tạo node mới → cập nhật shortcuts
        if new_node_id != EMPTY {
            self.update_shortcuts(key, breakpoint, new_node_id);
        }

        Ok(())
    }

    /// Cập nhật shortcuts cho một node mới: thêm node_id vào set của từng byte.
    fn update_shortcuts(&self, key: &[u8], breakpoint: usize, node_id: usize) {
        if let Ok(mut shortcuts) = self.shortcuts.lock() {
            let sharding = shortcuts.len();
            for (_, byte) in key.iter().enumerate().skip(breakpoint) {
                let si = radixtree::shard_of(*byte, sharding);
                shortcuts[si].entry(*byte).or_default().insert(node_id);
            }
        }
    }

    // ── Search LIKE ──

    /// Tìm kiếm substring — entries có key chứa `pattern`.
    ///
    /// Dùng KMP + DFS, với shortcut index để tìm candidate nodes.
    ///
    /// Trả về `Vec<(entry_id, name)>`.
    pub async fn search_like(&self, pattern: &[u8], limit: usize) -> Result<Vec<(i32, String)>> {
        if pattern.is_empty() {
            return Err(SearchError::NotFound);
        }

        let lps = Self::preprocess_pattern(pattern);
        let first_byte = pattern[0];
        let sharding = self.tree.sharding_count();
        let si = radixtree::shard_of(first_byte, sharding);

        // Collect candidates upfront, drop lock before any .await
        let candidates: Vec<usize> = {
            let shortcuts = self
                .shortcuts
                .lock()
                .map_err(|e| SearchError::Storage(e.to_string()))?;

            shortcuts[si]
                .get(&first_byte)
                .map(|byte_set| {
                    let mut cands: Vec<usize> = byte_set.iter().copied().collect();
                    cands.sort();
                    cands
                })
                .unwrap_or_default()
        };

        let mut results = Vec::new();
        let mut seen = HashSet::new();

        for &node_id in &candidates {
            if results.len() >= limit {
                break;
            }

            let found = self.dfs_search(node_id, pattern, &lps, 0, 0, limit).await?;

            for entry in found {
                if seen.insert(entry.0) {
                    results.push(entry);
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }

        if results.is_empty() {
            Err(SearchError::NotFound)
        } else {
            Ok(results)
        }
    }

    // ── KMP: LPS array ──

    /// Build Longest Proper Prefix which is also Suffix (LPS) array.
    fn preprocess_pattern(pattern: &[u8]) -> Vec<usize> {
        let n = pattern.len();
        let mut lps = vec![0; n];
        let mut j = 0;
        for i in 1..n {
            while j > 0 && pattern[i] != pattern[j] {
                j = lps[j - 1];
            }
            if pattern[i] == pattern[j] {
                j += 1;
                lps[i] = j;
            }
        }
        lps
    }

    // ── DFS Search ──

    /// DFS + KMP: tìm pattern bắt đầu từ `(data_pos, pattern_pos)` trong
    /// subtree của `node_id`.
    async fn dfs_search(
        &self,
        node_id: usize,
        pattern: &[u8],
        lps: &[usize],
        pattern_pos: usize,
        data_pos: usize,
        limit: usize,
    ) -> Result<Vec<(i32, String)>> {
        let prefix = self.tree.get_node_prefix(node_id).await?;

        // Nếu phần còn lại của prefix (từ data_pos) ngắn hơn phần còn lại
        // của pattern → cần đệ quy xuống children
        let remaining = pattern.len().saturating_sub(pattern_pos);
        let effective_prefix_len = prefix.len().saturating_sub(data_pos);
        let do_recursive = effective_prefix_len < remaining;

        let (found, keep, _, new_pattern_pos) =
            Self::kmp_match(pattern, &prefix, lps, pattern_pos, data_pos, do_recursive);

        if found {
            // Match hoàn chỉnh → collect toàn bộ records trong subtree
            let record_ids = self.collect_subtree_records(node_id).await?;
            return Ok(self.resolve_records(&record_ids, limit));
        }

        // Nếu match thất bại và ta đang bắt đầu fresh (pattern_pos == 0),
        // thử tất cả vị trí còn lại của pattern[0] trong cùng prefix
        // Dùng data_pos làm mốc (vị trí đã thử), không dùng new_data_pos
        // vì KMP có thể đã consume hết prefix (do_recursive=true) nhưng
        // vẫn còn vị trí hợp lệ chưa được thử làm điểm xuất phát.
        if !found && pattern_pos == 0 && (data_pos + 1) < prefix.len() {
            for next_start in (data_pos + 1)..prefix.len() {
                if prefix[next_start] == pattern[0] {
                    let result =
                        Box::pin(self.dfs_search(node_id, pattern, lps, 0, next_start, limit))
                            .await?;
                    if !result.is_empty() {
                        return Ok(result);
                    }
                    // Kết quả rỗng → thử vị trí tiếp theo
                }
            }
        }

        // Nếu còn có thể match tiếp và prefix đã hết → DFS xuống children
        if do_recursive && keep && new_pattern_pos < pattern.len() {
            let next_byte = pattern[new_pattern_pos];
            let children = self.tree.get_children_ids(node_id).await?;

            for &child in &children {
                let child_prefix = self.tree.get_node_prefix(child).await?;
                if !child_prefix.is_empty() && child_prefix[0] == next_byte {
                    let found =
                        Box::pin(self.dfs_search(child, pattern, lps, new_pattern_pos, 0, limit))
                            .await?;

                    if !found.is_empty() {
                        return Ok(found);
                    }
                }
            }
        }

        Ok(Vec::new())
    }

    // ── KMP Matching ──

    /// Chạy KMP trên một `data` slice (prefix của node).
    ///
    /// Trả về `(found, keep, data_pos, pattern_pos)`:
    /// - `found`: tìm thấy pattern hoàn chỉnh trong data
    /// - `keep`: có tiến triển (partial match) — chỉ có ý nghĩa khi `!found && do_recursive`
    /// - `data_pos` / `pattern_pos`: trạng thái mới sau khi match
    fn kmp_match(
        pattern: &[u8],
        data: &[u8],
        lps: &[usize],
        mut pattern_pos: usize,
        mut data_pos: usize,
        do_recursive: bool,
    ) -> (bool, bool, usize, usize) {
        let mut keep = false;

        while data_pos < data.len() {
            if data[data_pos] == pattern[pattern_pos] {
                keep = true;
                data_pos += 1;
                pattern_pos += 1;
            }

            if pattern_pos == pattern.len() {
                return (true, false, data_pos, pattern_pos);
            }

            if data_pos < data.len() && pattern[pattern_pos] != data[data_pos] {
                if !do_recursive {
                    return (false, false, data_pos, pattern_pos);
                }

                if pattern_pos != 0 {
                    pattern_pos = lps[pattern_pos - 1];
                } else {
                    data_pos += 1;
                    keep = false;
                }
            }
        }

        (false, keep, data_pos, pattern_pos)
    }

    // ── Helpers ──

    /// Collect toàn bộ record IDs trong subtree của `node_id` (DFS).
    async fn collect_subtree_records(&self, node_id: usize) -> Result<Vec<usize>> {
        let mut records = Vec::new();

        let record = self.tree.get_node_record(node_id).await?;
        if record != EMPTY {
            records.push(record);
        }

        let children = self.tree.get_children_ids(node_id).await?;
        for &child in &children {
            let child_records = Box::pin(self.collect_subtree_records(child)).await?;
            records.extend(child_records);
        }

        Ok(records)
    }

    /// Chuyển đổi record IDs (1-indexed) thành entries.
    fn resolve_records(&self, record_ids: &[usize], limit: usize) -> Vec<(i32, String)> {
        let mut results = Vec::new();
        let mut seen = HashSet::new();
        for &rid in record_ids {
            if rid == EMPTY {
                continue;
            }
            let idx = rid - 1; // 1-indexed → 0-indexed
            if idx < self.entries.len() && seen.insert(idx) {
                results.push(self.entries[idx].clone());
                if results.len() >= limit {
                    break;
                }
            }
        }
        results
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_and_search_like_simple() {
        let mut idx = SearchIndex::in_memory(4);
        idx.insert(b"hello", 1, "Hello").await.unwrap();
        idx.insert(b"world", 2, "World").await.unwrap();
        idx.insert(b"help", 3, "Help").await.unwrap();

        let results = idx.search_like(b"hel", 10).await.unwrap();
        assert_eq!(results.len(), 2, "should find 'hello' and 'help'");
        let ids: Vec<i32> = results.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&3));
    }

    #[tokio::test]
    async fn test_search_like_substring() {
        let mut idx = SearchIndex::in_memory(4);
        idx.insert(b"tiem vang", 1, "Tiệm Vàng").await.unwrap();
        idx.insert(b"tiem bac", 2, "Tiệm Bạc").await.unwrap();

        // Search "vang" — should find "tiem vang"
        let results = idx.search_like(b"vang", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    #[tokio::test]
    async fn test_search_like_partial_match_through_split() {
        let mut idx = SearchIndex::in_memory(4);
        // Insert keys that share prefix → trigger split
        idx.insert(b"hello", 1, "Hello").await.unwrap();
        idx.insert(b"help", 2, "Help").await.unwrap();
        idx.insert(b"held", 3, "Held").await.unwrap();

        // Search "llo" — should find "hello" via DFS after split
        let results = idx.search_like(b"llo", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    #[tokio::test]
    async fn test_search_like_not_found() {
        let mut idx = SearchIndex::in_memory(2);
        idx.insert(b"hello", 1, "Hello").await.unwrap();

        let result = idx.search_like(b"xyz", 10).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_like_empty_pattern() {
        let idx = SearchIndex::in_memory(2);
        assert!(idx.search_like(b"", 10).await.is_err());
    }

    #[tokio::test]
    async fn test_search_like_empty_index() {
        let idx = SearchIndex::in_memory(2);
        assert!(idx.search_like(b"anything", 10).await.is_err());
    }

    #[tokio::test]
    async fn test_search_like_limit() {
        let mut idx = SearchIndex::in_memory(4);
        for i in 0..10 {
            let name = format!("Item {i}");
            idx.insert(format!("item_{i}").as_bytes(), i, &name)
                .await
                .unwrap();
        }

        // Search "item" — tất cả 10 đều match, nhưng limit=3
        let results = idx.search_like(b"item", 3).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_search_like_with_unicode_bytes() {
        let mut idx = SearchIndex::in_memory(4);
        // "Hà Nội" in UTF-8
        let ha_noi = "Hà Nội".as_bytes();
        let sai_gon = "Sài Gòn".as_bytes();

        idx.insert(ha_noi, 1, "Hà Nội").await.unwrap();
        idx.insert(sai_gon, 2, "Sài Gòn").await.unwrap();

        // Search "Nội"
        let results = idx.search_like("Nội".as_bytes(), 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    #[tokio::test]
    async fn test_search_like_single_character() {
        let mut idx = SearchIndex::in_memory(4);
        idx.insert(b"aaaa", 1, "Aaaa").await.unwrap();
        idx.insert(b"bbbb", 2, "Bbbb").await.unwrap();

        let results = idx.search_like(b"a", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    #[tokio::test]
    async fn test_insert_duplicate_key() {
        let mut idx = SearchIndex::in_memory(4);
        idx.insert(b"hello", 1, "Hello").await.unwrap();
        // Insert same key again — RadixTree trả về (EMPTY, tail)
        // vì key đã tồn tại, không tạo node mới.
        // SearchIndex vẫn append entries, nhưng tree node trỏ đến
        // record cũ. Hành vi này OK vì duplicate key không phải use-case
        // chính của SearchIndex.
        let res = idx.insert(b"hello", 2, "Hello Again").await;
        // Insert vẫn thành công (RadixError không xảy ra)
        assert!(res.is_ok());

        let results = idx.search_like(b"hello", 10).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_search_like_no_dup_results() {
        let mut idx = SearchIndex::in_memory(4);
        // Insert two keys that share a subtree
        idx.insert(b"hello world", 1, "Hello World").await.unwrap();
        idx.insert(b"hello", 2, "Hello").await.unwrap();

        // Search "hello" — both entries should appear (no duplicates)
        let results = idx.search_like(b"hello", 10).await.unwrap();
        assert_eq!(results.len(), 2);
        let ids: Vec<i32> = results.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
    }

    #[tokio::test]
    async fn test_search_like_kmp_partial_at_end() {
        // KMP edge case: pattern partially matches at the end of the prefix,
        // then continues in child node
        let mut idx = SearchIndex::in_memory(4);
        // "abcde" stored with root prefix "abcd" and child prefix "e"
        // After insert "abcd" and "abcde", the tree might split
        idx.insert(b"abcd", 1, "ABCD").await.unwrap();
        idx.insert(b"abcde", 2, "ABCDE").await.unwrap();

        // Search "cde" — should find ABCDE via DFS
        let results = idx.search_like(b"cde", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 2);
    }

    // ==================== Benchmarks ====================

    #[tokio::test]
    async fn bench_search_like_bulk() {
        let mut idx = SearchIndex::in_memory(8);
        let store_names = [
            "Tiệm Vàng Hoàng Phát",
            "Tiệm Vàng Minh Châu",
            "Tiệm Vàng Bảo Tín",
            "Vàng Bạc Đá Quý Sài Gòn",
            "PNJ - Vàng Bạc Đá Quý",
            "DOJI - Trang Sức Cao Cấp",
            "Tiệm Vàng Kim Thành",
            "Vàng 9999 - Nguyên Liệu",
            "Tiệm Vàng Hồng Phát",
            "Vàng Mi Hồng - Quận 3",
            "Tiệm Vàng Phú Nhuận",
            "SJC - Công Ty Vàng Bạc Đá Quý",
            "Tiệm Vàng Ngọc Thạch",
            "Bảo Tín Minh Châu",
            "Vàng Thế Giới - Gold Price",
            "Tiệm Vàng An Phát",
            "Vàng 24K - Nữ Trang",
            "Tiệm Vàng Hồng Đức",
            "Vàng Mi Hồng - Cơ Sở 2",
            "Tiệm Vàng Bảo Tín Mạnh Hải",
        ];

        // Insert 100 entries (lặp lại 5 lần với tên khác nhau)
        for i in 0..100 {
            let name = store_names[i % store_names.len()];
            let key = format!("{name} - {i}");
            idx.insert(key.as_bytes(), i as i32, name).await.unwrap();
        }

        // Warmup
        let _ = idx.search_like("Vàng".as_bytes(), 10).await;

        // Benchmark prefix search
        let patterns: &[&[u8]] = &[
            "Vàng".as_bytes(),
            "Tiệm".as_bytes(),
            b"PNJ",
            b"SJC",
            "Bảo Tín".as_bytes(),
            b"9999",
        ];

        let start = std::time::Instant::now();
        let iterations = 50;
        for _ in 0..iterations {
            for pat in patterns {
                let _ = idx.search_like(pat, 10).await;
            }
        }
        let elapsed = start.elapsed();
        let avg_ns = elapsed.as_nanos() as f64 / (iterations * patterns.len()) as f64;

        eprintln!(
            "[bench] search_like bulk: {:.0} ns/call ({} iterations, {} patterns)",
            avg_ns,
            iterations,
            patterns.len()
        );

        // Verify correctness
        let results = idx.search_like("Vàng".as_bytes(), 10).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 10);
    }

    #[tokio::test]
    async fn bench_search_like_short_pattern() {
        let mut idx = SearchIndex::in_memory(8);
        let names = [
            "apple",
            "apricot",
            "banana",
            "cherry",
            "date",
            "elderberry",
            "fig",
            "grape",
        ];

        for i in 0..200 {
            let name = names[i % names.len()];
            let key = format!("{name}_{i}");
            idx.insert(key.as_bytes(), i as i32, name).await.unwrap();
        }

        // Single-character pattern (worst case — nhiều candidates)
        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = idx.search_like(b"a", 5).await;
        }
        let elapsed = start.elapsed();
        let avg_ns = elapsed.as_nanos() as f64 / 100.0;

        eprintln!("[bench] search_like single-char: {:.0} ns/call", avg_ns);

        // Two-character pattern
        let start = std::time::Instant::now();
        for _ in 0..100 {
            let _ = idx.search_like(b"ap", 5).await;
        }
        let elapsed = start.elapsed();
        let avg_ns = elapsed.as_nanos() as f64 / 100.0;

        eprintln!("[bench] search_like two-char: {:.0} ns/call", avg_ns);
    }

    #[tokio::test]
    async fn bench_search_like_not_found() {
        let mut idx = SearchIndex::in_memory(4);
        for i in 0..100 {
            let key = format!("store_{i}");
            idx.insert(key.as_bytes(), i, &key).await.unwrap();
        }

        // Pattern không tồn tại — đo tốc độ fail fast
        let start = std::time::Instant::now();
        for _ in 0..50 {
            let _ = idx.search_like(b"zzzzz", 10).await;
        }
        let elapsed = start.elapsed();
        let avg_ns = elapsed.as_nanos() as f64 / 50.0;

        eprintln!("[bench] search_like not-found: {:.0} ns/call", avg_ns);
    }

    #[tokio::test]
    async fn test_search_like_false_negative_case_abaa() {
        let mut idx = SearchIndex::in_memory(4);

        // Chèn chuỗi chứa prefix đặc biệt "abaa"
        // Giả sử RadixTree lưu nguyên cụm này thành 1 node prefix hoặc bị split
        idx.insert(b"abaadata", 1, "Target Node abaa")
            .await
            .unwrap();

        // Tìm kiếm "aa"
        // - Vị trí đầu tiên của 'a' là index 0 -> bắt đầu khớp 'a', gặp 'b' -> FAIL.
        // - Nếu lưu mọi vị trí, shortcut sẽ thử tiếp index 2 (chữ 'a' đầu của cặp "aa") -> SUCCESS.
        let results = idx.search_like(b"aa", 10).await;

        assert!(
            results.is_ok(),
            "False negative! Bản cũ chỉ lưu vị trí 'a' đầu tiên nên không bao giờ quét tới cặp 'aa' phía sau."
        );

        let res = results.unwrap();
        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_search_like_multiple_positions_in_single_prefix() {
        let mut idx = SearchIndex::in_memory(4);

        // Chuỗi có ký tự đầu tiên 'a' lặp lại liên tục ở nhiều cụm khác nhau
        idx.insert(b"xyz_ab_ab_ab", 1, "Repeated Pattern")
            .await
            .unwrap();

        // Tìm kiếm "ab"
        let results = idx.search_like(b"ab", 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_search_like_overlapping_candidates() {
        let mut idx = SearchIndex::in_memory(4);

        // Khớp chồng lấn (Overlapping)
        idx.insert(b"aaaaa", 1, "Five A").await.unwrap();

        // Tìm kiếm "aaa"
        let results = idx.search_like(b"aaa", 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_search_like_split_retains_all_valid_positions() {
        let mut idx = SearchIndex::in_memory(4);

        // Tạo một node dài chứa nhiều ký tự 'a'
        idx.insert(b"test_abaadata_one", 1, "First").await.unwrap();

        // Kích hoạt split tại vị trí "test_" bằng cách chèn key chung prefix
        // Callback OnSplit phải giữ lại chính xác các vị trí tương đối (rel_pos) của 'a' ở node leg phía sau
        idx.insert(b"test_other_route", 2, "Second").await.unwrap();

        // Kiểm tra xem sau khi split, các shortcut 'a' ở leg node vẫn tìm được "aa" hay không
        let results = idx.search_like(b"aa", 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    // ── Edge case: retry từ vị trí mà KMP đã match nhưng không phải start —

    #[tokio::test]
    async fn test_retry_from_within_kmp_matched_bytes() {
        // pattern "aab", key "aaab".
        // KMP từ data_pos=0: match 'a'=p0, 'a'=p1, fail 'a'≠'b' (p2).
        // new_data_pos=2. data_pos+1=1 → 'a' ở 1 → start tại 1 → FOUND.
        let mut idx = SearchIndex::in_memory(4);
        idx.insert(b"aaabyz", 1, "Target").await.unwrap();
        let results = idx.search_like(b"aab", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1);
    }

    #[tokio::test]
    async fn test_retry_cascade_across_dfs_boundary() {
        // pattern "abc", keys: "xaa" + "xaabcde"
        // Tree: Node_A "xaa", Child "bcde"
        // shortcut['a'] = {A}. 'a' ở A[1] và A[2].
        // - data_pos=1: KMP keep=true, DFS không match vì 'b' ở child → Vec::new()
        // - data_pos=2: KMP keep=true, DFS match vì 'b' ở child → FOUND
        // Retry loop KHÔNG return ngay nếu data_pos=1 rỗng → thử data_pos=2 → OK.
        let mut idx = SearchIndex::in_memory(4);
        idx.insert(b"xaa", 1, "First").await.unwrap();
        idx.insert(b"xaabcde", 2, "Target").await.unwrap();

        let results = idx.search_like(b"abc", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 2);
    }

    #[tokio::test]
    async fn test_retry_cascade_all_empty() {
        // pattern "abc" nhưng KHÔNG có trong tree → retry hết mọi vị trí đều rỗng
        let mut idx = SearchIndex::in_memory(4);
        idx.insert(b"xaa", 1, "First").await.unwrap();
        idx.insert(b"xaaxyzw", 2, "Other").await.unwrap();

        let result = idx.search_like(b"abc", 10).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_span_three_nodes() {
        // Tree: "ab" + "cde" + "f"  (keys "abcde" + "abcdef")
        // Insert "ab" → Node_A = "ab"
        // Insert "abcde" → split A: "ab" + "cde"
        // Insert "abcdef" → thêm child "f" dưới "cde"
        // Pattern "bcdef" trải A + B + C
        let mut idx = SearchIndex::in_memory(4);
        idx.insert(b"ab", 1, "AB").await.unwrap();
        idx.insert(b"abcde", 2, "ABCDE").await.unwrap();
        idx.insert(b"abcdef", 3, "ABCDEF").await.unwrap();

        let results = idx.search_like(b"bcdef", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 3);
    }

    #[tokio::test]
    async fn test_partial_match_exhausts_prefix_then_child() {
        // Prefix đủ dài tính toán (do_recursive=false) nhưng KMP match hết prefix
        // cần tiếp tục ở child
        // Tree như test 3 node ở trên
        let mut idx = SearchIndex::in_memory(4);
        idx.insert(b"ab", 1, "AB").await.unwrap();
        idx.insert(b"abcde", 2, "ABCDE").await.unwrap();
        idx.insert(b"abcdef", 3, "ABCDEF").await.unwrap();

        // "cdef" bắt đầu từ vị trí 2 ở A, match 'c','d','e' hết prefix B,
        // cần 'f' ở C
        let results = idx.search_like(b"cdef", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 3);
    }
}
