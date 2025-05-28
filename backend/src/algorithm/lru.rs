use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

struct Node<K, V> {
    key: K,
    value: V,
    next: usize,
    prev: usize,
}

pub struct LruCache<K, V> {
    mapping: HashMap<K, usize>,
    caching: Vec<Node<K, V>>,
    capacity: usize,
    first: usize,
    last: usize,

    // @NOTE: callbacks
    on_removing: Option<Box<dyn Fn(K, V)>>,
    on_updating: Option<Box<dyn Fn(K, V)>>,
    on_inserting: Option<Box<dyn Fn(K, V)>>,
}

impl<K: Clone + Hash + Eq + Debug, V: Debug + Clone> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            mapping: HashMap::new(),
            caching: Vec::new(),
            capacity,
            first: 0,
            last: 0,

            // @NOTE: callbacks
            on_removing: None,
            on_updating: None,
            on_inserting: None,
        }
    }

    pub fn put(&mut self, key: K, value: V) -> bool {
        if let Some(&index) = self.mapping.get(&key) {
            if let Some(callback) = &self.on_updating {
                let key = key.clone();
                let value = value.clone();

                callback(key, value);
            }
            self.caching[index].value = value;

            self.move_to_front(index);
        } else {
            if self.capacity == 0 {
                return false;
            }

            if self.caching.len() >= self.capacity {
                let first = self.first;

                if let Some(callback) = &self.on_removing {
                    let key = self.caching[first].key.clone();
                    let value = self.caching[first].value.clone();

                    callback(key, value);
                }

                // @NOTE: remove first node will lead to update mapping and
                //         relink nodes
                for (_, value) in self.mapping.iter_mut() {
                    if *value > first {
                        let origin = *value;
                        let prev = self.caching[origin].prev;
                        let next = self.caching[origin].next;

                        // @NOTE: reupdate links
                        *value -= 1;

                        // @NOTE: update new last
                        if self.last == origin {
                            self.last = *value;
                        }

                        // @NOTE: relink nodes
                        self.caching[prev].next = *value;
                        self.caching[next].prev = *value;
                    }
                }

                self.first = self.caching[first].next;
                self.mapping.remove(&self.caching[first].key);
                self.caching.remove(first);
            }

            let index = self.caching.len();
            let last = self.last;
            let mut next = 0;

            if self.caching.is_empty() {
                self.first = index;
                self.last = index;
                next = index;
            } else {
                self.caching[last].next = index;
                self.last = index;
            }

            if let Some(callback) = &self.on_inserting {
                let key = key.clone();
                let value = value.clone();

                callback(key, value);
            }

            self.caching.push(Node {
                key: key.clone(),
                prev: last,
                value,
                next,
            });
            self.mapping.insert(key, index);
        }

        true
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(&index) = self.mapping.get(key) {
            self.move_to_front(index);
            Some(&self.caching[index].value)
        } else {
            None
        }
    }

    pub fn set_on_removing_callback<F>(&mut self, callback: F)
    where
        F: Fn(K, V) + 'static,
    {
        self.on_removing = Some(Box::new(callback));
    }

    pub fn set_on_inserting_callback<F>(&mut self, callback: F)
    where
        F: Fn(K, V) + 'static,
    {
        self.on_inserting = Some(Box::new(callback));
    }

    pub fn set_on_updating_callback<F>(&mut self, callback: F)
    where
        F: Fn(K, V) + 'static,
    {
        self.on_updating = Some(Box::new(callback));
    }

    fn move_to_front(&mut self, index: usize) {
        let next = self.caching[index].next;
        let prev = self.caching[index].prev;

        // @NOTE: remove node at `index`
        self.caching[next].prev = prev;
        self.caching[prev].next = next;

        // @NOTE: put node currently at `index` to `last`
        self.caching[index].prev = self.last;
        self.caching[self.last].next = index;
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
