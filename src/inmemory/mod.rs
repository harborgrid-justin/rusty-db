// Oracle-like In-Memory Column Store Module
//
// This module provides enterprise-grade in-memory columnar storage with:
// - Dual-format architecture (row + column)
// - SIMD-accelerated vectorized operations
// - Advanced compression algorithms
// - Background population from disk
// - Vectorized join engine

pub mod column_store;
pub mod compression;
pub mod vectorized_ops;
pub mod population;
pub mod join_engine;

pub use column_store::{
    ColumnStore, ColumnStoreConfig, ColumnSegment, DualFormat,
    ColumnMetadata, ColumnStats, InMemoryArea,
};

pub use compression::{
    CompressionType, CompressionAlgorithm, DictionaryEncoder,
    RunLengthEncoder, BitPacker, DeltaEncoder, FrameOfReferenceEncoder,
    HybridCompressor, CompressionStats,
};

pub use vectorized_ops::{
    VectorizedFilter, VectorizedAggregator, SimdOperator,
    ComparisonOp, VectorMask, VectorBatch, CacheLine,
};

pub use population::{
    PopulationManager, PopulationStrategy, PopulationPriority,
    PopulationProgress, PopulationStats, MemoryPressureHandler,
};

pub use join_engine::{
    VectorizedJoin, HashJoinEngine, BloomFilter, JoinType,
    JoinStats, PartitionedJoin,
};

use std::sync::Arc;
use parking_lot::RwLock;

/// Global in-memory store instance
pub struct InMemoryStore {
    column_stores: RwLock<Vec<Arc<ColumnStore>>>,
    population_manager: Arc<PopulationManager>,
    config: InMemoryConfig,
}

/// Configuration for in-memory storage
#[derive(Debug, Clone)]
pub struct InMemoryConfig {
    /// Maximum memory for in-memory area (bytes)
    pub max_memory: usize,
    /// Enable automatic population
    pub auto_populate: bool,
    /// Enable compression
    pub enable_compression: bool,
    /// SIMD vector width (lanes)
    pub vector_width: usize,
    /// Cache line size (bytes)
    pub cache_line_size: usize,
    /// Number of population threads
    pub population_threads: usize,
    /// Memory pressure threshold (0.0-1.0)
    pub memory_pressure_threshold: f64,
}

impl Default for InMemoryConfig {
    fn default() -> Self {
        Self {
            max_memory: 4 * 1024 * 1024 * 1024, // 4GB
            auto_populate: true,
            enable_compression: true,
            vector_width: 8, // 8-lane SIMD (256-bit)
            cache_line_size: 64,
            population_threads: 4,
            memory_pressure_threshold: 0.9,
        }
    }
}

impl InMemoryStore {
    pub fn new(config: InMemoryConfig) -> Self {
        Self {
            column_stores: RwLock::new(Vec::new()),
            population_manager: Arc::new(PopulationManager::new(
                config.population_threads,
                config.max_memory,
            )),
            config,
        }
    }

    pub fn create_column_store(&self, name: String, schema: Vec<ColumnMetadata>) -> Arc<ColumnStore> {
        let store_config = ColumnStoreConfig {
            name: name.clone(),
            enable_compression: self.config.enable_compression,
            vector_width: self.config.vector_width,
            cache_line_size: self.config.cache_line_size,
        };

        let store = Arc::new(ColumnStore::new(store_config, schema));

        self.column_stores.write().push(store.clone());

        if self.config.auto_populate {
            self.population_manager.schedule_population(store.clone());
        }

        store
    }

    pub fn get_column_store(&self, name: &str) -> Option<Arc<ColumnStore>> {
        self.column_stores
            .read()
            .iter()
            .find(|s| s.name() == name)
            .cloned()
    }

    pub fn memory_usage(&self) -> usize {
        self.column_stores
            .read()
            .iter()
            .map(|s| s.memory_usage())
            .sum()
    }

    pub fn check_memory_pressure(&self) -> bool {
        let usage = self.memory_usage();
        let threshold = (self.config.max_memory as f64 * self.config.memory_pressure_threshold) as usize;
        usage > threshold
    }

    pub fn evict_if_needed(&self) {
        if self.check_memory_pressure() {
            // Implement LRU-based eviction
            let stores = self.column_stores.read();
            if let Some(store) = stores.last() {
                store.evict_cold_segments();
            }
        }
    }

    pub fn stats(&self) -> InMemoryStoreStats {
        let stores = self.column_stores.read();
        InMemoryStoreStats {
            total_stores: stores.len(),
            total_memory: self.memory_usage(),
            max_memory: self.config.max_memory,
            memory_pressure: self.memory_usage() as f64 / self.config.max_memory as f64,
            population_stats: self.population_manager.stats(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InMemoryStoreStats {
    pub total_stores: usize,
    pub total_memory: usize,
    pub max_memory: usize,
    pub memory_pressure: f64,
    pub population_stats: PopulationStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inmemory_store_creation() {
        let config = InMemoryConfig::default();
        let store = InMemoryStore::new(config);
        assert_eq!(store.memory_usage(), 0);
    }

    #[test]
    fn test_memory_pressure_detection() {
        let mut config = InMemoryConfig::default();
        config.max_memory = 1000;
        config.memory_pressure_threshold = 0.8;

        let store = InMemoryStore::new(config);
        assert!(!store.check_memory_pressure());
    }
}


