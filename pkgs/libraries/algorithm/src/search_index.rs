//! Search module — KMP + DFS substring ("LIKE") search trên RadixTree + Storage.
//!
//! ## Idea
//! Duy trì **shortcuts** (in-memory map) giúp tìm nhanh các node có chứa ký tự
//! đầu tiên của pattern. Với mỗi candidate, chạy **KMP** matching trên prefix
//! của node; nếu prefix ngắn hơn pattern thì **DFS** xuống children.
//!
//! ## Shortcut structure
//! ```text
//! shortcuts[shard][byte][node_id] = first_position
//! ```
//! - `shard` — shard index (0..sharding)
//! - `byte` — ký tự đầu tiên của pattern
//! - `node_id` — node trong RadixTree
//! - `first_position` — vị trí đầu tiên của `byte` trong prefix của node
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

// ==================== Constants ====================

const INF: usize = usize::MAX;

// ==================== Shortcut Data ====================

/// shortcuts[shard][byte][node_id] = first_position_of_byte_in_node_prefix
type ShortcutData = Vec<HashMap<u8, HashMap<usize, usize>>>;

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
                .map(|_| HashMap::<u8, HashMap<usize, usize>>::new())
                .collect::<Vec<_>>(),
        ));

        let mut tree = RadixTree::new(sharding, storage);

        // Register split callback
        // 1. Chuyển entries của parent (tại vị trí ≥ breakpoint) → leg
        // 2. Thêm shortcuts cho từng byte trong leg prefix
        let cb_shortcuts = shortcuts.clone();
        tree.with_callback(Arc::new(
            move |parent_id, leg_id, old_prefix, breakpoint| {
                let mut sc = match cb_shortcuts.lock() {
                    Ok(s) => s,
                    Err(_) => return Err(radixtree::RadixError::Callback),
                };

                let sharding = sc.len();

                for (pos, byte) in old_prefix.iter().enumerate().skip(breakpoint) {
                    let si = radixtree::shard_of(*byte, sharding);
                    let rel_pos = pos - breakpoint;

                    let byte_map = sc[si].entry(*byte).or_default();

                    // Remove parent entry nếu nó ở vị trí ≥ breakpoint (stale)
                    if let Some(&old_pos) = byte_map.get(&parent_id)
                        && old_pos >= breakpoint
                    {
                        byte_map.remove(&parent_id);
                    }

                    // Luôn thêm entry cho leg với vị trí tương đối
                    let entry = byte_map.entry(leg_id).or_insert(INF);
                    if rel_pos < *entry {
                        *entry = rel_pos;
                    }

                    if byte_map.is_empty() {
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

    /// Cập nhật shortcuts cho một node mới.
    fn update_shortcuts(&self, key: &[u8], breakpoint: usize, node_id: usize) {
        if let Ok(mut shortcuts) = self.shortcuts.lock() {
            let sharding = shortcuts.len();
            for (pos, byte) in key.iter().enumerate().skip(breakpoint) {
                let si = radixtree::shard_of(*byte, sharding);
                let byte_map = shortcuts[si].entry(*byte).or_default();
                let entry = byte_map.entry(node_id).or_insert(INF);
                let rel_pos = pos - breakpoint;

                if rel_pos < *entry {
                    *entry = rel_pos;
                }
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
        let candidates: Vec<(usize, usize)> = {
            let shortcuts = self
                .shortcuts
                .lock()
                .map_err(|e| SearchError::Storage(e.to_string()))?;

            shortcuts[si]
                .get(&first_byte)
                .map(|byte_map| {
                    let mut cands: Vec<(usize, usize)> =
                        byte_map.iter().map(|(&nid, &pos)| (nid, pos)).collect();
                    cands.sort_by_key(|&(_, pos)| pos);
                    cands
                })
                .unwrap_or_default()
        };

        let mut results = Vec::new();
        let mut seen = HashSet::new();

        for &(node_id, first_pos) in &candidates {
            if results.len() >= limit {
                break;
            }

            let found = self
                .dfs_search(node_id, pattern, &lps, 0, first_pos, limit)
                .await?;

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

        let (found, keep, _new_data_pos, new_pattern_pos) =
            Self::kmp_match(pattern, &prefix, lps, pattern_pos, data_pos, do_recursive);

        if found {
            // Match hoàn chỉnh → collect toàn bộ records trong subtree
            let record_ids = self.collect_subtree_records(node_id).await?;
            return Ok(self.resolve_records(&record_ids, limit));
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
            idx.insert(format!("item_{i}").as_bytes(), i as i32, &name)
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
            idx.insert(key.as_bytes(), i as i32, &key).await.unwrap();
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
}
