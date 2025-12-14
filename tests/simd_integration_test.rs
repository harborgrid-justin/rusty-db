// Integration tests for SIMD scan engine

use rusty_db::common::Value;
use rusty_db::simd::{
    scan::{ColumnData, ColumnarTable},
    AggregateOp, ColumnScan, FilterOp, PredicateType, SelectionVector, SimdAggregator,
};

#[test]
fn test_simd_filter_pipeline() {
    // Create columnar table
    let mut table = ColumnarTable::new(vec!["id".to_string(), "amount".to_string()]);

    // Add columns
    table
        .add_column(ColumnData::Int32(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]))
        .unwrap();
    table
        .add_column(ColumnData::Float64(vec![
            10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0,
        ]))
        .unwrap();

    // Create scan with filter: WHERE id > 5
    let mut scan = ColumnScan::new().add_filter(FilterOp {
        column_index: 0,
        predicate: PredicateType::GreaterThan,
        values: vec![Value::Integer(5)],
    });

    // Execute scan
    let results = scan.execute(&table).unwrap();

    // Verify results
    assert_eq!(results.len(), 5); // IDs 6, 7, 8, 9, 10
    assert_eq!(results[0][0], Value::Integer(6));
    assert_eq!(results[4][0], Value::Integer(10));
}

#[test]
fn test_simd_aggregation() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let mut agg = SimdAggregator::new();

    // Test SUM
    let sum = agg.aggregate_f64(&data, AggregateOp::Sum).unwrap();
    assert_eq!(sum, 55.0);

    // Test AVG
    let avg = agg.aggregate_f64(&data, AggregateOp::Avg).unwrap();
    assert_eq!(avg, 5.5);

    // Test MIN
    let min = agg.aggregate_f64(&data, AggregateOp::Min).unwrap();
    assert_eq!(min, 1.0);

    // Test MAX
    let max = agg.aggregate_f64(&data, AggregateOp::Max).unwrap();
    assert_eq!(max, 10.0);

    // Test COUNT
    let count = agg.aggregate_f64(&data, AggregateOp::Count).unwrap();
    assert_eq!(count, 10.0);
}

#[test]
fn test_simd_batch_processing() {
    // Create large dataset
    let mut table = ColumnarTable::new(vec!["value".to_string()]);
    let data: Vec<i32> = (0..10000).collect();
    table.add_column(ColumnData::Int32(data)).unwrap();

    // Filter for values between 1000 and 2000
    let mut scan = ColumnScan::new().add_filter(FilterOp::between(
        0,
        Value::Integer(1000),
        Value::Integer(2000),
    ));

    let results = scan.execute(&table).unwrap();
    assert_eq!(results.len(), 1001); // 1000, 1001, ..., 2000
}

#[test]
fn test_simd_late_materialization() {
    let mut table = ColumnarTable::new(vec![
        "id".to_string(),
        "name".to_string(),
        "score".to_string(),
    ]);

    table
        .add_column(ColumnData::Int32(vec![1, 2, 3, 4, 5]))
        .unwrap();
    table
        .add_column(ColumnData::String(vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "David".to_string(),
            "Eve".to_string(),
        ]))
        .unwrap();
    table
        .add_column(ColumnData::Float64(vec![95.5, 87.0, 92.3, 78.5, 88.9]))
        .unwrap();

    // Project only id and name where score > 85
    let mut scan = ColumnScan::new()
        .add_filter(FilterOp {
            column_index: 2,
            predicate: PredicateType::GreaterThan,
            values: vec![Value::Float(85.0)],
        })
        .with_projection(vec![0, 1]); // Only id and name

    let results = scan.execute(&table).unwrap();

    // Should get Alice, Bob, Charlie, Eve (scores > 85)
    assert_eq!(results.len(), 4);
    assert_eq!(results[0].len(), 2); // Only 2 columns projected
}

#[test]
fn test_simd_statistics() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let mut agg = SimdAggregator::new();

    // Perform operations
    let _ = agg.aggregate_i32(&data, AggregateOp::Sum).unwrap();
    let _ = agg.aggregate_i32(&data, AggregateOp::Min).unwrap();

    let stats = agg.stats();
    assert!(stats.rows_processed >= 20); // At least 2 operations * 10 rows
}
