// # Cache Invalidation System
//
// Fine-grained cache invalidation with table and row-level tracking.

use crate::common::{RowId, TableId};
use crate::error::Result;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Invalidation strategy for cache entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationStrategy {
    /// Invalidate immediately when table is modified
    Immediate,

    /// Invalidate after a delay (lazy invalidation)
    Lazy,

    /// Invalidate only on explicit request
    Manual,

    /// Row-level invalidation (most granular)
    RowLevel,
}

/// Table dependency information
#[derive(Debug, Clone)]
pub struct TableDependency {
    /// Table ID
    pub table_id: TableId,

    /// Specific rows this query depends on (None = entire table)
    pub row_ids: Option<HashSet<RowId>>,

    /// Whether this is a read or write dependency
    pub is_write: bool,
}

impl TableDependency {
    /// Create a table-level dependency
    pub fn table(table_id: TableId) -> Self {
        Self {
            table_id,
            row_ids: None,
            is_write: false,
        }
    }

    /// Create a row-level dependency
    pub fn rows(table_id: TableId, row_ids: HashSet<RowId>) -> Self {
        Self {
            table_id,
            row_ids: Some(row_ids),
            is_write: false,
        }
    }

    /// Mark as write dependency
    pub fn as_write(mut self) -> Self {
        self.is_write = true;
        self
    }

    /// Check if this dependency affects a specific row
    pub fn affects_row(&self, row_id: RowId) -> bool {
        match &self.row_ids {
            None => true, // Table-level dependency affects all rows
            Some(rows) => rows.contains(&row_id),
        }
    }
}

/// Cache key identifier (same as query hash)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKeyId(pub String);

/// Cache invalidation engine
pub struct CacheInvalidator {
    /// Invalidation strategy
    strategy: InvalidationStrategy,

    /// Table -> Cache keys mapping
    table_to_keys: Arc<RwLock<HashMap<TableId, HashSet<CacheKeyId>>>>,

    /// Row -> Cache keys mapping (for row-level invalidation)
    row_to_keys: Arc<RwLock<HashMap<(TableId, RowId), HashSet<CacheKeyId>>>>,

    /// Cache key -> Dependencies mapping
    key_to_dependencies: Arc<RwLock<HashMap<CacheKeyId, Vec<TableDependency>>>>,

    /// Invalidation statistics
    total_invalidations: Arc<RwLock<u64>>,
    table_invalidations: Arc<RwLock<u64>>,
    row_invalidations: Arc<RwLock<u64>>,
}

impl CacheInvalidator {
    /// Create a new cache invalidator
    pub fn new(strategy: InvalidationStrategy) -> Self {
        Self {
            strategy,
            table_to_keys: Arc::new(RwLock::new(HashMap::new())),
            row_to_keys: Arc::new(RwLock::new(HashMap::new())),
            key_to_dependencies: Arc::new(RwLock::new(HashMap::new())),
            total_invalidations: Arc::new(RwLock::new(0)),
            table_invalidations: Arc::new(RwLock::new(0)),
            row_invalidations: Arc::new(RwLock::new(0)),
        }
    }

    /// Register dependencies for a cache entry
    pub fn register_dependencies(
        &self,
        cache_key: CacheKeyId,
        dependencies: Vec<TableDependency>,
    ) -> Result<()> {
        let mut table_map = self.table_to_keys.write().unwrap();
        let mut row_map = self.row_to_keys.write().unwrap();
        let mut key_deps = self.key_to_dependencies.write().unwrap();

        // Store dependencies for the cache key
        key_deps.insert(cache_key.clone(), dependencies.clone());

        for dep in &dependencies {
            // Add to table mapping
            table_map
                .entry(dep.table_id)
                .or_insert_with(HashSet::new)
                .insert(cache_key.clone());

            // Add to row mapping if row-level dependency
            if let Some(row_ids) = &dep.row_ids {
                for &row_id in row_ids {
                    row_map
                        .entry((dep.table_id, row_id))
                        .or_insert_with(HashSet::new)
                        .insert(cache_key.clone());
                }
            }
        }

        Ok(())
    }

    /// Unregister a cache key and its dependencies
    pub fn unregister(&self, cache_key: &CacheKeyId) -> Result<()> {
        let mut key_deps = self.key_to_dependencies.write().unwrap();

        if let Some(dependencies) = key_deps.remove(cache_key) {
            let mut table_map = self.table_to_keys.write().unwrap();
            let mut row_map = self.row_to_keys.write().unwrap();

            for dep in &dependencies {
                // Remove from table mapping
                if let Some(keys) = table_map.get_mut(&dep.table_id) {
                    keys.remove(cache_key);
                }

                // Remove from row mapping
                if let Some(row_ids) = &dep.row_ids {
                    for &row_id in row_ids {
                        if let Some(keys) = row_map.get_mut(&(dep.table_id, row_id)) {
                            keys.remove(cache_key);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Invalidate all cache entries for a table
    pub fn invalidate_table(&self, table_id: TableId) -> Result<Vec<CacheKeyId>> {
        if self.strategy == InvalidationStrategy::Manual {
            return Ok(Vec::new());
        }

        let table_map = self.table_to_keys.read().unwrap();
        let keys_to_invalidate = table_map
            .get(&table_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();

        drop(table_map);

        // Update statistics
        let mut total = self.total_invalidations.write().unwrap();
        let mut table_count = self.table_invalidations.write().unwrap();
        *total += keys_to_invalidate.len() as u64;
        *table_count += 1;

        // Clean up dependencies for invalidated keys
        for key in &keys_to_invalidate {
            self.unregister(key)?;
        }

        Ok(keys_to_invalidate)
    }

    /// Invalidate cache entries for specific rows
    pub fn invalidate_rows(
        &self,
        table_id: TableId,
        row_ids: &[RowId],
    ) -> Result<Vec<CacheKeyId>> {
        if self.strategy == InvalidationStrategy::Manual {
            return Ok(Vec::new());
        }

        let row_map = self.row_to_keys.read().unwrap();
        let mut keys_to_invalidate = HashSet::new();

        // Collect all cache keys affected by these rows
        for &row_id in row_ids {
            if let Some(keys) = row_map.get(&(table_id, row_id)) {
                keys_to_invalidate.extend(keys.iter().cloned());
            }
        }

        // Also invalidate table-level dependencies (queries that read entire table)
        let table_map = self.table_to_keys.read().unwrap();
        if let Some(table_keys) = table_map.get(&table_id) {
            // Only include table-level dependencies (no specific rows)
            let key_deps = self.key_to_dependencies.read().unwrap();
            for key in table_keys {
                if let Some(deps) = key_deps.get(key) {
                    for dep in deps {
                        if dep.table_id == table_id && dep.row_ids.is_none() {
                            keys_to_invalidate.insert(key.clone());
                        }
                    }
                }
            }
        }

        drop(row_map);
        drop(table_map);

        let keys = keys_to_invalidate.into_iter().collect::<Vec<_>>();

        // Update statistics
        let mut total = self.total_invalidations.write().unwrap();
        let mut row_count = self.row_invalidations.write().unwrap();
        *total += keys.len() as u64;
        *row_count += 1;

        // Clean up dependencies for invalidated keys
        for key in &keys {
            self.unregister(key)?;
        }

        Ok(keys)
    }

    /// Invalidate specific cache keys
    pub fn invalidate_keys(&self, cache_keys: &[CacheKeyId]) -> Result<()> {
        for key in cache_keys {
            self.unregister(key)?;
        }

        let mut total = self.total_invalidations.write().unwrap();
        *total += cache_keys.len() as u64;

        Ok(())
    }

    /// Get cache keys dependent on a table
    pub fn get_dependent_keys(&self, table_id: TableId) -> Vec<CacheKeyId> {
        let table_map = self.table_to_keys.read().unwrap();
        table_map
            .get(&table_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect()
    }

    /// Get dependencies for a cache key
    pub fn get_dependencies(&self, cache_key: &CacheKeyId) -> Option<Vec<TableDependency>> {
        let key_deps = self.key_to_dependencies.read().unwrap();
        key_deps.get(cache_key).cloned()
    }

    /// Clear all dependencies
    pub fn clear(&self) -> Result<()> {
        self.table_to_keys.write().unwrap().clear();
        self.row_to_keys.write().unwrap().clear();
        self.key_to_dependencies.write().unwrap().clear();
        Ok(())
    }

    /// Get total number of invalidations
    pub fn total_invalidations(&self) -> u64 {
        *self.total_invalidations.read().unwrap()
    }

    /// Get number of table-level invalidations
    pub fn table_invalidations(&self) -> u64 {
        *self.table_invalidations.read().unwrap()
    }

    /// Get number of row-level invalidations
    pub fn row_invalidations(&self) -> u64 {
        *self.row_invalidations.read().unwrap()
    }

    /// Get number of tracked cache keys
    pub fn tracked_keys_count(&self) -> usize {
        self.key_to_dependencies.read().unwrap().len()
    }

    /// Get invalidation strategy
    pub fn strategy(&self) -> InvalidationStrategy {
        self.strategy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_dependency_creation() {
        let dep = TableDependency::table(1);
        assert_eq!(dep.table_id, 1);
        assert!(dep.row_ids.is_none());
        assert!(!dep.is_write);
    }

    #[test]
    fn test_row_dependency_creation() {
        let mut row_ids = HashSet::new();
        row_ids.insert(10);
        row_ids.insert(20);

        let dep = TableDependency::rows(1, row_ids.clone()).as_write();
        assert_eq!(dep.table_id, 1);
        assert_eq!(dep.row_ids, Some(row_ids));
        assert!(dep.is_write);
    }

    #[test]
    fn test_dependency_affects_row() {
        // Table-level dependency
        let table_dep = TableDependency::table(1);
        assert!(table_dep.affects_row(10));
        assert!(table_dep.affects_row(20));

        // Row-level dependency
        let mut row_ids = HashSet::new();
        row_ids.insert(10);
        let row_dep = TableDependency::rows(1, row_ids);

        assert!(row_dep.affects_row(10));
        assert!(!row_dep.affects_row(20));
    }

    #[test]
    fn test_register_dependencies() {
        let invalidator = CacheInvalidator::new(InvalidationStrategy::Immediate);
        let cache_key = CacheKeyId("test_key".to_string());

        let deps = vec![TableDependency::table(1), TableDependency::table(2)];

        assert!(invalidator
            .register_dependencies(cache_key.clone(), deps)
            .is_ok());

        let dependent_keys = invalidator.get_dependent_keys(1);
        assert_eq!(dependent_keys.len(), 1);
        assert_eq!(dependent_keys[0], cache_key);
    }

    #[test]
    fn test_table_invalidation() {
        let invalidator = CacheInvalidator::new(InvalidationStrategy::Immediate);

        let key1 = CacheKeyId("key1".to_string());
        let key2 = CacheKeyId("key2".to_string());

        invalidator
            .register_dependencies(key1.clone(), vec![TableDependency::table(1)])
            .unwrap();
        invalidator
            .register_dependencies(key2.clone(), vec![TableDependency::table(1)])
            .unwrap();

        let invalidated = invalidator.invalidate_table(1).unwrap();
        assert_eq!(invalidated.len(), 2);
        assert!(invalidated.contains(&key1));
        assert!(invalidated.contains(&key2));
    }

    #[test]
    fn test_row_invalidation() {
        let invalidator = CacheInvalidator::new(InvalidationStrategy::RowLevel);

        let mut row_ids = HashSet::new();
        row_ids.insert(10);

        let cache_key = CacheKeyId("key1".to_string());
        invalidator
            .register_dependencies(cache_key.clone(), vec![TableDependency::rows(1, row_ids)])
            .unwrap();

        // Invalidate specific row
        let invalidated = invalidator.invalidate_rows(1, &[10]).unwrap();
        assert_eq!(invalidated.len(), 1);
        assert_eq!(invalidated[0], cache_key);

        // Invalidate different row - should not affect
        let invalidated = invalidator.invalidate_rows(1, &[20]).unwrap();
        assert_eq!(invalidated.len(), 0);
    }

    #[test]
    fn test_manual_strategy() {
        let invalidator = CacheInvalidator::new(InvalidationStrategy::Manual);

        let cache_key = CacheKeyId("key1".to_string());
        invalidator
            .register_dependencies(cache_key, vec![TableDependency::table(1)])
            .unwrap();

        // Manual strategy should not auto-invalidate
        let invalidated = invalidator.invalidate_table(1).unwrap();
        assert_eq!(invalidated.len(), 0);
    }

    #[test]
    fn test_unregister() {
        let invalidator = CacheInvalidator::new(InvalidationStrategy::Immediate);

        let cache_key = CacheKeyId("key1".to_string());
        invalidator
            .register_dependencies(cache_key.clone(), vec![TableDependency::table(1)])
            .unwrap();

        assert_eq!(invalidator.tracked_keys_count(), 1);

        invalidator.unregister(&cache_key).unwrap();
        assert_eq!(invalidator.tracked_keys_count(), 0);
    }

    #[test]
    fn test_statistics() {
        let invalidator = CacheInvalidator::new(InvalidationStrategy::Immediate);

        let cache_key = CacheKeyId("key1".to_string());
        invalidator
            .register_dependencies(cache_key, vec![TableDependency::table(1)])
            .unwrap();

        assert_eq!(invalidator.total_invalidations(), 0);
        invalidator.invalidate_table(1).unwrap();
        assert_eq!(invalidator.total_invalidations(), 1);
        assert_eq!(invalidator.table_invalidations(), 1);
    }

    #[test]
    fn test_clear() {
        let invalidator = CacheInvalidator::new(InvalidationStrategy::Immediate);

        invalidator
            .register_dependencies(
                CacheKeyId("key1".to_string()),
                vec![TableDependency::table(1)],
            )
            .unwrap();

        assert_eq!(invalidator.tracked_keys_count(), 1);
        invalidator.clear().unwrap();
        assert_eq!(invalidator.tracked_keys_count(), 0);
    }
}
