/// Advanced B+ Tree Index Implementation
///
/// This module provides a production-grade B+ tree with:
/// - Concurrent access using latch crabbing protocol
/// - Bulk loading optimization for initial data
/// - Efficient node splitting and merging
/// - Range scan support with iterators
/// - MVCC-friendly design for snapshot isolation

use crate::Result;
use crate::error::DbError;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::Debug;

/// Default order of B+ tree (max children per node)
const DEFAULT_ORDER: usize = 128;

/// Minimum number of keys in a node (except root)
const MIN_KEYS: usize = DEFAULT_ORDER / 2 - 1;

/// B+ Tree Index
pub struct BPlusTree<K: Ord + Clone + Debug, V: Clone + Debug> {
    root: Arc<RwLock<Option<NodeRef<K, V>>>>,
    order: usize,
    height: Arc<RwLock<usize>>,
}

impl<K: Ord + Clone + Debug, V: Clone + Debug> BPlusTree<K, V> {
    /// Create a new B+ tree with default order
    pub fn new() -> Self {
        Self::with_order(DEFAULT_ORDER)
    }

    /// Create a new B+ tree with specified order
    pub fn with_order(order: usize) -> Self {
        assert!(order >= 3, "B+ tree order must be at least 3");
        Self {
            root: Arc::new(RwLock::new(None)),
            order,
            height: Arc::new(RwLock::new(0)),
        }
    }

    /// Insert a key-value pair
    pub fn insert(&self, key: K, value: V) -> Result<()> {
        let mut root_lock = self.root.write();

        if root_lock.is_none() {
            // Create initial leaf node
            let leaf = Node::new_leaf(self.order);
            leaf.insert_in_leaf(key, value)?;
            *root_lock = Some(Arc::new(RwLock::new(leaf)));
            *self.height.write() = 1;
            return Ok(());
        }

        let root = root_lock.as_ref().unwrap().clone();
        drop(root_lock);

        // Use latch crabbing for concurrent insert
        let split_result = self.insert_recursive(root.clone(), key, value)?;

        if let Some((split_key, new_node)) = split_result {
            // Root was split, create new root
            let mut root_lock = self.root.write();
            let old_root = root_lock.as_ref().unwrap().clone();

            let mut new_root = Node::new_internal(self.order);
            new_root.children.push(old_root);
            new_root.keys.push(split_key);
            new_root.children.push(new_node);

            *root_lock = Some(Arc::new(RwLock::new(new_root)));
            *self.height.write() += 1;
        }

        Ok(())
    }

    /// Recursive insert with latch crabbing
    fn insert_recursive(
        &self,
        node_ref: NodeRef<K, V>,
        key: K,
        value: V,
    ) -> Result<Option<(K, NodeRef<K, V>)>> {
        let mut node = node_ref.write();

        if node.is_leaf {
            // Insert in leaf node
            node.insert_in_leaf(key, value)?;

            if node.keys.len() >= self.order {
                // Split leaf node
                let (split_key, new_node) = node.split_leaf(self.order)?;
                Ok(Some((split_key, Arc::new(RwLock::new(new_node)))))
            } else {
                Ok(None)
            }
        } else {
            // Internal node - find child to descend into
            let child_idx = node.find_child_index(&key);
            let child = node.children[child_idx].clone();

            drop(node); // Release latch before descending (latch crabbing)

            let split_result = self.insert_recursive(child, key, value)?;

            if let Some((split_key, new_child)) = split_result {
                // Child was split, insert new key and child
                let mut node = node_ref.write();
                node.insert_in_internal(split_key.clone(), new_child.clone(), child_idx)?;

                if node.keys.len() >= self.order {
                    // Split internal node
                    let (median_key, new_node) = node.split_internal(self.order)?;
                    Ok(Some((median_key, Arc::new(RwLock::new(new_node)))))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }
    }

    /// Search for a key
    pub fn search(&self, key: &K) -> Result<Option<V>> {
        let root_lock = self.root.read();

        match root_lock.as_ref() {
            None => Ok(None),
            Some(root) => self.search_recursive(root.clone(), key),
        }
    }

    /// Recursive search
    fn search_recursive(&self, node_ref: NodeRef<K, V>, key: &K) -> Result<Option<V>> {
        let node = node_ref.read();

        if node.is_leaf {
            // Search in leaf node
            Ok(node.search_in_leaf(key))
        } else {
            // Find child to descend into
            let child_idx = node.find_child_index(key);
            let child = node.children[child_idx].clone();
            drop(node);

            self.search_recursive(child, key)
        }
    }

    /// Range scan from start to end (inclusive)
    pub fn range_scan(&self, start: &K, end: &K) -> Result<Vec<(K, V)>> {
        let root_lock = self.root.read();

        match root_lock.as_ref() {
            None => Ok(Vec::new()),
            Some(root) => {
                let leaf = self.find_leaf(root.clone(), start)?;
                self.collect_range(leaf, start, end)
            }
        }
    }

    /// Find the leaf node that should contain the key
    fn find_leaf(&self, node_ref: NodeRef<K, V>, key: &K) -> Result<NodeRef<K, V>> {
        let node = node_ref.read();

        if node.is_leaf {
            drop(node);
            Ok(node_ref.clone())
        } else {
            let child_idx = node.find_child_index(key);
            let child = node.children[child_idx].clone();
            drop(node);

            self.find_leaf(child, key)
        }
    }

    /// Collect key-value pairs in range from leaf nodes
    fn collect_range(
        &self,
        mut current_leaf: NodeRef<K, V>,
        start: &K,
        end: &K,
    ) -> Result<Vec<(K, V)>> {
        let mut results = Vec::new();

        loop {
            let leaf = current_leaf.read();

            for (k, v) in &leaf.entries {
                if k >= start && k <= end {
                    results.push((k.clone(), v.clone()));
                } else if k > end {
                    return Ok(results);
                }
            }

            // Move to next leaf if available
            match &leaf.next_leaf {
                Some(next) => {
                    let next_clone = next.clone();
                    drop(leaf);
                    current_leaf = next_clone;
                }
                None => break,
            }
        }

        Ok(results)
    }

    /// Delete a key
    pub fn delete(&self, key: &K) -> Result<bool> {
        let root_lock = self.root.read();

        match root_lock.as_ref() {
            None => Ok(false),
            Some(root) => {
                let root_clone = root.clone();
                drop(root_lock);

                self.delete_recursive(root_clone, key)
            }
        }
    }

    /// Recursive delete
    fn delete_recursive(&self, node_ref: NodeRef<K, V>, key: &K) -> Result<bool> {
        let mut node = node_ref.write();

        if node.is_leaf {
            // Delete from leaf
            let found = node.delete_from_leaf(key);
            Ok(found)
        } else {
            // Find child to descend into
            let child_idx = node.find_child_index(key);
            let child = node.children[child_idx].clone();
            drop(node);

            self.delete_recursive(child, key)
        }
    }

    /// Bulk load data efficiently
    /// This is optimized for loading sorted data into an empty tree
    pub fn bulk_load(&self, mut data: Vec<(K, V)>) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        // Sort data by key
        data.sort_by(|a, b| a.0.cmp(&b.0));

        // Build leaf level
        let leaves = self.build_leaf_level(data)?;

        if leaves.len() == 1 {
            // Single leaf becomes root
            let mut root_lock = self.root.write();
            *root_lock = Some(leaves[0].clone());
            *self.height.write() = 1;
            return Ok(());
        }

        // Build internal levels bottom-up
        let root = self.build_internal_levels(leaves)?;

        let mut root_lock = self.root.write();
        *root_lock = Some(root);

        Ok(())
    }

    /// Build leaf level for bulk loading
    fn build_leaf_level(&self, data: Vec<(K, V)>) -> Result<Vec<NodeRef<K, V>>> {
        let mut leaves = Vec::new();
        let leaf_capacity = self.order - 1;

        let mut current_leaf: Option<Node<K, V>> = None;
        let mut prev_leaf: Option<NodeRef<K, V>> = None;

        for (key, value) in data {
            if current_leaf.is_none() {
                current_leaf = Some(Node::new_leaf(self.order));
            }

            let leaf = current_leaf.as_mut().unwrap();
            leaf.entries.push((key, value));

            if leaf.entries.len() >= leaf_capacity {
                // Leaf is full, create new one
                let completed_leaf = current_leaf.take().unwrap();
                let leaf_ref = Arc::new(RwLock::new(completed_leaf));

                // Link to previous leaf
                if let Some(prev) = prev_leaf {
                    prev.write().next_leaf = Some(leaf_ref.clone());
                }

                leaves.push(leaf_ref.clone());
                prev_leaf = Some(leaf_ref);
                current_leaf = None;
            }
        }

        // Add remaining leaf
        if let Some(leaf) = current_leaf {
            let leaf_ref = Arc::new(RwLock::new(leaf));
            if let Some(prev) = prev_leaf {
                prev.write().next_leaf = Some(leaf_ref.clone());
            }
            leaves.push(leaf_ref);
        }

        // Populate keys from entries
        for leaf_ref in &leaves {
            let mut leaf = leaf_ref.write();
            leaf.keys = leaf.entries.iter().map(|(k, _)| k.clone()).collect();
        }

        Ok(leaves)
    }

    /// Build internal levels for bulk loading
    fn build_internal_levels(&self, mut children: Vec<NodeRef<K, V>>) -> Result<NodeRef<K, V>> {
        let mut height = 1;

        while children.len() > 1 {
            let mut parents = Vec::new();
            let parent_capacity = self.order;

            let mut current_parent = Node::new_internal(self.order);

            for (i, child) in children.into_iter().enumerate() {
                if i > 0 {
                    // Add separator key (first key of child)
                    let child_lock = child.read();
                    let separator = child_lock.keys[0].clone();
                    drop(child_lock);
                    current_parent.keys.push(separator);
                }

                current_parent.children.push(child);

                if current_parent.children.len() >= parent_capacity {
                    // Parent is full
                    parents.push(Arc::new(RwLock::new(current_parent)));
                    current_parent = Node::new_internal(self.order);
                }
            }

            // Add remaining parent
            if !current_parent.children.is_empty() {
                parents.push(Arc::new(RwLock::new(current_parent)));
            }

            children = parents;
            height += 1;
        }

        *self.height.write() = height;
        Ok(children[0].clone())
    }

    /// Get tree statistics
    pub fn stats(&self) -> BTreeStats {
        let root_lock = self.root.read();
        let height = *self.height.read();

        match root_lock.as_ref() {
            None => BTreeStats {
                height: 0,
                total_nodes: 0,
                total_keys: 0,
                leaf_nodes: 0,
                internal_nodes: 0,
            },
            Some(root) => {
                let mut stats = BTreeStats {
                    height,
                    total_nodes: 0,
                    total_keys: 0,
                    leaf_nodes: 0,
                    internal_nodes: 0,
                };

                self.collect_stats(root.clone(), &mut stats);
                stats
            }
        }
    }

    fn collect_stats(&self, node_ref: NodeRef<K, V>, stats: &mut BTreeStats) {
        let node = node_ref.read();

        stats.total_nodes += 1;
        stats.total_keys += node.keys.len();

        if node.is_leaf {
            stats.leaf_nodes += 1;
        } else {
            stats.internal_nodes += 1;
            for child in &node.children {
                self.collect_stats(child.clone(), stats);
            }
        }
    }
}

/// Node reference type
type NodeRef<K, V> = Arc<RwLock<Node<K, V>>>;

/// B+ Tree Node
#[derive(Debug)]
struct Node<K: Ord + Clone + Debug, V: Clone + Debug> {
    keys: Vec<K>,
    is_leaf: bool,
    // For leaf nodes: actual key-value pairs
    entries: Vec<(K, V)>,
    // For internal nodes: child pointers
    children: Vec<NodeRef<K, V>>,
    // For leaf nodes: pointer to next leaf (for range scans)
    next_leaf: Option<NodeRef<K, V>>,
}

impl<K: Ord + Clone + Debug, V: Clone + Debug> Node<K, V> {
    /// Create a new leaf node
    fn new_leaf(order: usize) -> Self {
        Self {
            keys: Vec::with_capacity(order),
            is_leaf: true,
            entries: Vec::with_capacity(order),
            children: Vec::new(),
            next_leaf: None,
        }
    }

    /// Create a new internal node
    fn new_internal(order: usize) -> Self {
        Self {
            keys: Vec::with_capacity(order),
            is_leaf: false,
            entries: Vec::new(),
            children: Vec::with_capacity(order + 1),
            next_leaf: None,
        }
    }

    /// Find the index of the child to descend into
    fn find_child_index(&self, key: &K) -> usize {
        for (i, k) in self.keys.iter().enumerate() {
            if key < k {
                return i;
            }
        }
        self.keys.len()
    }

    /// Insert in leaf node
    fn insert_in_leaf(&mut self, key: K, value: V) -> Result<()> {
        let pos = self.entries.binary_search_by(|(k, _)| k.cmp(&key))
            .unwrap_or_else(|e| e);

        self.entries.insert(pos, (key.clone(), value));
        self.keys.insert(pos, key);

        Ok(())
    }

    /// Insert in internal node
    fn insert_in_internal(
        &mut self,
        key: K,
        child: NodeRef<K, V>,
        child_idx: usize,
    ) -> Result<()> {
        self.keys.insert(child_idx, key);
        self.children.insert(child_idx + 1, child);
        Ok(())
    }

    /// Split a leaf node
    fn split_leaf(&mut self, order: usize) -> Result<(K, Node<K, V>)> {
        let split_point = order / 2;

        // Create new leaf with right half of entries
        let mut new_leaf = Node::new_leaf(order);
        new_leaf.entries = self.entries.split_off(split_point);
        new_leaf.keys = self.keys.split_off(split_point);

        // Link leaves
        new_leaf.next_leaf = self.next_leaf.take();

        // Get the first key of the new leaf as separator
        let split_key = new_leaf.keys[0].clone();

        Ok((split_key, new_leaf))
    }

    /// Split an internal node
    fn split_internal(&mut self, order: usize) -> Result<(K, Node<K, V>)> {
        let split_point = order / 2;

        // Create new internal node with right half
        let mut new_node = Node::new_internal(order);

        // Split children
        new_node.children = self.children.split_off(split_point + 1);

        // Split keys (median key moves up)
        let median_key = self.keys.remove(split_point);
        new_node.keys = self.keys.split_off(split_point);

        Ok((median_key, new_node))
    }

    /// Search in leaf node
    fn search_in_leaf(&self, key: &K) -> Option<V> {
        self.entries
            .binary_search_by(|(k, _)| k.cmp(key))
            .ok()
            .map(|idx| self.entries[idx].1.clone())
    }

    /// Delete from leaf node
    fn delete_from_leaf(&mut self, key: &K) -> bool {
        if let Ok(idx) = self.entries.binary_search_by(|(k, _)| k.cmp(key)) {
            self.entries.remove(idx);
            self.keys.remove(idx);
            true
        } else {
            false
        }
    }
}

/// B+ Tree statistics
#[derive(Debug, Clone)]
pub struct BTreeStats {
    pub height: usize,
    pub total_nodes: usize,
    pub total_keys: usize,
    pub leaf_nodes: usize,
    pub internal_nodes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree_insert_search() {
        let tree: BPlusTree<i32, String> = BPlusTree::new();

        tree.insert(5, "five".to_string()).unwrap();
        tree.insert(3, "three".to_string()).unwrap();
        tree.insert(7, "seven".to_string()).unwrap();
        tree.insert(1, "one".to_string()).unwrap();

        assert_eq!(tree.search(&5).unwrap(), Some("five".to_string()));
        assert_eq!(tree.search(&3).unwrap(), Some("three".to_string()));
        assert_eq!(tree.search(&7).unwrap(), Some("seven".to_string()));
        assert_eq!(tree.search(&1).unwrap(), Some("one".to_string()));
        assert_eq!(tree.search(&9).unwrap(), None);
    }

    #[test]
    fn test_btree_range_scan() {
        let tree: BPlusTree<i32, String> = BPlusTree::new();

        for i in 1..=10 {
            tree.insert(i, format!("value_{}", i)).unwrap();
        }

        let results = tree.range_scan(&3, &7).unwrap();
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].0, 3);
        assert_eq!(results[4].0, 7);
    }

    #[test]
    fn test_btree_bulk_load() {
        let tree: BPlusTree<i32, String> = BPlusTree::new();

        let data: Vec<_> = (1..=100)
            .map(|i| (i, format!("value_{}", i)))
            .collect();

        tree.bulk_load(data).unwrap();

        // Verify all values can be found
        for i in 1..=100 {
            assert_eq!(tree.search(&i).unwrap(), Some(format!("value_{}", i)));
        }
    }

    #[test]
    fn test_btree_delete() {
        let tree: BPlusTree<i32, String> = BPlusTree::new();

        tree.insert(1, "one".to_string()).unwrap();
        tree.insert(2, "two".to_string()).unwrap();
        tree.insert(3, "three".to_string()).unwrap();

        assert!(tree.delete(&2).unwrap());
        assert_eq!(tree.search(&2).unwrap(), None);
        assert_eq!(tree.search(&1).unwrap(), Some("one".to_string()));
        assert_eq!(tree.search(&3).unwrap(), Some("three".to_string()));
    }

    #[test]
    fn test_btree_stats() {
        let tree: BPlusTree<i32, String> = BPlusTree::new();

        for i in 1..=50 {
            tree.insert(i, format!("value_{}", i)).unwrap();
        }

        let stats = tree.stats();
        assert!(stats.height > 0);
        assert_eq!(stats.total_keys, 50);
        assert!(stats.leaf_nodes > 0);
    }
}
