// Vectorized Join Engine for In-Memory Column Store
//
// Implements high-performance join algorithms optimized for columnar data:
// - Vectorized hash joins with SIMD
// - Bloom filter pre-filtering
// - Partitioned parallel joins
// - Memory-efficient algorithms

use std::collections::HashSet;
use std::collections::{HashMap};
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

use crate::inmemory::column_store::ColumnSegment;
use crate::inmemory::vectorized_ops::VectorizedFilter;

// Join types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    LeftOuter,
    RightOuter,
    FullOuter,
    LeftSemi,
    LeftAnti,
}

// Join statistics
#[derive(Debug, Clone, Default)]
pub struct JoinStats {
    pub build_time_us: u64,
    pub probe_time_us: u64,
    pub total_time_us: u64,
    pub build_rows: usize,
    pub probe_rows: usize,
    pub output_rows: usize,
    pub hash_table_size: usize,
    pub bloom_filter_size: usize,
    pub bloom_filter_hits: usize,
    pub bloom_filter_misses: usize,
    pub partitions_created: usize,
}

// Bloom filter for pre-filtering join candidates
pub struct BloomFilter {
    bits: Vec<u64>,
    num_bits: usize,
    num_hashes: usize,
}

impl BloomFilter {
    pub fn new(expected_elements: usize, false_positive_rate: f64) -> Self {
        let num_bits = Self::optimal_num_bits(expected_elements, false_positive_rate);
        let num_hashes = Self::optimal_num_hashes(num_bits, expected_elements);

        let num_words = (num_bits + 63) / 64;

        Self {
            bits: vec![0u64; num_words],
            num_bits,
            num_hashes,
        }
    }

    fn optimal_num_bits(n: usize, p: f64) -> usize {
        let n = n as f64;
        let p = p.max(0.0001).min(0.9999);
        (-(n * p.ln()) / (2.0_f64.ln().powi(2))).ceil() as usize
    }

    fn optimal_num_hashes(m: usize, n: usize) -> usize {
        if n == 0 {
            return 1;
        }
        ((m as f64 / n as f64) * 2.0_f64.ln()).ceil() as usize
    }

    pub fn insert(&mut self, value: i64) {
        for i in 0..self.num_hashes {
            let hash = self.hash(value, i);
            let bit_index = hash % self.num_bits;
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;

            if word_index < self.bits.len() {
                self.bits[word_index] |= 1u64 << bit_offset;
            }
        }
    }

    pub fn contains(&self, value: i64) -> bool {
        for i in 0..self.num_hashes {
            let hash = self.hash(value, i);
            let bit_index = hash % self.num_bits;
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;

            if word_index >= self.bits.len() {
                return false;
            }

            if (self.bits[word_index] & (1u64 << bit_offset)) == 0 {
                return false;
            }
        }
        true
    }

    fn hash(&self, value: i64, seed: usize) -> usize {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        seed.hash(&mut hasher);
        hasher.finish() as usize
    }

    pub fn size_bytes(&self) -> usize {
        self.bits.len() * 8
    }

    pub fn clear(&mut self) {
        self.bits.fill(0);
    }
}

// Hash table entry for joins
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct HashTableEntry {
    key: i64,
    row_ids: Vec<usize>,
}

// Hash table for join operations
pub struct JoinHashTable {
    entries: HashMap<i64, Vec<usize>>,
    total_entries: usize,
}

impl JoinHashTable {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::with_capacity(capacity),
            total_entries: 0,
        }
    }

    pub fn insert(&mut self, key: i64, row_id: usize) {
        self.entries.entry(key).or_insert_with(Vec::new).push(row_id);
        self.total_entries += 1;
    }

    pub fn probe(&self, key: i64) -> Option<&Vec<usize>> {
        self.entries.get(&key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn total_entries(&self) -> usize {
        self.total_entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_entries = 0;
    }
}

// Partition for partitioned joins
pub struct JoinPartition {
    partition_id: usize,
    build_data: Vec<i64>,
    build_row_ids: Vec<usize>,
    probe_data: Vec<i64>,
    probe_row_ids: Vec<usize>,
}

impl JoinPartition {
    pub fn new(partition_id: usize) -> Self {
        Self {
            partition_id,
            build_data: Vec::new(),
            build_row_ids: Vec::new(),
            probe_data: Vec::new(),
            probe_row_ids: Vec::new(),
        }
    }

    pub fn add_build(&mut self, key: i64, row_id: usize) {
        self.build_data.push(key);
        self.build_row_ids.push(row_id);
    }

    pub fn add_probe(&mut self, key: i64, row_id: usize) {
        self.probe_data.push(key);
        self.probe_row_ids.push(row_id);
    }

    pub fn build_size(&self) -> usize {
        self.build_data.len()
    }

    pub fn probe_size(&self) -> usize {
        self.probe_data.len()
    }
}

// Partitioned join for parallel execution
pub struct PartitionedJoin {
    num_partitions: usize,
    partitions: Vec<JoinPartition>,
}

impl PartitionedJoin {
    pub fn new(num_partitions: usize) -> Self {
        let partitions = (0..num_partitions)
            .map(|i| JoinPartition::new(i))
            .collect();

        Self {
            num_partitions,
            partitions,
        }
    }

    pub fn partition_key(&self, key: i64) -> usize {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.num_partitions
    }

    pub fn add_build(&mut self, key: i64, row_id: usize) {
        let partition_id = self.partition_key(key);
        self.partitions[partition_id].add_build(key, row_id);
    }

    pub fn add_probe(&mut self, key: i64, row_id: usize) {
        let partition_id = self.partition_key(key);
        self.partitions[partition_id].add_probe(key, row_id);
    }

    pub fn execute_partition(&self, partition_id: usize, join_type: JoinType) -> Vec<(usize, usize)> {
        if partition_id >= self.num_partitions {
            return Vec::new();
        }

        let partition = &self.partitions[partition_id];
        let mut results = Vec::new();

        // Build hash table for this partition
        let mut hash_table = JoinHashTable::new(partition.build_size());
        for (i, &key) in partition.build_data.iter().enumerate() {
            let row_id = partition.build_row_ids[i];
            hash_table.insert(key, row_id);
        }

        // Probe
        for (i, &key) in partition.probe_data.iter().enumerate() {
            let probe_row_id = partition.probe_row_ids[i];

            if let Some(build_row_ids) = hash_table.probe(key) {
                for &build_row_id in build_row_ids {
                    results.push((build_row_id, probe_row_id));
                }
            } else if matches!(join_type, JoinType::LeftOuter | JoinType::FullOuter) {
                results.push((usize::MAX, probe_row_id));
            }
        }

        results
    }

    pub fn get_partition(&self, partition_id: usize) -> Option<&JoinPartition> {
        self.partitions.get(partition_id)
    }
}

// Vectorized hash join engine
pub struct HashJoinEngine {
    filter: Arc<VectorizedFilter>,
    bloom_filter_threshold: usize,
}

impl HashJoinEngine {
    pub fn new() -> Self {
        Self {
            filter: Arc::new(VectorizedFilter::new(8)),
            bloom_filter_threshold: 10000,
        }
    }

    pub fn join_int64(
        &self,
        build_segment: &ColumnSegment,
        probe_segment: &ColumnSegment,
        join_type: JoinType,
    ) -> Result<(Vec<usize>, Vec<usize>, JoinStats), String> {
        let start = std::time::Instant::now();

        // Initialize statistics
        let mut stats = JoinStats::default();
        stats.build_rows = build_segment.row_count;
        stats.probe_rows = probe_segment.row_count;

        // Build phase
        let build_start = std::time::Instant::now();

        let mut hash_table = JoinHashTable::new(build_segment.row_count);
        let mut bloom_filter = if build_segment.row_count > self.bloom_filter_threshold {
            Some(BloomFilter::new(build_segment.row_count, 0.01))
        } else {
            None
        };

        for i in 0..build_segment.row_count {
            if let Ok(key) = build_segment.read_int64(i) {
                hash_table.insert(key, i);

                if let Some(ref mut bf) = bloom_filter {
                    bf.insert(key);
                }
            }
        }

        stats.build_time_us = build_start.elapsed().as_micros() as u64;
        stats.hash_table_size = hash_table.len();

        if let Some(ref bf) = bloom_filter {
            stats.bloom_filter_size = bf.size_bytes();
        }

        // Probe phase
        let probe_start = std::time::Instant::now();

        let mut build_row_ids = Vec::new();
        let mut probe_row_ids = Vec::new();

        for i in 0..probe_segment.row_count {
            if let Ok(key) = probe_segment.read_int64(i) {
                // Bloom filter check
                if let Some(ref bf) = bloom_filter {
                    if !bf.contains(key) {
                        stats.bloom_filter_misses += 1;

                        if matches!(join_type, JoinType::LeftOuter | JoinType::FullOuter) {
                            build_row_ids.push(usize::MAX);
                            probe_row_ids.push(i);
                        }
                        continue;
                    }
                    stats.bloom_filter_hits += 1;
                }

                // Hash table probe
                if let Some(matching_build_rows) = hash_table.probe(key) {
                    for &build_row in matching_build_rows {
                        build_row_ids.push(build_row);
                        probe_row_ids.push(i);
                    }
                } else if matches!(join_type, JoinType::LeftOuter | JoinType::FullOuter) {
                    build_row_ids.push(usize::MAX);
                    probe_row_ids.push(i);
                }
            }
        }

        stats.probe_time_us = probe_start.elapsed().as_micros() as u64;
        stats.output_rows = build_row_ids.len();
        stats.total_time_us = start.elapsed().as_micros() as u64;

        Ok((build_row_ids, probe_row_ids, stats))
    }

    pub fn partitioned_join_int64(
        &self,
        build_segment: &ColumnSegment,
        probe_segment: &ColumnSegment,
        join_type: JoinType,
        num_partitions: usize,
    ) -> Result<(Vec<usize>, Vec<usize>, JoinStats), String> {
        let start = std::time::Instant::now();

        let mut stats = JoinStats::default();
        stats.build_rows = build_segment.row_count;
        stats.probe_rows = probe_segment.row_count;
        stats.partitions_created = num_partitions;

        // Create partitions
        let mut partitioned = PartitionedJoin::new(num_partitions);

        // Partition build side
        let build_start = std::time::Instant::now();
        for i in 0..build_segment.row_count {
            if let Ok(key) = build_segment.read_int64(i) {
                partitioned.add_build(key, i);
            }
        }

        // Partition probe side
        for i in 0..probe_segment.row_count {
            if let Ok(key) = probe_segment.read_int64(i) {
                partitioned.add_probe(key, i);
            }
        }

        stats.build_time_us = build_start.elapsed().as_micros() as u64;

        // Execute joins on each partition
        let probe_start = std::time::Instant::now();

        let mut build_row_ids = Vec::new();
        let mut probe_row_ids = Vec::new();

        for partition_id in 0..num_partitions {
            let results = partitioned.execute_partition(partition_id, join_type);

            for (build_row, probe_row) in results {
                build_row_ids.push(build_row);
                probe_row_ids.push(probe_row);
            }
        }

        stats.probe_time_us = probe_start.elapsed().as_micros() as u64;
        stats.output_rows = build_row_ids.len();
        stats.total_time_us = start.elapsed().as_micros() as u64;

        Ok((build_row_ids, probe_row_ids, stats))
    }

    pub fn semi_join_int64(
        &self,
        probe_segment: &ColumnSegment,
        build_keys: &[i64],
    ) -> Result<Vec<usize>, String> {
        // Build hash set of build keys
        let build_set: HashSet<i64> = build_keys.iter().copied().collect();

        let mut matching_rows = Vec::new();

        for i in 0..probe_segment.row_count {
            if let Ok(key) = probe_segment.read_int64(i) {
                if build_set.contains(&key) {
                    matching_rows.push(i);
                }
            }
        }

        Ok(matching_rows)
    }

    pub fn anti_join_int64(
        &self,
        probe_segment: &ColumnSegment,
        build_keys: &[i64],
    ) -> Result<Vec<usize>, String> {
        // Build hash set of build keys
        let build_set: HashSet<i64> = build_keys.iter().copied().collect();

        let mut matching_rows = Vec::new();

        for i in 0..probe_segment.row_count {
            if let Ok(key) = probe_segment.read_int64(i) {
                if !build_set.contains(&key) {
                    matching_rows.push(i);
                }
            }
        }

        Ok(matching_rows)
    }
}

impl Default for HashJoinEngine {
    fn default() -> Self {
        Self::new()
    }
}

// Vectorized join interface
pub trait VectorizedJoin {
    fn join(
        &self,
        build: &ColumnSegment,
        probe: &ColumnSegment,
        join_type: JoinType,
    ) -> Result<(Vec<usize>, Vec<usize>, JoinStats), String>;

    fn estimate_output_size(&self, build_rows: usize, probe_rows: usize, join_type: JoinType) -> usize {
        match join_type {
            JoinType::Inner => (build_rows * probe_rows) / 10, // Assume 10% selectivity
            JoinType::LeftOuter => probe_rows,
            JoinType::RightOuter => build_rows,
            JoinType::FullOuter => build_rows + probe_rows,
            JoinType::LeftSemi | JoinType::LeftAnti => probe_rows / 2,
        }
    }
}

impl VectorizedJoin for HashJoinEngine {
    fn join(
        &self,
        build: &ColumnSegment,
        probe: &ColumnSegment,
        join_type: JoinType,
    ) -> Result<(Vec<usize>, Vec<usize>, JoinStats), String> {
        // Choose between regular and partitioned join based on size
        let total_rows = build.row_count + probe.row_count;

        if total_rows > 1_000_000 {
            // Use partitioned join for large datasets
            let num_partitions = (total_rows / 100_000).min(16);
            self.partitioned_join_int64(build, probe, join_type, num_partitions)
        } else {
            // Use regular hash join
            self.join_int64(build, probe, join_type)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use crate::inmemory::column_store::{ColumnDataType};
use std::time::Instant;

    #[test]
    fn test_bloom_filter() {
        let mut bf = BloomFilter::new(1000, 0.01);

        // Insert values
        for i in 0..1000 {
            bf.insert(i);
        }

        // Test membership
        for i in 0..1000 {
            assert!(bf.contains(i));
        }

        // Test false positives (should be rare)
        let mut false_positives = 0;
        for i in 1000..2000 {
            if bf.contains(i) {
                false_positives += 1;
            }
        }

        // False positive rate should be around 1%
        assert!(false_positives < 50);
    }

    #[test]
    fn test_hash_table() {
        let mut ht = JoinHashTable::new(100);

        ht.insert(42, 0);
        ht.insert(42, 1);
        ht.insert(99, 2);

        let rows = ht.probe(42).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], 0);
        assert_eq!(rows[1], 1);

        let rows = ht.probe(99).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], 2);

        assert!(ht.probe(100).is_none());
    }

    #[test]
    fn test_partitioned_join() {
        let mut pjoin = PartitionedJoin::new(4);

        // Add build side
        for i in 0..10 {
            pjoin.add_build(i, i as usize);
        }

        // Add probe side
        for i in 0..10 {
            pjoin.add_probe(i, i as usize);
        }

        // Execute a partition
        let results = pjoin.execute_partition(0, JoinType::Inner);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_hash_join_engine() {
        let engine = HashJoinEngine::new();

        // Create test segments
        let mut build_seg = ColumnSegment::new(0, 0, ColumnDataType::Int64, 10);
        let mut probe_seg = ColumnSegment::new(1, 0, ColumnDataType::Int64, 10);

        // Populate build segment
        for i in 0..10 {
            build_seg.write_int64(i, i as i64).unwrap();
        }

        // Populate probe segment (with some overlap)
        for i in 5..15 {
            if i < 10 {
                probe_seg.write_int64(i - 5, i as i64).unwrap();
            }
        }

        // Execute join
        let (build_ids, probe_ids, stats) = engine
            .join_int64(&build_seg, &probe_seg, JoinType::Inner)
            .unwrap();

        assert_eq!(build_ids.len(), probe_ids.len());
        assert!(stats.output_rows > 0);
        assert!(stats.total_time_us > 0);
    }

    #[test]
    fn test_semi_join() {
        let engine = HashJoinEngine::new();

        let mut probe_seg = ColumnSegment::new(0, 0, ColumnDataType::Int64, 10);
        for i in 0..10 {
            probe_seg.write_int64(i, i as i64).unwrap();
        }

        let build_keys = vec![2, 4, 6, 8];

        let matching_rows = engine.semi_join_int64(&probe_seg, &build_keys).unwrap();

        assert_eq!(matching_rows.len(), 4);
        assert!(matching_rows.contains(&2));
        assert!(matching_rows.contains(&4));
        assert!(matching_rows.contains(&6));
        assert!(matching_rows.contains(&8));
    }

    #[test]
    fn test_anti_join() {
        let engine = HashJoinEngine::new();

        let mut probe_seg = ColumnSegment::new(0, 0, ColumnDataType::Int64, 10);
        for i in 0..10 {
            probe_seg.write_int64(i, i as i64).unwrap();
        }

        let build_keys = vec![2, 4, 6, 8];

        let matching_rows = engine.anti_join_int64(&probe_seg, &build_keys).unwrap();

        assert_eq!(matching_rows.len(), 6);
        assert!(matching_rows.contains(&0));
        assert!(matching_rows.contains(&1));
        assert!(matching_rows.contains(&3));
        assert!(matching_rows.contains(&5));
    }
}
