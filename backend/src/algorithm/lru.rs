use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

use log::debug;

struct Node<K, V> {
    key: K,
    value: V,
    next: usize,
    prev: usize,
}

pub struct LruCache<K, V> {
    mapping: Arc<RwLock<HashMap<K, usize>>>,
    caching: Arc<RwLock<Vec<Node<K, V>>>>,
    capacity: usize,
    first: Arc<RwLock<usize>>,
    last: Arc<RwLock<usize>>,
    on_removing: Option<Arc<dyn Fn(K, V) + Send + Sync>>,
    on_updating: Option<Arc<dyn Fn(K, V) + Send + Sync>>,
    on_inserting: Option<Arc<dyn Fn(K, V) + Send + Sync>>,
}

impl<K: Clone + Hash + Eq + Ord + Debug + Send + Sync, V: Debug + Clone + Send + Sync>
    LruCache<K, V>
{
    pub fn new(capacity: usize) -> Self {
        Self {
            mapping: Arc::new(RwLock::new(HashMap::new())),
            caching: Arc::new(RwLock::new(Vec::new())),
            first: Arc::new(RwLock::new(0)),
            last: Arc::new(RwLock::new(0)),
            capacity,
            on_removing: None,
            on_updating: None,
            on_inserting: None,
        }
    }

    pub fn put(&self, key: K, value: V) -> bool {
        let mapping_read = self.mapping.read().unwrap();
        if let Some(&index) = mapping_read.get(&key) {
            drop(mapping_read); // Release read lock early
            if let Some(callback) = &self.on_updating {
                let key = key.clone();
                let value = value.clone();
                callback(key, value);
            }
            self.caching.write().unwrap()[index].value = value;
            self.move_to_front(index);
        } else {
            drop(mapping_read); // Release read lock
            if self.capacity == 0 {
                return false;
            }

            let mut caching = self.caching.write().unwrap();
            if caching.len() >= self.capacity {
                debug!(
                    "Flush least used item duo to caching({}) > capacity({})",
                    caching.len(),
                    self.capacity
                );

                let first = *self.first.read().unwrap();
                if let Some(callback) = &self.on_removing {
                    let key = caching[first].key.clone();
                    let value = caching[first].value.clone();
                    callback(key, value);
                }

                let mut mapping = self.mapping.write().unwrap();
                for (_, value) in mapping.iter_mut() {
                    if *value > first {
                        let origin = *value;
                        let prev = caching[origin].prev;
                        let next = caching[origin].next;
                        *value -= 1;

                        if *self.last.read().unwrap() == origin {
                            *self.last.write().unwrap() = *value;
                        }

                        caching[prev].next = *value;
                        caching[next].prev = *value;
                    }
                }

                *self.first.write().unwrap() = caching[first].next;
                mapping.remove(&caching[first].key);
                caching.remove(first);
            }

            let index = caching.len();
            let last = *self.last.read().unwrap();
            let mut next = 0;

            if caching.is_empty() {
                *self.first.write().unwrap() = index;
                *self.last.write().unwrap() = index;
                next = index;
            } else {
                caching[last].next = index;
                *self.last.write().unwrap() = index;
            }

            if let Some(callback) = &self.on_inserting {
                let key = key.clone();
                let value = value.clone();
                callback(key, value);
            }

            caching.push(Node {
                key: key.clone(),
                prev: last,
                value,
                next,
            });
            self.mapping.write().unwrap().insert(key, index);
        }
        true
    }

    pub fn get(&self, key: &K) -> Option<V> {
        if let Some(&index) = self.mapping.read().unwrap().get(key) {
            self.move_to_front(index);
            Some(self.caching.read().unwrap()[index].value.clone())
        } else {
            None
        }
    }

    pub fn set_on_removing_callback<F>(&mut self, callback: F)
    where
        F: Fn(K, V) + Send + Sync + 'static,
    {
        self.on_removing = Some(Arc::new(callback));
    }

    pub fn set_on_inserting_callback<F>(&mut self, callback: F)
    where
        F: Fn(K, V) + Send + Sync + 'static,
    {
        self.on_inserting = Some(Arc::new(callback));
    }

    pub fn set_on_updating_callback<F>(&mut self, callback: F)
    where
        F: Fn(K, V) + Send + Sync + 'static,
    {
        self.on_updating = Some(Arc::new(callback));
    }

    fn move_to_front(&self, index: usize) {
        let mut caching = self.caching.write().unwrap();
        let next = caching[index].next;
        let prev = caching[index].prev;

        caching[next].prev = prev;
        caching[prev].next = next;

        caching[index].prev = *self.last.read().unwrap();
        caching[*self.last.read().unwrap()].next = index;
        *self.last.write().unwrap() = index;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_cache() {
        let mut cache = LruCache::new(2);

        cache.put(1, 10);
        cache.put(2, 20);

        assert_eq!(cache.get(&1), Some(&10)); // Access 1, making it most recent
        assert_eq!(cache.get(&2), Some(&20));

        cache.put(3, 30); // Cache full, evicts 1 (LRU)

        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some(&20)); // Access 2
        assert_eq!(cache.get(&3), Some(&30));

        cache.put(4, 40);

        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), None); // 2 is evicted
        assert_eq!(cache.get(&3), Some(&30)); // Access 3
        assert_eq!(cache.get(&4), Some(&40));

        cache.put(2, 20); // put 2 again, 3 is evicted

        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&3), None); // Access 3 fails
        assert_eq!(cache.get(&4), Some(&40));
        assert_eq!(cache.get(&2), Some(&20)); // Access 2

        cache.put(5, 50); // Cache is full, should evict 3 because it's LRU after 4 and 2

        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&3), None); // 3 is evicted
        assert_eq!(cache.get(&4), None); // 4 is evicted
        assert_eq!(cache.get(&2), Some(&20));
        assert_eq!(cache.get(&5), Some(&50));

        cache.put(1, 10); // Cache full, will evict 2
        assert_eq!(cache.get(&4), None); // 4 gets evicted.

        assert_eq!(cache.get(&1), Some(&10));
        assert_eq!(cache.get(&2), None); // 2 is evicted
        assert_eq!(cache.get(&5), Some(&50));
    }

    #[test]
    fn test_empty_cache() {
        let mut cache = LruCache::new(0);
        cache.put(1, 10);
        assert_eq!(cache.get(&1), None);
    }

    #[test]
    fn update_existing_key() {
        let mut cache = LruCache::new(2);
        cache.put(1, 10);
        cache.put(1, 20);
        assert_eq!(cache.get(&1), Some(&20));
        assert_eq!(cache.mapping.len(), 1); // Check that we still only have one entry for key 1
    }
}
