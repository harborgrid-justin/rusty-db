pub mod fulltext;

use std::collections::BTreeMap;
use parking_lot::RwLock;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::Result;

/// Index key type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IndexKey {
    Integer(i64),
    String(String),
}

/// Index value (row ID or pointer)
pub type IndexValue = u64;

/// B-Tree index implementation
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

/// Hash index implementation
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

/// Index types
pub enum Index {
    BTree(BTreeIndex),
    Hash(HashIndex),
}

impl Index {
    pub fn insert(&self, key: IndexKey, value: IndexValue) -> Result<()> {
        match self {
            Index::BTree(idx) => idx.insert(key, value),
            Index::Hash(idx) => idx.insert(key, value),
        }
    }
    
    pub fn search(&self, key: &IndexKey) -> Result<Vec<IndexValue>> {
        match self {
            Index::BTree(idx) => idx.search(key),
            Index::Hash(idx) => idx.search(key),
        }
    }
    
    pub fn delete(&self, key: &IndexKey, value: IndexValue) -> Result<()> {
        match self {
            Index::BTree(idx) => idx.delete(key, value),
            Index::Hash(idx) => idx.delete(key, value),
        }
    }
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
