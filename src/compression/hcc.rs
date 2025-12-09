// Hybrid Columnar Compression (HCC) - Oracle-like Implementation
// Provides columnar compression for OLAP workloads with excellent compression ratios

use std::collections::HashSet;
use super::*;
use super::algorithms::{LZ4Compressor, ZstdCompressor, DictionaryCompressor, CascadedCompressor, DeltaEncoder, RLEEncoder};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use std::time::UNIX_EPOCH;
use std::time::SystemTime;

/// Compression Unit (CU) - basic unit of HCC compression
/// Contains multiple rows organized in columnar format
#[derive(Debug, Clone)]
pub struct CompressionUnit {
    pub cu_id: u64,
    pub num_rows: usize,
    pub num_columns: usize,
    pub column_metadata: Vec<ColumnMetadata>,
    pub compressed_columns: Vec<Vec<u8>>,
    pub compression_level: CompressionLevel,
    pub created_at: u64,
    pub last_accessed: u64,
}

/// Metadata for each column in a CU
#[derive(Debug, Clone)]
pub struct ColumnMetadata {
    pub column_id: usize,
    pub data_type: ColumnDataType,
    pub uncompressed_size: usize,
    pub compressed_size: usize,
    pub algorithm: CompressionAlgorithm,
    pub min_value: Option<Vec<u8>>,
    pub max_value: Option<Vec<u8>>,
    pub null_count: usize,
    pub distinct_values: Option<usize>,
    pub checksum: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnDataType {
    Integer,
    BigInt,
    Float,
    Double,
    Varchar,
    Date,
    Timestamp,
    Boolean,
    Binary,
}

impl ColumnDataType {
    pub fn size_hint(&self) -> usize {
        match self {
            ColumnDataType::Boolean => 1,
            ColumnDataType::Integer => 4,
            ColumnDataType::BigInt => 8,
            ColumnDataType::Float => 4,
            ColumnDataType::Double => 8,
            ColumnDataType::Date => 4,
            ColumnDataType::Timestamp => 8,
            ColumnDataType::Varchar => 32,
            ColumnDataType::Binary => 64,
        }
    }
}

/// HCC Compression Strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HCCStrategy {
    QueryLow,    // Optimized for query performance
    QueryHigh,   // Balanced query/compression
    ArchiveLow,  // Good compression
    ArchiveHigh, // Maximum compression
}

impl HCCStrategy {
    pub fn compression_level(&self) -> CompressionLevel {
        match self {
            HCCStrategy::QueryLow => CompressionLevel::Fast,
            HCCStrategy::QueryHigh => CompressionLevel::Default,
            HCCStrategy::ArchiveLow => CompressionLevel::Default,
            HCCStrategy::ArchiveHigh => CompressionLevel::Maximum,
        }
    }

    pub fn cu_size(&self) -> usize {
        match self {
            HCCStrategy::QueryLow => 32 * 1024,    // 32K rows
            HCCStrategy::QueryHigh => 64 * 1024,   // 64K rows
            HCCStrategy::ArchiveLow => 128 * 1024, // 128K rows
            HCCStrategy::ArchiveHigh => 256 * 1024, // 256K rows
        }
    }
}

/// Hybrid Columnar Compression Engine
pub struct HCCEngine {
    strategy: HCCStrategy,
    stats: Arc<RwLock<CompressionStats>>,
    cu_cache: Arc<RwLock<HashMap<u64, CompressionUnit>>>,
    next_cu_id: Arc<RwLock<u64>>,
    lz4_compressor: LZ4Compressor,
    zstd_compressor: ZstdCompressor,
    dict_compressor: DictionaryCompressor,
}

impl HCCEngine {
    pub fn new(strategy: HCCStrategy) -> Self {
        let level = strategy.compression_level();

        Self {
            strategy,
            stats: Arc::new(RwLock::new(CompressionStats::new())),
            cu_cache: Arc::new(RwLock::new(HashMap::new())),
            next_cu_id: Arc::new(RwLock::new(0)),
            lz4_compressor: LZ4Compressor::new(level),
            zstd_compressor: ZstdCompressor::new(level),
            dict_compressor: DictionaryCompressor::new(level),
        }
    }

    /// Compress column using type-specific encoding (NEW!)
    pub fn compress_column_typed(&self, column: &[u8], col_type: &ColumnDataType) -> CompressionResult<Vec<u8>> {

        match col_type {
            ColumnDataType::Integer | ColumnDataType::BigInt => {
                // Convert bytes to u32/u64 and use cascaded compression
                let cascaded = CascadedCompressor::new();

                if *col_type == ColumnDataType::Integer && column.len() >= 4 {
                    let mut values = Vec::with_capacity(column.len() / 4);
                    for chunk in column.chunks_exact(4) {
                        values.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                    }

                    let mut result = vec![1u8]; // Marker: typed compression
                    result.extend_from_slice(&cascaded.compress_u32(&values)?);
                    Ok(result)
                } else {
                    // Fall back to LZ4 for odd sizes
                    let mut output = vec![0u8; self.lz4_compressor.max_compressed_size(column.len())];
                    let size = self.lz4_compressor.compress(column, &mut output)?;
                    output.truncate(size);
                    let mut result = vec![0u8]; // Marker: generic compression
                    result.extend_from_slice(&output);
                    Ok(result)
                }
            }

            ColumnDataType::Date | ColumnDataType::Timestamp => {
                // Use delta encoding for temporal data
                let delta_encoder = DeltaEncoder::new();

                if column.len() >= 4 {
                    let mut values = Vec::with_capacity(column.len() / 4);
                    for chunk in column.chunks_exact(4) {
                        values.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
                    }

                    let mut result = vec![2u8]; // Marker: delta encoding
                    result.extend_from_slice(&delta_encoder.encode(&values)?);
                    Ok(result)
                } else {
                    let mut output = vec![0u8; self.lz4_compressor.max_compressed_size(column.len())];
                    let size = self.lz4_compressor.compress(column, &mut output)?;
                    output.truncate(size);
                    let mut result = vec![0u8];
                    result.extend_from_slice(&output);
                    Ok(result)
                }
            }

            ColumnDataType::Boolean => {
                // Bit packing for booleans (8:1 compression minimum)
                let rle_encoder = RLEEncoder::new();
                let mut result = vec![3u8]; // Marker: RLE
                result.extend_from_slice(&rle_encoder.encode(column)?);
                Ok(result)
            }

            ColumnDataType::Varchar | ColumnDataType::Binary => {
                // Use dictionary or LZ4 based on cardinality
                let unique_count = column.chunks(32).collect::<std::collections::HashSet<_>>().len();
                let total_chunks = (column.len() + 31) / 32;

                if unique_count < total_chunks / 3 {
                    // Low cardinality - use dictionary encoding
                    // For simplicity, use Zstd which has dictionary support
                    let mut output = vec![0u8; self.zstd_compressor.max_compressed_size(column.len())];
                    let size = self.zstd_compressor.compress(column, &mut output)?;
                    output.truncate(size);
                    let mut result = vec![4u8]; // Marker: dictionary/zstd
                    result.extend_from_slice(&output);
                    Ok(result)
                } else {
                    // High cardinality - use LZ4
                    let mut output = vec![0u8; self.lz4_compressor.max_compressed_size(column.len())];
                    let size = self.lz4_compressor.compress(column, &mut output)?;
                    output.truncate(size);
                    let mut result = vec![0u8];
                    result.extend_from_slice(&output);
                    Ok(result)
                }
            }

            _ => {
                // Default: use LZ4
                let mut output = vec![0u8; self.lz4_compressor.max_compressed_size(column.len())];
                let size = self.lz4_compressor.compress(column, &mut output)?;
                output.truncate(size);
                let mut result = vec![0u8];
                result.extend_from_slice(&output);
                Ok(result)
            }
        }
    }

    /// Decompress column using type-specific encoding (NEW!)
    pub fn decompress_column_typed(&self, compressed: &[u8], col_type: &ColumnDataType) -> CompressionResult<Vec<u8>> {

        if compressed.is_empty() {
            return Ok(Vec::new());
        }

        let marker = compressed[0];
        let data = &compressed[1..];

        match marker {
            0 => {
                // Generic LZ4
                let mut output = vec![0u8; data.len() * 4];
                let size = self.lz4_compressor.decompress(data, &mut output)?;
                output.truncate(size);
                Ok(output)
            }
            1 => {
                // Cascaded integer compression
                let cascaded = CascadedCompressor::new();
                let mut values = Vec::new();
                cascaded.decompress_u32(data, &mut values)?;

                let mut bytes = Vec::with_capacity(values.len() * 4);
                for value in values {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
                Ok(bytes)
            }
            2 => {
                // Delta encoding
                let delta_encoder = DeltaEncoder::new();
                let decoded = delta_encoder.decode(data)?;
                Ok(decoded)
            }
            3 => {
                // RLE
                let rle_encoder = RLEEncoder::new();
                rle_encoder.decode(data)
            }
            4 => {
                // Dictionary/Zstd
                let mut output = vec![0u8; data.len() * 4];
                let size = self.zstd_compressor.decompress(data, &mut output)?;
                output.truncate(size);
                Ok(output)
            }
            _ => {
                Err(CompressionError::UnsupportedAlgorithm(
                    format!("Unknown compression marker: {}", marker)
                ))
            }
        }
    }

    pub fn with_strategy(strategy: HCCStrategy) -> Self {
        Self::new(strategy)
    }

    /// Transform row-major data to columnar format for compression
    pub fn transform_to_columnar(&self, rows: &[Vec<u8>], column_types: &[ColumnDataType])
        -> CompressionResult<Vec<Vec<u8>>> {

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let num_columns = column_types.len();
        let mut columns = vec![Vec::new(); num_columns];

        // Transform row-major to column-major
        for row in rows {
            let mut offset = 0;
            for (col_idx, col_type) in column_types.iter().enumerate() {
                let size = col_type.size_hint();
                if offset + size <= row.len() {
                    columns[col_idx].extend_from_slice(&row[offset..offset + size]);
                    offset += size;
                } else {
                    // Handle variable-length data
                    if offset < row.len() {
                        columns[col_idx].extend_from_slice(&row[offset..]);
                        offset = row.len();
                    }
                }
            }
        }

        Ok(columns)
    }

    /// Transform columnar data back to row-major format
    pub fn transform_to_rows(&self, columns: &[Vec<u8>], column_types: &[ColumnDataType],
                            numrows: usize) -> CompressionResult<Vec<Vec<u8>>> {

        let mut rows = vec![Vec::new(); numrows];

        for (column, col_type) in columns.iter().zip(column_types.iter()) {
            let cell_size = col_type.size_hint();

            for (row_idx, row) in rows.iter_mut().enumerate() {
                let offset = row_idx * cell_size;
                if offset + cell_size <= column.len() {
                    row.extend_from_slice(&column[offset..offset + cell_size]);
                }
            }
        }

        Ok(rows)
    }

    /// Create a Compression Unit from rows
    pub fn create_cu(&self, rows: Vec<Vec<u8>>, column_types: Vec<ColumnDataType>)
        -> CompressionResult<CompressionUnit> {

        let start = Instant::now();
        let num_rows = rows.len();
        let num_columns = column_types.len();

        if num_rows == 0 || num_columns == 0 {
            return Err(CompressionError::InvalidInput("Empty rows or columns".to_string()));
        }

        // Transform to columnar format
        let columns = self.transform_to_columnar(&rows, &column_types)?;

        // Compress each column
        let mut compressed_columns = Vec::new();
        let mut column_metadata = Vec::new();

        for (col_idx, (column, col_type)) in columns.iter().zip(column_types.iter()).enumerate() {
            let (compressed, metadata) = self.compress_column_with_metadata(column, col_type, col_idx)?;
            compressed_columns.push(compressed);
            column_metadata.push(metadata);
        }

        let cu_id = {
            let mut next_id = self.next_cu_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let cu = CompressionUnit {
            cu_id,
            num_rows,
            num_columns,
            column_metadata,
            compressed_columns,
            compression_level: self.strategy.compression_level(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_accessed: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Update statistics
        let mut stats = self.stats.write().unwrap();
        let total_uncompressed: usize = cu.column_metadata.iter().map(|m| m.uncompressed_size).sum();
        let total_compressed: usize = cu.column_metadata.iter().map(|m| m.compressed_size).sum();

        stats.uncompressed_size += total_uncompressed;
        stats.compressed_size += total_compressed;
        stats.compression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_compressed += 1;

        // Cache the CU
        self.cu_cache.write().unwrap().insert(cu_id, cu.clone());

        Ok(cu)
    }

    /// Compress a single column with optimal algorithm and metadata
    fn compress_column_with_metadata(&self, column: &[u8], col_type: &ColumnDataType, col_idx: usize)
        -> CompressionResult<(Vec<u8>, ColumnMetadata)> {

        let uncompressed_size = column.len();
        let algorithm = self.select_column_algorithm(column, col_type);

        let mut compressed = vec![0u8; self.max_compressed_size(column.len())];

        let compressed_size = match algorithm {
            CompressionAlgorithm::LZ4 => {
                self.lz4_compressor.compress(column, &mut compressed)?
            }
            CompressionAlgorithm::Zstandard => {
                self.zstd_compressor.compress(column, &mut compressed)?
            }
            CompressionAlgorithm::Dictionary => {
                self.dict_compressor.compress(column, &mut compressed)?
            }
            _ => {
                self.lz4_compressor.compress(column, &mut compressed)?
            }
        };

        compressed.truncate(compressed_size);

        // Calculate column statistics
        let min_value = self.find_min_value(column, col_type);
        let max_value = self.find_max_value(column, col_type);
        let null_count = self.count_nulls(column);
        let distinct_values = self.estimate_distinct_values(column, col_type);
        let checksum = utils::crc32(column);

        let metadata = ColumnMetadata {
            column_id: col_idx,
            data_type: *col_type,
            uncompressed_size,
            compressed_size,
            algorithm,
            min_value,
            max_value,
            null_count,
            distinct_values: Some(distinct_values),
            checksum,
        };

        Ok((compressed, metadata))
    }

    /// Decompress a Compression Unit
    pub fn decompress_cu(&self, cu: &CompressionUnit) -> CompressionResult<Vec<Vec<u8>>> {
        let start = Instant::now();
        let mut decompressed_columns = Vec::new();

        for (compressed_col, metadata) in cu.compressed_columns.iter().zip(&cu.column_metadata) {
            let mut decompressed = vec![0u8; metadata.uncompressed_size];

            let decomp_size = match metadata.algorithm {
                CompressionAlgorithm::LZ4 => {
                    self.lz4_compressor.decompress(compressed_col, &mut decompressed)?
                }
                CompressionAlgorithm::Zstandard => {
                    self.zstd_compressor.decompress(compressed_col, &mut decompressed)?
                }
                CompressionAlgorithm::Dictionary => {
                    self.dict_compressor.decompress(compressed_col, &mut decompressed)?
                }
                _ => {
                    self.lz4_compressor.decompress(compressed_col, &mut decompressed)?
                }
            };

            decompressed.truncate(decomp_size);

            // Verify checksum
            let checksum = utils::crc32(&decompressed);
            if checksum != metadata.checksum {
                return Err(CompressionError::CorruptedData(
                    format!("Checksum mismatch for column {}", metadata.column_id)
                ).into());
            }

            decompressed_columns.push(decompressed);
        }

        let mut stats = self.stats.write().unwrap();
        stats.decompression_time_us += start.elapsed().as_micros() as u64;
        stats.blocks_decompressed += 1;

        Ok(decompressed_columns)
    }

    /// Query-aware decompression - decompress only requested columns
    pub fn decompress_columns(&self, cu: &CompressionUnit, column_indices: &[usize])
        -> CompressionResult<Vec<Vec<u8>>> {

        let start = Instant::now();
        let mut decompressed_columns = Vec::new();

        for &col_idx in column_indices {
            if col_idx >= cu.num_columns {
                return Err(CompressionError::InvalidInput(
                    format!("Column index {} out of range", col_idx)
                ).into());
            }

            let compressed_col = &cu.compressed_columns[col_idx];
            let metadata = &cu.column_metadata[col_idx];
            let mut decompressed = vec![0u8; metadata.uncompressed_size];

            let decomp_size = match metadata.algorithm {
                CompressionAlgorithm::LZ4 => {
                    self.lz4_compressor.decompress(compressed_col, &mut decompressed)?
                }
                CompressionAlgorithm::Zstandard => {
                    self.zstd_compressor.decompress(compressed_col, &mut decompressed)?
                }
                CompressionAlgorithm::Dictionary => {
                    self.dict_compressor.decompress(compressed_col, &mut decompressed)?
                }
                _ => {
                    self.lz4_compressor.decompress(compressed_col, &mut decompressed)?
                }
            };

            decompressed.truncate(decomp_size);
            decompressed_columns.push(decompressed);
        }

        let mut stats = self.stats.write().unwrap();
        stats.decompression_time_us += start.elapsed().as_micros() as u64;

        Ok(decompressed_columns)
    }

    /// Select optimal compression algorithm for a column
    fn select_column_algorithm(&self, column: &[u8], col_type: &ColumnDataType)
        -> CompressionAlgorithm {

        match self.strategy {
            HCCStrategy::QueryLow => CompressionAlgorithm::LZ4,
            HCCStrategy::QueryHigh | HCCStrategy::ArchiveLow => {
                match col_type {
                    ColumnDataType::Varchar | ColumnDataType::Binary => {
                        // Text data often benefits from dictionary compression
                        if self.estimate_distinct_values(column, col_type) < 1000 {
                            CompressionAlgorithm::Dictionary
                        } else {
                            CompressionAlgorithm::Zstandard
                        }
                    }
                    _ => CompressionAlgorithm::LZ4
                }
            }
            HCCStrategy::ArchiveHigh => {
                // Use best compression for archive
                CompressionAlgorithm::Zstandard
            }
        }
    }

    /// Direct path load optimization - bulk load data into HCC format
    pub fn direct_path_load(&self, rows: Vec<Vec<u8>>, column_types: Vec<ColumnDataType>)
        -> CompressionResult<Vec<u64>> {

        let cu_size = self.strategy.cu_size();
        let mut cu_ids = Vec::new();

        // Split rows into CUs
        for chunk in rows.chunks(cu_size) {
            let cu = self.create_cu(chunk.to_vec(), column_types.clone())?;
            cu_ids.push(cu.cu_id);
        }

        Ok(cu_ids)
    }

    /// Get CU by ID from cache or storage
    pub fn get_cu(&self, cu_id: u64) -> Option<CompressionUnit> {
        let cache = self.cu_cache.read().unwrap();
        cache.get(&cu_id).cloned()
    }

    /// Get compression ratio for a CU
    pub fn get_cu_compression_ratio(&self, cu_id: u64) -> Option<f64> {
        self.get_cu(cu_id).map(|cu| {
            let total_uncompressed: usize = cu.column_metadata.iter()
                .map(|m| m.uncompressed_size).sum();
            let total_compressed: usize = cu.column_metadata.iter()
                .map(|m| m.compressed_size).sum();

            if total_compressed == 0 {
                0.0
            } else {
                total_uncompressed as f64 / total_compressed as f64
            }
        })
    }

    /// Get CU statistics
    pub fn get_cu_stats(&self, cu_id: u64) -> Option<CUStats> {
        self.get_cu(cu_id).map(|cu| {
            let total_uncompressed: usize = cu.column_metadata.iter()
                .map(|m| m.uncompressed_size).sum();
            let total_compressed: usize = cu.column_metadata.iter()
                .map(|m| m.compressed_size).sum();

            CUStats {
                cu_id,
                num_rows: cu.num_rows,
                num_columns: cu.num_columns,
                uncompressed_size: total_uncompressed,
                compressed_size: total_compressed,
                compression_ratio: if total_compressed == 0 { 0.0 } else {
                    total_uncompressed as f64 / total_compressed as f64
                },
                column_stats: cu.column_metadata.iter().map(|m| ColumnStats {
                    column_id: m.column_id,
                    uncompressed_size: m.uncompressed_size,
                    compressed_size: m.compressed_size,
                    null_count: m.null_count,
                    distinct_values: m.distinct_values.unwrap_or(0),
                    algorithm: m.algorithm,
                }).collect(),
            }
        })
    }

    fn max_compressed_size(&self, input_size: usize) -> usize {
        input_size + (input_size / 255) + 16
    }

    fn find_min_value(&self, column: &[u8], _col_type: &ColumnDataType) -> Option<Vec<u8>> {
        if column.is_empty() {
            return None;
        }
        // Simplified - just take first value
        Some(column.iter().take(8).cloned().collect())
    }

    fn find_max_value(&self, column: &[u8], _col_type: &ColumnDataType) -> Option<Vec<u8>> {
        if column.is_empty() {
            return None;
        }
        // Simplified - just take last value
        Some(column.iter().rev().take(8).cloned().collect())
    }

    fn count_nulls(&self, _column: &[u8]) -> usize {
        // Simplified - would need null bitmap in real implementation
        0
    }

    fn estimate_distinct_values(&self, column: &[u8], col_type: &ColumnDataType) -> usize {
        let sample_size = column.len().min(1000);
        let mut seen = std::collections::HashSet::new();
        let cell_size = col_type.size_hint();

        for i in (0..sample_size).step_by(cell_size) {
            if i + cell_size <= column.len() {
                seen.insert(&column[i..i + cell_size]);
            }
        }

        // Extrapolate to full column
        if sample_size >= column.len() {
            seen.len()
        } else {
            let ratio = column.len() as f64 / sample_size as f64;
            (seen.len() as f64 * ratio) as usize
        }
    }
}

impl ColumnarCompressor for HCCEngine {
    fn transform_to_columnar(&self, rows: &[Vec<u8>], numcolumns: usize)
        -> CompressionResult<Vec<Vec<u8>>> {

        if rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut columns = vec![Vec::new(); numcolumns];

        for row in rows {
            let bytes_per_column = row.len() / numcolumns;
            for (col_idx, column) in columns.iter_mut().enumerate() {
                let start = col_idx * bytes_per_column;
                let end = if col_idx == numcolumns - 1 {
                    row.len()
                } else {
                    start + bytes_per_column
                };
                column.extend_from_slice(&row[start..end]);
            }
        }

        Ok(columns)
    }

    fn transform_to_rows(&self, columns: &[Vec<u8>], numrows: usize)
        -> CompressionResult<Vec<Vec<u8>>> {

        if columns.is_empty() {
            return Ok(Vec::new());
        }

        let bytes_per_row_per_col = columns[0].len() / numrows;
        let mut rows = vec![Vec::new(); numrows];

        for (row_idx, row) in rows.iter_mut().enumerate() {
            for column in columns {
                let start = row_idx * bytes_per_row_per_col;
                let end = start + bytes_per_row_per_col;
                if end <= column.len() {
                    row.extend_from_slice(&column[start..end]);
                }
            }
        }

        Ok(rows)
    }

    fn compress_column(&self, column: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        self.lz4_compressor.compress(column, output)
    }

    fn decompress_column(&self, compressed: &[u8], output: &mut [u8]) -> CompressionResult<usize> {
        self.lz4_compressor.decompress(compressed, output)
    }

    fn compress_cu(&self, columns: &[Vec<u8>]) -> CompressionResult<Vec<Vec<u8>>> {
        let mut compressed_columns = Vec::new();

        for column in columns {
            let mut compressed = vec![0u8; self.max_compressed_size(column.len())];
            let size = self.compress_column(column, &mut compressed)?;
            compressed.truncate(size);
            compressed_columns.push(compressed);
        }

        Ok(compressed_columns)
    }

    fn decompress_cu(&self, compressed_columns: &[Vec<u8>]) -> CompressionResult<Vec<Vec<u8>>> {
        let mut decompressed_columns = Vec::new();

        for compressed in compressed_columns {
            // Estimate decompressed size (would be stored in metadata in real implementation)
            let estimated_size = compressed.len() * 4;
            let mut decompressed = vec![0u8; estimated_size];
            let size = self.decompress_column(compressed, &mut decompressed)?;
            decompressed.truncate(size);
            decompressed_columns.push(decompressed);
        }

        Ok(decompressed_columns)
    }
}

/// Statistics for a Compression Unit
#[derive(Debug, Clone)]
pub struct CUStats {
    pub cu_id: u64,
    pub num_rows: usize,
    pub num_columns: usize,
    pub uncompressed_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f64,
    pub column_stats: Vec<ColumnStats>,
}

/// Statistics for a single column
#[derive(Debug, Clone)]
pub struct ColumnStats {
    pub column_id: usize,
    pub uncompressed_size: usize,
    pub compressed_size: usize,
    pub null_count: usize,
    pub distinct_values: usize,
    pub algorithm: CompressionAlgorithm,
}

/// HCC Compression Advisor - recommends optimal strategy
pub struct HCCAdvisor {
    sample_size: usize,
}

impl HCCAdvisor {
    pub fn new() -> Self {
        Self {
            sample_size: 10000,
        }
    }

    /// Analyze data and recommend HCC strategy
    pub fn recommend_strategy(&self, rows: &[Vec<u8>], column_types: &[ColumnDataType])
        -> HCCStrategy {

        let sample = rows.iter().take(self.sample_size).cloned().collect::<Vec<_>>();

        if sample.is_empty() {
            return HCCStrategy::QueryHigh;
        }

        // Calculate compressibility
        let total_size: usize = sample.iter().map(|r| r.len()).sum();
        let avg_compressibility = self.estimate_avg_compressibility(&sample, column_types);

        // Estimate query pattern (simplified - would analyze access patterns in real system)
        if total_size < 1_000_000 {
            // Small dataset - optimize for query
            HCCStrategy::QueryLow
        } else if avg_compressibility > 3.0 {
            // Highly compressible - use archive
            HCCStrategy::ArchiveHigh
        } else if avg_compressibility > 2.0 {
            HCCStrategy::ArchiveLow
        } else {
            HCCStrategy::QueryHigh
        }
    }

    fn estimate_avg_compressibility(&self, rows: &[Vec<u8>], _column_types: &[ColumnDataType])
        -> f64 {

        let mut total_compressibility = 0.0;

        for row in rows.iter().take(100) {
            total_compressibility += utils::estimate_compressibility(row);
        }

        total_compressibility / rows.len().min(100) as f64
    }

    /// Recommend CU size based on data characteristics
    pub fn recommend_cu_size(&self, avg_row_size: usize, query_selectivity: f64) -> usize {
        if query_selectivity < 0.1 {
            // Highly selective queries - smaller CUs
            32 * 1024
        } else if avg_row_size < 100 {
            // Small rows - larger CUs
            128 * 1024
        } else {
            // Default
            64 * 1024
        }
    }
}

impl Default for HCCAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_hcc_compression() {
        let engine = HCCEngine::new(HCCStrategy::QueryHigh);

        let mut rows = Vec::new();
        for i in 0..1000 {
            let row = vec![
                (i % 256) as u8, ((i / 256) % 256) as u8,
                (i % 100) as u8, ((i / 100) % 256) as u8,
            ];
            rows.push(row);
        }

        let column_types = vec![
            ColumnDataType::Integer,
            ColumnDataType::Integer,
        ];

        let cu = engine.create_cu(rows, column_types).unwrap();
        assert_eq!(cu.num_rows, 1000);
        assert_eq!(cu.num_columns, 2);

        let decompressed = engine.decompress_cu(&cu).unwrap();
        assert_eq!(decompressed.len(), 2);
    }

    #[test]
    fn test_columnar_transformation() {
        let engine = HCCEngine::new(HCCStrategy::QueryHigh);

        let rows = vec![
            vec![1, 2, 3, 4],
            vec![5, 6, 7, 8],
            vec![9, 10, 11, 12],
        ];

        let column_types = vec![
            ColumnDataType::Integer,
        ];

        let columns = engine.transform_to_columnar(&rows, &column_types).unwrap();
        let restored = engine.transform_to_rows(&columns, &column_types, rows.len()).unwrap();

        assert_eq!(rows.len(), restored.len());
    }

    #[test]
    fn test_hcc_advisor() {
        let advisor = HCCAdvisor::new();

        let rows = vec![vec![1, 2, 3, 4]; 100];
        let column_types = vec![ColumnDataType::Integer];

        let strategy = advisor.recommend_strategy(&rows, &column_types);
        assert!(matches!(strategy, HCCStrategy::QueryLow | HCCStrategy::QueryHigh));
    }

    #[test]
    fn test_hcc_typed_compression_integers() {
        let engine = HCCEngine::new(HCCStrategy::QueryHigh);

        // Create column of sorted integers (perfect for cascaded compression)
        let values: Vec<u32> = (1000..1100).collect();
        let mut column = Vec::new();
        for &v in &values {
            column.extend_from_slice(&v.to_le_bytes());
        }

        let compressed = engine.compress_column_typed(&column, &ColumnDataType::Integer).unwrap();
        let decompressed = engine.decompress_column_typed(&compressed, &ColumnDataType::Integer).unwrap();

        assert_eq!(column, decompressed);

        // Check compression ratio (should be excellent for sorted integers)
        let ratio = column.len() as f64 / compressed.len() as f64;
        println!("Integer column compression ratio: {:.2}:1", ratio);
        assert!(ratio > 3.0, "Should achieve >3:1 on sorted integers");
    }

    #[test]
    fn test_hcc_typed_compression_timestamps() {
        let engine = HCCEngine::new(HCCStrategy::QueryHigh);

        // Create column of timestamps (perfect for delta encoding)
        let timestamps: Vec<u32> = (0..1000).map(|i| 1609459200 + i).collect();
        let mut column = Vec::new();
        for &ts in &timestamps {
            column.extend_from_slice(&ts.to_le_bytes());
        }

        let compressed = engine.compress_column_typed(&column, &ColumnDataType::Timestamp).unwrap();
        let decompressed = engine.decompress_column_typed(&compressed, &ColumnDataType::Timestamp).unwrap();

        assert_eq!(column, decompressed);

        // Check compression ratio (should be excellent for timestamps)
        let ratio = column.len() as f64 / compressed.len() as f64;
        println!("Timestamp column compression ratio: {:.2}:1", ratio);
        assert!(ratio > 5.0, "Should achieve >5:1 on timestamps");
    }

    #[test]
    fn test_hcc_typed_compression_booleans() {
        let engine = HCCEngine::new(HCCStrategy::QueryHigh);

        // Create column of booleans (perfect for RLE)
        let booleans = vec![1u8; 1000];

        let compressed = engine.compress_column_typed(&booleans, &ColumnDataType::Boolean).unwrap();
        let decompressed = engine.decompress_column_typed(&compressed, &ColumnDataType::Boolean).unwrap();

        assert_eq!(booleans, decompressed);

        // Check compression ratio (should be excellent for repetitive booleans)
        let ratio = booleans.len() as f64 / compressed.len() as f64;
        println!("Boolean column compression ratio: {:.2}:1", ratio);
        assert!(ratio > 8.0, "Should achieve >8:1 on repetitive booleans");
    }
}
