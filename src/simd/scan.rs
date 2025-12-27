// # Columnar Scan Operations
//
// SIMD-optimized table scanning with predicate pushdown, late materialization,
// and batch processing for maximum throughput.

use super::{
    filter::SimdFilter, FilterOp, PredicateType, SelectionVector, SimdContext, SimdStats,
    BATCH_SIZE,
};
use crate::common::Value;
use crate::error::{DbError, Result};

/// Scan strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanStrategy {
    /// Sequential scan of all rows
    Sequential,
    /// Index-guided scan
    IndexScan,
    /// Bitmap index scan
    BitmapScan,
    /// Skip scan for sparse predicates
    SkipScan,
}

/// Column data type for scanning
#[derive(Debug, Clone)]
pub enum ColumnData {
    /// i32 integer column
    Int32(Vec<i32>),
    /// i64 integer column
    Int64(Vec<i64>),
    /// f32 float column
    Float32(Vec<f32>),
    /// f64 float column
    Float64(Vec<f64>),
    /// String column
    String(Vec<String>),
    /// Boolean column
    Boolean(Vec<bool>),
    /// Nullable i32 column
    NullableInt32(Vec<Option<i32>>),
    /// Nullable i64 column
    NullableInt64(Vec<Option<i64>>),
    /// Nullable f64 column
    NullableFloat64(Vec<Option<f64>>),
}

impl ColumnData {
    /// Get number of rows in column
    pub fn len(&self) -> usize {
        match self {
            ColumnData::Int32(v) => v.len(),
            ColumnData::Int64(v) => v.len(),
            ColumnData::Float32(v) => v.len(),
            ColumnData::Float64(v) => v.len(),
            ColumnData::String(v) => v.len(),
            ColumnData::Boolean(v) => v.len(),
            ColumnData::NullableInt32(v) => v.len(),
            ColumnData::NullableInt64(v) => v.len(),
            ColumnData::NullableFloat64(v) => v.len(),
        }
    }

    /// Check if column is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get value at index
    pub fn get(&self, index: usize) -> Option<Value> {
        match self {
            ColumnData::Int32(v) => v.get(index).map(|&x| Value::Integer(x as i64)),
            ColumnData::Int64(v) => v.get(index).map(|&x| Value::Integer(x)),
            ColumnData::Float32(v) => v.get(index).map(|&x| Value::Float(x as f64)),
            ColumnData::Float64(v) => v.get(index).map(|&x| Value::Float(x)),
            ColumnData::String(v) => v.get(index).map(|x| Value::String(x.clone())),
            ColumnData::Boolean(v) => v.get(index).map(|&x| Value::Boolean(x)),
            ColumnData::NullableInt32(v) => v
                .get(index)
                .and_then(|x| x.map(|val| Value::Integer(val as i64))),
            ColumnData::NullableInt64(v) => {
                v.get(index).and_then(|x| x.map(|val| Value::Integer(val)))
            }
            ColumnData::NullableFloat64(v) => {
                v.get(index).and_then(|x| x.map(|val| Value::Float(val)))
            }
        }
    }
}

/// Columnar table for scanning
#[derive(Debug, Clone)]
pub struct ColumnarTable {
    /// Column data
    columns: Vec<ColumnData>,
    /// Column names
    column_names: Vec<String>,
    /// Number of rows
    row_count: usize,
}

impl ColumnarTable {
    /// Create new columnar table
    pub fn new(column_names: Vec<String>) -> Self {
        Self {
            columns: Vec::new(),
            column_names,
            row_count: 0,
        }
    }

    /// Add column data
    pub fn add_column(&mut self, data: ColumnData) -> Result<()> {
        if !self.columns.is_empty() && data.len() != self.row_count {
            return Err(DbError::InvalidArgument(
                "Column length must match table row count".to_string(),
            ));
        }

        if self.columns.is_empty() {
            self.row_count = data.len();
        }

        self.columns.push(data);
        Ok(())
    }

    /// Get column by index
    pub fn column(&self, index: usize) -> Option<&ColumnData> {
        self.columns.get(index)
    }

    /// Get column by name
    pub fn column_by_name(&self, name: &str) -> Option<(usize, &ColumnData)> {
        self.column_names
            .iter()
            .position(|n| n == name)
            .and_then(|idx| self.columns.get(idx).map(|col| (idx, col)))
    }

    /// Get row count
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Get column count
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Materialize row at index
    pub fn materialize_row(&self, index: usize) -> Option<Vec<Value>> {
        if index >= self.row_count {
            return None;
        }

        let mut values = Vec::with_capacity(self.columns.len());
        for col in &self.columns {
            values.push(col.get(index).unwrap_or(Value::Null));
        }
        Some(values)
    }

    /// Materialize rows from selection vector
    pub fn materialize_selection(&self, selection: &SelectionVector) -> Vec<Vec<Value>> {
        let mut rows = Vec::with_capacity(selection.len());
        for &idx in selection.indices() {
            if let Some(row) = self.materialize_row(idx) {
                rows.push(row);
            }
        }
        rows
    }
}

/// SIMD-optimized columnar scanner
pub struct ColumnScan {
    /// SIMD context
    context: SimdContext,
    /// Scan strategy
    strategy: ScanStrategy,
    /// Filter operations
    filters: Vec<FilterOp>,
    /// Projection (column indices to materialize)
    projection: Option<Vec<usize>>,
    /// Batch size for processing
    batch_size: usize,
}

impl ColumnScan {
    /// Create new column scanner
    pub fn new() -> Self {
        Self {
            context: SimdContext::new(),
            strategy: ScanStrategy::Sequential,
            filters: Vec::new(),
            projection: None,
            batch_size: BATCH_SIZE,
        }
    }

    /// Set scan strategy
    pub fn with_strategy(mut self, strategy: ScanStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Add filter operation
    pub fn add_filter(mut self, filter: FilterOp) -> Self {
        self.filters.push(filter);
        self
    }

    /// Set projection (columns to materialize)
    pub fn with_projection(mut self, projection: Vec<usize>) -> Self {
        self.projection = Some(projection);
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Execute scan on columnar table
    pub fn execute(&mut self, table: &ColumnarTable) -> Result<Vec<Vec<Value>>> {
        match self.strategy {
            ScanStrategy::Sequential => self.sequential_scan(table),
            ScanStrategy::IndexScan => self.index_scan(table),
            ScanStrategy::BitmapScan => self.bitmap_scan(table),
            ScanStrategy::SkipScan => self.skip_scan(table),
        }
    }

    /// Sequential scan with SIMD filtering
    fn sequential_scan(&mut self, table: &ColumnarTable) -> Result<Vec<Vec<Value>>> {
        let row_count = table.row_count();
        let mut selection = SelectionVector::with_capacity(row_count);

        // Initialize with all rows selected
        for i in 0..row_count {
            let _ = selection.add(i);
        }

        // Clone filters to avoid borrowing self
        let filters = self.filters.clone();

        // Apply filters with early termination
        for filter in &filters {
            if selection.is_empty() {
                break;
            }

            let filtered_selection = self.apply_filter(table, filter, &selection)?;
            selection = filtered_selection;
        }

        // Late materialization - only materialize selected rows
        Ok(self.materialize_with_projection(table, &selection))
    }

    /// Apply single filter to selection
    fn apply_filter(
        &mut self,
        table: &ColumnarTable,
        filter: &FilterOp,
        input_selection: &SelectionVector,
    ) -> Result<SelectionVector> {
        let column = table.column(filter.column_index).ok_or_else(|| {
            DbError::InvalidArgument(format!(
                "Column index {} out of bounds",
                filter.column_index
            ))
        })?;

        let mut output_selection = SelectionVector::with_capacity(input_selection.len());
        let mut simd_filter = SimdFilter::with_context(self.context.clone());

        match column {
            ColumnData::Int32(data) => {
                // If we have a full selection, filter entire column
                if input_selection.len() == table.row_count() {
                    simd_filter.filter_i32(
                        data,
                        filter.predicate,
                        &filter.values,
                        &mut output_selection,
                    )?;
                } else {
                    // Filter only selected rows (sparse selection)
                    self.filter_sparse_i32(data, filter, input_selection, &mut output_selection)?;
                }
            }
            ColumnData::Int64(data) => {
                self.filter_i64(data, filter, input_selection, &mut output_selection)?;
            }
            ColumnData::Float32(data) => {
                self.filter_f32(data, filter, input_selection, &mut output_selection)?;
            }
            ColumnData::Float64(data) => {
                self.filter_f64(data, filter, input_selection, &mut output_selection)?;
            }
            ColumnData::String(data) => {
                self.filter_string(data, filter, input_selection, &mut output_selection)?;
            }
            ColumnData::Boolean(data) => {
                self.filter_boolean(data, filter, input_selection, &mut output_selection)?;
            }
            ColumnData::NullableInt32(data) => {
                self.filter_nullable_i32(data, filter, input_selection, &mut output_selection)?;
            }
            _ => {
                return Err(DbError::InvalidArgument(
                    "Unsupported column type for filtering".to_string(),
                ));
            }
        }

        // Update context stats
        self.context.stats.merge(&simd_filter.stats());

        Ok(output_selection)
    }

    /// Filter sparse selection of i32 values
    fn filter_sparse_i32(
        &self,
        data: &[i32],
        filter: &FilterOp,
        input_selection: &SelectionVector,
        output_selection: &mut SelectionVector,
    ) -> Result<()> {
        if filter.values.is_empty() {
            return Err(DbError::InvalidArgument(
                "Filter requires at least one value".to_string(),
            ));
        }

        let value = match &filter.values[0] {
            Value::Integer(v) => *v as i32,
            _ => {
                return Err(DbError::InvalidArgument(
                    "Expected integer value".to_string(),
                ))
            }
        };

        // For sparse selections, use scalar filtering
        for &idx in input_selection.indices() {
            if idx >= data.len() {
                continue;
            }

            let matches = match filter.predicate {
                PredicateType::Equal => data[idx] == value,
                PredicateType::NotEqual => data[idx] != value,
                PredicateType::LessThan => data[idx] < value,
                PredicateType::LessThanOrEqual => data[idx] <= value,
                PredicateType::GreaterThan => data[idx] > value,
                PredicateType::GreaterThanOrEqual => data[idx] >= value,
                PredicateType::Between => {
                    if filter.values.len() < 2 {
                        return Err(DbError::InvalidArgument(
                            "BETWEEN requires two values".to_string(),
                        ));
                    }
                    let high = match &filter.values[1] {
                        Value::Integer(v) => *v as i32,
                        _ => {
                            return Err(DbError::InvalidArgument(
                                "Expected integer value".to_string(),
                            ))
                        }
                    };
                    data[idx] >= value && data[idx] <= high
                }
                _ => false,
            };

            if matches {
                let _ = output_selection.add(idx);
            }
        }

        Ok(())
    }

    /// Filter i64 column
    fn filter_i64(
        &self,
        data: &[i64],
        filter: &FilterOp,
        input_selection: &SelectionVector,
        output_selection: &mut SelectionVector,
    ) -> Result<()> {
        if filter.values.is_empty() {
            return Err(DbError::InvalidArgument(
                "Filter requires at least one value".to_string(),
            ));
        }

        let value = match &filter.values[0] {
            Value::Integer(v) => *v,
            _ => {
                return Err(DbError::InvalidArgument(
                    "Expected integer value".to_string(),
                ))
            }
        };

        for &idx in input_selection.indices() {
            if idx >= data.len() {
                continue;
            }

            let matches = match filter.predicate {
                PredicateType::Equal => data[idx] == value,
                PredicateType::NotEqual => data[idx] != value,
                PredicateType::LessThan => data[idx] < value,
                PredicateType::LessThanOrEqual => data[idx] <= value,
                PredicateType::GreaterThan => data[idx] > value,
                PredicateType::GreaterThanOrEqual => data[idx] >= value,
                _ => false,
            };

            if matches {
                let _ = output_selection.add(idx);
            }
        }

        Ok(())
    }

    /// Filter f32 column
    fn filter_f32(
        &self,
        data: &[f32],
        filter: &FilterOp,
        input_selection: &SelectionVector,
        output_selection: &mut SelectionVector,
    ) -> Result<()> {
        if filter.values.is_empty() {
            return Err(DbError::InvalidArgument(
                "Filter requires at least one value".to_string(),
            ));
        }

        let value = match &filter.values[0] {
            Value::Float(v) => *v as f32,
            _ => return Err(DbError::InvalidArgument("Expected float value".to_string())),
        };

        for &idx in input_selection.indices() {
            if idx >= data.len() {
                continue;
            }

            let matches = match filter.predicate {
                PredicateType::Equal => data[idx] == value,
                PredicateType::LessThan => data[idx] < value,
                PredicateType::GreaterThan => data[idx] > value,
                _ => false,
            };

            if matches {
                let _ = output_selection.add(idx);
            }
        }

        Ok(())
    }

    /// Filter f64 column
    fn filter_f64(
        &self,
        data: &[f64],
        filter: &FilterOp,
        input_selection: &SelectionVector,
        output_selection: &mut SelectionVector,
    ) -> Result<()> {
        if filter.values.is_empty() {
            return Err(DbError::InvalidArgument(
                "Filter requires at least one value".to_string(),
            ));
        }

        let value = match &filter.values[0] {
            Value::Float(v) => *v,
            _ => return Err(DbError::InvalidArgument("Expected float value".to_string())),
        };

        for &idx in input_selection.indices() {
            if idx >= data.len() {
                continue;
            }

            let matches = match filter.predicate {
                PredicateType::Equal => data[idx] == value,
                PredicateType::LessThan => data[idx] < value,
                PredicateType::GreaterThan => data[idx] > value,
                _ => false,
            };

            if matches {
                let _ = output_selection.add(idx);
            }
        }

        Ok(())
    }

    /// Filter string column
    fn filter_string(
        &self,
        data: &[String],
        filter: &FilterOp,
        input_selection: &SelectionVector,
        output_selection: &mut SelectionVector,
    ) -> Result<()> {
        if filter.values.is_empty() {
            return Err(DbError::InvalidArgument(
                "Filter requires at least one value".to_string(),
            ));
        }

        let value = match &filter.values[0] {
            Value::String(v) => v,
            _ => return Err(DbError::InvalidArgument("Expected text value".to_string())),
        };

        for &idx in input_selection.indices() {
            if idx >= data.len() {
                continue;
            }

            let matches = match filter.predicate {
                PredicateType::Equal => &data[idx] == value,
                PredicateType::NotEqual => &data[idx] != value,
                _ => false,
            };

            if matches {
                let _ = output_selection.add(idx);
            }
        }

        Ok(())
    }

    /// Filter boolean column
    fn filter_boolean(
        &self,
        data: &[bool],
        filter: &FilterOp,
        input_selection: &SelectionVector,
        output_selection: &mut SelectionVector,
    ) -> Result<()> {
        if filter.values.is_empty() {
            return Err(DbError::InvalidArgument(
                "Filter requires at least one value".to_string(),
            ));
        }

        let value = match &filter.values[0] {
            Value::Boolean(v) => *v,
            _ => {
                return Err(DbError::InvalidArgument(
                    "Expected boolean value".to_string(),
                ))
            }
        };

        for &idx in input_selection.indices() {
            if idx >= data.len() {
                continue;
            }

            if data[idx] == value {
                let _ = output_selection.add(idx);
            }
        }

        Ok(())
    }

    /// Filter nullable i32 column
    fn filter_nullable_i32(
        &self,
        data: &[Option<i32>],
        filter: &FilterOp,
        input_selection: &SelectionVector,
        output_selection: &mut SelectionVector,
    ) -> Result<()> {
        match filter.predicate {
            PredicateType::IsNull => {
                for &idx in input_selection.indices() {
                    if idx < data.len() && data[idx].is_none() {
                        let _ = output_selection.add(idx);
                    }
                }
                Ok(())
            }
            PredicateType::IsNotNull => {
                for &idx in input_selection.indices() {
                    if idx < data.len() && data[idx].is_some() {
                        let _ = output_selection.add(idx);
                    }
                }
                Ok(())
            }
            _ => {
                if filter.values.is_empty() {
                    return Err(DbError::InvalidArgument(
                        "Filter requires at least one value".to_string(),
                    ));
                }

                let value = match &filter.values[0] {
                    Value::Integer(v) => *v as i32,
                    _ => {
                        return Err(DbError::InvalidArgument(
                            "Expected integer value".to_string(),
                        ))
                    }
                };

                for &idx in input_selection.indices() {
                    if idx >= data.len() {
                        continue;
                    }

                    if let Some(val) = data[idx] {
                        let matches = match filter.predicate {
                            PredicateType::Equal => val == value,
                            PredicateType::LessThan => val < value,
                            PredicateType::GreaterThan => val > value,
                            _ => false,
                        };

                        if matches {
                            let _ = output_selection.add(idx);
                        }
                    }
                }

                Ok(())
            }
        }
    }

    /// Index-guided scan (placeholder)
    fn index_scan(&mut self, table: &ColumnarTable) -> Result<Vec<Vec<Value>>> {
        // For now, fallback to sequential scan
        // In a real implementation, this would use index structures
        self.sequential_scan(table)
    }

    /// Bitmap index scan (placeholder)
    fn bitmap_scan(&mut self, table: &ColumnarTable) -> Result<Vec<Vec<Value>>> {
        // For now, fallback to sequential scan
        // In a real implementation, this would use bitmap indexes
        self.sequential_scan(table)
    }

    /// Skip scan for sparse predicates (placeholder)
    fn skip_scan(&mut self, table: &ColumnarTable) -> Result<Vec<Vec<Value>>> {
        // For now, fallback to sequential scan
        // In a real implementation, this would skip large portions of data
        self.sequential_scan(table)
    }

    /// Materialize selected rows with projection
    fn materialize_with_projection(
        &self,
        table: &ColumnarTable,
        selection: &SelectionVector,
    ) -> Vec<Vec<Value>> {
        let mut rows = Vec::with_capacity(selection.len());

        if let Some(ref projection) = self.projection {
            // Materialize only projected columns
            for &idx in selection.indices() {
                let mut row = Vec::with_capacity(projection.len());
                for &col_idx in projection {
                    if let Some(col) = table.column(col_idx) {
                        row.push(col.get(idx).unwrap_or(Value::Null));
                    }
                }
                rows.push(row);
            }
        } else {
            // Materialize all columns
            for &idx in selection.indices() {
                if let Some(row) = table.materialize_row(idx) {
                    rows.push(row);
                }
            }
        }

        rows
    }

    /// Get scan statistics
    pub fn stats(&self) -> &SimdStats {
        &self.context.stats
    }
}

impl Default for ColumnScan {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch processor for streaming scans
pub struct BatchProcessor {
    /// Batch size
    batch_size: usize,
    /// Current position
    position: usize,
}

impl BatchProcessor {
    /// Create new batch processor
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            position: 0,
        }
    }

    /// Process next batch
    pub fn next_batch<'a>(&mut self, data: &'a [i32]) -> Option<&'a [i32]> {
        if self.position >= data.len() {
            return None;
        }

        let end = std::cmp::min(self.position + self.batch_size, data.len());
        let batch = &data[self.position..end];
        self.position = end;
        Some(batch)
    }

    /// Reset processor
    pub fn reset(&mut self) {
        self.position = 0;
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_columnar_table() {
        let mut table = ColumnarTable::new(vec!["id".to_string(), "value".to_string()]);

        table
            .add_column(ColumnData::Int32(vec![1, 2, 3, 4, 5]))
            .unwrap();
        table
            .add_column(ColumnData::Int32(vec![10, 20, 30, 40, 50]))
            .unwrap();

        assert_eq!(table.row_count(), 5);
        assert_eq!(table.column_count(), 2);

        let row = table.materialize_row(2).unwrap();
        assert_eq!(row.len(), 2);
    }

    #[test]
    fn test_column_scan() {
        let mut table = ColumnarTable::new(vec!["id".to_string(), "value".to_string()]);
        table
            .add_column(ColumnData::Int32(vec![1, 2, 3, 4, 5]))
            .unwrap();
        table
            .add_column(ColumnData::Int32(vec![10, 20, 30, 40, 50]))
            .unwrap();

        let mut scan = ColumnScan::new().add_filter(FilterOp::equal(0, Value::Integer(3)));

        let results = scan.execute(&table).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_batch_processor() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut processor = BatchProcessor::new(3);

        let batch1 = processor.next_batch(&data).unwrap();
        assert_eq!(batch1, &[1, 2, 3]);

        let batch2 = processor.next_batch(&data).unwrap();
        assert_eq!(batch2, &[4, 5, 6]);

        let batch3 = processor.next_batch(&data).unwrap();
        assert_eq!(batch3, &[7, 8, 9]);

        let batch4 = processor.next_batch(&data).unwrap();
        assert_eq!(batch4, &[10]);

        assert!(processor.next_batch(&data).is_none());
    }
}
