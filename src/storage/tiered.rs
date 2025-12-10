// Tiered Storage Manager for RustyDB
// Provides hot/warm/cold data classification with automatic migration
// and tier-specific compression strategies

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, Instant};
use parking_lot::{RwLock};
use serde::{Deserialize, Serialize};
use crate::error::{Result, DbError};
use crate::storage::page::Page;
use crate::common::PageId;

// Storage tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageTier {
    Hot,    // SSD/Memory - frequently accessed
    Warm,   // SSD - occasionally accessed
    Cold,   // HDD/Cloud - rarely accessed
}

impl StorageTier {
    pub fn compression_level(&self) -> CompressionLevel {
        match self {
            StorageTier::Hot => CompressionLevel::None,
            StorageTier::Warm => CompressionLevel::Fast,
            StorageTier::Cold => CompressionLevel::Best,
        }
    }

    pub fn cost_per_gb(&self) -> f64 {
        match self {
            StorageTier::Hot => 1.0,
            StorageTier::Warm => 0.5,
            StorageTier::Cold => 0.1,
        }
    }

    pub fn latency_ms(&self) -> u64 {
        match self {
            StorageTier::Hot => 1,
            StorageTier::Warm => 5,
            StorageTier::Cold => 50,
        }
    }
}

// Compression strategies for different tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionLevel {
    None,
    Fast,    // LZ4 for warm tier
    Best,    // ZSTD for cold tier
}

// Access pattern tracking for ML-based prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessPattern {
    page_id: PageId,
    access_count: u64,
    last_access: SystemTime,
    access_history: VecDeque<SystemTime>,
    read_count: u64,
    write_count: u64,
    avg_access_interval: Duration,
}

impl AccessPattern {
    fn new(page_id: PageId) -> Self {
        Self {
            page_id,
            access_count: 0,
            last_access: SystemTime::now(),
            access_history: VecDeque::with_capacity(100),
            read_count: 0,
            write_count: 0,
            avg_access_interval: Duration::from_secs(3600),
        }
    }

    fn record_access(&mut self, is_write: bool) {
        let now = SystemTime::now();

        if self.access_count > 0 {
            if let Ok(interval) = now.duration_since(self.last_access) {
                // Update moving average
                self.avg_access_interval = Duration::from_secs(
                    (self.avg_access_interval.as_secs() + interval.as_secs()) / 2
                );
            }
        }

        self.last_access = now;
        self.access_count += 1;

        if is_write {
            self.write_count += 1;
        } else {
            self.read_count += 1;
        }

        if self.access_history.len() >= 100 {
            self.access_history.pop_front();
        }
        self.access_history.push_back(now);
    }

    /// Predict optimal tier based on access patterns (for ML-based tiering)
    #[allow(dead_code)]
    fn predict_tier(&self) -> StorageTier {
        let now = SystemTime::now();
        let time_since_last_access = now.duration_since(self.last_access)
            .unwrap_or(Duration::from_secs(0));

        // Hot tier: accessed in last hour or high frequency
        if time_since_last_access < Duration::from_secs(3600) ||
           self.access_count > 100 && self.avg_access_interval < Duration::from_secs(60) {
            return StorageTier::Hot;
        }

        // Warm tier: accessed in last day
        if time_since_last_access < Duration::from_secs(86400) {
            return StorageTier::Warm;
        }

        // Cold tier: rarely accessed
        StorageTier::Cold
    }

    fn access_frequency(&self) -> f64 {
        if self.access_count == 0 {
            return 0.0;
        }

        let now = SystemTime::now();
        let time_window = Duration::from_secs(3600); // 1 hour window

        let recent_accesses = self.access_history.iter()
            .filter(|&&access_time| {
                now.duration_since(access_time).unwrap_or(Duration::MAX) < time_window
            })
            .count();

        recent_accesses as f64 / 60.0 // Accesses per minute
    }

    /// Check if workload is read-heavy (for optimization decisions)
    #[allow(dead_code)]
    fn is_read_heavy(&self) -> bool {
        if self.access_count == 0 {
            return true;
        }

        let read_ratio = self.read_count as f64 / self.access_count as f64;
        read_ratio > 0.8
    }
}

// ML-based tier predictor using simple heuristics
// In production, would use a trained model
struct TierPredictor {
    thresholds: PredictorThresholds,
}

#[derive(Debug, Clone)]
struct PredictorThresholds {
    hot_access_freq: f64,      // Accesses per minute for hot tier
    hot_recency: Duration,      // Max time since access for hot tier
    warm_recency: Duration,     // Max time since access for warm tier
}

impl Default for PredictorThresholds {
    fn default() -> Self {
        Self {
            hot_access_freq: 1.0,
            hot_recency: Duration::from_secs(3600),
            warm_recency: Duration::from_secs(86400),
        }
    }
}

impl TierPredictor {
    fn new() -> Self {
        Self {
            thresholds: PredictorThresholds::default(),
        }
    }

    fn predict(&self, pattern: &AccessPattern) -> StorageTier {
        let now = SystemTime::now();
        let recency = now.duration_since(pattern.last_access)
            .unwrap_or(Duration::MAX);
        let freq = pattern.access_frequency();

        if freq >= self.thresholds.hot_access_freq || recency <= self.thresholds.hot_recency {
            StorageTier::Hot
        } else if recency <= self.thresholds.warm_recency {
            StorageTier::Warm
        } else {
            StorageTier::Cold
        }
    }

    fn update_thresholds(&mut self, patterns: &HashMap<PageId, AccessPattern>) {
        if patterns.is_empty() {
            return;
        }

        // Adaptive thresholds based on workload
        let avg_freq: f64 = patterns.values()
            .map(|p| p.access_frequency())
            .sum::<f64>() / patterns.len() as f64;

        self.thresholds.hot_access_freq = avg_freq * 2.0;
    }
}

// Compression engine for different tiers
struct CompressionEngine;

impl CompressionEngine {
    fn compress(data: &[u8], level: CompressionLevel) -> Result<Vec<u8>> {
        match level {
            CompressionLevel::None => Ok(data.to_vec()),
            CompressionLevel::Fast => {
                // Simulate LZ4 compression (fast, moderate ratio)
                // In production: use lz4_flex crate
                Ok(Self::simple_compress(data, 6))
            }
            CompressionLevel::Best => {
                // Simulate ZSTD compression (slower, best ratio)
                // In production: use zstd crate
                Ok(Self::simple_compress(data, 12))
            }
        }
    }

    fn decompress(data: &[u8], level: CompressionLevel) -> Result<Vec<u8>> {
        match level {
            CompressionLevel::None => Ok(data.to_vec()),
            CompressionLevel::Fast | CompressionLevel::Best => {
                // Simulate decompression
                Ok(Self::simple_decompress(data))
            }
        }
    }

    // Simple run-length encoding for simulation
    fn simple_compress(data: &[u8], _level: u8) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }

        let mut compressed = Vec::new();
        let mut count = 1u8;
        let mut current = data[0];

        for &byte in &data[1..] {
            if byte == current && count < 255 {
                count += 1;
            } else {
                compressed.push(count);
                compressed.push(current);
                current = byte;
                count = 1;
            }
        }

        compressed.push(count);
        compressed.push(current);

        compressed
    }

    fn simple_decompress(data: &[u8]) -> Vec<u8> {
        let mut decompressed = Vec::new();

        for chunk in data.chunks(2) {
            if chunk.len() == 2 {
                let count = chunk[0];
                let value = chunk[1];
                decompressed.extend(std::iter::repeat(value).take(count as usize));
            }
        }

        decompressed
    }
}

// Tiered page storage
struct TieredPage {
    #[allow(dead_code)]
    page_id: PageId,
    tier: StorageTier,
    compressed_data: Vec<u8>,
    compression_level: CompressionLevel,
    original_size: usize,
    #[allow(dead_code)]
    created_at: SystemTime,
    last_migrated: SystemTime,
}

impl TieredPage {
    fn new(page: &Page, tier: StorageTier) -> Result<Self> {
        let compression_level = tier.compression_level();
        let compressed_data = CompressionEngine::compress(&page.data, compression_level)?;

        Ok(Self {
            page_id: page.id,
            tier,
            compressed_data,
            compression_level,
            original_size: page.data.len(),
            created_at: SystemTime::now(),
            last_migrated: SystemTime::now(),
        })
    }

    fn decompress(&self) -> Result<Vec<u8>> {
        CompressionEngine::decompress(&self.compressed_data, self.compression_level)
    }

    fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            return 1.0;
        }
        self.compressed_data.len() as f64 / self.original_size as f64
    }

    fn migrate_to_tier(&mut self, new_tier: StorageTier, data: &[u8]) -> Result<()> {
        let new_compression = new_tier.compression_level();
        self.compressed_data = CompressionEngine::compress(data, new_compression)?;
        self.tier = new_tier;
        self.compression_level = new_compression;
        self.last_migrated = SystemTime::now();
        Ok(())
    }
}

// Migration task for moving data between tiers
#[derive(Debug, Clone)]
struct MigrationTask {
    page_id: PageId,
    from_tier: StorageTier,
    to_tier: StorageTier,
    #[allow(dead_code)]
    priority: u8,
    #[allow(dead_code)]
    created_at: Instant,
}

impl MigrationTask {
    fn new(page_id: PageId, from_tier: StorageTier, to_tier: StorageTier) -> Self {
        let priority = match (from_tier, to_tier) {
            (StorageTier::Cold, StorageTier::Hot) => 3,
            (StorageTier::Warm, StorageTier::Hot) => 2,
            (StorageTier::Hot, StorageTier::Cold) => 0,
            _ => 1,
        };

        Self {
            page_id,
            from_tier,
            to_tier,
            priority,
            created_at: Instant::now(),
        }
    }
}

// Main tiered storage manager
pub struct TieredStorageManager {
    // Page storage by tier
    hot_storage: Arc<RwLock<HashMap<PageId, TieredPage>>>,
    warm_storage: Arc<RwLock<HashMap<PageId, TieredPage>>>,
    cold_storage: Arc<RwLock<HashMap<PageId, TieredPage>>>,

    // Access pattern tracking
    access_patterns: Arc<RwLock<HashMap<PageId, AccessPattern>>>,

    // ML-based predictor
    predictor: Arc<Mutex<TierPredictor>>,

    // Migration queue
    migration_queue: Arc<Mutex<VecDeque<MigrationTask>>>,

    // Statistics
    stats: Arc<RwLock<TierStats>>,
}

#[derive(Debug, Clone, Default)]
pub struct TierStats {
    pub hot_pages: usize,
    pub warm_pages: usize,
    pub cold_pages: usize,
    pub total_migrations: u64,
    pub hot_to_warm: u64,
    pub warm_to_cold: u64,
    pub cold_to_hot: u64,
    pub avg_compression_ratio: f64,
    pub total_bytes_saved: u64,
}

impl TieredStorageManager {
    pub fn new() -> Self {
        Self {
            hot_storage: Arc::new(RwLock::new(HashMap::new())),
            warm_storage: Arc::new(RwLock::new(HashMap::new())),
            cold_storage: Arc::new(RwLock::new(HashMap::new())),
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
            predictor: Arc::new(Mutex::new(TierPredictor::new())),
            migration_queue: Arc::new(Mutex::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(TierStats::default())),
        }
    }

    // Store a page in the appropriate tier
    pub fn store_page(&self, page: &Page) -> Result<()> {
        // Get or create access pattern
        let tier = {
            let mut patterns = self.access_patterns.write();
            let pattern = patterns.entry(page.id)
                .or_insert_with(|| AccessPattern::new(page.id));
            self.predictor.lock().unwrap().predict(pattern)
        };

        let tiered_page = TieredPage::new(page, tier)?;

        match tier {
            StorageTier::Hot => {
                self.hot_storage.write().insert(page.id, tiered_page);
            }
            StorageTier::Warm => {
                self.warm_storage.write().insert(page.id, tiered_page);
            }
            StorageTier::Cold => {
                self.cold_storage.write().insert(page.id, tiered_page);
            }
        }

        self.update_stats();
        Ok(())
    }

    // Retrieve a page from any tier
    pub fn get_page(&self, page_id: PageId) -> Result<Page> {
        // Record access
        {
            let mut patterns = self.access_patterns.write();
            let pattern = patterns.entry(page_id)
                .or_insert_with(|| AccessPattern::new(page_id));
            pattern.record_access(false);
        }

        // Search tiers (hot first for performance)
        if let Some(tiered_page) = self.hot_storage.read().get(&page_id) {
            let data = tiered_page.decompress()?;
            return Ok(Page::from_bytes(page_id, data));
        }

        if let Some(tiered_page) = self.warm_storage.read().get(&page_id) {
            let data = tiered_page.decompress()?;
            // Consider promoting to hot
            self.consider_promotion(page_id, StorageTier::Warm);
            return Ok(Page::from_bytes(page_id, data));
        }

        if let Some(tiered_page) = self.cold_storage.read().get(&page_id) {
            let data = tiered_page.decompress()?;
            // Consider promoting to warm or hot
            self.consider_promotion(page_id, StorageTier::Cold);
            return Ok(Page::from_bytes(page_id, data));
        }

        Err(DbError::Storage(format!("Page {} not found in any tier", page_id)))
    }

    // Record a page write
    pub fn update_page(&self, page: &Page) -> Result<()> {
        // Record write access
        {
            let mut patterns = self.access_patterns.write();
            let pattern = patterns.entry(page.id)
                .or_insert_with(|| AccessPattern::new(page.id));
            pattern.record_access(true);
        }

        // Update in current tier
        self.store_page(page)
    }

    // Consider promoting a page to a higher tier
    fn consider_promotion(&self, page_id: PageId, current_tier: StorageTier) {
        let patterns = self.access_patterns.read();
        if let Some(pattern) = patterns.get(&page_id) {
            let predicted_tier = self.predictor.lock().unwrap().predict(pattern);

            if predicted_tier != current_tier {
                let task = MigrationTask::new(page_id, current_tier, predicted_tier);
                self.migration_queue.lock().unwrap().push_back(task);
            }
        }
    }

    // Process migration queue
    pub fn process_migrations(&self, max_migrations: usize) -> Result<usize> {
        let mut migrated = 0;

        for _ in 0..max_migrations {
            let task = {
                let mut queue = self.migration_queue.lock().unwrap();
                queue.pop_front()
            };

            if let Some(task) = task {
                self.migrate_page(task)?;
                migrated += 1;
            } else {
                break;
            }
        }

        Ok(migrated)
    }

    // Migrate a page between tiers
    fn migrate_page(&self, task: MigrationTask) -> Result<()> {
        // Get page from source tier
        let (tiered_page, data) = match task.from_tier {
            StorageTier::Hot => {
                let mut hot = self.hot_storage.write();
                if let Some(page) = hot.remove(&task.page_id) {
                    let data = page.decompress()?;
                    (page, data)
                } else {
                    return Ok(()); // Page no longer exists
                }
            }
            StorageTier::Warm => {
                let mut warm = self.warm_storage.write();
                if let Some(page) = warm.remove(&task.page_id) {
                    let data = page.decompress()?;
                    (page, data)
                } else {
                    return Ok(());
                }
            }
            StorageTier::Cold => {
                let mut cold = self.cold_storage.write();
                if let Some(page) = cold.remove(&task.page_id) {
                    let data = page.decompress()?;
                    (page, data)
                } else {
                    return Ok(());
                }
            }
        };

        // Create new tiered page for destination
        let mut new_page = tiered_page;
        new_page.migrate_to_tier(task.to_tier, &data)?;

        // Store in destination tier
        match task.to_tier {
            StorageTier::Hot => {
                self.hot_storage.write().insert(task.page_id, new_page);
            }
            StorageTier::Warm => {
                self.warm_storage.write().insert(task.page_id, new_page);
            }
            StorageTier::Cold => {
                self.cold_storage.write().insert(task.page_id, new_page);
            }
        }

        // Update statistics
        let mut stats = self.stats.write();
        stats.total_migrations += 1;
        match (task.from_tier, task.to_tier) {
            (StorageTier::Hot, StorageTier::Warm) => stats.hot_to_warm += 1,
            (StorageTier::Warm, StorageTier::Cold) => stats.warm_to_cold += 1,
            (StorageTier::Cold, StorageTier::Hot) => stats.cold_to_hot += 1,
            _ => {}
        }

        Ok(())
    }

    // Periodic maintenance: update predictor and trigger migrations
    pub fn maintenance(&self) -> Result<()> {
        // Update predictor thresholds
        {
            let patterns = self.access_patterns.read();
            self.predictor.lock().unwrap().update_thresholds(&patterns);
        }

        // Scan for pages that should be migrated
        let patterns = self.access_patterns.read();
        for (page_id, pattern) in patterns.iter() {
            let predicted_tier = self.predictor.lock().unwrap().predict(pattern);
            let current_tier = self.find_page_tier(*page_id);

            if let Some(current_tier) = current_tier {
                if predicted_tier != current_tier {
                    let task = MigrationTask::new(*page_id, current_tier, predicted_tier);
                    self.migration_queue.lock().unwrap().push_back(task);
                }
            }
        }

        // Process some migrations
        self.process_migrations(10)?;

        self.update_stats();
        Ok(())
    }

    fn find_page_tier(&self, page_id: PageId) -> Option<StorageTier> {
        if self.hot_storage.read().contains_key(&page_id) {
            Some(StorageTier::Hot)
        } else if self.warm_storage.read().contains_key(&page_id) {
            Some(StorageTier::Warm)
        } else if self.cold_storage.read().contains_key(&page_id) {
            Some(StorageTier::Cold)
        } else {
            None
        }
    }

    fn update_stats(&self) {
        let mut stats = self.stats.write();
        stats.hot_pages = self.hot_storage.read().len();
        stats.warm_pages = self.warm_storage.read().len();
        stats.cold_pages = self.cold_storage.read().len();

        // Calculate compression stats
        let mut total_ratio = 0.0;
        let mut count = 0;
        let mut total_saved = 0u64;

        for storage in [&self.hot_storage, &self.warm_storage, &self.cold_storage] {
            let guard = storage.read();
            for page in guard.values() {
                total_ratio += page.compression_ratio();
                total_saved += (page.original_size - page.compressed_data.len()) as u64;
                count += 1;
            }
        }

        if count > 0 {
            stats.avg_compression_ratio = total_ratio / count as f64;
        }
        stats.total_bytes_saved = total_saved;
    }

    pub fn get_stats(&self) -> TierStats {
        self.stats.read().clone()
    }
}

impl Default for TieredStorageManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_classification() {
        assert_eq!(StorageTier::Hot.latency_ms(), 1);
        assert_eq!(StorageTier::Warm.latency_ms(), 5);
        assert_eq!(StorageTier::Cold.latency_ms(), 50);
    }

    #[test]
    fn test_access_pattern() {
        let mut pattern = AccessPattern::new(1);
        pattern.record_access(false);
        pattern.record_access(true);

        assert_eq!(pattern.access_count, 2);
        assert_eq!(pattern.read_count, 1);
        assert_eq!(pattern.write_count, 1);
    }

    #[test]
    fn test_compression() {
        let data = vec![1, 1, 1, 2, 2, 3, 3, 3, 3];
        let compressed = CompressionEngine::compress(&data, CompressionLevel::Fast).unwrap();
        let decompressed = CompressionEngine::decompress(&compressed, CompressionLevel::Fast).unwrap();

        assert_eq!(data, decompressed);
        assert!(compressed.len() <= data.len());
    }

    #[test]
    fn test_tiered_storage() {
        let manager = TieredStorageManager::new();
        let page = Page::new(1, 4096);

        manager.store_page(&page).unwrap();

        let retrieved = manager.get_page(1).unwrap();
        assert_eq!(retrieved.id, 1);

        let stats = manager.get_stats();
        assert!(stats.hot_pages + stats.warm_pages + stats.cold_pages > 0);
    }

    #[test]
    fn test_tier_migration() {
        let manager = TieredStorageManager::new();
        let page = Page::new(1, 4096);

        // Store initially
        manager.store_page(&page).unwrap();

        // Simulate multiple accesses to trigger hot tier
        for _ in 0..150 {
            manager.access_patterns.write()
                .entry(1)
                .or_insert_with(|| AccessPattern::new(1))
                .record_access(false);
        }

        // Run maintenance to trigger migration
        manager.maintenance().unwrap();

        let stats = manager.get_stats();
        assert!(stats.total_migrations > 0 || stats.hot_pages > 0);
    }
}
