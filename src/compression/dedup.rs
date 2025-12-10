// Deduplication Engine - Block-level and content-defined deduplication
// Eliminates duplicate data across tables and databases

use super::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use std::time::UNIX_EPOCH;
use std::time::SystemTime;

// Chunk store entry
#[derive(Debug, Clone)]
pub struct ChunkEntry {
    pub chunk_id: Vec<u8>,
    pub chunk_hash: u64,
    pub chunk_data: Vec<u8>,
    pub reference_count: usize,
    pub first_seen: u64,
    pub last_accessed: u64,
    pub size: usize,
}

// Deduplication metadata
#[derive(Debug, Clone)]
pub struct DedupMetadata {
    pub chunk_map: Vec<ChunkReference>,
    pub original_size: usize,
    pub deduplicated_size: usize,
    pub unique_chunks: usize,
    pub duplicate_chunks: usize,
}

// Reference to a chunk in the chunk store
#[derive(Debug, Clone)]
pub struct ChunkReference {
    pub chunk_id: Vec<u8>,
    pub offset: usize,
    pub length: usize,
}

// Content-Defined Chunking parameters
#[derive(Debug, Clone)]
pub struct ChunkingParams {
    pub min_chunk_size: usize,
    pub avg_chunk_size: usize,
    pub max_chunk_size: usize,
    pub window_size: usize,
    pub mask_bits: usize,
}

impl Default for ChunkingParams {
    fn default() -> Self {
        Self {
            min_chunk_size: 2 * 1024,      // 2KB
            avg_chunk_size: 8 * 1024,      // 8KB
            max_chunk_size: 64 * 1024,     // 64KB
            window_size: 48,
            mask_bits: 13,                  // Target ~8KB chunks
        }
    }
}

// Deduplication Engine
pub struct DedupEngine {
    chunk_store: Arc<RwLock<HashMap<u64, ChunkEntry>>>,
    chunk_index: Arc<RwLock<HashMap<u64, Vec<u8>>>>,
    params: ChunkingParams,
    stats: Arc<RwLock<DedupStats>>,
    enable_compression: bool,
    inline_dedup: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DedupStats {
    pub total_bytes_processed: usize,
    pub unique_bytes: usize,
    pub duplicate_bytes: usize,
    pub total_chunks: usize,
    pub unique_chunks: usize,
    pub duplicate_chunks: usize,
    pub chunk_store_size: usize,
}

impl DedupStats {
    pub fn dedup_ratio(&self) -> f64 {
        if self.unique_bytes == 0 {
            0.0
        } else {
            self.total_bytes_processed as f64 / self.unique_bytes as f64
        }
    }

    pub fn space_saved_bytes(&self) -> usize {
        self.duplicate_bytes
    }

    pub fn space_saved_percent(&self) -> f64 {
        if self.total_bytes_processed == 0 {
            0.0
        } else {
            100.0 * (self.duplicate_bytes as f64 / self.total_bytes_processed as f64)
        }
    }
}

impl DedupEngine {
    pub fn new() -> Self {
        Self::with_params(ChunkingParams::default())
    }

    pub fn with_params(params: ChunkingParams) -> Self {
        Self {
            chunk_store: Arc::new(RwLock::new(HashMap::new())),
            chunk_index: Arc::new(RwLock::new(HashMap::new())),
            params,
            stats: Arc::new(RwLock::new(DedupStats::default())),
            enable_compression: true,
            inline_dedup: true,
        }
    }

    pub fn with_compression(mut self, enable: bool) -> Self {
        self.enable_compression = enable;
        self
    }

    pub fn with_inline_dedup(mut self, enable: bool) -> Self {
        self.inline_dedup = enable;
        self
    }

    // Deduplicate data using content-defined chunking
    pub fn deduplicate(&mut self, data: &[u8]) -> CompressionResult<DedupMetadata> {
        let _start = Instant::now();

        if data.is_empty() {
            return Ok(DedupMetadata {
                chunk_map: Vec::new(),
                original_size: 0,
                deduplicated_size: 0,
                unique_chunks: 0,
                duplicate_chunks: 0,
            });
        }

        // Find chunk boundaries using content-defined chunking
        let boundaries = self.find_chunk_boundaries(data);
        let mut chunk_map = Vec::new();
        let mut unique_chunks = 0;
        let mut duplicate_chunks = 0;

        let mut prev_boundary = 0;

        for &boundary in &boundaries {
            if boundary <= prev_boundary {
                continue;
            }

            let chunk = &data[prev_boundary..boundary];
            let chunk_hash = self.compute_chunk_hash(chunk);

            // Check if chunk already exists
            let (chunk_id, _is_new) = if self.is_duplicate(chunk_hash) {
                duplicate_chunks += 1;
                let chunk_id = self.chunk_index.read().unwrap()
                    .get(&chunk_hash)
                    .cloned()
                    .unwrap();

                // Increment reference count
                self.increment_reference(chunk_hash);

                (chunk_id, false)
            } else {
                unique_chunks += 1;
                let chunk_id = self.store_chunk(chunk, chunk_hash)?;
                (chunk_id, true)
            };

            chunk_map.push(ChunkReference {
                chunk_id,
                offset: prev_boundary,
                length: boundary - prev_boundary,
            });

            prev_boundary = boundary;
        }

        // Handle remaining data
        if prev_boundary < data.len() {
            let chunk = &data[prev_boundary..];
            let chunk_hash = self.compute_chunk_hash(chunk);

            let (chunk_id, _is_new) = if self.is_duplicate(chunk_hash) {
                duplicate_chunks += 1;
                let chunk_id = self.chunk_index.read().unwrap()
                    .get(&chunk_hash)
                    .cloned()
                    .unwrap();
                self.increment_reference(chunk_hash);
                (chunk_id, false)
            } else {
                unique_chunks += 1;
                let chunk_id = self.store_chunk(chunk, chunk_hash)?;
                (chunk_id, true)
            };

            chunk_map.push(ChunkReference {
                chunk_id,
                offset: prev_boundary,
                length: data.len() - prev_boundary,
            });
        }

        // Calculate sizes
        let deduplicated_size = self.calculate_deduplicated_size(&chunk_map);

        // Update statistics
        let mut stats = self.stats.write().unwrap();
        stats.total_bytes_processed += data.len();
        stats.total_chunks += chunk_map.len();
        stats.unique_chunks += unique_chunks;
        stats.duplicate_chunks += duplicate_chunks;

        let metadata = DedupMetadata {
            chunk_map,
            original_size: data.len(),
            deduplicated_size,
            unique_chunks,
            duplicate_chunks,
        };

        Ok(metadata)
    }

    // Restore deduplicated data
    pub fn restore(&self, metadata: &DedupMetadata) -> CompressionResult<Vec<u8>> {
        let mut restored = Vec::with_capacity(metadata.original_size);

        for chunk_ref in &metadata.chunk_map {
            let chunk_data = self.retrieve_chunk(&chunk_ref.chunk_id)?;
            restored.extend_from_slice(&chunk_data);
        }

        if restored.len() != metadata.original_size {
            return Err(CompressionError::DecompressionFailed(
                format!("Size mismatch: expected {}, got {}", metadata.original_size, restored.len())
            ).into());
        }

        Ok(restored)
    }

    // Find chunk boundaries using Rabin fingerprinting
    pub fn find_chunk_boundaries(&self, data: &[u8]) -> Vec<usize> {
        let mut boundaries = Vec::new();
        let mask = (1u64 << self.params.mask_bits) - 1;

        if data.len() <= self.params.min_chunk_size {
            boundaries.push(data.len());
            return boundaries;
        }

        let mut hash = 0u64;
        let mut pos = 0;
        let mut last_boundary = 0;

        while pos < data.len() {
            // Rolling hash (simplified Rabin fingerprint)
            hash = hash.wrapping_mul(31).wrapping_add(data[pos] as u64);

            pos += 1;
            let chunk_size = pos - last_boundary;

            // Check for boundary conditions
            if chunk_size >= self.params.min_chunk_size {
                if (hash & mask) == 0 || chunk_size >= self.params.max_chunk_size {
                    boundaries.push(pos);
                    last_boundary = pos;
                    hash = 0;
                }
            }
        }

        // Add final boundary
        if last_boundary < data.len() {
            boundaries.push(data.len());
        }

        boundaries
    }

    // Compute hash for a chunk (using xxHash-like algorithm)
    pub fn compute_chunk_hash(&self, data: &[u8]) -> u64 {
        const PRIME1: u64 = 11400714785074694791;
        const PRIME2: u64 = 14029467366897019727;
        const PRIME3: u64 = 1609587929392839161;
        const PRIME5: u64 = 2870177450012600261;

        let mut hash = PRIME5;

        for &byte in data {
            hash ^= (byte as u64).wrapping_mul(PRIME5);
            hash = hash.rotate_left(11).wrapping_mul(PRIME1);
        }

        hash ^= hash >> 33;
        hash = hash.wrapping_mul(PRIME2);
        hash ^= hash >> 29;
        hash = hash.wrapping_mul(PRIME3);
        hash ^= hash >> 32;

        hash
    }

    // Check if a chunk is a duplicate
    pub fn is_duplicate(&self, chunk_hash: u64) -> bool {
        self.chunk_index.read().unwrap().contains_key(&chunk_hash)
    }

    // Store a chunk and return its identifier
    pub fn store_chunk(&mut self, chunk: &[u8], hash: u64) -> CompressionResult<Vec<u8>> {
        let chunk_id = self.generate_chunk_id(hash);

        let entry = ChunkEntry {
            chunk_id: chunk_id.clone(),
            chunk_hash: hash,
            chunk_data: chunk.to_vec(),
            reference_count: 1,
            first_seen: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_accessed: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            size: chunk.len(),
        };

        self.chunk_store.write().unwrap().insert(hash, entry);
        self.chunk_index.write().unwrap().insert(hash, chunk_id.clone());

        let mut stats = self.stats.write().unwrap();
        stats.unique_bytes += chunk.len();
        stats.chunk_store_size += chunk.len();

        Ok(chunk_id)
    }

    // Retrieve a chunk by its identifier
    pub fn retrieve_chunk(&self, chunk_id: &[u8]) -> CompressionResult<Vec<u8>> {
        let chunk_store = self.chunk_store.read().unwrap();

        for entry in chunk_store.values() {
            if entry.chunk_id == chunk_id {
                return Ok(entry.chunk_data.clone());
            }
        }

        Err(CompressionError::InvalidInput(
            format!("Chunk not found: {:?}", chunk_id)
        ))
    }

    fn increment_reference(&self, chunk_hash: u64) {
        if let Some(entry) = self.chunk_store.write().unwrap().get_mut(&chunk_hash) {
            entry.reference_count += 1;
            entry.last_accessed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let mut stats = self.stats.write().unwrap();
            stats.duplicate_bytes += entry.size;
        }
    }

    fn generate_chunk_id(&self, hash: u64) -> Vec<u8> {
        hash.to_le_bytes().to_vec()
    }

    fn calculate_deduplicated_size(&self, chunk_map: &[ChunkReference]) -> usize {
        let mut seen_chunks = HashSet::new();
        let mut size = 0;

        for chunk_ref in chunk_map {
            if seen_chunks.insert(chunk_ref.chunk_id.clone()) {
                size += chunk_ref.length;
            }
        }

        size
    }

    // Get deduplication ratio
    pub fn dedup_ratio(&self) -> f64 {
        self.stats.read().unwrap().dedup_ratio()
    }

    // Get deduplication statistics
    pub fn stats(&self) -> DedupStats {
        self.stats.read().unwrap().clone()
    }

    // Garbage collection - remove chunks with zero references
    pub fn garbage_collect(&mut self) -> usize {
        let mut removed = 0;
        let mut to_remove = Vec::new();

        {
            let chunk_store = self.chunk_store.read().unwrap();
            for (hash, entry) in chunk_store.iter() {
                if entry.reference_count == 0 {
                    to_remove.push(*hash);
                }
            }
        }

        {
            let mut chunk_store = self.chunk_store.write().unwrap();
            let mut chunk_index = self.chunk_index.write().unwrap();
            let mut stats = self.stats.write().unwrap();

            for hash in to_remove {
                if let Some(entry) = chunk_store.remove(&hash) {
                    chunk_index.remove(&hash);
                    stats.chunk_store_size -= entry.size;
                    removed += 1;
                }
            }
        }

        removed
    }

    // Decrement reference count for chunks
    pub fn decrement_references(&mut self, metadata: &DedupMetadata) {
        for chunk_ref in &metadata.chunk_map {
            let chunk_hash = {
                let chunk_store = self.chunk_store.read().unwrap();
                let mut hash = None;
                for entry in chunk_store.values() {
                    if entry.chunk_id == chunk_ref.chunk_id {
                        hash = Some(entry.chunk_hash);
                        break;
                    }
                }
                hash
            };

            if let Some(hash) = chunk_hash {
                if let Some(entry) = self.chunk_store.write().unwrap().get_mut(&hash) {
                    if entry.reference_count > 0 {
                        entry.reference_count -= 1;
                    }
                }
            }
        }
    }
}

impl Default for DedupEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Deduplicator for DedupEngine {
    fn compute_chunk_hash(&self, data: &[u8]) -> u64 {
        self.compute_chunk_hash(data)
    }

    fn find_chunk_boundaries(&self, data: &[u8]) -> Vec<usize> {
        self.find_chunk_boundaries(data)
    }

    fn is_duplicate(&self, chunk_hash: u64) -> bool {
        self.is_duplicate(chunk_hash)
    }

    fn store_chunk(&mut self, chunk: &[u8], hash: u64) -> CompressionResult<Vec<u8>> {
        self.store_chunk(chunk, hash)
    }

    fn retrieve_chunk(&self, chunk_id: &[u8]) -> CompressionResult<Vec<u8>> {
        self.retrieve_chunk(chunk_id)
    }

    fn dedup_ratio(&self) -> f64 {
        self.dedup_ratio()
    }
}

// Cross-table deduplication manager
pub struct CrossTableDedup {
    dedup_engine: DedupEngine,
    table_metadata: Arc<RwLock<HashMap<u64, TableDedupMetadata>>>,
}

#[derive(Debug, Clone)]
pub struct TableDedupMetadata {
    pub table_id: u64,
    pub total_size: usize,
    pub deduplicated_size: usize,
    pub chunks: Vec<DedupMetadata>,
}

impl CrossTableDedup {
    pub fn new() -> Self {
        Self {
            dedup_engine: DedupEngine::new(),
            table_metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // Deduplicate a table's data
    pub fn deduplicate_table(&mut self, table_id: u64, data: Vec<Vec<u8>>)
        -> CompressionResult<Vec<DedupMetadata>> {

        let mut chunks = Vec::new();
        let mut total_size = 0;
        let mut deduplicated_size = 0;

        for row in data {
            total_size += row.len();
            let metadata = self.dedup_engine.deduplicate(&row)?;
            deduplicated_size += metadata.deduplicated_size;
            chunks.push(metadata);
        }

        let table_metadata = TableDedupMetadata {
            table_id,
            total_size,
            deduplicated_size,
            chunks: chunks.clone(),
        };

        self.table_metadata.write().unwrap().insert(table_id, table_metadata);

        Ok(chunks)
    }

    // Get deduplication ratio for a table
    pub fn table_dedup_ratio(&self, table_id: u64) -> Option<f64> {
        self.table_metadata.read().unwrap().get(&table_id).map(|metadata| {
            if metadata.deduplicated_size == 0 {
                0.0
            } else {
                metadata.total_size as f64 / metadata.deduplicated_size as f64
            }
        })
    }

    // Get overall deduplication statistics
    pub fn overall_stats(&self) -> DedupStats {
        self.dedup_engine.stats()
    }
}

impl Default for CrossTableDedup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::compression::dedup::{CrossTableDedup, DedupEngine};

    #[test]
    fn test_chunk_boundaries() {
        let engine = DedupEngine::new();
        let data = vec![0u8; 100000];
        let boundaries = engine.find_chunk_boundaries(&data);
        assert!(!boundaries.is_empty());
    }

    #[test]
    fn test_deduplication() {
        let mut engine = DedupEngine::new();

        let data1 = b"Hello, World! This is a test.".to_vec();
        let data2 = b"Hello, World! This is a test.".to_vec();

        let metadata1 = engine.deduplicate(&data1).unwrap();
        let metadata2 = engine.deduplicate(&data2).unwrap();

        assert!(metadata2.duplicate_chunks > 0);

        let restored = engine.restore(&metadata1).unwrap();
        assert_eq!(restored, data1);
    }

    #[test]
    fn test_chunk_hash() {
        let engine = DedupEngine::new();
        let data = b"test data";

        let hash1 = engine.compute_chunk_hash(data);
        let hash2 = engine.compute_chunk_hash(data);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_cross_table_dedup() {
        let mut dedup = CrossTableDedup::new();

        let table1_data = vec![
            b"row1".to_vec(),
            b"row2".to_vec(),
            b"row1".to_vec(), // Duplicate
        ];

        let metadata = dedup.deduplicate_table(1, table1_data).unwrap();
        assert_eq!(metadata.len(), 3);

        let ratio = dedup.table_dedup_ratio(1).unwrap();
        assert!(ratio > 1.0);
    }

    #[test]
    fn test_dedup_stats() {
        let mut engine = DedupEngine::new();

        let data = b"test".repeat(100);
        engine.deduplicate(&data).unwrap();

        let stats = engine.stats();
        assert!(stats.total_bytes_processed > 0);
        assert!(stats.dedup_ratio() > 0.0);
    }
}
