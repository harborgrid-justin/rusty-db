/// Columnar Storage Engine for RustyDB
/// Optimized for analytical (OLAP) workloads
/// Features: Dictionary encoding, run-length encoding, delta encoding, SIMD decompression

use std::collections::HashSet;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::error::Result;
use crate::DbError;

/// Column data type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnType {
    Int32,
    Int64,
    Float32,
    Float64,
    String,
    Boolean,
    Timestamp,
}

impl ColumnType {
    pub fn size_bytes(&self) -> usize {
        match self {
            ColumnType::Int32 | ColumnType::Float32 => 4,
            ColumnType::Int64 | ColumnType::Float64 | ColumnType::Timestamp => 8,
            ColumnType::Boolean => 1,
            ColumnType::String => 0, // Variable length
        }
    }
}

/// Encoding strategy for column data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncodingType {
    Plain,          // No encoding
    Dictionary,     // Dictionary encoding for low cardinality
    RunLength,      // Run-length encoding for repeated values
    Delta,          // Delta encoding for sorted/sequential data
    BitPacked,      // Bit-packing for small integers
}

/// Column statistics for query optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStats {
    pub num_values: usize,
    pub num_nulls: usize,
    pub min_value: Option<ColumnValue>,
    pub max_value: Option<ColumnValue>,
    pub distinct_count: usize,
    pub encoding: EncodingType,
    pub compression_ratio: f64,
}

impl ColumnStats {
    fn new() -> Self {
        Self {
            num_values: 0,
            num_nulls: 0,
            min_value: None,
            max_value: None,
            distinct_count: 0,
            encoding: EncodingType::Plain,
            compression_ratio: 1.0,
        }
    }

    fn cardinality_ratio(&self) -> f64 {
        if self.num_values == 0 {
            return 0.0;
        }
        self.distinct_count as f64 / self.num_values as f64
    }

    fn should_use_dictionary(&self) -> bool {
        self.cardinality_ratio() < 0.5 && self.distinct_count < 10000
    }
}

/// Column value wrapper
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ColumnValue {
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    String(String),
    Boolean(bool),
    Timestamp(i64),
    Null,
}

impl ColumnValue {
    fn column_type(&self) -> ColumnType {
        match self {
            ColumnValue::Int32(_) => ColumnType::Int32,
            ColumnValue::Int64(_) => ColumnType::Int64,
            ColumnValue::Float32(_) => ColumnType::Float32,
            ColumnValue::Float64(_) => ColumnType::Float64,
            ColumnValue::String(_) => ColumnType::String,
            ColumnValue::Boolean(_) => ColumnType::Boolean,
            ColumnValue::Timestamp(_) => ColumnType::Timestamp,
            ColumnValue::Null => ColumnType::Int32, // Default
        }
    }

    fn is_null(&self) -> bool {
        matches!(self, ColumnValue::Null)
    }
}

/// Dictionary encoder for low-cardinality columns
struct DictionaryEncoder {
    dictionary: HashMap<String, u32>,
    reverse_dict: Vec<String>,
    next_id: u32,
}

impl DictionaryEncoder {
    fn new() -> Self {
        Self {
            dictionary: HashMap::new(),
            reverse_dict: Vec::new(),
            next_id: 0,
        }
    }

    fn encode(&mut self, value: &str) -> u32 {
        if let Some(&id) = self.dictionary.get(value) {
            id
        } else {
            let id = self.next_id;
            self.dictionary.insert(value.to_string(), id);
            self.reverse_dict.push(value.to_string());
            self.next_id += 1;
            id
        }
    }

    fn decode(&self, id: u32) -> Option<&str> {
        self.reverse_dict.get(id as usize).map(|s| s.as_str())
    }

    fn size(&self) -> usize {
        self.dictionary.len()
    }

    fn memory_usage(&self) -> usize {
        self.reverse_dict.iter().map(|s| s.len()).sum::<usize>()
            + self.dictionary.len() * 8
    }
}

/// Run-length encoder for repeated values
struct RunLengthEncoder {
    runs: Vec<(ColumnValue, usize)>,
}

impl RunLengthEncoder {
    fn new() -> Self {
        Self {
            runs: Vec::new(),
        }
    }

    fn encode(&mut self, values: &[ColumnValue]) {
        if values.is_empty() {
            return;
        }

        let mut current_value = values[0].clone();
        let mut count = 1;

        for value in &values[1..] {
            if value == &current_value {
                count += 1;
            } else {
                self.runs.push((current_value, count));
                current_value = value.clone();
                count = 1;
            }
        }

        self.runs.push((current_value, count));
    }

    fn decode(&self) -> Vec<ColumnValue> {
        let mut result = Vec::new();

        for (value, count) in &self.runs {
            result.extend(std::iter::repeat(value.clone()).take(*count));
        }

        result
    }

    fn compression_ratio(&self, original_size: usize) -> f64 {
        if original_size == 0 {
            return 1.0;
        }

        let compressed_size = self.runs.len() * 12; // Approximate
        compressed_size as f64 / original_size as f64
    }
}

/// Delta encoder for sequential/sorted data
struct DeltaEncoder {
    base_value: i64,
    deltas: Vec<i32>,
}

impl DeltaEncoder {
    fn new() -> Self {
        Self {
            base_value: 0,
            deltas: Vec::new(),
        }
    }

    fn encode(&mut self, values: &[i64]) {
        if values.is_empty() {
            return;
        }

        self.base_value = values[0];
        self.deltas.clear();

        for i in 1..values.len() {
            let delta = (values[i] - values[i - 1]) as i32;
            self.deltas.push(delta);
        }
    }

    fn decode(&self) -> Vec<i64> {
        if self.deltas.is_empty() {
            return vec![self.base_value];
        }

        let mut result = Vec::with_capacity(self.deltas.len() + 1);
        result.push(self.base_value);

        let mut current = self.base_value;
        for &delta in &self.deltas {
            current += delta as i64;
            result.push(current);
        }

        result
    }

    fn is_suitable(values: &[i64]) -> bool {
        if values.len() < 2 {
            return false;
        }

        // Check if deltas are small enough to fit in i32
        for i in 1..values.len() {
            let delta = values[i] - values[i - 1];
            if delta < i32::MIN as i64 || delta > i32::MAX as i64 {
                return false;
            }
        }

        true
    }
}

/// Bit-packing encoder for small integers
struct BitPackedEncoder {
    bit_width: u8,
    values: Vec<u64>,
}

impl BitPackedEncoder {
    fn new(bit_width: u8) -> Self {
        Self {
            bit_width,
            values: Vec::new(),
        }
    }

    fn encode(&mut self, values: &[i32]) {
        self.values = values.iter()
            .map(|&v| v as u64 & ((1u64 << self.bit_width) - 1))
            .collect();
    }

    fn decode(&self) -> Vec<i32> {
        self.values.iter().map(|&v| v as i32).collect()
    }

    fn determine_bit_width(values: &[i32]) -> u8 {
        let max_value = values.iter().map(|&v| v.abs()).max().unwrap_or(0);

        if max_value == 0 {
            return 1;
        }

        ((max_value as f64).log2().ceil() as u8 + 1).min(32)
    }
}

/// Column chunk with encoding
struct ColumnChunk {
    column_type: ColumnType,
    encoding: EncodingType,
    data: Vec<u8>,
    null_bitmap: Vec<bool>,
    stats: ColumnStats,
}

impl ColumnChunk {
    fn new(column_type: ColumnType) -> Self {
        Self {
            column_type,
            encoding: EncodingType::Plain,
            data: Vec::new(),
            null_bitmap: Vec::new(),
            stats: ColumnStats::new(),
        }
    }

    fn encode_plain(&mut self, values: &[ColumnValue]) -> Result<()> {
        self.encoding = EncodingType::Plain;
        self.data = bincode::serialize(values)
            .map_err(|e| DbError::Storage(format!("Encoding error: {}", e)))?;

        self.update_stats(values);
        Ok(())
    }

    fn encode_dictionary(&mut self, values: &[ColumnValue]) -> Result<()> {
        let mut encoder = DictionaryEncoder::new();
        let mut encoded_ids = Vec::new();

        for value in values {
            if let ColumnValue::String(s) = value {
                encoded_ids.push(encoder.encode(s));
            }
        }

        self.encoding = EncodingType::Dictionary;
        self.data = bincode::serialize(&(&encoder.reverse_dict, &encoded_ids))
            .map_err(|e| DbError::Storage(format!("Encoding error: {}", e)))?;

        self.update_stats(values);
        Ok(())
    }

    fn encode_rle(&mut self, values: &[ColumnValue]) -> Result<()> {
        let mut encoder = RunLengthEncoder::new();
        encoder.encode(values);

        self.encoding = EncodingType::RunLength;
        self.data = bincode::serialize(&encoder.runs)
            .map_err(|e| DbError::Storage(format!("Encoding error: {}", e)))?;

        self.update_stats(values);
        Ok(())
    }

    fn encode_delta(&mut self, values: &[i64]) -> Result<()> {
        let mut encoder = DeltaEncoder::new();
        encoder.encode(values);

        self.encoding = EncodingType::Delta;
        self.data = bincode::serialize(&(encoder.base_value, &encoder.deltas))
            .map_err(|e| DbError::Storage(format!("Encoding error: {}", e)))?;

        // Convert i64 to ColumnValue for stats
        let col_values: Vec<ColumnValue> = values.iter()
            .map(|&v| ColumnValue::Int64(v))
            .collect();
        self.update_stats(&col_values);

        Ok(())
    }

    fn auto_encode(&mut self, values: &[ColumnValue]) -> Result<()> {
        // Determine best encoding based on statistics
        self.update_stats(values);

        if self.stats.should_use_dictionary() {
            self.encode_dictionary(values)?;
        } else {
            // Check for run-length encoding opportunity
            let mut rle_encoder = RunLengthEncoder::new();
            rle_encoder.encode(values);

            if rle_encoder.compression_ratio(values.len()) < 0.7 {
                self.encode_rle(values)?;
            } else {
                self.encode_plain(values)?;
            }
        }

        Ok(())
    }

    fn decode(&self) -> Result<Vec<ColumnValue>> {
        match self.encoding {
            EncodingType::Plain => {
                bincode::deserialize(&self.data)
                    .map_err(|e| DbError::Storage(format!("Decoding error: {}", e)))
            }
            EncodingType::Dictionary => {
                let (dict, ids): (Vec<String>, Vec<u32>) = bincode::deserialize(&self.data)
                    .map_err(|e| DbError::Storage(format!("Decoding error: {}", e)))?;

                let values = ids.iter()
                    .map(|&id| {
                        dict.get(id as usize)
                            .map(|s| ColumnValue::String(s.clone()))
                            .unwrap_or(ColumnValue::Null)
                    })
                    .collect();

                Ok(values)
            }
            EncodingType::RunLength => {
                let runs: Vec<(ColumnValue, usize)> = bincode::deserialize(&self.data)
                    .map_err(|e| DbError::Storage(format!("Decoding error: {}", e)))?;

                let mut result = Vec::new();
                for (value, count) in runs {
                    result.extend(std::iter::repeat(value).take(count));
                }

                Ok(result)
            }
            EncodingType::Delta => {
                let (base_value, deltas): (i64, Vec<i32>) = bincode::deserialize(&self.data)
                    .map_err(|e| DbError::Storage(format!("Decoding error: {}", e)))?;

                let mut result = vec![ColumnValue::Int64(base_value)];
                let mut current = base_value;

                for delta in deltas {
                    current += delta as i64;
                    result.push(ColumnValue::Int64(current));
                }

                Ok(result)
            }
            EncodingType::BitPacked => {
                // Simplified bit-packed decoding
                bincode::deserialize(&self.data)
                    .map_err(|e| DbError::Storage(format!("Decoding error: {}", e)))
            }
        }
    }

    fn update_stats(&mut self, values: &[ColumnValue]) {
        self.stats.num_values = values.len();
        self.stats.num_nulls = values.iter().filter(|v| v.is_null()).count();

        let mut distinct = std::collections::HashSet::new();
        let mut min = None;
        let mut max = None;

        for value in values {
            if !value.is_null() {
                distinct.insert(format!("{:?}", value));

                if min.is_none() || value < min.as_ref().unwrap() {
                    min = Some(value.clone());
                }

                if max.is_none() || value > max.as_ref().unwrap() {
                    max = Some(value.clone());
                }
            }
        }

        self.stats.distinct_count = distinct.len();
        self.stats.min_value = min;
        self.stats.max_value = max;

        let original_size = values.len() * 8; // Approximate
        self.stats.compression_ratio = self.data.len() as f64 / original_size as f64;
    }

    /// SIMD-accelerated decompression stub
    /// In production, would use SIMD intrinsics for parallel decompression
    fn simd_decode(&self) -> Result<Vec<ColumnValue>> {
        // Stub for SIMD decompression
        // In production, would use:
        // - std::simd for portable SIMD
        // - Or platform-specific intrinsics (AVX2, NEON)
        self.decode()
    }
}

/// Column definition in a table
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub column_type: ColumnType,
    pub nullable: bool,
}

impl ColumnDef {
    pub fn new(name: String, column_type: ColumnType, nullable: bool) -> Self {
        Self {
            name,
            column_type,
            nullable,
        }
    }
}

/// Columnar table for analytics
pub struct ColumnarTable {
    name: String,
    columns: Vec<ColumnDef>,
    chunks: Arc<RwLock<HashMap<String, Vec<ColumnChunk>>>>,
    row_count: usize,
}

impl ColumnarTable {
    pub fn new(name: String, columns: Vec<ColumnDef>) -> Self {
        Self {
            name,
            columns,
            chunks: Arc::new(RwLock::new(HashMap::new())),
            row_count: 0,
        }
    }

    /// Insert a batch of rows
    pub fn insert_batch(&mut self, rows: Vec<HashMap<String, ColumnValue>>) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        let mut chunks = self.chunks.write();

        for col_def in &self.columns {
            let values: Vec<ColumnValue> = rows.iter()
                .map(|row| {
                    row.get(&col_def.name)
                        .cloned()
                        .unwrap_or(ColumnValue::Null)
                })
                .collect();

            let mut chunk = ColumnChunk::new(col_def.column_type);
            chunk.auto_encode(&values)?;

            chunks.entry(col_def.name.clone())
                .or_insert_with(Vec::new)
                .push(chunk);
        }

        self.row_count += rows.len();
        Ok(())
    }

    /// Scan a single column
    pub fn scan_column(&self, column_name: &str) -> Result<Vec<ColumnValue>> {
        let chunks = self.chunks.read();

        let column_chunks = chunks.get(column_name)
            .ok_or_else(|| DbError::Storage(format!("Column {} not found", column_name)))?;

        let mut result = Vec::new();

        for chunk in column_chunks {
            let values = chunk.simd_decode()?;
            result.extend(values);
        }

        Ok(result)
    }

    /// Project multiple columns
    pub fn project(&self, column_names: &[String]) -> Result<Vec<HashMap<String, ColumnValue>>> {
        let mut columns_data = HashMap::new();

        for col_name in column_names {
            columns_data.insert(col_name.clone(), self.scan_column(col_name)?);
        }

        let mut result = Vec::new();

        for i in 0..self.row_count {
            let mut row = HashMap::new();

            for col_name in column_names {
                if let Some(values) = columns_data.get(col_name) {
                    if i < values.len() {
                        row.insert(col_name.clone(), values[i].clone());
                    }
                }
            }

            result.push(row);
        }

        Ok(result)
    }

    /// Get statistics for a column
    pub fn column_stats(&self, column_name: &str) -> Result<ColumnStats> {
        let chunks = self.chunks.read();

        let column_chunks = chunks.get(column_name)
            .ok_or_else(|| DbError::Storage(format!("Column {} not found", column_name)))?;

        if column_chunks.is_empty() {
            return Ok(ColumnStats::new());
        }

        // Aggregate stats from all chunks
        let mut stats = ColumnStats::new();

        for chunk in column_chunks {
            stats.num_values += chunk.stats.num_values;
            stats.num_nulls += chunk.stats.num_nulls;
            stats.distinct_count += chunk.stats.distinct_count; // Approximation
        }

        if let Some(first_chunk) = column_chunks.first() {
            stats.encoding = first_chunk.encoding;
        }

        Ok(stats)
    }

    pub fn row_count(&self) -> usize {
        self.row_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_encoder() {
        let mut encoder = DictionaryEncoder::new();

        let id1 = encoder.encode("apple");
        let id2 = encoder.encode("banana");
        let id3 = encoder.encode("apple");

        assert_eq!(id1, id3);
        assert_ne!(id1, id2);
        assert_eq!(encoder.decode(id1), Some("apple"));
    }

    #[test]
    fn test_run_length_encoder() {
        let mut encoder = RunLengthEncoder::new();
        let values = vec![
            ColumnValue::Int32(1),
            ColumnValue::Int32(1),
            ColumnValue::Int32(2),
            ColumnValue::Int32(2),
            ColumnValue::Int32(2),
        ];

        encoder.encode(&values);
        let decoded = encoder.decode();

        assert_eq!(values, decoded);
        assert!(encoder.runs.len() < values.len());
    }

    #[test]
    fn test_delta_encoder() {
        let mut encoder = DeltaEncoder::new();
        let values = vec![100, 101, 102, 103, 104];

        encoder.encode(&values);
        let decoded = encoder.decode();

        assert_eq!(values, decoded);
    }

    #[test]
    fn test_columnar_table() {
        let columns = vec![
            ColumnDef::new("id".to_string(), ColumnType::Int32, false),
            ColumnDef::new("name".to_string(), ColumnType::String, true),
        ];

        let mut table = ColumnarTable::new("users".to_string(), columns);

        let mut row1 = HashMap::new();
        row1.insert("id".to_string(), ColumnValue::Int32(1));
        row1.insert("name".to_string(), ColumnValue::String("Alice".to_string()));

        let mut row2 = HashMap::new();
        row2.insert("id".to_string(), ColumnValue::Int32(2));
        row2.insert("name".to_string(), ColumnValue::String("Bob".to_string()));

        table.insert_batch(vec![row1, row2]).unwrap();

        assert_eq!(table.row_count(), 2);

        let ids = table.scan_column("id").unwrap();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_column_projection() {
        let columns = vec![
            ColumnDef::new("a".to_string(), ColumnType::Int32, false),
            ColumnDef::new("b".to_string(), ColumnType::Int32, false),
        ];

        let mut table = ColumnarTable::new("test".to_string(), columns);

        let mut row = HashMap::new();
        row.insert("a".to_string(), ColumnValue::Int32(1));
        row.insert("b".to_string(), ColumnValue::Int32(2));

        table.insert_batch(vec![row]).unwrap();

        let projected = table.project(&vec!["a".to_string()]).unwrap();
        assert_eq!(projected.len(), 1);
    }
}


