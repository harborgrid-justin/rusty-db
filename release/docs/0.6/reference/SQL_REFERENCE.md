# RustyDB v0.6.0 - SQL Quick Reference

**Version**: 0.6.0 | **Oracle Compatible** | **Updated**: December 28, 2025

---

## DDL Statements

### CREATE TABLE
```sql
-- Basic table
CREATE TABLE users (
    id INTEGER,
    name TEXT,
    email TEXT,
    age INTEGER,
    active BOOLEAN
);

-- With types
CREATE TABLE products (
    id BIGINT,
    price FLOAT,
    name TEXT,
    description TEXT,
    created DATE,
    updated TIMESTAMP
);
```

### DROP TABLE
```sql
DROP TABLE users;
```

### CREATE INDEX
```sql
-- Single column
CREATE INDEX idx_email ON users (email);

-- Multiple columns
CREATE INDEX idx_name_email ON users (name, email);

-- Bitmap index (low cardinality)
CREATE BITMAP INDEX idx_active ON users (active);

-- Expression index
CREATE INDEX idx_upper_email ON users (UPPER(email));
```

### DROP INDEX
```sql
DROP INDEX idx_email;
```

### CREATE VIEW
```sql
CREATE VIEW active_users AS
SELECT * FROM users WHERE active = true;
```

### DROP VIEW
```sql
DROP VIEW active_users;
```

---

## DML Statements

### INSERT
```sql
-- Single row
INSERT INTO users (id, name, email)
VALUES (1, 'Alice', 'alice@example.com');

-- With boolean
INSERT INTO users (id, name, active)
VALUES (2, 'Bob', true);
```

### UPDATE
```sql
-- Update single row
UPDATE users
SET email = 'newemail@example.com'
WHERE id = 1;

-- Update multiple rows
UPDATE users
SET active = false
WHERE age < 18;
```

### DELETE
```sql
-- Delete specific row
DELETE FROM users WHERE id = 1;

-- Delete with conditions
DELETE FROM users
WHERE age < 18 AND active = false;

-- Delete all rows (use with caution!)
DELETE FROM users;
```

---

## SELECT Statements

### Basic SELECT
```sql
-- All columns
SELECT * FROM users;

-- Specific columns
SELECT id, name, email FROM users;

-- With alias
SELECT id AS user_id, name AS user_name FROM users;
```

### WHERE Clauses
```sql
-- Equality
SELECT * FROM users WHERE age = 25;

-- Comparison
SELECT * FROM users WHERE age > 18;
SELECT * FROM users WHERE age >= 21;
SELECT * FROM users WHERE age < 65;
SELECT * FROM users WHERE age <= 30;

-- Multiple conditions
SELECT * FROM users WHERE age > 18 AND active = true;
SELECT * FROM users WHERE age < 18 OR age > 65;

-- BETWEEN
SELECT * FROM users WHERE age BETWEEN 18 AND 65;

-- LIKE pattern matching
SELECT * FROM users WHERE name LIKE 'A%';
SELECT * FROM users WHERE email LIKE '%@gmail.com';

-- NULL checks
SELECT * FROM users WHERE email IS NULL;
SELECT * FROM users WHERE email IS NOT NULL;

-- NOT conditions
SELECT * FROM users WHERE name NOT LIKE '%test%';
```

### ORDER BY
```sql
-- Ascending
SELECT * FROM users ORDER BY name ASC;

-- Descending
SELECT * FROM users ORDER BY age DESC;

-- Multiple columns
SELECT * FROM users ORDER BY age DESC, name ASC;
```

### LIMIT
```sql
-- Limit results
SELECT * FROM users LIMIT 10;

-- With offset
SELECT * FROM users LIMIT 10 OFFSET 20;
```

### DISTINCT
```sql
SELECT DISTINCT name FROM users;
SELECT DISTINCT age, active FROM users;
```

---

## Aggregate Functions

### COUNT
```sql
SELECT COUNT(*) FROM users;
SELECT COUNT(DISTINCT name) FROM users;
```

### SUM
```sql
SELECT SUM(price) FROM products;
SELECT SUM(quantity * price) AS total FROM orders;
```

### AVG
```sql
SELECT AVG(age) FROM users;
SELECT AVG(price) FROM products;
```

### MIN / MAX
```sql
SELECT MIN(price) FROM products;
SELECT MAX(price) FROM products;
SELECT MIN(age), MAX(age) FROM users;
```

### GROUP BY
```sql
-- Simple grouping
SELECT active, COUNT(*)
FROM users
GROUP BY active;

-- With aggregate
SELECT age, COUNT(*) AS count, AVG(age) AS avg_age
FROM users
GROUP BY age;

-- HAVING clause
SELECT active, COUNT(*) AS count
FROM users
GROUP BY active
HAVING COUNT(*) > 5;
```

---

## String Functions

```sql
-- Upper/Lower case
SELECT UPPER(name) FROM users;
SELECT LOWER(email) FROM users;

-- Length
SELECT LENGTH(name) FROM users;

-- Substring
SELECT SUBSTRING(name, 1, 5) FROM users;

-- Concatenation
SELECT CONCAT(name, ' ', email) FROM users;
SELECT name || ' ' || email FROM users;

-- Trim
SELECT TRIM(name) FROM users;
SELECT LTRIM(name) FROM users;
SELECT RTRIM(name) FROM users;

-- Replace
SELECT REPLACE(email, '@', ' AT ') FROM users;
```

---

## Numeric Functions

```sql
-- Absolute value
SELECT ABS(price - cost) FROM products;

-- Rounding
SELECT ROUND(price, 2) FROM products;
SELECT CEIL(price) FROM products;
SELECT FLOOR(price) FROM products;
SELECT TRUNC(price, 2) FROM products;

-- Power and Square Root
SELECT POWER(2, 3);  -- 8
SELECT SQRT(16);     -- 4

-- Modulo
SELECT MOD(10, 3);   -- 1

-- Sign
SELECT SIGN(-5);     -- -1
```

---

## Arithmetic Expressions

```sql
-- Addition
SELECT price + tax FROM products;

-- Subtraction
SELECT price - discount FROM products;

-- Multiplication
SELECT price * quantity FROM orders;

-- Division
SELECT price / quantity FROM products;

-- Complex expressions
SELECT (price * quantity) + tax AS total FROM orders;
SELECT price * (1 + tax_rate) AS final_price FROM products;
```

---

## Conversion Functions

```sql
-- To Character
SELECT TO_CHAR(123);
SELECT TO_CHAR(price) FROM products;

-- To Number
SELECT TO_NUMBER('123');
SELECT TO_NUMBER(string_value) FROM data;

-- To Date
SELECT TO_DATE('2025-12-28');
```

---

## NULL Functions

```sql
-- NVL - Return default if null
SELECT NVL(email, 'no-email@example.com') FROM users;

-- NVL2 - Different values for null/not-null
SELECT NVL2(email, 'Has Email', 'No Email') FROM users;

-- COALESCE - First non-null value
SELECT COALESCE(email, phone, address, 'No Contact') FROM users;
```

---

## Conditional Functions

```sql
-- DECODE
SELECT DECODE(status,
    'active', 'Active User',
    'inactive', 'Inactive User',
    'Unknown'
) FROM users;

-- CASE statement
SELECT
    CASE
        WHEN age < 18 THEN 'Minor'
        WHEN age >= 18 AND age < 65 THEN 'Adult'
        ELSE 'Senior'
    END AS age_group
FROM users;

-- Simple CASE
SELECT
    CASE status
        WHEN 'active' THEN 'Active'
        WHEN 'inactive' THEN 'Inactive'
        ELSE 'Unknown'
    END AS status_label
FROM users;

-- GREATEST / LEAST
SELECT GREATEST(10, 20, 30);  -- 30
SELECT LEAST(10, 20, 30);     -- 10
```

---

## Transaction Control

```sql
-- Start transaction (implicit with first DML)
BEGIN;

-- Or explicitly
START TRANSACTION;

-- Commit changes
COMMIT;

-- Rollback changes
ROLLBACK;

-- Savepoint
SAVEPOINT sp1;
-- ... operations ...
ROLLBACK TO sp1;
```

---

## Data Types

| Type | Description | Example |
|------|-------------|---------|
| `INTEGER` | Integer number | `42` |
| `BIGINT` | Large integer | `9223372036854775807` |
| `FLOAT` | Floating point | `3.14` |
| `DOUBLE` | Double precision | `3.14159265359` |
| `DECIMAL(p,s)` | Fixed precision | `DECIMAL(10,2)` |
| `NUMBER(p,s)` | Oracle numeric | `NUMBER(10,2)` |
| `TEXT` | Variable text | `'Hello World'` |
| `VARCHAR(n)` | Variable char | `VARCHAR(255)` |
| `VARCHAR2(n)` | Oracle varchar | `VARCHAR2(255)` |
| `CHAR(n)` | Fixed char | `CHAR(10)` |
| `BOOLEAN` | True/False | `true`, `false` |
| `DATE` | Date only | `'2025-12-28'` |
| `TIMESTAMP` | Date & time | `'2025-12-28 10:30:00'` |
| `BLOB` | Binary large object | Binary data |
| `CLOB` | Character large object | Large text |
| `JSON` | JSON data | `'{"key":"value"}'` |

---

## Operators

### Comparison
- `=` Equal
- `!=` or `<>` Not equal
- `>` Greater than
- `>=` Greater than or equal
- `<` Less than
- `<=` Less than or equal

### Logical
- `AND` Logical AND
- `OR` Logical OR
- `NOT` Logical NOT

### Pattern Matching
- `LIKE` Pattern match (`%` = any chars, `_` = single char)
- `NOT LIKE` Negated pattern match

### Range
- `BETWEEN x AND y` Value in range
- `IN (list)` Value in list
- `NOT IN (list)` Value not in list

### NULL
- `IS NULL` Is null
- `IS NOT NULL` Is not null

---

## Common Query Patterns

### Find duplicates
```sql
SELECT email, COUNT(*)
FROM users
GROUP BY email
HAVING COUNT(*) > 1;
```

### Top N records
```sql
SELECT * FROM users
ORDER BY age DESC
LIMIT 10;
```

### Pagination
```sql
-- Page 1 (first 10 records)
SELECT * FROM users LIMIT 10 OFFSET 0;

-- Page 2 (next 10 records)
SELECT * FROM users LIMIT 10 OFFSET 10;
```

### Conditional UPDATE
```sql
UPDATE users
SET status = CASE
    WHEN age < 18 THEN 'minor'
    WHEN age >= 65 THEN 'senior'
    ELSE 'adult'
END;
```

### Upsert pattern (conditional insert/update)
```sql
-- Check if exists
SELECT COUNT(*) FROM users WHERE id = 1;

-- If count > 0: UPDATE
UPDATE users SET name = 'New Name' WHERE id = 1;

-- If count = 0: INSERT
INSERT INTO users (id, name) VALUES (1, 'New Name');
```

---

## Performance Tips

1. **Use indexes** on frequently queried columns
2. **Avoid SELECT *** - specify needed columns
3. **Use LIMIT** for large result sets
4. **Create compound indexes** for multi-column WHERE clauses
5. **Use EXPLAIN** to analyze query plans
6. **Use appropriate data types** - smaller is faster
7. **Batch INSERT operations** when possible
8. **Use WHERE conditions** to filter early

---

## Security Best Practices

1. **Use parameterized queries** (prevents SQL injection)
2. **Avoid dynamic SQL** with user input
3. **Validate input** before queries
4. **Use least privilege** - minimal permissions
5. **Sanitize strings** in application layer
6. **Enable audit logging** for sensitive operations
7. **Use prepared statements** when available

---

## SQL Injection Prevention

**RustyDB automatically blocks**:
- UNION attacks
- Comment-based attacks
- Tautology attacks (e.g., `1=1`)
- Stacked queries

**Always use parameterized queries in applications**:
```sql
-- ❌ UNSAFE
query = "SELECT * FROM users WHERE id = " + user_input;

-- ✅ SAFE (use API parameters)
{
  "sql": "SELECT * FROM users WHERE id = ?",
  "params": [user_input]
}
```

---

## Limitations

**Not supported in v0.6.0**:
- JOINs (use separate queries)
- Subqueries
- Window functions
- Common Table Expressions (CTEs)
- Foreign keys
- Triggers (use Procedures API)

**Use GraphQL for**:
- Complex joins
- Nested queries
- Batch operations

---

**SQL Reference** | RustyDB v0.6.0 | Oracle-Compatible Database
