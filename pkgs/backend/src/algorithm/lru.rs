use log::{error, info};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, RwLock};

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
            println!("{} {}", caching.len(), self.capacity);
            if caching.len() >= self.capacity {
                let first = *self.first.read().unwrap();

                if let Some(callback) = &self.on_removing {
                    let key = caching[first].key.clone();
                    let value = caching[first].value.clone();
                    callback(key, value);
                }

                // Update self.first before removing
                *self.first.write().unwrap() = caching[first].next;

                // Remove from mapping
                let mut mapping = self.mapping.write().unwrap();
                mapping.remove(&caching[first].key);

                // Update indices in mapping
                for (_, value) in mapping.iter_mut() {
                    if *value > first {
                        *value -= 1;
                    }
                }

                // Update self.last if necessary
                let mut last = self.last.write().unwrap();
                if *last > first {
                    info!("update last to reduce to remove first");
                    *last -= 1;
                } else if *last == first {
                    info!("update last when last equal to first");
                    *last = if caching.len() > 1 {
                        caching[first].prev
                    } else {
                        0
                    };
                } else {
                    error!("cannot update last");
                }

                // Update next and prev links
                if caching.len() > 1 {
                    let next = caching[first].next;
                    let prev = caching[first].prev;

                    caching[next].prev = prev;
                    caching[prev].next = next;
                }

                // Remove the element
                caching.remove(first);

                // If cache is empty, reset first and last
                if caching.is_empty() {
                    *self.first.write().unwrap() = 0;
                    *self.last.write().unwrap() = 0;
                }
            }

            let index = caching.len();
            let last = *self.last.read().unwrap();
            let mut next = 0;
            if caching.is_empty() {
                *self.first.write().unwrap() = index;
                *self.last.write().unwrap() = index;
                next = index;
            } else if last < caching.len() {
                caching[last].next = index;
                *self.last.write().unwrap() = index;
            } else {
                return false;
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
        let first = *self.first.read().unwrap();
        let last = *self.last.read().unwrap();

        if index == first {
            return; // Already at front
        }

        if caching.len() <= 1 {
            return; // Only one element, no need to move
        }

        // Remove the node from its current position
        let next = caching[index].next;
        let prev = caching[index].prev;
        caching[prev].next = next;
        caching[next].prev = prev;

        // Place the node at the front
        caching[index].next = first;
        caching[index].prev = index; // Self-loop for prev of the front node
        caching[first].prev = index;
        *self.first.write().unwrap() = index;

        // Update last if the moved node was the last one
        if index == last {
            *self.last.write().unwrap() = prev;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_lru_cache() {
        let mut cache = LruCache::new(2);
        cache.put(1, 10);
        cache.put(2, 20);
        assert_eq!(cache.get(&1), Some(10)); // Access 1, making it most recent
        assert_eq!(cache.get(&2), Some(20));
        cache.put(3, 30); // Cache full, evicts 1 (LRU)
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some(20)); // Access 2
        assert_eq!(cache.get(&3), Some(30));
        cache.put(4, 40);
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), None); // 2 is evicted
        assert_eq!(cache.get(&3), Some(30)); // Access 3
        assert_eq!(cache.get(&4), Some(40));
        cache.put(2, 20); // put 2 again, 3 is evicted
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&3), None); // Access 3 fails
        assert_eq!(cache.get(&4), Some(40));
        assert_eq!(cache.get(&2), Some(20)); // Access 2
        cache.put(5, 50); // Cache is full, should evict 4
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&3), None); // 3 is evicted
        assert_eq!(cache.get(&4), None); // 4 is evicted
        assert_eq!(cache.get(&2), Some(20));
        assert_eq!(cache.get(&5), Some(50));
        cache.put(1, 10); // Cache full, will evict 2
        assert_eq!(cache.get(&4), None); // 4 gets evicted
        assert_eq!(cache.get(&1), Some(10));
        assert_eq!(cache.get(&2), None); // 2 is evicted
        assert_eq!(cache.get(&5), Some(50));
    }

    #[test]
    fn test_empty_cache() {
        let mut cache = LruCache::new(0);
        cache.put(1, 10);
        assert_eq!(cache.get(&1), None);
    }

    #[test]
    fn test_update_existing_key() {
        let mut cache = LruCache::new(2);
        cache.put(1, 10);
        cache.put(1, 20);
        assert_eq!(cache.get(&1), Some(20));
        assert_eq!(cache.mapping.read().unwrap().len(), 1); // Check that we still only have one entry for key 1
    }

    #[test]
    fn test_lru_cache_single_element() {
        let mut cache = LruCache::new(1);
        cache.put(1, 10);
        assert_eq!(cache.get(&1), Some(10));
        cache.put(2, 20); // Replace the only element
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some(20));
    }

    #[test]
    fn test_lru_cache_remove_last() {
        let mut cache = LruCache::new(2);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30); // Evict 1, check self.last
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some(20));
        assert_eq!(cache.get(&3), Some(30));
    }

    #[test]
    fn test_lru_cache_concurrent() {
        let cache = Arc::new(LruCache::new(2));
        let mut handles = vec![];

        // Spawn multiple threads to put and get concurrently
        for i in 0..10 {
            let cache = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                cache.put(i, i * 10);
                cache.get(&i);
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify cache state
        let mapping = cache.mapping.read().unwrap();
        assert!(mapping.len() <= 2, "Cache should not exceed capacity");
    }
}
