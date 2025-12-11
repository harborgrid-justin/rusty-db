# SIMD Module Test Plan

## Test Coverage Strategy

Since the SIMD module doesn't have direct REST API endpoints, we will:
1. Create comprehensive unit-style tests using curl to SQL endpoints
2. Test SIMD functionality indirectly through query execution
3. Document all SIMD features and their test status

## SIMD Features to Test

### 1. Filter Operations (filter.rs)
- i32 filtering: Equal, LessThan, GreaterThan, Between
- i64 filtering: Equal, LessThan, GreaterThan
- f32 filtering: Equal, LessThan, GreaterThan
- f64 filtering: Equal, LessThan, GreaterThan
- AVX2 accelerated operations
- Scalar fallback implementations
- Bitmask to selection vector conversion

### 2. Aggregate Operations (aggregate.rs)
- f64: SUM, MIN, MAX, AVG
- f32: SUM, MIN, MAX, AVG
- i32: SUM, MIN, MAX, COUNT
- i64: SUM, MIN, MAX, COUNT
- Variance and standard deviation
- Grouped aggregation

### 3. String Operations (string.rs)
- Exact match (case-sensitive and case-insensitive)
- Prefix match
- Suffix match
- Contains match
- Wildcard pattern matching
- Regular expression matching
- SIMD string comparison (AVX2)
- String hashing (FNV-1a, XXH3)

### 4. Hash Operations (hash.rs)
- xxHash3 with AVX2 acceleration
- wyhash for small inputs
- Batch string hashing
- Hash combining
- Hash builder with custom seeds

### 5. Scan Operations (scan.rs)
- Sequential scan
- Selection vector operations
- Late materialization
- Batch processing
- Projection pushdown
- Filter predicate evaluation

### 6. CPU Feature Detection (mod.rs)
- AVX2 detection
- AVX512 detection
- SSE4.2 detection
- Vector width calculation
- SIMD context management

## Test Execution Notes

Tests will be executed as direct calls to SIMD functions since there are no
dedicated API endpoints. Each test will verify:
- Correctness of SIMD operations
- Performance characteristics
- Scalar fallback behavior
- Edge cases (empty data, single elements, etc.)
