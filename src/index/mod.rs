// Index Module - Enterprise-Grade Indexing Engine for RustyDB
//
// This module provides comprehensive indexing capabilities:
// - B+ Tree: Concurrent implementation with latch crabbing
// - LSM Tree: Write-optimized indexing with compaction
// - Hash Indexes: Extendible and linear hashing
// - Bitmap Indexes: Compressed bitmaps for low-cardinality data
// - Spatial Indexes: R-tree for geographic/spatial queries
// - Full-Text Search: Inverted indexes with TF-IDF
// - Partial Indexes: Filtered indexes with predicates
// - Expression Indexes: Function-based computed indexes
// - Index Advisor: Intelligent index recommendations

pub mod advisor;
pub mod bitmap;
pub mod bitmap_compressed;
pub mod btree;
pub mod btree_optimized;
pub mod fulltext;
pub mod hash_index;
pub mod hash_helpers;
pub mod lsm_index;
pub mod partial;
pub mod simd_bloom;
pub mod spatial;
pub mod swiss_table;

use crate::error::{DbError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

// ============================================================================
// Common Index Traits
// ============================================================================

/// Trait for node splitting operations across different index structures
///
/// This trait consolidates the common splitting logic found in:
/// - B+Tree (btree.rs): split_leaf() and split_internal()
/// - LSM Tree (lsm_index.rs): quadratic_split() for memtable overflow
/// - R-Tree (spatial.rs): quadratic_split() for spatial node overflow
///
/// ## Consolidation Benefits
///
/// 1. **Reduced Code Duplication**: Common split algorithms in one place
/// 2. **Consistent Behavior**: All indexes split the same way
/// 3. **Easier Testing**: Test splitting logic once, use everywhere
/// 4. **Maintainability**: Changes propagate to all index types
///
/// ## Split Strategies
///
/// - **B+Tree**: Median split - divides keys at the middle point
/// - **LSM Tree**: Quadratic split - minimizes wasted space
/// - **R-Tree**: Quadratic split - minimizes bounding box overlap
///
/// ## Usage Example
///
/// ```rust,ignore
/// impl NodeSplitting for BTreeNode<K, V> {
///     type Entry = (K, V);
///     type SplitKey = K;
///
///     fn split(&mut self, capacity: usize) -> Result<(K, Self)> {
///         let split_point = self.find_split_point(capacity);
///         // Split at middle
///         let right_entries = self.entries.split_off(split_point);
///         let separator_key = right_entries[0].0.clone();
///         Ok((separator_key, Self::new_with_entries(right_entries)))
///     }
///
///     fn needs_split(&self, capacity: usize) -> bool {
///         self.entries.len() >= capacity
///     }
/// }
/// ```
pub trait NodeSplitting {
    /// Type of entries stored in the node
    type Entry;

    /// Type representing the split key/separator
    type SplitKey;

    /// Split a node that has exceeded capacity
    ///
    /// Returns a tuple of:
    /// - The split key/separator that divides the two groups
    /// - The new node containing the right half of entries
    fn split(&mut self, capacity: usize) -> Result<(Self::SplitKey, Self)>
    where
        Self: Sized;

    /// Determine if a node needs splitting based on capacity
    fn needs_split(&self, capacity: usize) -> bool;

    /// Calculate the optimal split point for entries
    ///
    /// Default implementation uses median split (divides at middle).
    /// Override for custom strategies (e.g., quadratic split for R-trees)
    fn find_split_point(&self, capacity: usize) -> usize {
        capacity / 2
    }
}

/// Helper utilities for node splitting operations
pub mod split_utils {
    use super::*;

    /// Calculate the optimal split point using median strategy
    ///
    /// Used by B+Tree for balanced splits
    #[inline]
    pub fn median_split_point(num_entries: usize) -> usize {
        num_entries / 2
    }

    /// Calculate split point with minimum fill threshold
    ///
    /// Ensures nodes maintain at least `min_fill_ratio` fullness
    /// Used to prevent underflow in subsequent operations
    #[inline]
    pub fn split_point_with_min_fill(num_entries: usize, min_fill_ratio: f64) -> usize {
        let min_entries = ((num_entries as f64) * min_fill_ratio).ceil() as usize;
        num_entries.saturating_sub(min_entries).max(num_entries / 2)
    }

    /// Find the best split point that minimizes a cost function
    ///
    /// Generic helper for quadratic split algorithms (LSM, R-Tree)
    /// The cost function receives (left_size, right_size) and returns cost
    pub fn find_best_split<F>(num_entries: usize, cost_fn: F) -> usize
    where
        F: Fn(usize, usize) -> f64,
    {
        let mut best_split = num_entries / 2;
        let mut best_cost = f64::MAX;

        // Try split points from 1/3 to 2/3 of entries
        let start = num_entries / 3;
        let end = (num_entries * 2) / 3;

        for split_point in start..=end {
            let left_size = split_point;
            let right_size = num_entries - split_point;
            let cost = cost_fn(left_size, right_size);

            if cost < best_cost {
                best_cost = cost;
                best_split = split_point;
            }
        }

        best_split
    }

    /// Calculate imbalance factor for a split
    ///
    /// Returns a value from 0.0 (perfectly balanced) to 1.0 (maximally imbalanced)
    #[inline]
    pub fn split_imbalance(left_size: usize, right_size: usize) -> f64 {
        let total = (left_size + right_size) as f64;
        if total == 0.0 {
            return 0.0;
        }
        let ideal = total / 2.0;
        let actual_left = left_size as f64;
        (actual_left - ideal).abs() / ideal
    }
}

/// Trait for index iteration patterns
///
/// This trait consolidates iterator patterns found in:
/// - B+Tree (btree.rs): range_scan() and collect_range()
/// - LSM Tree (lsm_index.rs): range() with merge iterator
/// - Hash Index (hash_index.rs): full table scan patterns
///
/// ## Common Iterator Patterns Consolidated
///
/// ### Pattern 1: Range Scan with Leaf Linking (B+Tree)
/// ```rust,ignore
/// // B+Tree traverses leaf chain for range scans
/// let mut current = find_leaf(start_key);
/// while let Some(leaf) = current {
///     for (k, v) in leaf.entries {
///         if k > end_key { break; }
///         yield (k, v);
///     }
///     current = leaf.next_leaf;
/// }
/// ```
///
/// ### Pattern 2: Merge Iterator (LSM Tree)
/// ```rust,ignore
/// // LSM merges multiple sorted sources
/// let mut heap = BinaryHeap::new();
/// for level in levels {
///     heap.push(level.iter());
/// }
/// while let Some(entry) = heap.pop() {
///     yield entry;
///     if let Some(next) = entry.iter.next() {
///         heap.push(next);
///     }
/// }
/// ```
///
/// ### Pattern 3: Hash Table Scan
/// ```rust,ignore
/// // Hash index scans all buckets
/// for bucket in buckets {
///     for entry in bucket {
///         yield entry;
///     }
/// }
/// ```
///
/// ## Design Considerations
///
/// - **Late Materialization**: Iterator yields keys/row IDs, fetches values on demand
/// - **Prefetching**: Iterator hints at next access for cache optimization
/// - **Lazy Evaluation**: Results computed only when pulled from iterator
/// - **Memory Efficiency**: No intermediate vectors for large result sets
pub trait IndexIterator {
    /// Key type for iteration
    type Key;

    /// Value type for iteration
    type Value;

    /// Item type returned by iterator
    type Item;

    /// Create an iterator over all entries
    ///
    /// For ordered indexes (B+Tree, LSM), this yields in sorted order.
    /// For unordered indexes (Hash), order is undefined.
    fn iter(&self) -> Box<dyn Iterator<Item = Self::Item> + '_>;

    /// Create an iterator over a range of keys
    ///
    /// Yields all entries where `start <= key <= end`.
    /// Only implemented for ordered indexes.
    fn range_iter(&self, start: &Self::Key, end: &Self::Key) -> Box<dyn Iterator<Item = Self::Item> + '_>;
}

/// Helper utilities for index iteration
pub mod iter_utils {
    use super::*;

    /// Merge multiple sorted iterators into a single sorted stream
    ///
    /// Used by LSM Tree to merge results from multiple levels
    pub struct MergeIterator<K: Ord, V, I: Iterator<Item = (K, V)>> {
        iterators: Vec<std::iter::Peekable<I>>,
    }

    impl<K: Ord, V, I: Iterator<Item = (K, V)>> MergeIterator<K, V, I> {
        pub fn new(iterators: Vec<I>) -> Self {
            Self {
                iterators: iterators.into_iter().map(|i| i.peekable()).collect(),
            }
        }
    }

    impl<K: Ord, V, I: Iterator<Item = (K, V)>> Iterator for MergeIterator<K, V, I> {
        type Item = (K, V);

        fn next(&mut self) -> Option<Self::Item> {
            // Find iterator with smallest key
            let mut min_idx = None;
            let mut min_key: Option<&K> = None;

            for (idx, iter) in self.iterators.iter_mut().enumerate() {
                if let Some((key, _)) = iter.peek() {
                    if min_key.map_or(true, |mk| key < mk) {
                        min_key = Some(key);
                        min_idx = Some(idx);
                    }
                }
            }

            min_idx.and_then(|idx| self.iterators[idx].next())
        }
    }

    /// Batching iterator that yields results in chunks
    ///
    /// Improves cache locality for processing large result sets
    pub struct BatchIterator<I: Iterator> {
        inner: I,
        batch_size: usize,
    }

    impl<I: Iterator> BatchIterator<I> {
        pub fn new(inner: I, batch_size: usize) -> Self {
            Self { inner, batch_size }
        }
    }

    impl<I: Iterator> Iterator for BatchIterator<I> {
        type Item = Vec<I::Item>;

        fn next(&mut self) -> Option<Self::Item> {
            let mut batch = Vec::with_capacity(self.batch_size);
            for _ in 0..self.batch_size {
                match self.inner.next() {
                    Some(item) => batch.push(item),
                    None => break,
                }
            }

            if batch.is_empty() {
                None
            } else {
                Some(batch)
            }
        }
    }
}

/// Common statistics interface for all index types
pub trait IndexStatistics {
    /// Get the total number of entries in the index
    fn entry_count(&self) -> usize;

    /// Get the storage size in bytes (approximate)
    fn storage_size(&self) -> usize;

    /// Get the height/depth of the index structure (if applicable)
    fn height(&self) -> Option<usize> {
        None
    }

    /// Get load factor (0.0 to 1.0) indicating fullness
    fn load_factor(&self) -> f64 {
        0.0
    }
}

// Index key type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IndexKey {
    Integer(i64),
    String(String),
    Binary(Vec<u8>),
}

// Index value (row ID or pointer)
pub type IndexValue = u64;

// Simple B-Tree index implementation (for backward compatibility)
#[derive(Clone)]
pub struct BTreeIndex {
    name: String,
    tree: Arc<RwLock<BTreeMap<IndexKey, Vec<IndexValue>>>>,
}

impl BTreeIndex {
    pub fn new(name: String) -> Self {
        Self {
            name,
            tree: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    pub fn insert(&self, key: IndexKey, value: IndexValue) -> Result<()> {
        let mut tree = self.tree.write();
        tree.entry(key).or_insert_with(Vec::new).push(value);
        Ok(())
    }

    pub fn search(&self, key: &IndexKey) -> Result<Vec<IndexValue>> {
        let tree = self.tree.read();
        Ok(tree.get(key).cloned().unwrap_or_default())
    }

    pub fn range_search(&self, start: &IndexKey, end: &IndexKey) -> Result<Vec<IndexValue>> {
        let tree = self.tree.read();
        let mut results = Vec::new();

        for (_, values) in tree.range(start.clone()..=end.clone()) {
            results.extend(values);
        }

        Ok(results)
    }

    pub fn delete(&self, key: &IndexKey, value: IndexValue) -> Result<()> {
        let mut tree = self.tree.write();

        if let Some(values) = tree.get_mut(key) {
            values.retain(|&v| v != value);
            if values.is_empty() {
                tree.remove(key);
            }
        }

        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

// Simple hash index implementation (for backward compatibility)
#[derive(Clone)]
pub struct HashIndex {
    name: String,
    map: Arc<RwLock<std::collections::HashMap<IndexKey, Vec<IndexValue>>>>,
}

impl HashIndex {
    pub fn new(name: String) -> Self {
        Self {
            name,
            map: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn insert(&self, key: IndexKey, value: IndexValue) -> Result<()> {
        let mut map = self.map.write();
        map.entry(key).or_insert_with(Vec::new).push(value);
        Ok(())
    }

    pub fn search(&self, key: &IndexKey) -> Result<Vec<IndexValue>> {
        let map = self.map.read();
        Ok(map.get(key).cloned().unwrap_or_default())
    }

    pub fn delete(&self, key: &IndexKey, value: IndexValue) -> Result<()> {
        let mut map = self.map.write();

        if let Some(values) = map.get_mut(key) {
            values.retain(|&v| v != value);
            if values.is_empty() {
                map.remove(key);
            }
        }

        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

// Unified index types supporting all index structures
#[derive(Clone)]
pub enum Index {
    BTree(BTreeIndex),
    Hash(HashIndex),
    BPlusTree(btree::BPlusTree<IndexKey, IndexValue>),
    LSMTree(lsm_index::LSMTreeIndex<IndexKey, IndexValue>),
    ExtendibleHash(hash_index::ExtendibleHashIndex<IndexKey, IndexValue>),
    LinearHash(hash_index::LinearHashIndex<IndexKey, IndexValue>),
    Bitmap(bitmap::BitmapIndex<IndexKey>),
    Spatial(spatial::RTree<IndexValue>),
    FullText(fulltext::FullTextIndex),
    Partial(partial::PartialIndex<IndexKey, IndexValue>),
    Expression(partial::ExpressionIndex<IndexValue>),
    Covering(partial::CoveringIndex<IndexKey>),
}

impl Index {
    pub fn insert(&self, key: IndexKey, value: IndexValue) -> Result<()> {
        match self {
            Index::BTree(idx) => idx.insert(key, value),
            Index::Hash(idx) => idx.insert(key, value),
            Index::BPlusTree(idx) => idx.insert(key, value),
            Index::LSMTree(idx) => idx.insert(key, value),
            Index::ExtendibleHash(idx) => idx.insert(key, value),
            Index::LinearHash(idx) => idx.insert(key, value),
            _ => Err(DbError::Internal(
                "Insert not supported for this index type".into(),
            )),
        }
    }

    pub fn search(&self, key: &IndexKey) -> Result<Vec<IndexValue>> {
        match self {
            Index::BTree(idx) => idx.search(key),
            Index::Hash(idx) => idx.search(key),
            Index::BPlusTree(idx) => idx.search(key).map(|opt| opt.into_iter().collect()),
            Index::LSMTree(idx) => idx.get(key).map(|opt| opt.into_iter().collect()),
            Index::ExtendibleHash(idx) => idx.get(key).map(|opt| opt.into_iter().collect()),
            Index::LinearHash(idx) => idx.get(key).map(|opt| opt.into_iter().collect()),
            Index::Bitmap(idx) => idx
                .get(key)
                .map(|v| v.into_iter().map(|i| i as u64).collect()),
            _ => Err(DbError::Internal(
                "Search not supported for this index type".into(),
            )),
        }
    }

    pub fn delete(&self, key: &IndexKey, value: IndexValue) -> Result<()> {
        match self {
            Index::BTree(idx) => idx.delete(key, value),
            Index::Hash(idx) => idx.delete(key, value),
            Index::BPlusTree(idx) => {
                idx.delete(key)?;
                Ok(())
            }
            Index::LSMTree(idx) => idx.delete(key.clone()),
            Index::ExtendibleHash(idx) => {
                idx.delete(key)?;
                Ok(())
            }
            Index::LinearHash(idx) => {
                idx.delete(key)?;
                Ok(())
            }
            _ => Err(DbError::Internal(
                "Delete not supported for this index type".into(),
            )),
        }
    }
}

// Index Manager - Central management for all indexes
pub struct IndexManager {
    indexes: Arc<RwLock<std::collections::HashMap<String, Index>>>,
    advisor: Arc<RwLock<advisor::IndexAdvisor>>,
}

impl IndexManager {
    // Create a new index manager
    pub fn new() -> Self {
        Self {
            indexes: Arc::new(RwLock::new(std::collections::HashMap::new())),
            advisor: Arc::new(RwLock::new(advisor::IndexAdvisor::new(
                advisor::AdvisorConfig::default(),
            ))),
        }
    }

    // Create a new index
    pub fn create_index(&self, name: String, index_type: IndexType) -> Result<()> {
        let mut indexes = self.indexes.write();

        if indexes.contains_key(&name) {
            return Err(DbError::Internal(format!(
                "Index '{}' already exists",
                name
            )));
        }

        let index = match index_type {
            IndexType::BTree => Index::BTree(BTreeIndex::new(name.clone())),
            IndexType::Hash => Index::Hash(HashIndex::new(name.clone())),
            IndexType::BPlusTree => Index::BPlusTree(btree::BPlusTree::new()),
            IndexType::LSMTree => {
                Index::LSMTree(lsm_index::LSMTreeIndex::new(lsm_index::LSMConfig::default()))
            }
            IndexType::ExtendibleHash => {
                Index::ExtendibleHash(hash_index::ExtendibleHashIndex::new(64))
            }
            IndexType::LinearHash => Index::LinearHash(hash_index::LinearHashIndex::new(16, 64)),
            IndexType::Bitmap => Index::Bitmap(bitmap::BitmapIndex::new()),
            IndexType::Spatial => Index::Spatial(spatial::RTree::new()),
        };

        indexes.insert(name, index);
        Ok(())
    }

    // Get an index by name
    pub fn get_index(&self, name: &str) -> Result<Index> {
        let indexes = self.indexes.read();
        indexes
            .get(name)
            .cloned()
            .ok_or_else(|| DbError::Internal(format!("Index '{}' not found", name)))
    }

    // Drop an index
    pub fn drop_index(&self, name: &str) -> Result<()> {
        let mut indexes = self.indexes.write();
        if indexes.remove(name).is_some() {
            Ok(())
        } else {
            Err(DbError::Internal(format!("Index '{}' not found", name)))
        }
    }

    // Get index recommendations from the advisor
    pub fn get_recommendations(&self) -> Result<Vec<advisor::IndexRecommendation>> {
        let advisor = self.advisor.read();
        advisor.analyze()
    }

    // Record a query for workload analysis
    pub fn record_query(&self, query: &advisor::Query) {
        let mut advisor = self.advisor.write();
        advisor.record_query(query);
    }

    // List all indexes
    pub fn list_indexes(&self) -> Vec<String> {
        let indexes = self.indexes.read();
        indexes.keys().cloned().collect()
    }

    // Get index statistics
    pub fn get_index_stats(&self, name: &str) -> Result<IndexStats> {
        let indexes = self.indexes.read();
        let index = indexes
            .get(name)
            .ok_or_else(|| DbError::Internal(format!("Index '{}' not found", name)))?;

        Ok(match index {
            Index::BPlusTree(idx) => {
                let stats = idx.stats();
                IndexStats::BPlusTree(stats)
            }
            Index::LSMTree(idx) => {
                let stats = idx.stats();
                IndexStats::LSMTree(stats)
            }
            Index::ExtendibleHash(idx) => {
                let stats = idx.stats();
                IndexStats::ExtendibleHash(stats)
            }
            Index::LinearHash(idx) => {
                let stats = idx.stats();
                IndexStats::LinearHash(stats)
            }
            Index::Bitmap(idx) => {
                let stats = idx.stats();
                IndexStats::Bitmap(stats)
            }
            _ => IndexStats::Unknown,
        })
    }
}

impl Default for IndexManager {
    fn default() -> Self {
        Self::new()
    }
}

// Index type enumeration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    BPlusTree,
    LSMTree,
    ExtendibleHash,
    LinearHash,
    Bitmap,
    Spatial,
}

// Index statistics
#[derive(Debug, Clone)]
pub enum IndexStats {
    BPlusTree(btree::BTreeStats),
    LSMTree(lsm_index::LSMStats),
    ExtendibleHash(hash_index::ExtendibleHashStats),
    LinearHash(hash_index::LinearHashStats),
    Bitmap(bitmap::BitmapIndexStats),
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btree_index() -> Result<()> {
        let idx = BTreeIndex::new("test_idx".to_string());

        idx.insert(IndexKey::Integer(1), 100)?;
        idx.insert(IndexKey::Integer(2), 200)?;
        idx.insert(IndexKey::Integer(3), 300)?;

        let results = idx.search(&IndexKey::Integer(2))?;
        assert_eq!(results, vec![200]);

        let range_results = idx.range_search(&IndexKey::Integer(1), &IndexKey::Integer(3))?;
        assert_eq!(range_results.len(), 3);

        Ok(())
    }

    #[test]
    fn test_hash_index() -> Result<()> {
        let idx = HashIndex::new("test_hash".to_string());

        idx.insert(IndexKey::String("key1".to_string()), 100)?;
        idx.insert(IndexKey::String("key2".to_string()), 200)?;

        let results = idx.search(&IndexKey::String("key1".to_string()))?;
        assert_eq!(results, vec![100]);

        Ok(())
    }
}
