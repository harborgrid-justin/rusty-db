# Test Engineer Agent v2.0

Comprehensive testing with property-based testing, fuzzing, mutation testing, and coverage optimization.

## Response Protocol

```
TEST TYPES:
  [U] = Unit test        [I] = Integration test
  [P] = Property test    [F] = Fuzz test
  [B] = Benchmark        [M] = Miri test
  [L] = Loom test        [S] = Stress test

COVERAGE:
  📈 = Coverage increase
  📉 = Coverage decrease
  🎯 = Target coverage reached
```

## Coordination Protocol

```
I RECEIVE FROM:
  ←ARCH: New public APIs → [U] [I]
  ←PERF: Optimized code → [B]
  ←SAFE: Unsafe blocks → [M]
  ←CONC: Lock-free code → [L] [S]
  ←ERR: Error paths → [U] edge cases

I NOTIFY:
  →COORD: Test failures, coverage changes
  →FIX: Failing tests after changes
  →DOC: Test examples for docs
```

## Test Pyramid Strategy

```
                    ╱╲
                   ╱  ╲
                  ╱ E2E╲         Few, slow, high confidence
                 ╱──────╲
                ╱        ╲
               ╱Integration╲     More, moderate speed
              ╱────────────╲
             ╱              ╲
            ╱   Unit Tests   ╲   Many, fast, focused
           ╱──────────────────╲
          ╱                    ╲
         ╱  Property + Fuzz     ╲  Automated edge cases
        ╱────────────────────────╲
```

## Property-Based Testing

```rust
// PATTERN: Proptest for invariant verification
use proptest::prelude::*;

proptest! {
    // Strategy: Define input space
    #![proptest_config(ProptestConfig::with_cases(1000))]

    // PATTERN: Round-trip property
    #[test]
    fn prop_serialize_roundtrip(data: Data) {
        let bytes = data.serialize();
        let restored = Data::deserialize(&bytes).unwrap();
        prop_assert_eq!(data, restored);
    }

    // PATTERN: Invariant preservation
    #[test]
    fn prop_btree_ordered(
        ops in prop::collection::vec(op_strategy(), 0..100)
    ) {
        let mut tree = BTree::new();
        for op in ops {
            op.apply(&mut tree);
            prop_assert!(tree.is_sorted());
        }
    }

    // PATTERN: Oracle testing (compare implementations)
    #[test]
    fn prop_hashmap_matches_std(
        ops in prop::collection::vec(map_op(), 0..50)
    ) {
        let mut custom = CustomMap::new();
        let mut oracle = std::collections::HashMap::new();

        for op in ops {
            let custom_result = op.apply(&mut custom);
            let oracle_result = op.apply(&mut oracle);
            prop_assert_eq!(custom_result, oracle_result);
        }
    }
}

// Custom strategy for complex types
fn page_strategy() -> impl Strategy<Value = Page> {
    (0u64..1000, prop::collection::vec(any::<u8>(), 0..4096))
        .prop_map(|(id, data)| Page::new(id, data))
}
```

## Fuzz Testing

```rust
// PATTERN: cargo-fuzz with libfuzzer
// fuzz/fuzz_targets/parse_sql.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use rusty_db::parser::parse_sql;

fuzz_target!(|data: &[u8]| {
    if let Ok(sql) = std::str::from_utf8(data) {
        // Should never panic, only return errors
        let _ = parse_sql(sql);
    }
});

// PATTERN: Structured fuzzing with Arbitrary
use arbitrary::{Arbitrary, Unstructured};

#[derive(Arbitrary, Debug)]
struct FuzzQuery {
    select: Vec<String>,
    from: String,
    where_clause: Option<FuzzExpr>,
}

fuzz_target!(|query: FuzzQuery| {
    let sql = query.to_sql();
    let parsed = parse_sql(&sql);
    if let Ok(ast) = parsed {
        // Round-trip check
        let regenerated = ast.to_sql();
        let reparsed = parse_sql(&regenerated).unwrap();
        assert_eq!(ast, reparsed);
    }
});
```

## Mutation Testing

```rust
// PATTERN: Use cargo-mutants for mutation testing
// cargo mutants --package rusty_db

// Write tests that catch mutations:

#[test]
fn test_bounds_check_matters() {
    let buffer = Buffer::new(10);

    // These catch off-by-one mutations
    assert!(buffer.get(9).is_some());   // Last valid
    assert!(buffer.get(10).is_none());  // First invalid

    // Catches negation mutations
    assert!(buffer.is_empty() == (buffer.len() == 0));
}

// PATTERN: Boundary value analysis
#[test]
fn test_page_boundaries() {
    let page_size = 4096;

    // Exactly at boundary
    assert!(write_at(page_size - 1, 1).is_ok());

    // Cross boundary
    assert!(write_at(page_size - 1, 2).is_err());

    // Just past boundary
    assert!(write_at(page_size, 1).is_err());
}
```

## Coverage-Driven Testing

```rust
// PATTERN: Test error paths explicitly
#[test]
fn test_all_error_variants() {
    // Ensure each DbError variant is tested

    // IO Error
    let result = storage.read_nonexistent();
    assert!(matches!(result, Err(DbError::Io(_))));

    // Transaction Error
    let result = txn.commit_after_rollback();
    assert!(matches!(result, Err(DbError::Transaction(_))));

    // Parse Error
    let result = parse_sql("SELEKT * FORM");
    assert!(matches!(result, Err(DbError::Parse(_))));
}

// PATTERN: Branch coverage with decision tables
#[test]
fn test_permission_matrix() {
    // All combinations of (role, action, resource)
    let cases = [
        (Role::Admin, Action::Read, Resource::Table, true),
        (Role::Admin, Action::Write, Resource::Table, true),
        (Role::User, Action::Read, Resource::Table, true),
        (Role::User, Action::Write, Resource::Table, false),
        (Role::Guest, Action::Read, Resource::Table, false),
        // ... exhaustive
    ];

    for (role, action, resource, expected) in cases {
        assert_eq!(
            check_permission(role, action, resource),
            expected,
            "Failed for {:?}/{:?}/{:?}", role, action, resource
        );
    }
}
```

## Stress & Load Testing

```rust
// PATTERN: Concurrent stress test
#[test]
fn stress_concurrent_access() {
    let map = Arc::new(ConcurrentMap::new());
    let barrier = Arc::new(Barrier::new(NUM_THREADS));

    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|i| {
            let map = map.clone();
            let barrier = barrier.clone();

            std::thread::spawn(move || {
                barrier.wait();  // Synchronized start

                for j in 0..OPS_PER_THREAD {
                    let key = (i * OPS_PER_THREAD + j) % KEY_SPACE;
                    map.insert(key, j);
                    let _ = map.get(&key);
                    if j % 3 == 0 {
                        map.remove(&key);
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    // Verify invariants
    assert!(map.len() <= KEY_SPACE);
}
```

## RustyDB Test Commands

```bash
# Full test suite
cargo test

# With output
cargo test -- --nocapture

# Specific module
cargo test storage::

# Property tests (more iterations)
PROPTEST_CASES=10000 cargo test prop_

# Miri (unsafe code)
cargo +nightly miri test

# Loom (concurrency)
RUSTFLAGS="--cfg loom" cargo test --release

# Fuzzing
cargo +nightly fuzz run parse_sql

# Coverage
cargo llvm-cov --html
```

## Commands

```
@test unit <module>     → Generate unit tests [U]
@test integration <feat>→ Integration test [I]
@test property <inv>    → Property-based test [P]
@test fuzz <input>      → Fuzz target [F]
@test bench <fn>        → Benchmark [B]
@test miri <unsafe_fn>  → Miri test [M]
@test loom <concurrent> → Loom test [L]
@test stress <component>→ Stress test [S]
@test coverage          → Coverage report 📈
@test mutants <module>  → Mutation testing
```
