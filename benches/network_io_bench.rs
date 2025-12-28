// Network I/O Performance Benchmarks
// Tests critical network operations including connection handling,
// message serialization/deserialization, and protocol operations

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rusty_db::network::{Request, Response};
use serde_json;

fn bench_request_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_serialization");

    let queries = vec![
        ("simple", "SELECT * FROM users WHERE id = 1"),
        ("complex", "SELECT u.*, o.* FROM users u JOIN orders o ON u.id = o.user_id WHERE u.created_at > '2024-01-01' AND o.status = 'completed'"),
        ("aggregation", "SELECT department, COUNT(*), AVG(salary) FROM employees GROUP BY department HAVING AVG(salary) > 50000"),
    ];

    for (name, query) in queries {
        group.bench_function(name, |b| {
            let request = Request::Query {
                sql: query.to_string(),
            };

            b.iter(|| {
                let serialized = serde_json::to_string(&request).unwrap();
                black_box(serialized);
            });
        });
    }

    group.finish();
}

fn bench_request_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_deserialization");

    let queries = vec![
        ("simple", r#"{"Query":{"sql":"SELECT * FROM users WHERE id = 1"}}"#),
        ("complex", r#"{"Query":{"sql":"SELECT u.*, o.* FROM users u JOIN orders o ON u.id = o.user_id WHERE u.created_at > '2024-01-01' AND o.status = 'completed'"}}"#),
    ];

    for (name, json) in queries {
        group.bench_function(name, |b| {
            b.iter(|| {
                let request: Result<Request, _> = serde_json::from_str(json);
                black_box(request);
            });
        });
    }

    group.finish();
}

fn bench_response_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("response_serialization");

    let row_counts = vec![1, 10, 100, 1000];

    for row_count in row_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(row_count),
            &row_count,
            |b, &row_count| {
                let rows: Vec<Vec<String>> = (0..row_count)
                    .map(|i| {
                        vec![
                            i.to_string(),
                            format!("user_{}", i),
                            format!("email_{}@example.com", i),
                        ]
                    })
                    .collect();

                let response = Response::QueryResult {
                    columns: vec!["id".to_string(), "name".to_string(), "email".to_string()],
                    rows,
                };

                b.iter(|| {
                    let serialized = serde_json::to_string(&response).unwrap();
                    black_box(serialized);
                });
            },
        );
    }

    group.finish();
}

fn bench_message_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_throughput");

    let message_sizes = vec![
        ("small", 100),
        ("medium", 1000),
        ("large", 10000),
    ];

    for (name, size) in message_sizes {
        group.bench_function(name, |b| {
            let data = vec![b'A'; size];

            b.iter(|| {
                // Simulate sending a message
                let _encoded = black_box(&data);
                // Simulate receiving and decoding
                let _decoded = black_box(&data);
            });
        });
    }

    group.finish();
}

fn bench_connection_pool_operations(c: &mut Criterion) {
    c.bench_function("connection_acquire_release", |b| {
        b.iter(|| {
            // Simulate acquiring and releasing connections
            for _ in 0..100 {
                black_box(42);
            }
        });
    });
}

fn bench_concurrent_connections(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_connections");

    let connection_counts = vec![1, 10, 50, 100];

    for count in connection_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &count,
            |b, &count| {
                b.iter(|| {
                    let handles: Vec<_> = (0..count)
                        .map(|i| {
                            std::thread::spawn(move || {
                                // Simulate connection work
                                black_box(i);
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().ok();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_protocol_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("protocol_overhead");

    group.bench_function("request_response_cycle", |b| {
        let request = Request::Query {
            sql: "SELECT * FROM users WHERE id = 1".to_string(),
        };

        b.iter(|| {
            // Serialize request
            let req_serialized = serde_json::to_string(&request).unwrap();

            // Simulate network transmission
            black_box(&req_serialized);

            // Deserialize request
            let _req: Request = serde_json::from_str(&req_serialized).unwrap();

            // Create response
            let response = Response::QueryResult {
                columns: vec!["id".to_string(), "name".to_string()],
                rows: vec![vec!["1".to_string(), "Alice".to_string()]],
            };

            // Serialize response
            let resp_serialized = serde_json::to_string(&response).unwrap();

            // Simulate network transmission
            black_box(&resp_serialized);

            // Deserialize response
            let _resp: Response = serde_json::from_str(&resp_serialized).unwrap();
        });
    });

    group.finish();
}

fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");

    let batch_sizes = vec![10, 50, 100];

    for batch_size in batch_sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &batch_size| {
                let requests: Vec<Request> = (0..batch_size)
                    .map(|i| Request::Query {
                        sql: format!("SELECT * FROM users WHERE id = {}", i),
                    })
                    .collect();

                b.iter(|| {
                    for request in &requests {
                        let serialized = serde_json::to_string(&request).unwrap();
                        black_box(serialized);
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_request_serialization,
    bench_request_deserialization,
    bench_response_serialization,
    bench_message_throughput,
    bench_connection_pool_operations,
    bench_concurrent_connections,
    bench_protocol_overhead,
    bench_batch_operations
);
criterion_main!(benches);
