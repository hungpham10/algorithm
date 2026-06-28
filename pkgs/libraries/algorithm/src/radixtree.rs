use std::sync::Arc;
use thiserror::Error;

use crate::storage::{self, Storage};

pub const EMPTY: usize = 0;

#[derive(Debug, Error)]
pub enum RadixError {
    #[error("index must not be zero or negative")]
    InvalidIndex,
    #[error("branch id {0} out of range")]
    BranchOutOfRange(usize),
    #[error("key not found")]
    NotFound,
    #[error("storage error: {0}")]
    Storage(String),
    #[error("callback error")]
    Callback,
}

impl From<storage::StorageError> for RadixError {
    fn from(e: storage::StorageError) -> Self {
        RadixError::Storage(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, RadixError>;

pub type OnSplitCallback = Arc<dyn Fn(usize, usize, &[u8], usize) -> Result<()> + Send + Sync>;

pub struct RadixTree {
    endpoints: Vec<usize>,
    sharding: usize,
    storage: Box<dyn Storage>,
    on_split: Option<OnSplitCallback>,
}

pub(crate) fn shard_of(byte: u8, sharding: usize) -> usize {
    ((byte as usize).wrapping_sub(b'a' as usize)) % sharding
}

impl RadixTree {
    pub fn new<S: Storage + 'static>(sharding: usize, storage: S) -> Self {
        Self {
            endpoints: vec![EMPTY; sharding.max(1)],
            sharding: sharding.max(1),
            storage: Box::new(storage),
            on_split: None,
        }
    }

    pub fn with_callback(&mut self, cb: OnSplitCallback) {
        self.on_split = Some(cb);
    }

    pub async fn insert(&mut self, key: &[u8], index: usize) -> Result<(usize, usize)> {
        if index == EMPTY {
            return Err(RadixError::InvalidIndex);
        }
        if key.is_empty() {
            return Err(RadixError::NotFound);
        }

        let mut tail = 0;
        let mut node_id = self.endpoints[shard_of(key[0], self.sharding)];

        while node_id != EMPTY {
            let mut found = false;
            let (prefix, _) = self.storage.get_node(node_id).await?;
            let common = prefix
                .iter()
                .zip(key[tail..].iter())
                .take_while(|(a, b)| a == b)
                .count();

            if common < prefix.len() {
                let split_off = tail + common;
                let id = self.new_split(node_id, common, &key[split_off..], index).await?;
                return Ok((id, tail));
            }

            tail += common;
            if tail == key.len() {
                return Ok((EMPTY, tail));
            }

            let next_byte = key[tail];
            let children = self.storage.get_children(node_id).await?;
            for &child in &children {
                let (p, _) = self.storage.get_node(child).await?;
                if !p.is_empty() && p[0] == next_byte {
                    node_id = child;
                    found = true;
                    break;
                }
            }
            if !found {
                let id = self.extend(node_id, &key[tail..], index).await?;
                return Ok((id, tail));
            }
        }

        let id = self.storage.new_node(key.to_vec(), index).await?;
        let si = shard_of(key[0], self.sharding);
        self.storage.set_root(si, id).await?;
        self.endpoints[si] = id;
        Ok((id, tail))
    }

    pub async fn r#match(&self, key: &[u8]) -> Result<usize> {
        let mut node_id = self.endpoints[shard_of(key[0], self.sharding)];
        let mut pos = 0;

        while node_id != EMPTY {
            let (prefix, record) = self.storage.get_node(node_id).await?;
            let common = prefix
                .iter()
                .zip(&key[pos..])
                .take_while(|(a, b)| a == b)
                .count();

            if common == prefix.len() {
                pos += common;
                if pos == key.len() {
                    return Ok(record);
                }
                let next_byte = key[pos];
                let children = self.storage.get_children(node_id).await?;
                let mut found_child = None;
                for &c in &children {
                    if let Ok((p, _)) = self.storage.get_node(c).await {
                        if !p.is_empty() && p[0] == next_byte {
                            found_child = Some(c);
                            break;
                        }
                    }
                }
                if let Some(child) = found_child {
                    node_id = child;
                    continue;
                }
            }
            break;
        }
        Err(RadixError::NotFound)
    }

    async fn extend(&mut self, parent: usize, suffix: &[u8], value: usize) -> Result<usize> {
        let id = self.storage.new_node(suffix.to_vec(), value).await?;
        self.storage.add_child(parent, id).await?;
        Ok(id)
    }

    async fn new_split(
        &mut self,
        parent: usize,
        breakpoint: usize,
        suffix: &[u8],
        value: usize,
    ) -> Result<usize> {
        let (old_prefix, old_record) = self.storage.get_node(parent).await?;

        let root_prefix = old_prefix[..breakpoint].to_vec();
        let leg_prefix = old_prefix[breakpoint..].to_vec();

        let new_id = self.storage.new_node(suffix.to_vec(), value).await?;
        let leg_id = self.storage.new_node(leg_prefix, old_record).await?;

        self.storage
            .update_node(parent, Some(root_prefix), Some(EMPTY)).await?;
        self.storage.add_child(parent, leg_id).await?;
        self.storage.add_child(parent, new_id).await?;

        if let Some(cb) = &self.on_split {
            cb(parent, leg_id, &old_prefix, breakpoint)?;
        }

        Ok(new_id)
    }
}

impl RadixTree {
    pub fn in_memory(sharding: usize) -> Self {
        RadixTree::new(sharding, storage::InMemoryStorage::default())
    }

    // ==================== CRATE-INTERNAL HELPERS ====================

    pub(crate) fn sharding_count(&self) -> usize {
        self.sharding
    }

    pub(crate) async fn get_node_prefix(&self, id: usize) -> Result<Vec<u8>> {
        let (p, _) = self.storage.get_node(id).await?;
        Ok(p)
    }

    pub(crate) async fn get_node_record(&self, id: usize) -> Result<usize> {
        let (_, r) = self.storage.get_node(id).await?;
        Ok(r)
    }

    pub(crate) async fn get_children_ids(&self, id: usize) -> Result<Vec<usize>> {
        Ok(self.storage.get_children(id).await?)
    }

    // ==================== PREFIX SEARCH ====================

    /// Tìm tất cả record có key bắt đầu bằng `prefix`.
    ///
    /// Trả về `Vec<(full_key, record)>` – key đầy đủ và giá trị record của từng node lá.
    pub async fn search_prefix(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, usize)>> {
        if prefix.is_empty() {
            return Err(RadixError::NotFound);
        }

        let si = shard_of(prefix[0], self.sharding);
        let mut node_id = self.endpoints[si];
        if node_id == EMPTY {
            return Err(RadixError::NotFound);
        }

        let mut pos = 0;
        let mut path = Vec::new(); // key tích luỹ từ root → node hiện tại

        loop {
            let (node_prefix, _) = self.storage.get_node(node_id).await?;
            let remaining = &prefix[pos..];
            let common = node_prefix
                .iter()
                .zip(remaining.iter())
                .take_while(|(a, b)| a == b)
                .count();

            if common < node_prefix.len() {
                if pos + common == prefix.len() {
                    // Prefix khớp một phần node_prefix – collect từ node này
                    // full key: path + toàn bộ node_prefix
                    path.extend_from_slice(&node_prefix);
                    let mut results = Vec::new();
                    self.collect_records_from(node_id, path, &mut results).await?;
                    return Ok(results);
                }
                // Node_prefix khác với prefix – không match
                break;
            }

            // Khớp toàn bộ node_prefix
            pos += common;
            path.extend_from_slice(&node_prefix);

            if pos == prefix.len() {
                // Đã match hết prefix – collect từ node này trở xuống
                let mut results = Vec::new();
                self.collect_records_from(node_id, path, &mut results).await?;
                return Ok(results);
            }

            // Đi tiếp xuống child phù hợp
            let next_byte = prefix[pos];
            let children = self.storage.get_children(node_id).await?;
            let mut found = false;
            for &child in &children {
                let (cp, _) = self.storage.get_node(child).await?;
                if !cp.is_empty() && cp[0] == next_byte {
                    node_id = child;
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }

        Err(RadixError::NotFound)
    }

    /// Duyệt toàn bộ subtree từ `node_id`, thu thập tất cả record.
    /// `key_sofar` là key đầy đủ tính đến node này (đã gồm prefix của node này).
    async fn collect_records_from(
        &self,
        node_id: usize,
        key_sofar: Vec<u8>,
        results: &mut Vec<(Vec<u8>, usize)>,
    ) -> Result<()> {
        let (_, record) = self.storage.get_node(node_id).await?;

        if record != EMPTY {
            results.push((key_sofar.clone(), record));
        }

        let children = self.storage.get_children(node_id).await?;
        for &child in &children {
            let (child_prefix, _) = self.storage.get_node(child).await?;
            let mut child_key = key_sofar.clone();
            child_key.extend_from_slice(&child_prefix);
            Box::pin(self.collect_records_from(child, child_key, results)).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insert_and_match() {
        let mut tree = RadixTree::in_memory(4);
        assert!(tree.insert(b"hello", 1).await.is_ok());
        assert!(tree.insert(b"world", 2).await.is_ok());
        assert!(tree.insert(b"help", 3).await.is_ok());

        assert_eq!(tree.r#match(b"hello").await.unwrap(), 1);
        assert_eq!(tree.r#match(b"world").await.unwrap(), 2);
        assert_eq!(tree.r#match(b"help").await.unwrap(), 3);
        assert!(tree.r#match(b"notfound").await.is_err());
    }

    #[tokio::test]
    async fn test_insert_empty_key() {
        let mut tree = RadixTree::in_memory(1);
        assert!(tree.insert(b"", 1).await.is_err());
    }

    #[tokio::test]
    async fn test_insert_zero_index() {
        let mut tree = RadixTree::in_memory(1);
        assert!(tree.insert(b"key", 0).await.is_err());
    }

    #[tokio::test]
    async fn test_match_empty_tree() {
        let tree = RadixTree::in_memory(2);
        assert!(tree.r#match(b"anything").await.is_err());
    }

    #[tokio::test]
    async fn test_search_prefix_exact() {
        let mut tree = RadixTree::in_memory(4);
        tree.insert(b"hello", 1).await.unwrap();
        tree.insert(b"help", 2).await.unwrap();
        tree.insert(b"world", 3).await.unwrap();

        let results = tree.search_prefix(b"he").await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains(&(b"hello".to_vec(), 1)));
        assert!(results.contains(&(b"help".to_vec(), 2)));
    }

    #[tokio::test]
    async fn test_search_prefix_partial() {
        let mut tree = RadixTree::in_memory(4);
        tree.insert(b"hello", 1).await.unwrap();
        tree.insert(b"help", 2).await.unwrap();
        tree.insert(b"held", 3).await.unwrap();

        let results = tree.search_prefix(b"hel").await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_search_prefix_full_key() {
        let mut tree = RadixTree::in_memory(4);
        tree.insert(b"hello", 42).await.unwrap();

        let results = tree.search_prefix(b"hello").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], (b"hello".to_vec(), 42));
    }

    #[tokio::test]
    async fn test_search_prefix_not_found() {
        let mut tree = RadixTree::in_memory(4);
        tree.insert(b"hello", 1).await.unwrap();

        assert!(tree.search_prefix(b"xyz").await.is_err());
    }

    #[tokio::test]
    async fn test_search_prefix_empty_input() {
        let tree = RadixTree::in_memory(4);
        assert!(tree.search_prefix(b"").await.is_err());
    }

    #[tokio::test]
    async fn test_search_prefix_single_result() {
        let mut tree = RadixTree::in_memory(2);
        tree.insert(b"tiem vang", 1).await.unwrap();
        tree.insert(b"tiem bac", 2).await.unwrap();

        let results = tree.search_prefix(b"tiem v").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, 1);
    }

    #[tokio::test]
    async fn test_search_prefix_empty_tree() {
        let tree = RadixTree::in_memory(2);
        assert!(tree.search_prefix(b"anything").await.is_err());
    }
}
