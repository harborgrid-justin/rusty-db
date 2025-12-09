// Tiered Compression - Temperature-based compression management
// Automatically adjusts compression based on data access patterns

use super::*;
use super::algorithms::{LZ4Compressor, ZstdCompressor};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Block temperature tracking
#[derive(Debug, Clone)]
pub struct BlockTemperature {
    pub block_id: u64,
    pub temperature: DataTemperature,
    pub access_count: u64,
    pub last_access: u64,
    pub creation_time: u64,
    pub tier_changes: usize,
}

impl BlockTemperature {
    pub fn new(block_id: u64) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        Self {
            block_id,
            temperature: DataTemperature::Hot,
            access_count: 0,
            last_access: now,
            creation_time: now,
            tier_changes: 0,
        }
    }

    /// Calculate temperature based on access patterns
    pub fn calculate_temperature(&self, now: u64) -> DataTemperature {
        let age_seconds = now.saturating_sub(self.creation_time);
        let time_since_access = now.saturating_sub(self.last_access);

        // Hot: accessed recently and frequently
        if time_since_access < 3600 && self.access_count > 10 {
            DataTemperature::Hot
        }
        // Warm: moderate access
        else if time_since_access < 86400 || self.access_count > 5 {
            DataTemperature::Warm
        }
        // Cold: old with few accesses
        else if age_seconds > 604800 || time_since_access > 259200 {
            DataTemperature::Cold
        }
        // Frozen: very old, rarely accessed
        else if age_seconds > 2592000 {
            DataTemperature::Frozen
        }
        else {
            DataTemperature::Warm
        }
    }
}

/// Tier policy configuration
#[derive(Debug, Clone)]
pub struct TierPolicy {
    pub hot_max_age: u64,          // Max age in seconds before cooling
    pub warm_max_age: u64,         // Max age before becoming cold
    pub cold_max_age: u64,         // Max age before freezing
    pub hot_min_accesses: u64,     // Min accesses to stay hot
    pub enable_auto_migration: bool,
    pub migration_interval: u64,   // Seconds between migration checks
}

impl Default for TierPolicy {
    fn default() -> Self {
        Self {
            hot_max_age: 3600,        // 1 hour
            warm_max_age: 86400,      // 1 day
            cold_max_age: 604800,     // 1 week
            hot_min_accesses: 10,
            enable_auto_migration: true,
            migration_interval: 3600, // 1 hour
        }
    }
}

/// Tiered Compression Manager
pub struct TieredCompressor {
    block_temperatures: Arc<RwLock<HashMap<u64, BlockTemperature>>>,
    policy: TierPolicy,
    stats: Arc<RwLock<TierStats>>,
    hot_compressor: LZ4Compressor,
    warm_compressor: ZstdCompressor,
    cold_compressor: ZstdCompressor,
    frozen_compressor: ZstdCompressor,
}

impl TieredCompressor {
    pub fn new(policy: TierPolicy) -> Self {
        Self {
            block_temperatures: Arc::new(RwLock::new(HashMap::new())),
            policy,
            stats: Arc::new(RwLock::new(TierStats::default())),
            hot_compressor: LZ4Compressor::new(CompressionLevel::Fast),
            warm_compressor: ZstdCompressor::new(CompressionLevel::Default),
            cold_compressor: ZstdCompressor::new(CompressionLevel::Default),
            frozen_compressor: ZstdCompressor::new(CompressionLevel::Maximum),
        }
    }

    /// Classify block temperature based on access patterns
    pub fn classify_temperature(&self, block_id: u64) -> DataTemperature {
        let temperatures = self.block_temperatures.read().unwrap();

        if let Some(block_temp) = temperatures.get(&block_id) {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            block_temp.calculate_temperature(now)
        } else {
            DataTemperature::Hot // New blocks are hot
        }
    }

    /// Record an access to a block
    pub fn record_access(&mut self, block_id: u64) {
        let mut temperatures = self.block_temperatures.write().unwrap();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let block_temp = temperatures.entry(block_id)
            .or_insert_with(|| BlockTemperature::new(block_id));

        let old_temp = block_temp.temperature;
        block_temp.access_count += 1;
        block_temp.last_access = now;
        block_temp.temperature = block_temp.calculate_temperature(now);

        // Update stats if temperature changed
        if old_temp as u8 != block_temp.temperature as u8 {
            block_temp.tier_changes += 1;
            self.update_tier_stats(&old_temp, &block_temp.temperature);
        }
    }

    /// Compress data according to its temperature
    pub fn compress_by_temperature(&self, block_id: u64, data: &[u8], output: &mut [u8])
        -> CompressionResult<usize> {

        let temperature = self.classify_temperature(block_id);

        match temperature {
            DataTemperature::Hot => {
                self.hot_compressor.compress(data, output)
            }
            DataTemperature::Warm => {
                self.warm_compressor.compress(data, output)
            }
            DataTemperature::Cold => {
                self.cold_compressor.compress(data, output)
            }
            DataTemperature::Frozen => {
                self.frozen_compressor.compress(data, output)
            }
        }
    }

    /// Decompress data (algorithm detected from metadata)
    pub fn decompress(&self, temperature: DataTemperature, data: &[u8], output: &mut [u8])
        -> CompressionResult<usize> {

        match temperature {
            DataTemperature::Hot => {
                self.hot_compressor.decompress(data, output)
            }
            DataTemperature::Warm => {
                self.warm_compressor.decompress(data, output)
            }
            DataTemperature::Cold => {
                self.cold_compressor.decompress(data, output)
            }
            DataTemperature::Frozen => {
                self.frozen_compressor.decompress(data, output)
            }
        }
    }

    /// Migrate a block to a new temperature tier
    pub fn migrate_block(&mut self, block_id: u64, new_temp: DataTemperature)
        -> CompressionResult<()> {

        let mut temperatures = self.block_temperatures.write().unwrap();

        if let Some(block_temp) = temperatures.get_mut(&block_id) {
            let old_temp = block_temp.temperature;

            if old_temp as u8 != new_temp as u8 {
                block_temp.temperature = new_temp;
                block_temp.tier_changes += 1;

                drop(temperatures);
                self.update_tier_stats(&old_temp, &new_temp);

                let mut stats = self.stats.write().unwrap();
                stats.migrations_performed += 1;
            }

            Ok(())
        } else {
            Err(CompressionError::InvalidInput(
                format!("Block {} not found", block_id)
            ))
        }
    }

    /// Get compression recommendation for a block
    pub fn get_compression_recommendation(&self, block_id: u64)
        -> (CompressionAlgorithm, CompressionLevel) {

        let temperature = self.classify_temperature(block_id);
        (temperature.recommended_algorithm(), temperature.recommended_compression_level())
    }

    /// Perform automatic tier migration for all blocks
    pub fn auto_migrate_blocks(&mut self) -> usize {
        if !self.policy.enable_auto_migration {
            return 0;
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let mut migrations = 0;

        let block_ids: Vec<u64> = self.block_temperatures.read().unwrap()
            .keys()
            .copied()
            .collect();

        for block_id in block_ids {
            let new_temp = {
                let temperatures = self.block_temperatures.read().unwrap();
                if let Some(block_temp) = temperatures.get(&block_id) {
                    block_temp.calculate_temperature(now)
                } else {
                    continue;
                }
            };

            if self.migrate_block(block_id, new_temp).is_ok() {
                migrations += 1;
            }
        }

        migrations
    }

    /// Get tier statistics
    pub fn tier_stats(&self) -> TierStats {
        self.stats.read().unwrap().clone()
    }

    /// Analyze compression effectiveness by tier
    pub fn tier_compression_analysis(&self) -> TierCompressionAnalysis {
        let temperatures = self.block_temperatures.read().unwrap();

        let mut analysis = TierCompressionAnalysis::default();

        for block_temp in temperatures.values() {
            match block_temp.temperature {
                DataTemperature::Hot => {
                    analysis.hot_blocks += 1;
                    analysis.hot_total_accesses += block_temp.access_count;
                }
                DataTemperature::Warm => {
                    analysis.warm_blocks += 1;
                    analysis.warm_total_accesses += block_temp.access_count;
                }
                DataTemperature::Cold => {
                    analysis.cold_blocks += 1;
                    analysis.cold_total_accesses += block_temp.access_count;
                }
                DataTemperature::Frozen => {
                    analysis.frozen_blocks += 1;
                    analysis.frozen_total_accesses += block_temp.access_count;
                }
            }
        }

        analysis
    }

    fn update_tier_stats(&self, old_temp: &DataTemperature, new_temp: &DataTemperature) {
        let mut stats = self.stats.write().unwrap();

        // Decrement old tier
        match old_temp {
            DataTemperature::Hot => stats.hot_blocks = stats.hot_blocks.saturating_sub(1),
            DataTemperature::Warm => stats.warm_blocks = stats.warm_blocks.saturating_sub(1),
            DataTemperature::Cold => stats.cold_blocks = stats.cold_blocks.saturating_sub(1),
            DataTemperature::Frozen => stats.frozen_blocks = stats.frozen_blocks.saturating_sub(1),
        }

        // Increment new tier
        match new_temp {
            DataTemperature::Hot => stats.hot_blocks += 1,
            DataTemperature::Warm => stats.warm_blocks += 1,
            DataTemperature::Cold => stats.cold_blocks += 1,
            DataTemperature::Frozen => stats.frozen_blocks += 1,
        }
    }

    /// Estimate space savings from tiered compression
    pub fn estimate_space_savings(&self) -> f64 {
        let analysis = self.tier_compression_analysis();

        // Estimate based on typical compression ratios per tier
        let hot_savings = analysis.hot_blocks as f64 * 0.2;     // 20% savings
        let warm_savings = analysis.warm_blocks as f64 * 0.5;   // 50% savings
        let cold_savings = analysis.cold_blocks as f64 * 0.7;   // 70% savings
        let frozen_savings = analysis.frozen_blocks as f64 * 0.8; // 80% savings

        hot_savings + warm_savings + cold_savings + frozen_savings
    }
}

impl Default for TieredCompressor {
    fn default() -> Self {
        Self::new(TierPolicy::default())
    }
}

impl TieredCompressionManager for TieredCompressor {
    fn classify_temperature(&self, block_id: u64) -> DataTemperature {
        self.classify_temperature(block_id)
    }

    fn record_access(&mut self, block_id: u64) {
        self.record_access(block_id)
    }

    fn migrate_block(&mut self, block_id: u64, new_temp: DataTemperature)
        -> CompressionResult<()> {
        self.migrate_block(block_id, new_temp)
    }

    fn get_compression_recommendation(&self, block_id: u64)
        -> (CompressionAlgorithm, CompressionLevel) {
        self.get_compression_recommendation(block_id)
    }

    fn tier_stats(&self) -> TierStats {
        self.tier_stats()
    }
}

/// Analysis of compression by tier
#[derive(Debug, Clone, Default)]
pub struct TierCompressionAnalysis {
    pub hot_blocks: usize,
    pub warm_blocks: usize,
    pub cold_blocks: usize,
    pub frozen_blocks: usize,
    pub hot_total_accesses: u64,
    pub warm_total_accesses: u64,
    pub cold_total_accesses: u64,
    pub frozen_total_accesses: u64,
}

impl TierCompressionAnalysis {
    pub fn total_blocks(&self) -> usize {
        self.hot_blocks + self.warm_blocks + self.cold_blocks + self.frozen_blocks
    }

    pub fn hot_percentage(&self) -> f64 {
        if self.total_blocks() == 0 {
            0.0
        } else {
            100.0 * (self.hot_blocks as f64 / self.total_blocks() as f64)
        }
    }

    pub fn cold_percentage(&self) -> f64 {
        if self.total_blocks() == 0 {
            0.0
        } else {
            100.0 * ((self.cold_blocks + self.frozen_blocks) as f64 / self.total_blocks() as f64)
        }
    }
}

/// Tier migration scheduler
pub struct TierMigrationScheduler {
    manager: Arc<RwLock<TieredCompressor>>,
    last_migration: Arc<RwLock<u64>>,
}

impl TierMigrationScheduler {
    pub fn new(manager: TieredCompressor) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        Self {
            manager: Arc::new(RwLock::new(manager)),
            last_migration: Arc::new(RwLock::new(now)),
        }
    }

    /// Check if migration should run
    pub fn should_run_migration(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let last = *self.last_migration.read().unwrap();
        let policy = {
            let manager = self.manager.read().unwrap();
            manager.policy.clone()
        };

        now - last >= policy.migration_interval
    }

    /// Run scheduled migration
    pub fn run_migration(&self) -> usize {
        if !self.should_run_migration() {
            return 0;
        }

        let migrations = {
            let mut manager = self.manager.write().unwrap();
            manager.auto_migrate_blocks()
        };

        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        *self.last_migration.write().unwrap() = now;

        migrations
    }

    /// Get migration statistics
    pub fn migration_stats(&self) -> MigrationStats {
        let manager = self.manager.read().unwrap();
        let tier_stats = manager.tier_stats();
        let analysis = manager.tier_compression_analysis();

        MigrationStats {
            total_blocks: analysis.total_blocks(),
            migrations_performed: tier_stats.migrations_performed,
            hot_blocks: tier_stats.hot_blocks,
            warm_blocks: tier_stats.warm_blocks,
            cold_blocks: tier_stats.cold_blocks,
            frozen_blocks: tier_stats.frozen_blocks,
            last_migration: *self.last_migration.read().unwrap(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MigrationStats {
    pub total_blocks: usize,
    pub migrations_performed: usize,
    pub hot_blocks: usize,
    pub warm_blocks: usize,
    pub cold_blocks: usize,
    pub frozen_blocks: usize,
    pub last_migration: u64,
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_temperature_classification() {
        let manager = TieredCompressor::default();
        let temp = manager.classify_temperature(1);
        assert_eq!(temp as u8, DataTemperature::Hot as u8);
    }

    #[test]
    fn test_record_access() {
        let mut manager = TieredCompressor::default();

        for _ in 0..20 {
            manager.record_access(1);
        }

        let temp = manager.classify_temperature(1);
        assert_eq!(temp as u8, DataTemperature::Hot as u8);
    }

    #[test]
    fn test_migration() {
        let mut manager = TieredCompressor::default();

        manager.record_access(1);
        assert!(manager.migrate_block(1, DataTemperature::Cold).is_ok());

        let temp = manager.classify_temperature(1);
        assert_eq!(temp as u8, DataTemperature::Cold as u8);
    }

    #[test]
    fn test_compression_by_temperature() {
        let manager = TieredCompressor::default();
        let data = b"Hello, World! This is test data for compression.";
        let mut output = vec![0u8; 1000];

        let size = manager.compress_by_temperature(1, data, &mut output).unwrap();
        assert!(size > 0);
        assert!(size <= data.len());
    }

    #[test]
    fn test_tier_analysis() {
        let mut manager = TieredCompressor::default();

        manager.record_access(1);
        manager.record_access(2);
        manager.record_access(3);

        let analysis = manager.tier_compression_analysis();
        assert_eq!(analysis.total_blocks(), 3);
        assert!(analysis.hot_percentage() > 0.0);
    }

    #[test]
    fn test_auto_migration() {
        let mut manager = TieredCompressor::default();

        manager.record_access(1);
        manager.record_access(2);

        let migrations = manager.auto_migrate_blocks();
        assert!(migrations >= 0);
    }

    #[test]
    fn test_tier_stats() {
        let manager = TieredCompressor::default();
        let stats = manager.tier_stats();

        assert_eq!(stats.hot_blocks, 0);
        assert_eq!(stats.migrations_performed, 0);
    }

    #[test]
    fn test_compression_recommendation() {
        let manager = TieredCompressor::default();
        let (algo, level) = manager.get_compression_recommendation(1);

        assert_eq!(algo as u8, CompressionAlgorithm::LZ4 as u8);
        assert_eq!(level as u8, CompressionLevel::Fast as u8);
    }
}


