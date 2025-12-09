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

pub mod btree;
pub mod lsm_index;
pub mod hash_index;
pub mod bitmap;
pub mod spatial;
pub mod fulltext;
pub mod advisor;
pub mod partial;
pub mod swiss_table;
pub mod simd_bloom;

use std::collections::BTreeMap;
use parking_lot::RwLock;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::error::{DbError, Result};

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
            _ => Err(DbError::Internal("Insert not supported for this index type".into())),
        }
    }

    pub fn search(&self, key: &IndexKey) -> Result<Vec<IndexValue>> {
        match self {
            Index::BTree(idx) => idx.search(key),
            Index::Hash(idx) => idx.search(key),
            Index::BPlusTree(idx) => {
                idx.search(key).map(|opt| opt.into_iter().collect())
            }
            Index::LSMTree(idx) => {
                idx.get(key).map(|opt| opt.into_iter().collect())
            }
            Index::ExtendibleHash(idx) => {
                idx.get(key).map(|opt| opt.into_iter().collect())
            }
            Index::LinearHash(idx) => {
                idx.get(key).map(|opt| opt.into_iter().collect())
            }
            Index::Bitmap(idx) => idx.get(key).map(|v| v.into_iter().map(|i| i as u64).collect()),
            _ => Err(DbError::Internal("Search not supported for this index type".into())),
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
            _ => Err(DbError::Internal("Delete not supported for this index type".into())),
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
                advisor::AdvisorConfig::default()
            ))),
        }
    }

    // Create a new index
    pub fn create_index(&self, name: String, index_type: IndexType) -> Result<()> {
        let mut indexes = self.indexes.write();

        if indexes.contains_key(&name) {
            return Err(DbError::Internal(format!("Index '{}' already exists", name)));
        }

        let index = match index_type {
            IndexType::BTree => Index::BTree(BTreeIndex::new(name.clone())),
            IndexType::Hash => Index::Hash(HashIndex::new(name.clone())),
            IndexType::BPlusTree => Index::BPlusTree(btree::BPlusTree::new()),
            IndexType::LSMTree => Index::LSMTree(lsm_index::LSMTreeIndex::new(
                lsm_index::LSMConfig::default()
            )),
            IndexType::ExtendibleHash => {
                Index::ExtendibleHash(hash_index::ExtendibleHashIndex::new(64))
            }
            IndexType::LinearHash => {
                Index::LinearHash(hash_index::LinearHashIndex::new(16, 64))
            }
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
use std::collections::HashMap;

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
