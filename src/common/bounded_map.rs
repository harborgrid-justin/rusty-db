// # Bounded HashMap with LRU Eviction
//
// This module provides a capacity-bounded HashMap that automatically evicts
// least-recently-used entries when the capacity is exceeded.
//
// ## Purpose
//
// Many specialized engines use unbounded HashMaps that can lead to memory
// exhaustion. This module addresses that issue by providing a bounded
// alternative with configurable capacity and eviction policies.
//
// ## Usage
//
// ```rust
// use rusty_db::common::BoundedHashMap;
//
// let mut map = BoundedHashMap::new(1000); // Max 1000 entries
// map.insert(key, value);
// ```

use std::collections::HashMap;
use std::hash::Hash;

/// A HashMap with a maximum capacity that evicts least-recently-used entries
/// when the capacity is exceeded.
///
/// # Examples
///
/// ```
/// use rusty_db::common::BoundedHashMap;
///
/// let mut map = BoundedHashMap::new(3);
/// map.insert("a", 1);
/// map.insert("b", 2);
/// map.insert("c", 3);
/// map.insert("d", 4); // This will evict "a" (least recently used)
/// assert!(map.get(&"a").is_none());
/// ```
pub struct BoundedHashMap<K: Eq + Hash + Clone, V> {
    map: HashMap<K, V>,
    access_order: Vec<K>,
    capacity: usize,
}

impl<K: Eq + Hash + Clone, V> BoundedHashMap<K, V> {
    /// Creates a new BoundedHashMap with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of entries before eviction occurs
    ///
    /// # Panics
    ///
    /// Panics if capacity is 0.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "BoundedHashMap capacity must be > 0");
        Self {
            map: HashMap::with_capacity(capacity),
            access_order: Vec::with_capacity(capacity),
            capacity,
        }
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the key already exists, the value is updated and the key is moved
    /// to the most recently used position.
    ///
    /// If inserting would exceed capacity, the least recently used entry is evicted.
    ///
    /// Returns the old value if the key already existed.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // Update access order
        self.access_order.retain(|k| k != &key);
        self.access_order.push(key.clone());

        // Evict if at capacity and key is new
        if self.map.len() >= self.capacity && !self.map.contains_key(&key) {
            if let Some(lru_key) = self.access_order.first().cloned() {
                self.access_order.remove(0);
                self.map.remove(&lru_key);
            }
        }

        self.map.insert(key, value)
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// Updates the access order to mark this key as recently used.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            // Update access order
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.clone());
            self.map.get(key)
        } else {
            None
        }
    }

    /// Returns a reference to the value without updating access order.
    ///
    /// Use this for "peek" operations that shouldn't affect eviction.
    pub fn get_no_update(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    /// Removes a key from the map, returning the value if it existed.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.access_order.retain(|k| k != key);
        self.map.remove(key)
    }

    /// Returns true if the map contains the specified key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// Returns the number of entries in the map.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns true if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns the maximum capacity of the map.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clears all entries from the map.
    pub fn clear(&mut self) {
        self.map.clear();
        self.access_order.clear();
    }

    /// Returns an iterator over the entries in the map.
    ///
    /// Note: Iteration does not update access order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.map.iter()
    }

    /// Returns an iterator over the keys in the map.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.map.keys()
    }

    /// Returns an iterator over the values in the map.
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.map.values()
    }
}

impl<K: Eq + Hash + Clone, V> Default for BoundedHashMap<K, V> {
    /// Creates a BoundedHashMap with a default capacity of 10,000 entries.
    fn default() -> Self {
        Self::new(10_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut map = BoundedHashMap::new(3);
        assert!(map.is_empty());

        map.insert("a", 1);
        map.insert("b", 2);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&"a"), Some(&1));
    }

    #[test]
    fn test_eviction() {
        let mut map = BoundedHashMap::new(3);
        map.insert("a", 1);
        map.insert("b", 2);
        map.insert("c", 3);

        // This should evict "a" (least recently used)
        map.insert("d", 4);

        assert_eq!(map.len(), 3);
        assert!(map.get(&"a").is_none());
        assert!(map.get(&"b").is_some());
        assert!(map.get(&"c").is_some());
        assert!(map.get(&"d").is_some());
    }

    #[test]
    fn test_access_order_update() {
        let mut map = BoundedHashMap::new(3);
        map.insert("a", 1);
        map.insert("b", 2);
        map.insert("c", 3);

        // Access "a" to make it recently used
        map.get(&"a");

        // This should now evict "b" (least recently used)
        map.insert("d", 4);

        assert!(map.get(&"a").is_some());
        assert!(map.get(&"b").is_none());
        assert!(map.get(&"c").is_some());
        assert!(map.get(&"d").is_some());
    }

    #[test]
    #[should_panic(expected = "BoundedHashMap capacity must be > 0")]
    fn test_zero_capacity() {
        let _ = BoundedHashMap::<String, i32>::new(0);
    }
}
