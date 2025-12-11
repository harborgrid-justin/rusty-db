# RustyDB SQL Parser Test Quick Reference

## Test ID Index

### DDL Tests (PARSER-001 to PARSER-010)

| ID | SQL | Status |
|----|-----|--------|
| PARSER-001 | `CREATE TABLE test1 (id INT, name VARCHAR(255))` | ❌ (security) |
| PARSER-002 | `CREATE TABLE test2 (id BIGINT, price FLOAT, name TEXT, active BOOLEAN)` | ✅ |
| PARSER-003 | `CREATE TABLE test3 (id INT, created DATE, updated TIMESTAMP)` | ✅ |
| PARSER-004 | `DROP TABLE nonexistent_table` | ✅ |
| PARSER-005 | `TRUNCATE TABLE parser_test_users` | ❌ (security) |
| PARSER-006 | `CREATE INDEX idx_email ON parser_test_users (email)` | ✅ |
| PARSER-007 | `CREATE INDEX idx_multi ON parser_test_users (name, email)` | ✅ |
| PARSER-008 | `DROP INDEX idx_email` | ❌ (security) |
| PARSER-009 | `CREATE VIEW active_users_view AS SELECT * FROM parser_test_users WHERE active = true` | ✅ |
| PARSER-010 | `DROP VIEW active_users_view` | ❌ (security) |

### SELECT Tests (PARSER-011 to PARSER-017)

| ID | SQL | Status |
|----|-----|--------|
| PARSER-011 | `SELECT * FROM parser_test_users` | ✅ |
| PARSER-012 | `SELECT id, name, email FROM parser_test_users` | ✅ |
| PARSER-013 | `SELECT * FROM parser_test_users WHERE age > 18` | ✅ |
| PARSER-014 | `SELECT * FROM parser_test_users WHERE age > 18 AND active = true` | ✅ |
| PARSER-015 | `SELECT * FROM parser_test_users ORDER BY name ASC` | ✅ |
| PARSER-016 | `SELECT * FROM parser_test_users LIMIT 10` | ✅ |
| PARSER-017 | `SELECT DISTINCT name FROM parser_test_users` | ✅ |

### INSERT Tests (PARSER-018 to PARSER-021)

| ID | SQL | Status |
|----|-----|--------|
| PARSER-018 | `INSERT INTO parser_test_users (name, email) VALUES ('Alice', 'alice@test.com')` | ✅ |
| PARSER-019 | `INSERT INTO parser_test_products (id, price) VALUES (1, 99.99)` | ✅ |
| PARSER-020 | `INSERT INTO parser_test_users (name, active) VALUES ('Bob', true)` | ✅ |
| PARSER-021 | `INSERT INTO parser_test_users (name, email) VALUES ('Charlie', 'c@test.com'), ('David', 'd@test.com')` | ❌ (security) |

### DELETE Tests (PARSER-022 to PARSER-024)

| ID | SQL | Status |
|----|-----|--------|
| PARSER-022 | `DELETE FROM parser_test_users WHERE id = 1` | ✅ |
| PARSER-023 | `DELETE FROM parser_test_users WHERE age < 18 AND active = false` | ✅ |
| PARSER-024 | `DELETE FROM parser_test_users` | ✅ |

### Complex Queries (PARSER-025 to PARSER-030)

| ID | SQL | Status |
|----|-----|--------|
| PARSER-025 | `SELECT * FROM parser_test_users WHERE age BETWEEN 18 AND 65` | ✅ |
| PARSER-026 | `SELECT * FROM parser_test_users WHERE name IN ('Alice', 'Bob', 'Charlie')` | ❌ (security) |
| PARSER-027 | `SELECT * FROM parser_test_users WHERE name LIKE 'A%'` | ✅ |
| PARSER-028 | `SELECT * FROM parser_test_users WHERE email IS NULL` | ✅ |
| PARSER-029 | `SELECT * FROM parser_test_users WHERE email IS NOT NULL` | ✅ |
| PARSER-030 | `SELECT * FROM parser_test_users WHERE name NOT LIKE '%test%'` | ✅ |

### Aggregate Functions (PARSER-031 to PARSER-036)

| ID | SQL | Status |
|----|-----|--------|
| PARSER-031 | `SELECT COUNT(*) FROM parser_test_users` | ✅ |
| PARSER-032 | `SELECT SUM(price) FROM parser_test_products` | ✅ |
| PARSER-033 | `SELECT AVG(age) FROM parser_test_users` | ✅ |
| PARSER-034 | `SELECT MIN(price), MAX(price) FROM parser_test_products` | ✅ |
| PARSER-035 | `SELECT active, COUNT(*) FROM parser_test_users GROUP BY active` | ✅ |
| PARSER-036 | `SELECT active, COUNT(*) FROM parser_test_users GROUP BY active HAVING COUNT(*) > 5` | ✅ |

### String Functions (PARSER-037 to PARSER-041)

| ID | SQL | Status |
|----|-----|--------|
| PARSER-037 | `SELECT UPPER(name) FROM parser_test_users` | ✅ |
| PARSER-038 | `SELECT LOWER(email) FROM parser_test_users` | ✅ |
| PARSER-039 | `SELECT LENGTH(name) FROM parser_test_users` | ✅ |
| PARSER-040 | `SELECT CONCAT(name, email) FROM parser_test_users` | ✅ |
| PARSER-041 | `SELECT SUBSTRING(name, 1, 5) FROM parser_test_users` | ✅ |

### Arithmetic Expressions (PARSER-042 to PARSER-046)

| ID | SQL | Status |
|----|-----|--------|
| PARSER-042 | `SELECT price + tax FROM parser_test_products` | ✅ |
| PARSER-043 | `SELECT price - tax FROM parser_test_products` | ✅ |
| PARSER-044 | `SELECT price * quantity FROM parser_test_products` | ✅ |
| PARSER-045 | `SELECT price / quantity FROM parser_test_products` | ✅ |
| PARSER-046 | `SELECT (price * quantity) + tax FROM parser_test_products` | ✅ |

### SQL Injection Prevention (PARSER-047 to PARSER-050)

| ID | Attack Type | SQL | Status |
|----|-------------|-----|--------|
| PARSER-047 | UNION | `SELECT * FROM parser_test_users WHERE id = 1 UNION SELECT * FROM passwords` | ✅ (blocked) |
| PARSER-048 | Comment | `SELECT * FROM parser_test_users WHERE id = 1 -- AND active = true` | ✅ (blocked) |
| PARSER-049 | Tautology | `SELECT * FROM parser_test_users WHERE id = 1 OR 1=1` | ✅ (blocked) |
| PARSER-050 | Stacked | `SELECT * FROM parser_test_users; DROP TABLE parser_test_users;` | ✅ (blocked) |

### Error Handling (PARSER-051 to PARSER-056)

| ID | Error Type | SQL | Status |
|----|------------|-----|--------|
| PARSER-051 | Missing FROM | `SELECT * WHERE id = 1` | ✅ (error) |
| PARSER-052 | Missing table name | `CREATE TABLE (id INT)` | ✅ (error) |
| PARSER-053 | Invalid keyword | `SELCT * FROM parser_test_users` | ✅ (error) |
| PARSER-054 | Empty SQL | (empty string) | ✅ (error) |
| PARSER-055 | Incomplete WHERE | `SELECT * FROM parser_test_users WHERE` | ✅ (error) |
| PARSER-056 | Unmatched parens | `SELECT * FROM parser_test_users WHERE (age > 18` | ✅ (error) |

## Summary Statistics

- **Total Tests:** 56
- **Passed:** 50 (89.29%)
- **Failed:** 6 (10.71%)
  - All 6 failures due to overly aggressive security blocking legitimate SQL

## Tests by Category Success Rate

| Category | Tests | Passed | Rate |
|----------|-------|--------|------|
| DDL Statements | 10 | 6 | 60.0% |
| SELECT Statements | 7 | 7 | 100% |
| INSERT Statements | 4 | 3 | 75.0% |
| DELETE Statements | 3 | 3 | 100% |
| Complex Queries | 6 | 5 | 83.3% |
| Aggregate Functions | 6 | 6 | 100% |
| String Functions | 5 | 5 | 100% |
| Arithmetic Expressions | 5 | 5 | 100% |
| SQL Injection Prevention | 4 | 4 | 100% |
| Error Handling | 6 | 6 | 100% |

## curl Command Examples

### Basic Query
```bash
curl -s -X POST http://localhost:8080/api/v1/query \
  -H 'Content-Type: application/json' \
  -d '{"sql": "SELECT * FROM parser_test_users"}' | jq '.'
```

### Query with WHERE
```bash
curl -s -X POST http://localhost:8080/api/v1/query \
  -H 'Content-Type: application/json' \
  -d '{"sql": "SELECT * FROM parser_test_users WHERE age > 18"}' | jq '.'
```

### INSERT Statement
```bash
curl -s -X POST http://localhost:8080/api/v1/query \
  -H 'Content-Type: application/json' \
  -d '{"sql": "INSERT INTO parser_test_users (name, email) VALUES ('\''Alice'\'', '\''alice@test.com'\'')"}' | jq '.'
```

### CREATE TABLE
```bash
curl -s -X POST http://localhost:8080/api/v1/query \
  -H 'Content-Type: application/json' \
  -d '{"sql": "CREATE TABLE test (id BIGINT, name TEXT)"}' | jq '.'
```

## Response Interpretation

### Success Response
```json
{
  "query_id": "...",
  "rows": [...],
  "columns": [...],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0
}
```
**Meaning:** Parser succeeded, query executed

### Execution Error
```json
{
  "code": "EXECUTION_ERROR",
  "message": "Catalog error: Table not found",
  "timestamp": ...
}
```
**Meaning:** Parser succeeded, but query failed during execution

### Parser Error
```json
{
  "code": "SQL_PARSE_ERROR",
  "message": "SQL parsing error: ...",
  "timestamp": ...
}
```
**Meaning:** Parser failed to parse the SQL

### Security Block
```json
{
  "code": "SQL_PARSE_ERROR",
  "message": "Injection attempt detected: ...",
  "timestamp": ...
}
```
**Meaning:** Injection prevention blocked the query

## Known False Positives (Security Over-blocking)

1. **VARCHAR keyword** - Blocks CREATE TABLE with VARCHAR columns
2. **TRUNCATE** - Blocks TRUNCATE TABLE statements
3. **DROP INDEX** - Blocks DROP INDEX statements
4. **DROP VIEW** - Blocks DROP VIEW statements
5. **Multi-row INSERT** - Blocks INSERT with multiple VALUES rows
6. **IN clause** - Blocks SELECT with IN (value1, value2, ...)

## Workarounds

### Instead of VARCHAR, use TEXT
```sql
-- ❌ Blocked
CREATE TABLE users (name VARCHAR(255))

-- ✅ Works
CREATE TABLE users (name TEXT)
```

### Instead of multi-row INSERT, use multiple statements
```sql
-- ❌ Blocked
INSERT INTO users VALUES ('Alice', 'a@test.com'), ('Bob', 'b@test.com')

-- ✅ Works
INSERT INTO users VALUES ('Alice', 'a@test.com')
INSERT INTO users VALUES ('Bob', 'b@test.com')
```

### Instead of IN clause, use OR
```sql
-- ❌ Blocked
SELECT * FROM users WHERE name IN ('Alice', 'Bob')

-- ✅ Works
SELECT * FROM users WHERE name = 'Alice' OR name = 'Bob'
```

---

**Quick Reference Generated:** December 11, 2025
**For Full Report:** See PARSER_TEST_REPORT_FINAL.md
