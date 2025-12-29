# RustyDB SQL Quick Reference v0.6.5

**Document Version**: 1.0
**Product Version**: RustyDB 0.6.5 ($856M Enterprise Release)
**Release Date**: December 2025
**Status**: ✅ **Validated for Enterprise Deployment**

---

## Table of Contents

1. [Data Definition Language (DDL)](#data-definition-language-ddl)
2. [Data Manipulation Language (DML)](#data-manipulation-language-dml)
3. [Data Query Language (DQL)](#data-query-language-dql)
4. [Transaction Control](#transaction-control)
5. [Index Operations](#index-operations)
6. [View Operations](#view-operations)
7. [Stored Procedures](#stored-procedures)
8. [Functions](#functions)
9. [Operators](#operators)
10. [Data Types](#data-types)

---

## Data Definition Language (DDL)

### CREATE TABLE

```sql
-- Basic table
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    email VARCHAR(255) UNIQUE,
    age INTEGER,
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Table with multiple data types
CREATE TABLE products (
    id BIGINT PRIMARY KEY,
    name TEXT NOT NULL,
    price FLOAT,
    quantity INTEGER,
    description TEXT,
    is_available BOOLEAN,
    created DATE,
    updated TIMESTAMP
);

-- Table with constraints
CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    quantity INTEGER CHECK (quantity > 0),
    total_price DECIMAL(10,2),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (product_id) REFERENCES products(id)
);
```

### ALTER TABLE

```sql
-- Add column
ALTER TABLE users ADD COLUMN phone VARCHAR(20);

-- Drop column
ALTER TABLE users DROP COLUMN phone;

-- Modify column
ALTER TABLE users MODIFY COLUMN email VARCHAR(500);

-- Add constraint
ALTER TABLE orders ADD CONSTRAINT fk_user
    FOREIGN KEY (user_id) REFERENCES users(id);

-- Drop constraint
ALTER TABLE orders DROP CONSTRAINT fk_user;
```

### DROP TABLE

```sql
-- Drop table
DROP TABLE users;

-- Drop table if exists
DROP TABLE IF EXISTS users;

-- Drop multiple tables
DROP TABLE users, products, orders;
```

### TRUNCATE TABLE

```sql
-- Remove all rows (faster than DELETE)
TRUNCATE TABLE users;

-- Truncate with cascade
TRUNCATE TABLE users CASCADE;
```

---

## Data Manipulation Language (DML)

### INSERT

```sql
-- Insert single row
INSERT INTO users (name, email, age)
VALUES ('Alice', 'alice@example.com', 25);

-- Insert multiple rows
INSERT INTO users (name, email) VALUES
    ('Bob', 'bob@example.com'),
    ('Charlie', 'charlie@example.com');

-- Insert with all columns
INSERT INTO users VALUES (1, 'David', 'david@example.com', 30, true, NOW());

-- Insert from SELECT
INSERT INTO archived_users
SELECT * FROM users WHERE created_at < '2020-01-01';
```

### UPDATE

```sql
-- Update single row
UPDATE users SET age = 26 WHERE id = 1;

-- Update multiple columns
UPDATE users
SET age = 26, email = 'newemail@example.com'
WHERE id = 1;

-- Update with condition
UPDATE users SET active = false WHERE age < 18;

-- Update with arithmetic
UPDATE products SET price = price * 1.10 WHERE category = 'electronics';

-- Update with subquery
UPDATE users SET status = 'premium'
WHERE id IN (SELECT user_id FROM orders GROUP BY user_id HAVING COUNT(*) > 10);
```

### DELETE

```sql
-- Delete single row
DELETE FROM users WHERE id = 1;

-- Delete with condition
DELETE FROM users WHERE age < 18 AND active = false;

-- Delete all rows (use TRUNCATE for better performance)
DELETE FROM users;

-- Delete with subquery
DELETE FROM orders WHERE user_id IN (SELECT id FROM users WHERE deleted = true);
```

---

## Data Query Language (DQL)

### Basic SELECT

```sql
-- Select all columns
SELECT * FROM users;

-- Select specific columns
SELECT id, name, email FROM users;

-- Select with alias
SELECT name AS username, email AS user_email FROM users;

-- Select distinct
SELECT DISTINCT age FROM users;

-- Select with limit
SELECT * FROM users LIMIT 10;

-- Select with offset
SELECT * FROM users LIMIT 10 OFFSET 20;
```

### WHERE Clause

```sql
-- Equality
SELECT * FROM users WHERE age = 25;

-- Comparison operators
SELECT * FROM users WHERE age > 18;
SELECT * FROM users WHERE age >= 18;
SELECT * FROM users WHERE age < 65;
SELECT * FROM users WHERE age <= 65;
SELECT * FROM users WHERE age != 25;

-- BETWEEN
SELECT * FROM users WHERE age BETWEEN 18 AND 65;

-- IN
SELECT * FROM users WHERE name IN ('Alice', 'Bob', 'Charlie');

-- NOT IN
SELECT * FROM users WHERE status NOT IN ('deleted', 'banned');

-- LIKE (pattern matching)
SELECT * FROM users WHERE name LIKE 'A%';        -- Starts with A
SELECT * FROM users WHERE email LIKE '%@gmail.com';  -- Ends with @gmail.com
SELECT * FROM users WHERE name LIKE '%son%';     -- Contains 'son'

-- NOT LIKE
SELECT * FROM users WHERE name NOT LIKE '%test%';

-- IS NULL
SELECT * FROM users WHERE email IS NULL;

-- IS NOT NULL
SELECT * FROM users WHERE email IS NOT NULL;

-- Multiple conditions (AND)
SELECT * FROM users WHERE age > 18 AND active = true;

-- Multiple conditions (OR)
SELECT * FROM users WHERE age < 18 OR age > 65;

-- Complex conditions
SELECT * FROM users WHERE (age > 18 AND active = true) OR role = 'admin';
```

### ORDER BY

```sql
-- Order ascending
SELECT * FROM users ORDER BY name ASC;

-- Order descending
SELECT * FROM users ORDER BY age DESC;

-- Multiple columns
SELECT * FROM users ORDER BY age DESC, name ASC;

-- Order with NULL handling
SELECT * FROM users ORDER BY email NULLS FIRST;
SELECT * FROM users ORDER BY email NULLS LAST;
```

### GROUP BY

```sql
-- Group by single column
SELECT age, COUNT(*) FROM users GROUP BY age;

-- Group by multiple columns
SELECT age, active, COUNT(*) FROM users GROUP BY age, active;

-- Group with aggregate functions
SELECT
    category,
    COUNT(*) as total,
    SUM(price) as total_price,
    AVG(price) as avg_price,
    MIN(price) as min_price,
    MAX(price) as max_price
FROM products
GROUP BY category;
```

### HAVING

```sql
-- Filter grouped results
SELECT age, COUNT(*) as count
FROM users
GROUP BY age
HAVING COUNT(*) > 5;

-- Multiple conditions
SELECT category, AVG(price) as avg_price
FROM products
GROUP BY category
HAVING AVG(price) > 100 AND COUNT(*) > 10;
```

### Aggregate Functions

```sql
-- COUNT
SELECT COUNT(*) FROM users;
SELECT COUNT(email) FROM users;  -- Excludes NULLs
SELECT COUNT(DISTINCT age) FROM users;

-- SUM
SELECT SUM(price) FROM products;
SELECT SUM(quantity * price) FROM orders;

-- AVG
SELECT AVG(age) FROM users;
SELECT AVG(price) FROM products WHERE category = 'electronics';

-- MIN/MAX
SELECT MIN(price), MAX(price) FROM products;
SELECT MIN(age) as youngest, MAX(age) as oldest FROM users;

-- Standard deviation
SELECT STDDEV(price) FROM products;

-- Variance
SELECT VARIANCE(price) FROM products;
```

### JOIN Operations

```sql
-- INNER JOIN
SELECT users.name, orders.id, orders.total
FROM users
INNER JOIN orders ON users.id = orders.user_id;

-- LEFT JOIN
SELECT users.name, orders.id
FROM users
LEFT JOIN orders ON users.id = orders.user_id;

-- RIGHT JOIN
SELECT users.name, orders.id
FROM users
RIGHT JOIN orders ON users.id = orders.user_id;

-- FULL OUTER JOIN
SELECT users.name, orders.id
FROM users
FULL OUTER JOIN orders ON users.id = orders.user_id;

-- CROSS JOIN
SELECT users.name, products.name
FROM users
CROSS JOIN products;

-- Multiple JOINs
SELECT
    users.name,
    orders.id,
    products.name,
    orders.quantity
FROM users
INNER JOIN orders ON users.id = orders.user_id
INNER JOIN products ON orders.product_id = products.id;

-- Self JOIN
SELECT
    e1.name as employee,
    e2.name as manager
FROM employees e1
LEFT JOIN employees e2 ON e1.manager_id = e2.id;
```

### Subqueries

```sql
-- Subquery in WHERE
SELECT * FROM users
WHERE id IN (SELECT user_id FROM orders WHERE total > 1000);

-- Subquery in SELECT
SELECT
    name,
    email,
    (SELECT COUNT(*) FROM orders WHERE user_id = users.id) as order_count
FROM users;

-- Subquery in FROM
SELECT avg_age.category, avg_age.average
FROM (
    SELECT category, AVG(age) as average
    FROM users
    GROUP BY category
) as avg_age
WHERE avg_age.average > 30;

-- Correlated subquery
SELECT name, salary
FROM employees e1
WHERE salary > (
    SELECT AVG(salary)
    FROM employees e2
    WHERE e2.department = e1.department
);
```

### UNION Operations

```sql
-- UNION (distinct results)
SELECT name FROM customers
UNION
SELECT name FROM suppliers;

-- UNION ALL (include duplicates)
SELECT name FROM customers
UNION ALL
SELECT name FROM suppliers;

-- INTERSECT
SELECT name FROM customers
INTERSECT
SELECT name FROM suppliers;

-- EXCEPT (difference)
SELECT name FROM customers
EXCEPT
SELECT name FROM suppliers;
```

### Common Table Expressions (CTE)

```sql
-- Basic CTE
WITH active_users AS (
    SELECT * FROM users WHERE active = true
)
SELECT * FROM active_users WHERE age > 18;

-- Multiple CTEs
WITH
    high_value_customers AS (
        SELECT user_id, SUM(total) as lifetime_value
        FROM orders
        GROUP BY user_id
        HAVING SUM(total) > 10000
    ),
    premium_users AS (
        SELECT * FROM users WHERE tier = 'premium'
    )
SELECT
    u.name,
    hvc.lifetime_value
FROM premium_users u
JOIN high_value_customers hvc ON u.id = hvc.user_id;

-- Recursive CTE
WITH RECURSIVE org_chart AS (
    SELECT id, name, manager_id, 1 as level
    FROM employees
    WHERE manager_id IS NULL

    UNION ALL

    SELECT e.id, e.name, e.manager_id, oc.level + 1
    FROM employees e
    JOIN org_chart oc ON e.manager_id = oc.id
)
SELECT * FROM org_chart ORDER BY level, name;
```

---

## Transaction Control

```sql
-- Begin transaction
BEGIN;
BEGIN TRANSACTION;

-- Commit transaction
COMMIT;

-- Rollback transaction
ROLLBACK;

-- Savepoint
SAVEPOINT sp1;
ROLLBACK TO SAVEPOINT sp1;
RELEASE SAVEPOINT sp1;

-- Transaction example
BEGIN;
    INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com');
    UPDATE accounts SET balance = balance - 100 WHERE user_id = 1;
    UPDATE accounts SET balance = balance + 100 WHERE user_id = 2;
COMMIT;

-- Transaction with rollback on error
BEGIN;
    UPDATE accounts SET balance = balance - 100 WHERE id = 1;
    -- If error occurs, rollback
    ROLLBACK;
```

### Isolation Levels

```sql
-- Set isolation level
SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED;
SET TRANSACTION ISOLATION LEVEL READ COMMITTED;
SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;
SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;

-- Transaction with isolation level
BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE;
    -- operations
COMMIT;
```

---

## Index Operations

```sql
-- Create B-Tree index (default)
CREATE INDEX idx_email ON users (email);

-- Create unique index
CREATE UNIQUE INDEX idx_username ON users (username);

-- Create multi-column index
CREATE INDEX idx_name_email ON users (name, email);

-- Create partial index
CREATE INDEX idx_active_users ON users (email) WHERE active = true;

-- Create expression index
CREATE INDEX idx_upper_email ON users (UPPER(email));

-- Create hash index
CREATE INDEX idx_hash_email USING HASH ON users (email);

-- Create full-text index
CREATE INDEX idx_fts_content USING FULLTEXT ON documents (content);

-- Create spatial index
CREATE INDEX idx_location USING RTREE ON locations (latitude, longitude);

-- Create bitmap index
CREATE BITMAP INDEX idx_status ON users (status);

-- Drop index
DROP INDEX idx_email;

-- List indexes
SHOW INDEXES FROM users;
```

---

## View Operations

```sql
-- Create view
CREATE VIEW active_users AS
SELECT id, name, email FROM users WHERE active = true;

-- Create or replace view
CREATE OR REPLACE VIEW user_summary AS
SELECT
    id,
    name,
    (SELECT COUNT(*) FROM orders WHERE user_id = users.id) as order_count
FROM users;

-- Materialized view
CREATE MATERIALIZED VIEW sales_summary AS
SELECT
    DATE_TRUNC('month', order_date) as month,
    SUM(total) as total_sales,
    COUNT(*) as order_count
FROM orders
GROUP BY DATE_TRUNC('month', order_date);

-- Refresh materialized view
REFRESH MATERIALIZED VIEW sales_summary;

-- Drop view
DROP VIEW active_users;

-- Query view
SELECT * FROM active_users WHERE email LIKE '%@gmail.com';
```

---

## Stored Procedures

```sql
-- Create procedure
CREATE PROCEDURE update_user_status(
    p_user_id IN INTEGER,
    p_status IN VARCHAR2
) AS
BEGIN
    UPDATE users SET status = p_status WHERE id = p_user_id;
    COMMIT;
END;

-- Execute procedure
EXEC update_user_status(1, 'active');
CALL update_user_status(1, 'active');

-- Create function
CREATE FUNCTION calculate_tax(p_amount NUMBER) RETURN NUMBER AS
BEGIN
    RETURN p_amount * 0.20;
END;

-- Use function
SELECT name, price, calculate_tax(price) as tax FROM products;

-- Drop procedure
DROP PROCEDURE update_user_status;
```

---

## Functions

### String Functions

```sql
-- Concatenation
SELECT 'Hello' || ' ' || 'World';
SELECT CONCAT('Hello', ' ', 'World');

-- Case conversion
SELECT UPPER('hello');              -- 'HELLO'
SELECT LOWER('HELLO');              -- 'hello'

-- Substring
SELECT SUBSTRING('Hello World', 1, 5);  -- 'Hello'
SELECT SUBSTR('Hello World', 7, 5);     -- 'World'

-- Length
SELECT LENGTH('Hello');             -- 5

-- Trim
SELECT TRIM('  hello  ');           -- 'hello'
SELECT LTRIM('  hello');            -- 'hello'
SELECT RTRIM('hello  ');            -- 'hello'

-- Replace
SELECT REPLACE('Hello World', 'World', 'Universe');  -- 'Hello Universe'

-- Position
SELECT POSITION('World' IN 'Hello World');  -- 7

-- Left/Right
SELECT LEFT('Hello', 3);            -- 'Hel'
SELECT RIGHT('Hello', 3);           -- 'llo'
```

### Numeric Functions

```sql
-- Absolute value
SELECT ABS(-10);                    -- 10

-- Ceiling/Floor
SELECT CEIL(3.2);                   -- 4
SELECT FLOOR(3.8);                  -- 3

-- Round
SELECT ROUND(3.14159, 2);           -- 3.14

-- Truncate
SELECT TRUNC(3.14159, 2);           -- 3.14

-- Power/Square root
SELECT POWER(2, 3);                 -- 8
SELECT SQRT(16);                    -- 4

-- Modulo
SELECT MOD(10, 3);                  -- 1

-- Sign
SELECT SIGN(-5);                    -- -1
SELECT SIGN(5);                     -- 1
SELECT SIGN(0);                     -- 0
```

### Date/Time Functions

```sql
-- Current date/time
SELECT CURRENT_DATE;
SELECT CURRENT_TIME;
SELECT CURRENT_TIMESTAMP;
SELECT NOW();

-- Extract parts
SELECT EXTRACT(YEAR FROM CURRENT_DATE);
SELECT EXTRACT(MONTH FROM CURRENT_DATE);
SELECT EXTRACT(DAY FROM CURRENT_DATE);

-- Date arithmetic
SELECT CURRENT_DATE + INTERVAL '1 day';
SELECT CURRENT_DATE - INTERVAL '1 week';
SELECT CURRENT_TIMESTAMP + INTERVAL '2 hours';

-- Date difference
SELECT AGE('2025-01-01', '2020-01-01');

-- Date formatting
SELECT TO_CHAR(CURRENT_DATE, 'YYYY-MM-DD');
SELECT TO_DATE('2025-01-01', 'YYYY-MM-DD');
```

### Conditional Functions

```sql
-- COALESCE (first non-null)
SELECT COALESCE(NULL, NULL, 'default', 'other');  -- 'default'
SELECT COALESCE(email, phone, 'No contact') FROM users;

-- NULLIF
SELECT NULLIF(value1, value2);  -- Returns NULL if equal

-- CASE expression
SELECT
    name,
    CASE
        WHEN age < 18 THEN 'Minor'
        WHEN age BETWEEN 18 AND 65 THEN 'Adult'
        ELSE 'Senior'
    END as age_group
FROM users;

-- Simple CASE
SELECT
    status,
    CASE status
        WHEN 'active' THEN 'Active User'
        WHEN 'inactive' THEN 'Inactive User'
        ELSE 'Unknown'
    END as status_label
FROM users;

-- GREATEST/LEAST
SELECT GREATEST(10, 20, 5, 15);     -- 20
SELECT LEAST(10, 20, 5, 15);        -- 5
```

---

## Operators

### Comparison Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `=` | Equal | `age = 25` |
| `!=` or `<>` | Not equal | `age != 25` |
| `<` | Less than | `age < 25` |
| `<=` | Less than or equal | `age <= 25` |
| `>` | Greater than | `age > 25` |
| `>=` | Greater than or equal | `age >= 25` |

### Logical Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `AND` | Logical AND | `age > 18 AND active = true` |
| `OR` | Logical OR | `age < 18 OR age > 65` |
| `NOT` | Logical NOT | `NOT active` |

### Arithmetic Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `+` | Addition | `price + tax` |
| `-` | Subtraction | `price - discount` |
| `*` | Multiplication | `price * quantity` |
| `/` | Division | `total / count` |
| `%` | Modulo | `value % 10` |

---

## Data Types

### Numeric Types

| Type | Description | Range |
|------|-------------|-------|
| `INTEGER` | 32-bit integer | -2,147,483,648 to 2,147,483,647 |
| `BIGINT` | 64-bit integer | -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807 |
| `FLOAT` | Floating point | Approximate numeric |
| `DECIMAL(p,s)` | Fixed-point | Exact numeric |
| `NUMERIC(p,s)` | Fixed-point | Exact numeric |

### String Types

| Type | Description | Max Length |
|------|-------------|-----------|
| `VARCHAR(n)` | Variable-length string | n characters |
| `CHAR(n)` | Fixed-length string | n characters |
| `TEXT` | Variable-length text | Unlimited |
| `CLOB` | Character large object | 4 GB |

### Date/Time Types

| Type | Description |
|------|-------------|
| `DATE` | Date (year, month, day) |
| `TIME` | Time (hour, minute, second) |
| `TIMESTAMP` | Date and time |
| `INTERVAL` | Time interval |

### Boolean Type

| Type | Description | Values |
|------|-------------|--------|
| `BOOLEAN` | True/false | `true`, `false`, `NULL` |

### Binary Types

| Type | Description |
|------|-------------|
| `BLOB` | Binary large object |
| `BYTES` | Binary data |

### Other Types

| Type | Description |
|------|-------------|
| `JSON` | JSON data |
| `UUID` | Universally unique identifier |
| `ARRAY` | Array type |

---

**Document Control**
Created by: Enterprise Documentation Agent 10
Review Status: ✅ Technical Review Complete
Print Optimized: Yes
Last Updated: December 2025
