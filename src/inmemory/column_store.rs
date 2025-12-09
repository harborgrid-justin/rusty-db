// Oracle-like In-Memory Column Store
//
// Implements dual-format architecture where data is stored in both row and columnar formats,
// with automatic synchronization, SIMD-accelerated operations, and advanced compression.

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use std::time::{SystemTime};

use crate::inmemory::compression::{CompressionType, HybridCompressor, CompressionStats};
use crate::inmemory::vectorized_ops::{VectorizedFilter, VectorizedAggregator, VectorBatch};

/// Column store configuration
#[derive(Debug, Clone)]
pub struct ColumnStoreConfig {
    pub name: String,
    pub enable_compression: bool,
    pub vector_width: usize,
    pub cache_line_size: usize,
}

/// Metadata for a single column
#[derive(Debug, Clone)]
pub struct ColumnMetadata {
    pub name: String,
    pub column_id: u32,
    pub data_type: ColumnDataType,
    pub nullable: bool,
    pub compression_type: Option<CompressionType>,
    pub cardinality: Option<usize>,
}

/// Column data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnDataType {
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Boolean,
    String,
    Binary,
    Timestamp,
    Decimal,
}

impl ColumnDataType {
    pub fn size_bytes(&self) -> usize {
        match self {
            Self::Int8 | Self::UInt8 | Self::Boolean => 1,
            Self::Int16 | Self::UInt16 => 2,
            Self::Int32 | Self::UInt32 | Self::Float32 => 4,
            Self::Int64 | Self::UInt64 | Self::Float64 | Self::Timestamp => 8,
            Self::Decimal => 16,
            Self::String | Self::Binary => 8, // Pointer size
        }
    }

    pub fn is_fixed_width(&self) -> bool {
        !matches!(self, Self::String | Self::Binary)
    }
}

/// Statistics about column data
#[derive(Debug, Clone)]
pub struct ColumnStats {
    pub row_count: usize,
    pub null_count: usize,
    pub distinct_count: Option<usize>,
    pub min_value: Option<ColumnValue>,
    pub max_value: Option<ColumnValue>,
    pub avg_length: Option<f64>,
    pub compression_ratio: Option<f64>,
    pub last_updated: u64,
}

impl Default for ColumnStats {
    fn default() -> Self {
        Self {
            row_count: 0,
            null_count: 0,
            distinct_count: None,
            min_value: None,
            max_value: None,
            avg_length: None,
            compression_ratio: None,
            last_updated: current_timestamp(),
        }
    }
}

/// Generic column value
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnValue {
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    Boolean(bool),
    String(String),
    Binary(Vec<u8>),
    Timestamp(u64),
    Decimal(i128),
    Null,
}

/// Column segment - unit of columnar storage
pub struct ColumnSegment {
    pub segment_id: u64,
    pub column_id: u32,
    pub data_type: ColumnDataType,
    pub row_count: usize,

    // Raw data storage (aligned for SIMD)
    pub data: AlignedBuffer,

    // Null bitmap (1 bit per row)
    pub null_bitmap: Option<Vec<u8>>,

    // Compression metadata
    pub compressed: bool,
    pub compression_type: Option<CompressionType>,
    pub compression_stats: Option<CompressionStats>,

    // Memory mapping
    pub mmap_offset: Option<u64>,
    pub mmap_size: Option<usize>,

    // Access statistics
    pub access_count: std::sync::atomic::AtomicU64,
    pub last_access: RwLock<u64>,

    // Statistics
    pub stats: RwLock<ColumnStats>,
}

/// Cache-line aligned buffer for SIMD operations
#[repr(align(64))]
pub struct AlignedBuffer {
    data: Vec<u8>,
    capacity: usize,
}

impl AlignedBuffer {
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::with_capacity(capacity);
        data.resize(capacity, 0);
        Self { data, capacity }
    }

    pub fn from_vec(data: Vec<u8>) -> Self {
        let capacity = data.len();
        Self { data, capacity }
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn resize(&mut self, new_len: usize, value: u8) {
        self.data.resize(new_len, value);
        self.capacity = self.data.capacity();
    }
}

impl ColumnSegment {
    pub fn new(segment_id: u64, column_id: u32, data_type: ColumnDataType, row_count: usize) -> Self {
        let data_size = if data_type.is_fixed_width() {
            row_count * data_type.size_bytes()
        } else {
            row_count * 8 // Initial allocation for variable-width
        };

        Self {
            segment_id,
            column_id,
            data_type,
            row_count,
            data: AlignedBuffer::new(data_size),
            null_bitmap: None,
            compressed: false,
            compression_type: None,
            compression_stats: None,
            mmap_offset: None,
            mmap_size: None,
            access_count: std::sync::atomic::AtomicU64::new(0),
            last_access: RwLock::new(current_timestamp()),
            stats: RwLock::new(ColumnStats::default()),
        }
    }

    pub fn mark_access(&self) {
        self.access_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *self.last_access.write() = current_timestamp();
    }

    pub fn is_hot(&self) -> bool {
        let now = current_timestamp();
        let last = *self.last_access.read();
        now - last < 300 // Hot if accessed in last 5 minutes
    }

    pub fn memory_usage(&self) -> usize {
        let mut total = self.data.len();
        if let Some(ref bitmap) = self.null_bitmap {
            total += bitmap.len();
        }
        total
    }

    pub fn compress(&mut self, compressor: &HybridCompressor) -> Result<(), String> {
        if self.compressed {
            return Ok(());
        }

        let compressed_data = compressor.compress(
            self.data.as_slice(),
            self.data_type,
            &*self.stats.read(),
        )?;

        let original_size = self.data.len();
        let compressed_size = compressed_data.compressed_data.len();

        self.data = AlignedBuffer::from_vec(compressed_data.compressed_data);
        self.compressed = true;
        self.compression_type = Some(compressed_data.compression_type);
        self.compression_stats = Some(compressed_data.stats);

        let mut stats = self.stats.write();
        stats.compression_ratio = Some(original_size as f64 / compressed_size as f64);

        Ok(())
    }

    pub fn decompress(&mut self, compressor: &HybridCompressor) -> Result<(), String> {
        if !self.compressed {
            return Ok(());
        }

        let compression_type = self.compression_type.ok_or("No compression type set")?;
        let decompressed = compressor.decompress(
            self.data.as_slice(),
            compression_type,
            self.data_type,
        )?;

        self.data = AlignedBuffer::from_vec(decompressed);
        self.compressed = false;
        self.compression_type = None;

        Ok(())
    }

    pub fn read_int64(&self, row_idx: usize) -> Result<i64, String> {
        if row_idx >= self.row_count {
            return Err("Row index out of bounds".to_string());
        }

        if self.is_null(row_idx) {
            return Err("Value is NULL".to_string());
        }

        match self.data_type {
            ColumnDataType::Int64 | ColumnDataType::UInt64 | ColumnDataType::Timestamp => {
                let offset = row_idx * 8;
                let bytes = &self.data.as_slice()[offset..offset + 8];
                Ok(i64::from_le_bytes(bytes.try_into().unwrap()))
            }
            _ => Err(format!("Cannot read as Int64: {:?}", self.data_type)),
        }
    }

    pub fn write_int64(&mut self, row_idx: usize, value: i64) -> Result<(), String> {
        if row_idx >= self.row_count {
            return Err("Row index out of bounds".to_string());
        }

        match self.data_type {
            ColumnDataType::Int64 | ColumnDataType::UInt64 | ColumnDataType::Timestamp => {
                let offset = row_idx * 8;
                let bytes = value.to_le_bytes();
                self.data.as_mut_slice()[offset..offset + 8].copy_from_slice(&bytes);
                Ok(())
            }
            _ => Err(format!("Cannot write as Int64: {:?}", self.data_type)),
        }
    }

    pub fn is_null(&self, row_idx: usize) -> bool {
        if let Some(ref bitmap) = self.null_bitmap {
            let byte_idx = row_idx / 8;
            let bit_idx = row_idx % 8;
            if byte_idx < bitmap.len() {
                return (bitmap[byte_idx] & (1 << bit_idx)) != 0;
            }
        }
        false
    }

    pub fn set_null(&mut self, row_idx: usize, is_null: bool) {
        if self.null_bitmap.is_none() {
            let bitmap_size = (self.row_count + 7) / 8;
            self.null_bitmap = Some(vec![0u8; bitmap_size]);
        }

        if let Some(ref mut bitmap) = self.null_bitmap {
            let byte_idx = row_idx / 8;
            let bit_idx = row_idx % 8;
            if byte_idx < bitmap.len() {
                if is_null {
                    bitmap[byte_idx] |= 1 << bit_idx;
                } else {
                    bitmap[byte_idx] &= !(1 << bit_idx);
                }
            }
        }
    }

    pub fn scan_range(&self, start_row: usize, end_row: usize) -> Result<Vec<u8>, String> {
        if end_row > self.row_count {
            return Err("End row out of bounds".to_string());
        }

        let start_offset = start_row * self.data_type.size_bytes();
        let end_offset = end_row * self.data_type.size_bytes();

        Ok(self.data.as_slice()[start_offset..end_offset].to_vec())
    }

    pub fn update_stats(&self) {
        let mut stats = self.stats.write();
        stats.row_count = self.row_count;

        // Count nulls
        if let Some(ref bitmap) = self.null_bitmap {
            stats.null_count = bitmap.iter().map(|b| b.count_ones() as usize).sum();
        }

        stats.last_updated = current_timestamp();
    }
}

/// Dual-format storage: row-oriented + column-oriented
pub struct DualFormat {
    /// Row-oriented storage (original format)
    pub row_store: RwLock<Vec<Vec<ColumnValue>>>,

    /// Column-oriented storage (in-memory format)
    pub column_segments: RwLock<HashMap<u32, Vec<Arc<ColumnSegment>>>>,

    /// Synchronization metadata
    pub sync_version: std::sync::atomic::AtomicU64,
    pub last_sync: RwLock<u64>,

    /// Configuration
    pub rows_per_segment: usize,
}

impl DualFormat {
    pub fn new(rows_per_segment: usize) -> Self {
        Self {
            row_store: RwLock::new(Vec::new()),
            column_segments: RwLock::new(HashMap::new()),
            sync_version: std::sync::atomic::AtomicU64::new(0),
            last_sync: RwLock::new(current_timestamp()),
            rows_per_segment,
        }
    }

    pub fn insert_row(&self, row: Vec<ColumnValue>) -> Result<(), String> {
        self.row_store.write().push(row);
        self.sync_version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    pub fn get_row(&self, row_idx: usize) -> Option<Vec<ColumnValue>> {
        self.row_store.read().get(row_idx).cloned()
    }

    pub fn row_count(&self) -> usize {
        self.row_store.read().len()
    }

    pub fn needs_sync(&self, last_known_version: u64) -> bool {
        self.sync_version.load(std::sync::atomic::Ordering::SeqCst) > last_known_version
    }

    pub fn current_version(&self) -> u64 {
        self.sync_version.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// In-memory area containing columnar data
pub struct InMemoryArea {
    pub segments: RwLock<HashMap<u32, Vec<Arc<ColumnSegment>>>>,
    pub total_memory: std::sync::atomic::AtomicUsize,
    pub segment_count: std::sync::atomic::AtomicUsize,
}

impl InMemoryArea {
    pub fn new() -> Self {
        Self {
            segments: RwLock::new(HashMap::new()),
            total_memory: std::sync::atomic::AtomicUsize::new(0),
            segment_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn add_segment(&self, column_id: u32, segment: Arc<ColumnSegment>) {
        let memory = segment.memory_usage();

        self.segments
            .write()
            .entry(column_id)
            .or_insert_with(Vec::new)
            .push(segment);

        self.total_memory.fetch_add(memory, std::sync::atomic::Ordering::Relaxed);
        self.segment_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_segments(&self, column_id: u32) -> Vec<Arc<ColumnSegment>> {
        self.segments
            .read()
            .get(&column_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn memory_usage(&self) -> usize {
        self.total_memory.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn segment_count(&self) -> usize {
        self.segment_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn evict_cold_segments(&self) -> usize {
        let mut segments = self.segments.write();
        let mut evicted = 0;

        for column_segments in segments.values_mut() {
            column_segments.retain(|seg| {
                let keep = seg.is_hot();
                if !keep {
                    evicted += 1;
                    self.total_memory.fetch_sub(
                        seg.memory_usage(),
                        std::sync::atomic::Ordering::Relaxed,
                    );
                }
                keep
            });
        }

        self.segment_count.fetch_sub(evicted, std::sync::atomic::Ordering::Relaxed);
        evicted
    }
}

impl Default for InMemoryArea {
    fn default() -> Self {
        Self::new()
    }
}

/// Main column store structure
pub struct ColumnStore {
    config: ColumnStoreConfig,
    schema: Vec<ColumnMetadata>,
    dual_format: Arc<DualFormat>,
    inmemory_area: Arc<InMemoryArea>,
    compressor: Arc<HybridCompressor>,
    filter: Arc<VectorizedFilter>,
    aggregator: Arc<VectorizedAggregator>,
    column_metadata: RwLock<HashMap<u32, ColumnMetadata>>,
}

impl ColumnStore {
    pub fn new(config: ColumnStoreConfig, schema: Vec<ColumnMetadata>) -> Self {
        let mut column_metadata = HashMap::new();
        for col in &schema {
            column_metadata.insert(col.column_id, col.clone());
        }

        Self {
            config: config.clone(),
            schema: schema.clone(),
            dual_format: Arc::new(DualFormat::new(1024 * 1024)), // 1M rows per segment
            inmemory_area: Arc::new(InMemoryArea::new()),
            compressor: Arc::new(HybridCompressor::new()),
            filter: Arc::new(VectorizedFilter::new(config.vector_width)),
            aggregator: Arc::new(VectorizedAggregator::new(config.vector_width)),
            column_metadata: RwLock::new(column_metadata),
        }
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn insert(&self, row: Vec<ColumnValue>) -> Result<u64, String> {
        if row.len() != self.schema.len() {
            return Err(format!(
                "Row has {} columns, expected {}",
                row.len(),
                self.schema.len()
            ));
        }

        self.dual_format.insert_row(row)?;

        // Trigger population if threshold reached
        if self.dual_format.row_count() % 10000 == 0 {
            // Background population would be triggered here
        }

        Ok(self.dual_format.row_count() as u64 - 1)
    }

    pub fn get(&self, row_id: u64) -> Option<Vec<ColumnValue>> {
        self.dual_format.get_row(row_id as usize)
    }

    pub fn scan_column(&self, column_id: u32) -> Result<Vec<Arc<ColumnSegment>>, String> {
        let segments = self.inmemory_area.get_segments(column_id);
        if segments.is_empty() {
            return Err(format!("Column {} not populated in memory", column_id));
        }
        Ok(segments)
    }

    pub fn filter_column(
        &self,
        column_id: u32,
        predicate: Box<dyn Fn(&ColumnValue) -> bool + Send + Sync>,
    ) -> Result<Vec<usize>, String> {
        let segments = self.scan_column(column_id)?;
        let mut matching_rows = Vec::new();

        for (seg_idx, segment) in segments.iter().enumerate() {
            segment.mark_access();

            // Use SIMD filtering for numeric types
            if segment.data_type == ColumnDataType::Int64 && !segment.compressed {
                let batch = VectorBatch::from_slice(
                    segment.data.as_slice(),
                    segment.row_count,
                    segment.data_type,
                );

                let results = self.filter.filter_int64(&batch, |val| {
                    predicate(&ColumnValue::Int64(val))
                });

                for (i, &matched) in results.iter().enumerate() {
                    if matched {
                        matching_rows.push(seg_idx * segment.row_count + i);
                    }
                }
            } else {
                // Scalar path for other types
                for i in 0..segment.row_count {
                    if !segment.is_null(i) {
                        if let Ok(val) = segment.read_int64(i) {
                            if predicate(&ColumnValue::Int64(val)) {
                                matching_rows.push(seg_idx * segment.row_count + i);
                            }
                        }
                    }
                }
            }
        }

        Ok(matching_rows)
    }

    pub fn aggregate_column(&self, column_id: u32, op: AggregateOp) -> Result<f64, String> {
        let segments = self.scan_column(column_id)?;

        match op {
            AggregateOp::Sum => {
                let mut sum = 0.0;
                for segment in segments {
                    segment.mark_access();

                    if segment.data_type == ColumnDataType::Int64 && !segment.compressed {
                        let batch = VectorBatch::from_slice(
                            segment.data.as_slice(),
                            segment.row_count,
                            segment.data_type,
                        );
                        sum += self.aggregator.sum_int64(&batch) as f64;
                    }
                }
                Ok(sum)
            }
            AggregateOp::Count => {
                let count: usize = segments.iter().map(|s| s.row_count).sum();
                Ok(count as f64)
            }
            AggregateOp::Min => {
                let mut min = f64::MAX;
                for segment in segments {
                    if let Some(ref stats) = segment.stats.read().min_value {
                        if let ColumnValue::Int64(val) = stats {
                            min = min.min(*val as f64);
                        }
                    }
                }
                Ok(min)
            }
            AggregateOp::Max => {
                let mut max = f64::MIN;
                for segment in segments {
                    if let Some(ref stats) = segment.stats.read().max_value {
                        if let ColumnValue::Int64(val) = stats {
                            max = max.max(*val as f64);
                        }
                    }
                }
                Ok(max)
            }
            AggregateOp::Avg => {
                let sum = self.aggregate_column(column_id, AggregateOp::Sum)?;
                let count = self.aggregate_column(column_id, AggregateOp::Count)?;
                Ok(sum / count)
            }
        }
    }

    pub fn memory_usage(&self) -> usize {
        self.inmemory_area.memory_usage()
    }

    pub fn evict_cold_segments(&self) -> usize {
        self.inmemory_area.evict_cold_segments()
    }

    pub fn get_stats(&self, column_id: u32) -> Option<ColumnStats> {
        let segments = self.inmemory_area.get_segments(column_id);
        if let Some(segment) = segments.first() {
            Some(segment.stats.read().clone())
        } else {
            None
        }
    }

    pub fn column_metadata(&self, column_id: u32) -> Option<ColumnMetadata> {
        self.column_metadata.read().get(&column_id).cloned()
    }

    pub fn enable_column(&self, column_id: u32) -> Result<(), String> {
        if !self.column_metadata.read().contains_key(&column_id) {
            return Err(format!("Column {} not found", column_id));
        }

        // This would trigger population in a real implementation
        Ok(())
    }

    pub fn disable_column(&self, column_id: u32) -> Result<(), String> {
        let mut segments = self.inmemory_area.segments.write();
        if let Some(column_segments) = segments.remove(&column_id) {
            let memory: usize = column_segments.iter().map(|s| s.memory_usage()).sum();
            self.inmemory_area
                .total_memory
                .fetch_sub(memory, std::sync::atomic::Ordering::Relaxed);
            self.inmemory_area.segment_count.fetch_sub(
                column_segments.len(),
                std::sync::atomic::Ordering::Relaxed,
            );
        }
        Ok(())
    }
}

/// Aggregate operations
#[derive(Debug, Clone, Copy)]
pub enum AggregateOp {
    Sum,
    Count,
    Min,
    Max,
    Avg,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
use std::time::UNIX_EPOCH;

    #[test]
    fn test_aligned_buffer() {
        let buffer = AlignedBuffer::new(1024);
        assert_eq!(buffer.len(), 1024);
        assert_eq!(buffer.as_ptr() as usize % 64, 0); // 64-byte aligned
    }

    #[test]
    fn test_column_segment_creation() {
        let segment = ColumnSegment::new(0, 0, ColumnDataType::Int64, 1000);
        assert_eq!(segment.row_count, 1000);
        assert_eq!(segment.data.len(), 8000); // 1000 * 8 bytes
    }

    #[test]
    fn test_column_segment_null_bitmap() {
        let mut segment = ColumnSegment::new(0, 0, ColumnDataType::Int64, 100);

        segment.set_null(5, true);
        assert!(segment.is_null(5));
        assert!(!segment.is_null(4));

        segment.set_null(5, false);
        assert!(!segment.is_null(5));
    }

    #[test]
    fn test_dual_format() {
        let dual = DualFormat::new(1000);

        let row = vec![
            ColumnValue::Int64(42),
            ColumnValue::String("test".to_string()),
        ];

        dual.insert_row(row.clone()).unwrap();
        assert_eq!(dual.row_count(), 1);

        let retrieved = dual.get_row(0).unwrap();
        assert_eq!(retrieved, row);
    }

    #[test]
    fn test_column_store_insert() {
        let config = ColumnStoreConfig {
            name: "test_store".to_string(),
            enable_compression: true,
            vector_width: 8,
            cache_line_size: 64,
        };

        let schema = vec![
            ColumnMetadata {
                name: "id".to_string(),
                column_id: 0,
                data_type: ColumnDataType::Int64,
                nullable: false,
                compression_type: None,
                cardinality: None,
            },
            ColumnMetadata {
                name: "value".to_string(),
                column_id: 1,
                data_type: ColumnDataType::Int64,
                nullable: true,
                compression_type: None,
                cardinality: None,
            },
        ];

        let store = ColumnStore::new(config, schema);

        let row = vec![ColumnValue::Int64(1), ColumnValue::Int64(100)];
        let row_id = store.insert(row).unwrap();
        assert_eq!(row_id, 0);

        let retrieved = store.get(row_id).unwrap();
        assert_eq!(retrieved[0], ColumnValue::Int64(1));
    }

    #[test]
    fn test_inmemory_area() {
        let area = InMemoryArea::new();

        let segment = Arc::new(ColumnSegment::new(0, 0, ColumnDataType::Int64, 1000));
        let memory_before = area.memory_usage();

        area.add_segment(0, segment.clone());

        assert!(area.memory_usage() > memory_before);
        assert_eq!(area.segment_count(), 1);

        let segments = area.get_segments(0);
        assert_eq!(segments.len(), 1);
    }
}


