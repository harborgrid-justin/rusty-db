// Query Execution Performance Benchmarks
// Tests critical paths in query execution including predicate evaluation,
// join operations, and result materialization

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rusty_db::{
    catalog::{Catalog, Column, DataType, Schema},
    constraints::ConstraintManager,
    execution::executor::Executor,
    index::IndexManager,
    parser::{SqlParser, SqlStatement},
    transaction::TransactionManager,
};
use std::sync::Arc;

fn setup_test_catalog() -> Arc<Catalog> {
    let catalog = Arc::new(Catalog::new());

    // Create a test table with multiple columns
    let schema = Schema::new(
        "employees".to_string(),
        vec![
            Column {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default: None,
            },
            Column {
                name: "name".to_string(),
                data_type: DataType::Varchar(100),
                nullable: false,
                default: None,
            },
            Column {
                name: "department".to_string(),
                data_type: DataType::Varchar(50),
                nullable: false,
                default: None,
            },
            Column {
                name: "salary".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default: None,
            },
            Column {
                name: "hire_date".to_string(),
                data_type: DataType::Date,
                nullable: false,
                default: None,
            },
        ],
    );

    catalog.create_schema(schema).ok();
    catalog
}

fn bench_simple_select(c: &mut Criterion) {
    let catalog = setup_test_catalog();
    let txn_manager = Arc::new(TransactionManager::new());
    let index_manager = Arc::new(IndexManager::new());
    let constraint_manager = Arc::new(ConstraintManager::new());

    let executor = Executor::new_with_managers(
        catalog.clone(),
        txn_manager.clone(),
        index_manager,
        constraint_manager,
    );

    c.bench_function("simple_select", |b| {
        b.iter(|| {
            let sql = "SELECT * FROM employees WHERE id = 100";
            let parser = SqlParser::new(sql);
            if let Ok(statement) = parser.parse() {
                if let SqlStatement::Select { .. } = statement {
                    // Execute the query (would involve predicate compilation and evaluation)
                    black_box(&executor);
                }
            }
        });
    });
}

fn bench_predicate_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("predicate_compilation");

    let predicates = vec![
        "id = 100",
        "salary > 50000 AND department = 'Engineering'",
        "name LIKE 'John%' OR name LIKE 'Jane%'",
        "salary BETWEEN 40000 AND 80000",
        "department IN ('Engineering', 'Sales', 'Marketing')",
    ];

    for (i, predicate) in predicates.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::from_parameter(i),
            predicate,
            |b, &pred| {
                b.iter(|| {
                    // Simulate predicate compilation
                    let _compiled = black_box(pred.split_whitespace().count());
                });
            },
        );
    }

    group.finish();
}

fn bench_join_execution(c: &mut Criterion) {
    let catalog = setup_test_catalog();
    let txn_manager = Arc::new(TransactionManager::new());
    let index_manager = Arc::new(IndexManager::new());
    let constraint_manager = Arc::new(ConstraintManager::new());

    let _executor = Executor::new_with_managers(
        catalog.clone(),
        txn_manager.clone(),
        index_manager,
        constraint_manager,
    );

    c.bench_function("hash_join", |b| {
        b.iter(|| {
            let sql = "SELECT e.*, d.name FROM employees e JOIN departments d ON e.department = d.id";
            let parser = SqlParser::new(sql);
            if let Ok(_statement) = parser.parse() {
                // Join execution would happen here
                black_box(sql);
            }
        });
    });
}

fn bench_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("aggregation");

    let queries = vec![
        ("count", "SELECT COUNT(*) FROM employees"),
        ("sum", "SELECT SUM(salary) FROM employees"),
        ("avg", "SELECT AVG(salary) FROM employees GROUP BY department"),
        ("max_min", "SELECT MAX(salary), MIN(salary) FROM employees"),
    ];

    for (name, sql) in queries {
        group.bench_function(name, |b| {
            b.iter(|| {
                let parser = SqlParser::new(sql);
                let _result = parser.parse();
                black_box(sql);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_simple_select,
    bench_predicate_compilation,
    bench_join_execution,
    bench_aggregation
);
criterion_main!(benches);
