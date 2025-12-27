# RustyDB v0.5.1 SQL Reference

**Version**: 0.5.1
**Last Updated**: December 27, 2025
**SQL Standard**: SQL:2016 Core Features
**Compatibility**: Oracle Database 19c, PostgreSQL 14+

---

## Table of Contents

1. [SQL Overview](#sql-overview)
2. [Data Definition Language (DDL)](#data-definition-language-ddl)
3. [Data Manipulation Language (DML)](#data-manipulation-language-dml)
4. [Transaction Control](#transaction-control)
5. [Query Features](#query-features)
6. [Built-in Functions](#built-in-functions)
7. [Operators](#operators)
8. [Data Types](#data-types)
9. [Constraints](#constraints)
10. [SQL Compliance](#sql-compliance)

---

## SQL Overview

### SQL Standard Compliance

RustyDB v0.5.1 provides **SQL:2016 core feature compliance** with extensive Oracle Database and PostgreSQL compatibility. The implementation includes:

- **Core SQL-92**: Full support for SELECT, INSERT, UPDATE, DELETE
- **SQL:1999**: Support for CTEs, CASE expressions, window functions
- **SQL:2003**: Window functions, MERGE operations
- **SQL:2016**: JSON support, temporal data operations

### Oracle Compatibility Features

- PL/SQL-compatible stored procedures
- Oracle-style outer join syntax (`(+)` operator)
- DUAL table support
- ROWNUM pseudo-column (via LIMIT/OFFSET)
- Flashback query capabilities
- Advanced security features (VPD, TDE, data masking)

### PostgreSQL Compatibility

- PostgreSQL wire protocol support
- JSONB data type
- Array data types
- Full-text search
- Geometric data types (via spatial module)

### RustyDB Extensions

- **Advanced MVCC**: Nanosecond-precision snapshots
- **Multi-Engine**: Relational, graph, document, spatial in one database
- **ML Integration**: In-database machine learning
- **Real-time Analytics**: Streaming query support
- **Blockchain Audit**: Immutable audit logs with cryptographic verification

---

## Data Definition Language (DDL)

### CREATE TABLE

Creates a new table in the database.

**Syntax**:
```sql
CREATE TABLE table_name (
    column_name data_type [column_constraint] [, ...]
    [, table_constraint] [, ...]
);
```

**Examples**:
```sql
-- Basic table creation
CREATE TABLE employees (
    id INTEGER,
    name VARCHAR(100),
    email VARCHAR(255),
    hire_date DATE
);

-- Table with constraints
CREATE TABLE customers (
    customer_id INTEGER PRIMARY KEY,
    customer_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    credit_limit DECIMAL(10,2) CHECK (credit_limit >= 0)
);

-- Table with foreign key
CREATE TABLE orders (
    order_id INTEGER PRIMARY KEY,
    customer_id INTEGER,
    order_date DATE NOT NULL,
    total_amount DECIMAL(12,2),
    FOREIGN KEY (customer_id) REFERENCES customers(customer_id)
);
```

**Column Constraints**:
- `PRIMARY KEY` - Uniquely identifies each row
- `NOT NULL` - Column cannot contain NULL values
- `UNIQUE` - All values must be unique
- `CHECK (condition)` - Value must satisfy condition
- `DEFAULT value` - Default value when not specified
- `FOREIGN KEY` - References another table

---

### CREATE INDEX

Creates an index on one or more columns to improve query performance.

**Syntax**:
```sql
CREATE [UNIQUE] INDEX index_name
ON table_name (column_name [, ...]);
```

**Examples**:
```sql
-- Simple index
CREATE INDEX idx_employee_name ON employees(name);

-- Unique index
CREATE UNIQUE INDEX idx_employee_email ON employees(email);

-- Composite index
CREATE INDEX idx_order_customer_date
ON orders(customer_id, order_date);

-- Partial index (with WHERE clause)
CREATE INDEX idx_active_customers
ON customers(customer_id)
WHERE status = 'active';
```

**Index Types Available**:
- **B-Tree Index** (default) - General purpose, sorted data
- **Hash Index** - Equality comparisons
- **LSM-Tree Index** - Write-optimized
- **Bitmap Index** - Low-cardinality columns
- **Full-Text Index** - Text search
- **Spatial Index** (R-Tree) - Geospatial data

---

### CREATE VIEW

Creates a virtual table based on a SELECT query.

**Syntax**:
```sql
CREATE [OR REPLACE] VIEW view_name AS
    SELECT_statement;
```

**Examples**:
```sql
-- Simple view
CREATE VIEW active_employees AS
SELECT id, name, email
FROM employees
WHERE status = 'active';

-- View with joins
CREATE VIEW order_summary AS
SELECT
    o.order_id,
    c.customer_name,
    o.order_date,
    o.total_amount
FROM orders o
JOIN customers c ON o.customer_id = c.customer_id;

-- Replace existing view
CREATE OR REPLACE VIEW high_value_customers AS
SELECT customer_id, customer_name, total_purchases
FROM customer_stats
WHERE total_purchases > 10000;
```

---

### CREATE DATABASE

Creates a new database.

**Syntax**:
```sql
CREATE DATABASE database_name;
```

**Examples**:
```sql
CREATE DATABASE production_db;
CREATE DATABASE analytics_warehouse;
```

---

### CREATE PROCEDURE

Creates a stored procedure (PL/SQL compatible).

**Syntax**:
```sql
CREATE PROCEDURE procedure_name (
    parameter_name [IN|OUT|IN OUT] data_type [, ...]
)
AS
BEGIN
    -- Procedure body
END;
```

**Examples**:
```sql
-- Simple procedure
CREATE PROCEDURE update_salary (
    emp_id IN INTEGER,
    new_salary IN DECIMAL
)
AS
BEGIN
    UPDATE employees
    SET salary = new_salary
    WHERE id = emp_id;
END;

-- Procedure with output parameter
CREATE PROCEDURE get_employee_count (
    dept_id IN INTEGER,
    emp_count OUT INTEGER
)
AS
BEGIN
    SELECT COUNT(*) INTO emp_count
    FROM employees
    WHERE department_id = dept_id;
END;
```

---

### ALTER TABLE

Modifies an existing table structure.

**Syntax**:
```sql
ALTER TABLE table_name
    ADD COLUMN column_name data_type [constraints]
  | DROP COLUMN column_name
  | ALTER COLUMN column_name SET DATA TYPE data_type
  | MODIFY COLUMN column_name data_type [NULL|NOT NULL]
  | ADD CONSTRAINT constraint_name constraint_definition
  | DROP CONSTRAINT constraint_name
  | DROP DEFAULT column_name;
```

**Examples**:
```sql
-- Add column
ALTER TABLE employees ADD COLUMN phone VARCHAR(20);

-- Drop column
ALTER TABLE employees DROP COLUMN middle_name;

-- Change column type
ALTER TABLE employees ALTER COLUMN salary SET DATA TYPE DECIMAL(12,2);

-- Modify column nullability
ALTER TABLE employees MODIFY COLUMN email VARCHAR(255) NOT NULL;

-- Add constraint
ALTER TABLE employees ADD CONSTRAINT chk_salary
    CHECK (salary > 0);

-- Add foreign key
ALTER TABLE orders ADD CONSTRAINT fk_customer
    FOREIGN KEY (customer_id) REFERENCES customers(customer_id);

-- Drop constraint
ALTER TABLE employees DROP CONSTRAINT chk_salary;

-- Drop default value
ALTER TABLE employees DROP DEFAULT hire_date;
```

---

### DROP Statements

Removes database objects.

**Syntax**:
```sql
DROP TABLE [IF EXISTS] table_name;
DROP INDEX [IF EXISTS] index_name;
DROP VIEW [IF EXISTS] view_name;
DROP DATABASE [IF EXISTS] database_name;
```

**Examples**:
```sql
-- Drop table
DROP TABLE old_employees;
DROP TABLE IF EXISTS temp_data;

-- Drop index
DROP INDEX idx_old_column;

-- Drop view
DROP VIEW IF EXISTS deprecated_view;

-- Drop database
DROP DATABASE test_database;
```

---

### TRUNCATE TABLE

Removes all rows from a table quickly (faster than DELETE).

**Syntax**:
```sql
TRUNCATE TABLE table_name;
```

**Examples**:
```sql
TRUNCATE TABLE session_logs;
TRUNCATE TABLE staging_data;
```

**Notes**:
- Much faster than `DELETE FROM table_name`
- Resets auto-increment counters
- Cannot be rolled back in some configurations
- Removes all rows without firing DELETE triggers

---

## Data Manipulation Language (DML)

### SELECT

Retrieves data from one or more tables.

**Basic Syntax**:
```sql
SELECT [DISTINCT] column_list
FROM table_name
[WHERE condition]
[GROUP BY column_list]
[HAVING condition]
[ORDER BY column_list [ASC|DESC]]
[LIMIT count]
[OFFSET skip_count];
```

**Examples**:
```sql
-- Simple SELECT
SELECT * FROM employees;

-- Select specific columns
SELECT id, name, email FROM employees;

-- SELECT with WHERE clause
SELECT name, salary
FROM employees
WHERE salary > 50000;

-- SELECT DISTINCT
SELECT DISTINCT department_id FROM employees;

-- SELECT with ORDER BY
SELECT name, hire_date
FROM employees
ORDER BY hire_date DESC;

-- SELECT with LIMIT and OFFSET (pagination)
SELECT * FROM products
ORDER BY product_id
LIMIT 10 OFFSET 20;

-- SELECT with aggregate functions
SELECT department_id, COUNT(*), AVG(salary)
FROM employees
GROUP BY department_id;

-- SELECT with HAVING
SELECT department_id, AVG(salary) as avg_salary
FROM employees
GROUP BY department_id
HAVING AVG(salary) > 60000;
```

---

### INSERT

Adds new rows to a table.

**Syntax**:
```sql
-- Insert specific columns
INSERT INTO table_name (column1, column2, ...)
VALUES (value1, value2, ...);

-- Insert all columns
INSERT INTO table_name
VALUES (value1, value2, ...);

-- Insert multiple rows
INSERT INTO table_name (column1, column2)
VALUES
    (value1a, value2a),
    (value1b, value2b),
    (value1c, value2c);
```

**Examples**:
```sql
-- Insert single row
INSERT INTO employees (id, name, email, hire_date)
VALUES (1, 'John Doe', 'john@example.com', '2025-01-15');

-- Insert with auto-generated ID
INSERT INTO customers (name, email)
VALUES ('Jane Smith', 'jane@example.com');

-- Insert multiple rows
INSERT INTO products (name, price, category)
VALUES
    ('Laptop', 999.99, 'Electronics'),
    ('Mouse', 29.99, 'Electronics'),
    ('Desk', 299.99, 'Furniture');
```

---

### INSERT INTO SELECT

Copies data from one table to another.

**Syntax**:
```sql
INSERT INTO target_table (column_list)
SELECT column_list
FROM source_table
WHERE condition;
```

**Examples**:
```sql
-- Copy all active employees
INSERT INTO active_employees (id, name, email)
SELECT id, name, email
FROM employees
WHERE status = 'active';

-- Copy with transformation
INSERT INTO employee_archive (emp_id, full_name, archived_date)
SELECT id, CONCAT(first_name, ' ', last_name), CURRENT_TIMESTAMP
FROM employees
WHERE termination_date IS NOT NULL;
```

---

### SELECT INTO

Creates a new table from query results.

**Syntax**:
```sql
SELECT column_list
INTO new_table
FROM source_table
WHERE condition;
```

**Examples**:
```sql
-- Create backup table
SELECT *
INTO employees_backup
FROM employees;

-- Create summary table
SELECT department_id, COUNT(*) as emp_count, AVG(salary) as avg_salary
INTO department_summary
FROM employees
GROUP BY department_id;
```

---

### UPDATE

Modifies existing rows in a table.

**Syntax**:
```sql
UPDATE table_name
SET column1 = value1, column2 = value2, ...
WHERE condition;
```

**Examples**:
```sql
-- Update single column
UPDATE employees
SET salary = 75000
WHERE id = 1001;

-- Update multiple columns
UPDATE employees
SET salary = salary * 1.1,
    last_updated = CURRENT_TIMESTAMP
WHERE department_id = 10;

-- Update with subquery
UPDATE employees
SET department_id = (
    SELECT id FROM departments WHERE name = 'Engineering'
)
WHERE job_title = 'Software Engineer';

-- Update all rows (use with caution!)
UPDATE employees
SET status = 'active';
```

**Warning**: Always use WHERE clause unless you intend to update all rows.

---

### DELETE

Removes rows from a table.

**Syntax**:
```sql
DELETE FROM table_name
WHERE condition;
```

**Examples**:
```sql
-- Delete specific row
DELETE FROM employees WHERE id = 1001;

-- Delete with condition
DELETE FROM orders WHERE order_date < '2024-01-01';

-- Delete with subquery
DELETE FROM order_items
WHERE order_id IN (
    SELECT order_id FROM orders WHERE status = 'cancelled'
);

-- Delete all rows (use TRUNCATE TABLE for better performance)
DELETE FROM session_logs;
```

**Warning**: Always use WHERE clause unless you intend to delete all rows.

---

### UNION

Combines results from multiple SELECT statements.

**Syntax**:
```sql
SELECT column_list FROM table1
UNION [ALL]
SELECT column_list FROM table2;
```

**Examples**:
```sql
-- UNION (removes duplicates)
SELECT name FROM employees WHERE department_id = 10
UNION
SELECT name FROM employees WHERE department_id = 20;

-- UNION ALL (keeps duplicates, faster)
SELECT product_id, product_name FROM current_products
UNION ALL
SELECT product_id, product_name FROM archived_products;

-- UNION with ORDER BY
SELECT customer_id, 'Premium' as tier FROM premium_customers
UNION
SELECT customer_id, 'Standard' as tier FROM standard_customers
ORDER BY customer_id;
```

**Notes**:
- Column count must be identical
- Column types must be compatible
- `UNION` removes duplicates (slower)
- `UNION ALL` keeps duplicates (faster)

---

## Transaction Control

RustyDB provides ACID-compliant transactions with advanced MVCC (Multi-Version Concurrency Control).

### BEGIN TRANSACTION

Starts a new database transaction.

**Syntax**:
```sql
BEGIN TRANSACTION;
BEGIN;  -- Short form
START TRANSACTION;  -- PostgreSQL style
```

**With Isolation Level**:
```sql
BEGIN TRANSACTION ISOLATION LEVEL isolation_level;
```

**Isolation Levels**:
- `READ UNCOMMITTED` - Allows dirty reads (lowest isolation)
- `READ COMMITTED` - Default level, prevents dirty reads
- `REPEATABLE READ` - Prevents non-repeatable reads
- `SERIALIZABLE` - Highest isolation, prevents phantom reads
- `SNAPSHOT ISOLATION` - MVCC-based snapshot isolation

**Examples**:
```sql
-- Simple transaction
BEGIN TRANSACTION;
UPDATE accounts SET balance = balance - 100 WHERE id = 1;
UPDATE accounts SET balance = balance + 100 WHERE id = 2;
COMMIT;

-- Transaction with serializable isolation
BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE;
SELECT * FROM inventory WHERE product_id = 100;
UPDATE inventory SET quantity = quantity - 1 WHERE product_id = 100;
COMMIT;
```

---

### COMMIT

Saves all changes made in the current transaction.

**Syntax**:
```sql
COMMIT;
COMMIT TRANSACTION;  -- Explicit form
```

**Examples**:
```sql
BEGIN TRANSACTION;
INSERT INTO audit_log (action, timestamp) VALUES ('User login', CURRENT_TIMESTAMP);
UPDATE users SET last_login = CURRENT_TIMESTAMP WHERE id = 123;
COMMIT;
```

---

### ROLLBACK

Undoes all changes made in the current transaction.

**Syntax**:
```sql
ROLLBACK;
ROLLBACK TRANSACTION;  -- Explicit form
ROLLBACK TO SAVEPOINT savepoint_name;  -- Partial rollback
```

**Examples**:
```sql
-- Full rollback
BEGIN TRANSACTION;
DELETE FROM orders WHERE order_id = 5000;
-- Oops, wrong order!
ROLLBACK;

-- Rollback to savepoint
BEGIN TRANSACTION;
UPDATE accounts SET balance = balance - 100 WHERE id = 1;
SAVEPOINT after_debit;
UPDATE accounts SET balance = balance + 100 WHERE id = 2;
-- Error occurred
ROLLBACK TO SAVEPOINT after_debit;
COMMIT;
```

---

### SAVEPOINT

Creates a point within a transaction to which you can rollback.

**Syntax**:
```sql
SAVEPOINT savepoint_name;
```

**Examples**:
```sql
BEGIN TRANSACTION;

INSERT INTO customers (name) VALUES ('Customer A');
SAVEPOINT after_customer;

INSERT INTO orders (customer_id, total) VALUES (LAST_INSERT_ID(), 100);
SAVEPOINT after_order;

-- Error occurred, rollback order only
ROLLBACK TO SAVEPOINT after_order;

-- Keep customer insert
COMMIT;
```

---

### Transaction Isolation Levels

**Isolation Level Matrix**:

| Isolation Level | Dirty Read | Non-Repeatable Read | Phantom Read | Performance |
|----------------|------------|---------------------|--------------|-------------|
| READ UNCOMMITTED | Yes | Yes | Yes | Fastest |
| READ COMMITTED | No | Yes | Yes | Fast |
| REPEATABLE READ | No | No | Yes | Moderate |
| SERIALIZABLE | No | No | No | Slowest |
| SNAPSHOT ISOLATION | No | No | No | Good |

**Best Practices**:
- Use `READ COMMITTED` (default) for most applications
- Use `SERIALIZABLE` for financial transactions
- Use `SNAPSHOT ISOLATION` for long-running reports
- Keep transactions short and focused
- Always handle deadlocks and conflicts

---

## Query Features

### JOINs

Combines rows from multiple tables based on relationships.

#### INNER JOIN

Returns only matching rows from both tables.

**Syntax**:
```sql
SELECT columns
FROM table1
INNER JOIN table2 ON table1.column = table2.column;
```

**Example**:
```sql
-- Get orders with customer information
SELECT o.order_id, c.customer_name, o.order_date, o.total_amount
FROM orders o
INNER JOIN customers c ON o.customer_id = c.customer_id;
```

---

#### LEFT JOIN (LEFT OUTER JOIN)

Returns all rows from left table, with matching rows from right table (NULL if no match).

**Syntax**:
```sql
SELECT columns
FROM table1
LEFT JOIN table2 ON table1.column = table2.column;
```

**Example**:
```sql
-- Get all customers and their orders (including customers with no orders)
SELECT c.customer_name, o.order_id, o.order_date
FROM customers c
LEFT JOIN orders o ON c.customer_id = o.customer_id;
```

---

#### RIGHT JOIN (RIGHT OUTER JOIN)

Returns all rows from right table, with matching rows from left table (NULL if no match).

**Syntax**:
```sql
SELECT columns
FROM table1
RIGHT JOIN table2 ON table1.column = table2.column;
```

**Example**:
```sql
-- Get all orders and customer info (including orders without customer match)
SELECT c.customer_name, o.order_id, o.order_date
FROM customers c
RIGHT JOIN orders o ON c.customer_id = o.customer_id;
```

---

#### FULL OUTER JOIN

Returns all rows from both tables (NULL where no match).

**Syntax**:
```sql
SELECT columns
FROM table1
FULL OUTER JOIN table2 ON table1.column = table2.column;
```

**Example**:
```sql
-- Get all customers and orders, showing unmatched records from both
SELECT c.customer_name, o.order_id, o.order_date
FROM customers c
FULL OUTER JOIN orders o ON c.customer_id = o.customer_id;
```

---

#### CROSS JOIN

Returns Cartesian product of both tables.

**Syntax**:
```sql
SELECT columns
FROM table1
CROSS JOIN table2;
```

**Example**:
```sql
-- Generate all combinations of products and stores
SELECT p.product_name, s.store_name
FROM products p
CROSS JOIN stores s;
```

---

### Subqueries

A query nested within another query.

**Types of Subqueries**:
- **Scalar**: Returns single value
- **Row**: Returns single row
- **Table**: Returns multiple rows/columns
- **Correlated**: References outer query

**Examples**:
```sql
-- Scalar subquery
SELECT name, salary
FROM employees
WHERE salary > (SELECT AVG(salary) FROM employees);

-- IN subquery
SELECT name
FROM employees
WHERE department_id IN (
    SELECT id FROM departments WHERE location = 'New York'
);

-- EXISTS subquery
SELECT c.customer_name
FROM customers c
WHERE EXISTS (
    SELECT 1 FROM orders o WHERE o.customer_id = c.customer_id
);

-- Correlated subquery
SELECT e1.name, e1.salary
FROM employees e1
WHERE e1.salary > (
    SELECT AVG(e2.salary)
    FROM employees e2
    WHERE e2.department_id = e1.department_id
);
```

---

### Common Table Expressions (CTEs)

Named temporary result sets that exist within a single query.

**Syntax**:
```sql
WITH cte_name AS (
    SELECT ...
)
SELECT ... FROM cte_name;
```

**Examples**:
```sql
-- Simple CTE
WITH high_earners AS (
    SELECT * FROM employees WHERE salary > 100000
)
SELECT department_id, COUNT(*) as high_earner_count
FROM high_earners
GROUP BY department_id;

-- Multiple CTEs
WITH
    sales_2024 AS (
        SELECT * FROM orders WHERE YEAR(order_date) = 2024
    ),
    top_customers AS (
        SELECT customer_id, SUM(total_amount) as total
        FROM sales_2024
        GROUP BY customer_id
        HAVING SUM(total_amount) > 10000
    )
SELECT c.customer_name, tc.total
FROM top_customers tc
JOIN customers c ON tc.customer_id = c.customer_id;

-- Recursive CTE (employee hierarchy)
WITH RECURSIVE employee_hierarchy AS (
    -- Base case: top-level employees
    SELECT id, name, manager_id, 0 as level
    FROM employees
    WHERE manager_id IS NULL

    UNION ALL

    -- Recursive case: employees reporting to previous level
    SELECT e.id, e.name, e.manager_id, eh.level + 1
    FROM employees e
    JOIN employee_hierarchy eh ON e.manager_id = eh.id
)
SELECT * FROM employee_hierarchy;
```

---

### Window Functions

Perform calculations across rows related to the current row.

**Syntax**:
```sql
function_name([arguments]) OVER (
    [PARTITION BY column_list]
    [ORDER BY column_list]
    [ROWS|RANGE frame_clause]
)
```

**Ranking Functions**:
```sql
-- ROW_NUMBER: Unique sequential number
SELECT
    name,
    salary,
    ROW_NUMBER() OVER (ORDER BY salary DESC) as rank
FROM employees;

-- RANK: Rank with gaps for ties
SELECT
    name,
    salary,
    RANK() OVER (ORDER BY salary DESC) as rank
FROM employees;

-- DENSE_RANK: Rank without gaps
SELECT
    name,
    salary,
    DENSE_RANK() OVER (ORDER BY salary DESC) as rank
FROM employees;

-- NTILE: Divide into N buckets
SELECT
    name,
    salary,
    NTILE(4) OVER (ORDER BY salary) as quartile
FROM employees;
```

**Aggregate Window Functions**:
```sql
-- Running total
SELECT
    order_date,
    amount,
    SUM(amount) OVER (ORDER BY order_date) as running_total
FROM orders;

-- Moving average
SELECT
    order_date,
    amount,
    AVG(amount) OVER (
        ORDER BY order_date
        ROWS BETWEEN 6 PRECEDING AND CURRENT ROW
    ) as moving_avg_7day
FROM daily_sales;

-- Partition by department
SELECT
    department_id,
    name,
    salary,
    AVG(salary) OVER (PARTITION BY department_id) as dept_avg_salary
FROM employees;
```

**Analytic Functions**:
```sql
-- LAG: Previous row value
SELECT
    order_date,
    amount,
    LAG(amount, 1) OVER (ORDER BY order_date) as prev_day_amount
FROM daily_sales;

-- LEAD: Next row value
SELECT
    order_date,
    amount,
    LEAD(amount, 1) OVER (ORDER BY order_date) as next_day_amount
FROM daily_sales;

-- FIRST_VALUE and LAST_VALUE
SELECT
    department_id,
    name,
    salary,
    FIRST_VALUE(name) OVER (
        PARTITION BY department_id
        ORDER BY salary DESC
    ) as highest_paid_in_dept
FROM employees;
```

---

### Aggregate Functions

Perform calculations on sets of rows.

**Common Aggregate Functions**:
```sql
-- COUNT: Count rows
SELECT COUNT(*) FROM employees;
SELECT COUNT(DISTINCT department_id) FROM employees;

-- SUM: Total of values
SELECT SUM(salary) FROM employees;
SELECT department_id, SUM(salary) as total_salary
FROM employees
GROUP BY department_id;

-- AVG: Average value
SELECT AVG(salary) FROM employees;

-- MIN: Minimum value
SELECT MIN(salary) FROM employees;

-- MAX: Maximum value
SELECT MAX(salary) FROM employees;

-- Statistical functions
SELECT
    department_id,
    COUNT(*) as emp_count,
    AVG(salary) as avg_salary,
    MIN(salary) as min_salary,
    MAX(salary) as max_salary,
    STDDEV(salary) as salary_stddev,
    VARIANCE(salary) as salary_variance
FROM employees
GROUP BY department_id;
```

---

## Built-in Functions

### String Functions

Comprehensive SQL Server-compatible string functions.

#### Character Functions

```sql
-- ASCII: Get ASCII value
SELECT ASCII('A');  -- Returns 65

-- CHAR: Get character from ASCII
SELECT CHAR(65);  -- Returns 'A'

-- UNICODE: Get Unicode value
SELECT UNICODE('©');  -- Returns 169

-- NCHAR: Get Unicode character
SELECT NCHAR(169);  -- Returns '©'
```

#### String Manipulation

```sql
-- UPPER: Convert to uppercase
SELECT UPPER('hello world');  -- 'HELLO WORLD'

-- LOWER: Convert to lowercase
SELECT LOWER('HELLO WORLD');  -- 'hello world'

-- LEFT: Extract from left
SELECT LEFT('Hello World', 5);  -- 'Hello'

-- RIGHT: Extract from right
SELECT RIGHT('Hello World', 5);  -- 'World'

-- SUBSTRING: Extract substring
SELECT SUBSTRING('Hello World', 7, 5);  -- 'World'

-- REVERSE: Reverse string
SELECT REVERSE('Hello');  -- 'olleH'

-- REPLACE: Replace substring
SELECT REPLACE('Hello World', 'World', 'Rust');  -- 'Hello Rust'

-- STUFF: Delete and insert
SELECT STUFF('Hello World', 7, 5, 'Rust');  -- 'Hello Rust'

-- TRANSLATE: Translate characters
SELECT TRANSLATE('2*[3+4]/{7-2}', '[]{}', '()()');  -- '2*(3+4)/(7-2)'
```

#### String Operations

```sql
-- CONCAT: Concatenate strings
SELECT CONCAT('Hello', ' ', 'World');  -- 'Hello World'

-- CONCAT_WS: Concatenate with separator
SELECT CONCAT_WS('-', '2025', '12', '27');  -- '2025-12-27'

-- REPLICATE: Repeat string
SELECT REPLICATE('*', 5);  -- '*****'

-- SPACE: Generate spaces
SELECT SPACE(5);  -- '     '

-- QUOTENAME: Add delimiters
SELECT QUOTENAME('My Table');  -- '[My Table]'
```

#### String Analysis

```sql
-- LEN: Length (excludes trailing spaces)
SELECT LEN('Hello  ');  -- 5

-- DATALENGTH: Byte length
SELECT DATALENGTH('Hello');  -- 5

-- CHARINDEX: Find substring position
SELECT CHARINDEX('World', 'Hello World');  -- 7

-- PATINDEX: Find pattern position
SELECT PATINDEX('%[0-9]%', 'abc123def');  -- 4
```

#### String Trimming

```sql
-- LTRIM: Trim left
SELECT LTRIM('  Hello');  -- 'Hello'

-- RTRIM: Trim right
SELECT RTRIM('Hello  ');  -- 'Hello'

-- TRIM: Trim both sides
SELECT TRIM('  Hello  ');  -- 'Hello'
```

#### Phonetic Functions

```sql
-- SOUNDEX: Phonetic encoding
SELECT SOUNDEX('Robert');  -- 'R163'

-- DIFFERENCE: Compare SOUNDEX values (0-4)
SELECT DIFFERENCE('Robert', 'Rupert');  -- 4 (exact match)
SELECT DIFFERENCE('Smith', 'Jones');   -- 0 (no match)
```

#### String Formatting

```sql
-- FORMAT: Format value
SELECT FORMAT(1234.56, 'C');  -- '$1234.56'

-- STR: Number to string
SELECT STR(1234.5, 10, 2);  -- '   1234.50'
```

---

### Numeric Functions

```sql
-- ABS: Absolute value
SELECT ABS(-42);  -- 42

-- CEILING: Round up
SELECT CEILING(4.3);  -- 5

-- FLOOR: Round down
SELECT FLOOR(4.7);  -- 4

-- ROUND: Round to precision
SELECT ROUND(123.456, 2);  -- 123.46

-- TRUNCATE: Truncate to precision
SELECT TRUNCATE(123.456, 2);  -- 123.45

-- POWER: Exponentiation
SELECT POWER(2, 10);  -- 1024

-- SQRT: Square root
SELECT SQRT(16);  -- 4

-- MOD: Modulo
SELECT MOD(17, 5);  -- 2

-- SIGN: Sign of number (-1, 0, 1)
SELECT SIGN(-42);  -- -1

-- Random
SELECT RANDOM();  -- Random number 0-1
```

---

### Date/Time Functions

```sql
-- CURRENT_DATE: Current date
SELECT CURRENT_DATE;  -- '2025-12-27'

-- CURRENT_TIME: Current time
SELECT CURRENT_TIME;  -- '14:30:45'

-- CURRENT_TIMESTAMP: Current date and time
SELECT CURRENT_TIMESTAMP;  -- '2025-12-27 14:30:45.123'

-- NOW: Current timestamp (alias)
SELECT NOW();

-- YEAR: Extract year
SELECT YEAR('2025-12-27');  -- 2025

-- MONTH: Extract month
SELECT MONTH('2025-12-27');  -- 12

-- DAY: Extract day
SELECT DAY('2025-12-27');  -- 27

-- DATE_ADD: Add interval
SELECT DATE_ADD('2025-12-27', INTERVAL 7 DAY);  -- '2026-01-03'

-- DATE_SUB: Subtract interval
SELECT DATE_SUB('2025-12-27', INTERVAL 1 MONTH);  -- '2025-11-27'

-- DATEDIFF: Difference in days
SELECT DATEDIFF('2025-12-27', '2025-12-20');  -- 7

-- DATE_FORMAT: Format date
SELECT DATE_FORMAT('2025-12-27', '%Y-%m-%d');  -- '2025-12-27'

-- EXTRACT: Extract part
SELECT EXTRACT(YEAR FROM TIMESTAMP '2025-12-27 14:30:45');  -- 2025
```

---

### Conversion Functions

```sql
-- CAST: Type conversion
SELECT CAST('123' AS INTEGER);
SELECT CAST(123.45 AS VARCHAR);

-- CONVERT: Type conversion (SQL Server style)
SELECT CONVERT(INTEGER, '123');
SELECT CONVERT(VARCHAR, 123.45);

-- TO_CHAR: Convert to string
SELECT TO_CHAR(12345.67, '999,999.99');

-- TO_NUMBER: Convert to number
SELECT TO_NUMBER('12345.67');

-- TO_DATE: Convert to date
SELECT TO_DATE('2025-12-27', 'YYYY-MM-DD');
```

---

### Conditional Functions

```sql
-- COALESCE: First non-NULL value
SELECT COALESCE(NULL, NULL, 'default', 'other');  -- 'default'

-- NULLIF: Return NULL if equal
SELECT NULLIF(10, 10);  -- NULL
SELECT NULLIF(10, 20);  -- 10

-- IFNULL / NVL: Replace NULL
SELECT IFNULL(NULL, 'default');  -- 'default'
SELECT NVL(column_name, 'N/A') FROM table_name;

-- CASE expression
SELECT
    name,
    CASE
        WHEN salary < 50000 THEN 'Low'
        WHEN salary < 100000 THEN 'Medium'
        ELSE 'High'
    END as salary_band
FROM employees;

-- Simple CASE
SELECT
    name,
    CASE status
        WHEN 'A' THEN 'Active'
        WHEN 'I' THEN 'Inactive'
        WHEN 'P' THEN 'Pending'
        ELSE 'Unknown'
    END as status_desc
FROM employees;
```

---

## Operators

### Comparison Operators

```sql
-- Equality
SELECT * FROM employees WHERE salary = 50000;

-- Inequality
SELECT * FROM employees WHERE salary <> 50000;
SELECT * FROM employees WHERE salary != 50000;

-- Less than
SELECT * FROM employees WHERE salary < 50000;

-- Less than or equal
SELECT * FROM employees WHERE salary <= 50000;

-- Greater than
SELECT * FROM employees WHERE salary > 50000;

-- Greater than or equal
SELECT * FROM employees WHERE salary >= 50000;
```

---

### Logical Operators

```sql
-- AND: Both conditions must be true
SELECT * FROM employees
WHERE department_id = 10 AND salary > 50000;

-- OR: At least one condition must be true
SELECT * FROM employees
WHERE department_id = 10 OR department_id = 20;

-- NOT: Negates condition
SELECT * FROM employees
WHERE NOT department_id = 10;
```

---

### Arithmetic Operators

```sql
-- Addition
SELECT salary + bonus as total_compensation FROM employees;

-- Subtraction
SELECT revenue - cost as profit FROM financials;

-- Multiplication
SELECT quantity * price as total FROM order_items;

-- Division
SELECT total_salary / employee_count as avg_salary FROM departments;

-- Modulo
SELECT order_id % 10 as hash_bucket FROM orders;
```

---

### Special Operators

#### BETWEEN

Tests if value is within range (inclusive).

```sql
-- Numeric range
SELECT * FROM employees
WHERE salary BETWEEN 50000 AND 100000;

-- Date range
SELECT * FROM orders
WHERE order_date BETWEEN '2025-01-01' AND '2025-12-31';

-- NOT BETWEEN
SELECT * FROM products
WHERE price NOT BETWEEN 10 AND 50;
```

---

#### IN

Tests if value matches any in a list.

```sql
-- IN with literal list
SELECT * FROM employees
WHERE department_id IN (10, 20, 30);

-- IN with subquery
SELECT * FROM employees
WHERE department_id IN (
    SELECT id FROM departments WHERE location = 'New York'
);

-- NOT IN
SELECT * FROM products
WHERE category_id NOT IN (1, 2, 3);
```

---

#### LIKE

Pattern matching with wildcards.

**Wildcards**:
- `%` - Matches zero or more characters
- `_` - Matches exactly one character

```sql
-- Starts with
SELECT * FROM employees WHERE name LIKE 'John%';

-- Ends with
SELECT * FROM employees WHERE email LIKE '%@example.com';

-- Contains
SELECT * FROM products WHERE description LIKE '%laptop%';

-- Single character wildcard
SELECT * FROM products WHERE code LIKE 'ABC_123';

-- NOT LIKE
SELECT * FROM employees WHERE name NOT LIKE 'J%';

-- Case-insensitive (use ILIKE in PostgreSQL mode)
SELECT * FROM employees WHERE name ILIKE 'john%';

-- ESCAPE clause for literal wildcards
SELECT * FROM products WHERE code LIKE '50\%_off' ESCAPE '\';
```

---

#### SIMILAR TO

Advanced pattern matching with regular expressions (PostgreSQL style).

```sql
-- Regular expression pattern
SELECT * FROM products
WHERE code SIMILAR TO 'ABC[0-9]{3}';

-- Alternative patterns
SELECT * FROM emails
WHERE address SIMILAR TO '%@(gmail|yahoo|hotmail).com';
```

---

#### IS NULL / IS NOT NULL

Tests for NULL values.

```sql
-- IS NULL
SELECT * FROM employees WHERE manager_id IS NULL;

-- IS NOT NULL
SELECT * FROM employees WHERE email IS NOT NULL;
```

**Note**: Cannot use `= NULL` or `<> NULL` - must use `IS NULL` / `IS NOT NULL`.

---

#### EXISTS

Tests if subquery returns any rows.

```sql
-- EXISTS
SELECT * FROM customers c
WHERE EXISTS (
    SELECT 1 FROM orders o WHERE o.customer_id = c.customer_id
);

-- NOT EXISTS
SELECT * FROM customers c
WHERE NOT EXISTS (
    SELECT 1 FROM orders o WHERE o.customer_id = c.customer_id
);
```

---

## Data Types

### Numeric Types

```sql
-- Integer types
INTEGER       -- 32-bit integer (-2,147,483,648 to 2,147,483,647)
BIGINT        -- 64-bit integer
SMALLINT      -- 16-bit integer
TINYINT       -- 8-bit integer

-- Decimal/Numeric
DECIMAL(p, s)   -- Exact numeric with precision and scale
NUMERIC(p, s)   -- Same as DECIMAL
NUMBER(p, s)    -- Oracle-style numeric

-- Floating point
REAL            -- 32-bit floating point
FLOAT           -- 64-bit floating point
DOUBLE          -- Double precision floating point
DOUBLE PRECISION -- Same as DOUBLE

-- Examples
CREATE TABLE numbers (
    id INTEGER,
    count BIGINT,
    price DECIMAL(10, 2),
    weight FLOAT,
    ratio DOUBLE
);
```

---

### Character Types

```sql
-- Variable-length character strings
VARCHAR(n)      -- Variable-length, max n characters
VARCHAR2(n)     -- Oracle-style VARCHAR
NVARCHAR(n)     -- Unicode variable-length

-- Fixed-length character strings
CHAR(n)         -- Fixed-length, padded with spaces
NCHAR(n)        -- Unicode fixed-length

-- Large text
TEXT            -- Unlimited length text
CLOB            -- Character Large Object
NCLOB           -- Unicode CLOB

-- Examples
CREATE TABLE strings (
    code CHAR(10),
    name VARCHAR(100),
    description TEXT,
    notes CLOB
);
```

---

### Date/Time Types

```sql
-- Date types
DATE            -- Date only (YYYY-MM-DD)
TIME            -- Time only (HH:MM:SS)
TIMESTAMP       -- Date and time with microseconds
TIMESTAMP WITH TIME ZONE  -- Timestamp with timezone

-- Interval
INTERVAL        -- Time interval

-- Examples
CREATE TABLE events (
    event_date DATE,
    event_time TIME,
    created_at TIMESTAMP,
    scheduled_at TIMESTAMP WITH TIME ZONE
);
```

---

### Boolean Type

```sql
BOOLEAN         -- TRUE, FALSE, or NULL

-- Example
CREATE TABLE settings (
    setting_name VARCHAR(50),
    is_enabled BOOLEAN DEFAULT TRUE
);

-- Usage
INSERT INTO settings VALUES ('feature_x', TRUE);
SELECT * FROM settings WHERE is_enabled = TRUE;
```

---

### Binary Types

```sql
-- Binary data
BINARY(n)       -- Fixed-length binary
VARBINARY(n)    -- Variable-length binary
BLOB            -- Binary Large Object
BYTEA           -- PostgreSQL-style binary

-- Examples
CREATE TABLE files (
    file_id INTEGER,
    file_name VARCHAR(255),
    file_data BLOB,
    checksum BINARY(32)
);
```

---

### JSON Type

```sql
JSON            -- JSON document
JSONB           -- Binary JSON (PostgreSQL-style, more efficient)

-- Examples
CREATE TABLE documents (
    doc_id INTEGER,
    metadata JSON,
    content JSONB
);

-- JSON operations
INSERT INTO documents VALUES (
    1,
    '{"author": "John Doe", "version": 1}',
    '{"title": "Document 1", "tags": ["important", "draft"]}'
);

-- Query JSON
SELECT metadata->>'author' FROM documents;
SELECT * FROM documents WHERE content @> '{"tags": ["important"]}';
```

---

### Array Types

```sql
-- Array types (PostgreSQL style)
INTEGER[]       -- Array of integers
VARCHAR[]       -- Array of strings
TEXT[]          -- Array of text

-- Examples
CREATE TABLE tags_table (
    item_id INTEGER,
    tags TEXT[]
);

INSERT INTO tags_table VALUES (1, ARRAY['tag1', 'tag2', 'tag3']);
SELECT * FROM tags_table WHERE 'tag1' = ANY(tags);
```

---

### UUID Type

```sql
UUID            -- Universally Unique Identifier

-- Example
CREATE TABLE users (
    user_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(50)
);
```

---

### Special Types

```sql
-- Network types (PostgreSQL)
INET            -- IP address
CIDR            -- Network address
MACADDR         -- MAC address

-- Geometric types (PostgreSQL)
POINT           -- Point (x,y)
LINE            -- Infinite line
LSEG            -- Line segment
BOX             -- Rectangle
PATH            -- Geometric path
POLYGON         -- Polygon
CIRCLE          -- Circle
```

---

## Constraints

Constraints enforce data integrity rules.

### PRIMARY KEY

Uniquely identifies each row in a table.

**Characteristics**:
- Must be unique
- Cannot be NULL
- Only one per table
- Automatically creates index

```sql
-- Single column
CREATE TABLE employees (
    id INTEGER PRIMARY KEY,
    name VARCHAR(100)
);

-- Composite primary key
CREATE TABLE order_items (
    order_id INTEGER,
    item_id INTEGER,
    quantity INTEGER,
    PRIMARY KEY (order_id, item_id)
);

-- Named constraint
CREATE TABLE products (
    product_id INTEGER CONSTRAINT pk_products PRIMARY KEY
);

-- Add via ALTER TABLE
ALTER TABLE employees ADD PRIMARY KEY (id);
ALTER TABLE employees ADD CONSTRAINT pk_employees PRIMARY KEY (id);
```

---

### FOREIGN KEY

Enforces referential integrity between tables.

**Referential Actions**:
- `CASCADE` - Delete/update cascades to child rows
- `SET NULL` - Set child foreign key to NULL
- `SET DEFAULT` - Set child foreign key to default value
- `RESTRICT` - Prevent delete/update if child rows exist
- `NO ACTION` - Same as RESTRICT

```sql
-- Basic foreign key
CREATE TABLE orders (
    order_id INTEGER PRIMARY KEY,
    customer_id INTEGER,
    FOREIGN KEY (customer_id) REFERENCES customers(customer_id)
);

-- With referential actions
CREATE TABLE order_items (
    item_id INTEGER PRIMARY KEY,
    order_id INTEGER,
    FOREIGN KEY (order_id) REFERENCES orders(order_id)
        ON DELETE CASCADE
        ON UPDATE CASCADE
);

-- Named constraint
CREATE TABLE employees (
    emp_id INTEGER PRIMARY KEY,
    manager_id INTEGER,
    CONSTRAINT fk_manager FOREIGN KEY (manager_id)
        REFERENCES employees(emp_id)
);

-- Add via ALTER TABLE
ALTER TABLE orders
ADD CONSTRAINT fk_customer
FOREIGN KEY (customer_id) REFERENCES customers(customer_id);

-- Composite foreign key
CREATE TABLE order_items (
    order_id INTEGER,
    item_seq INTEGER,
    product_id INTEGER,
    FOREIGN KEY (order_id, item_seq)
        REFERENCES order_details(order_id, seq_num)
);
```

---

### UNIQUE

Ensures all values in column(s) are unique.

**Characteristics**:
- Allows NULL values (multiple NULLs permitted)
- Automatically creates index
- Can have multiple UNIQUE constraints per table

```sql
-- Single column
CREATE TABLE employees (
    id INTEGER PRIMARY KEY,
    email VARCHAR(255) UNIQUE
);

-- Multiple columns
CREATE TABLE products (
    product_id INTEGER PRIMARY KEY,
    sku VARCHAR(50) UNIQUE,
    upc VARCHAR(20) UNIQUE
);

-- Composite unique
CREATE TABLE allocations (
    user_id INTEGER,
    resource_id INTEGER,
    UNIQUE (user_id, resource_id)
);

-- Named constraint
CREATE TABLE users (
    user_id INTEGER,
    username VARCHAR(50) CONSTRAINT uq_username UNIQUE
);

-- Add via ALTER TABLE
ALTER TABLE employees ADD UNIQUE (email);
ALTER TABLE employees ADD CONSTRAINT uq_email UNIQUE (email);
```

---

### CHECK

Validates that column values satisfy a condition.

```sql
-- Simple CHECK
CREATE TABLE employees (
    id INTEGER PRIMARY KEY,
    age INTEGER CHECK (age >= 18),
    salary DECIMAL(10,2) CHECK (salary > 0)
);

-- Named constraint
CREATE TABLE products (
    product_id INTEGER PRIMARY KEY,
    price DECIMAL(10,2),
    discount_pct DECIMAL(5,2),
    CONSTRAINT chk_price CHECK (price > 0),
    CONSTRAINT chk_discount CHECK (discount_pct BETWEEN 0 AND 100)
);

-- Multi-column CHECK
CREATE TABLE date_ranges (
    range_id INTEGER PRIMARY KEY,
    start_date DATE,
    end_date DATE,
    CHECK (end_date > start_date)
);

-- Complex CHECK
CREATE TABLE orders (
    order_id INTEGER PRIMARY KEY,
    status VARCHAR(20),
    shipped_date DATE,
    CHECK (
        (status = 'shipped' AND shipped_date IS NOT NULL) OR
        (status != 'shipped' AND shipped_date IS NULL)
    )
);

-- Add via ALTER TABLE
ALTER TABLE employees ADD CHECK (salary > 0);
ALTER TABLE employees ADD CONSTRAINT chk_age CHECK (age >= 18);
```

---

### NOT NULL

Ensures column cannot contain NULL values.

```sql
-- Column definition
CREATE TABLE employees (
    id INTEGER NOT NULL,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) NOT NULL,
    phone VARCHAR(20)  -- NULL allowed
);

-- Modify via ALTER TABLE
ALTER TABLE employees MODIFY COLUMN email VARCHAR(255) NOT NULL;
```

---

### DEFAULT

Specifies default value when no value is provided.

```sql
-- Simple defaults
CREATE TABLE employees (
    id INTEGER PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_admin BOOLEAN DEFAULT FALSE
);

-- Named constraint
CREATE TABLE orders (
    order_id INTEGER PRIMARY KEY,
    order_date DATE CONSTRAINT df_order_date DEFAULT CURRENT_DATE
);

-- Expression defaults
CREATE TABLE audit_log (
    log_id INTEGER PRIMARY KEY,
    log_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    user_id INTEGER DEFAULT CURRENT_USER_ID()
);

-- Drop default
ALTER TABLE employees DROP DEFAULT status;
```

---

### Constraint Management

```sql
-- Add constraint
ALTER TABLE employees ADD CONSTRAINT chk_salary CHECK (salary > 0);

-- Drop constraint
ALTER TABLE employees DROP CONSTRAINT chk_salary;

-- Disable constraint (if supported)
ALTER TABLE employees DISABLE CONSTRAINT chk_salary;

-- Enable constraint
ALTER TABLE employees ENABLE CONSTRAINT chk_salary;

-- View constraints
SELECT constraint_name, constraint_type
FROM information_schema.table_constraints
WHERE table_name = 'employees';
```

---

## SQL Compliance

### SQL:2016 Core Features

RustyDB implements the following SQL:2016 core features:

**Feature E011**: Numeric data types
- ✅ INTEGER, SMALLINT, BIGINT
- ✅ DECIMAL, NUMERIC
- ✅ REAL, FLOAT, DOUBLE PRECISION

**Feature E021**: Character string types
- ✅ CHARACTER, CHAR
- ✅ CHARACTER VARYING, VARCHAR
- ✅ TEXT, CLOB

**Feature E031**: Identifiers
- ✅ Delimited identifiers
- ✅ Case-insensitive matching
- ✅ 128-character identifier length

**Feature E051**: Basic query specification
- ✅ SELECT with FROM, WHERE
- ✅ DISTINCT
- ✅ Column aliases
- ✅ Table aliases

**Feature E061**: Basic predicates
- ✅ Comparison predicates (=, <>, <, <=, >, >=)
- ✅ BETWEEN
- ✅ IN with value list
- ✅ LIKE
- ✅ NULL predicate (IS NULL, IS NOT NULL)

**Feature E071**: Basic query expressions
- ✅ UNION, UNION ALL
- ✅ INTERSECT (planned)
- ✅ EXCEPT (planned)

**Feature E081**: Basic Privileges
- ✅ SELECT privilege
- ✅ INSERT, UPDATE, DELETE privileges
- ✅ REFERENCES privilege

**Feature E091**: Set functions
- ✅ AVG, COUNT, MAX, MIN, SUM
- ✅ DISTINCT in set functions

**Feature E101**: Basic data manipulation
- ✅ INSERT
- ✅ UPDATE
- ✅ DELETE
- ✅ UPDATE with searched condition

**Feature E111**: Single row SELECT
- ✅ Subqueries in SELECT
- ✅ Scalar subqueries

**Feature E121**: Basic cursor support
- 🟡 Planned for future release

**Feature E131**: Null value support
- ✅ NULL literal
- ✅ IS NULL, IS NOT NULL

**Feature E141**: Basic integrity constraints
- ✅ NOT NULL
- ✅ UNIQUE
- ✅ PRIMARY KEY
- ✅ FOREIGN KEY with CASCADE
- ✅ CHECK

**Feature E151**: Transaction support
- ✅ BEGIN, COMMIT, ROLLBACK
- ✅ SAVEPOINT
- ✅ Isolation levels

**Feature E152**: Basic SET TRANSACTION
- ✅ READ COMMITTED
- ✅ REPEATABLE READ
- ✅ SERIALIZABLE

**Feature E153**: Updatable queries with subqueries
- ✅ UPDATE with subquery
- ✅ DELETE with subquery

**Feature F031**: Basic schema manipulation
- ✅ CREATE TABLE
- ✅ DROP TABLE
- ✅ ALTER TABLE (ADD/DROP COLUMN)

**Feature F041**: Basic joined table
- ✅ INNER JOIN
- ✅ LEFT OUTER JOIN
- ✅ RIGHT OUTER JOIN
- ✅ FULL OUTER JOIN

**Feature F051**: Basic date and time
- ✅ DATE, TIME, TIMESTAMP
- ✅ CURRENT_DATE, CURRENT_TIME, CURRENT_TIMESTAMP

**Feature F081**: UNION and EXCEPT in views
- ✅ CREATE VIEW with UNION

**Feature F111**: Isolation levels other than SERIALIZABLE
- ✅ READ UNCOMMITTED
- ✅ READ COMMITTED

**Feature F131**: Grouped operations
- ✅ GROUP BY
- ✅ HAVING
- ✅ Aggregate functions in HAVING

**Feature F181**: Multiple module support
- ✅ Multiple concurrent transactions

**Feature F201**: CAST function
- ✅ CAST between numeric types
- ✅ CAST between string types
- ✅ CAST between date/time types

**Feature F221**: Explicit defaults
- ✅ DEFAULT clause

**Feature F261**: CASE expression
- ✅ Simple CASE
- ✅ Searched CASE
- ✅ COALESCE, NULLIF

**Feature F311**: Schema definition statement
- ✅ CREATE SCHEMA
- ✅ DROP SCHEMA

**Feature F471**: Scalar subquery values
- ✅ Subqueries in SELECT list
- ✅ Subqueries in WHERE clause

**Feature F491**: Constraint management
- ✅ ALTER TABLE ADD/DROP CONSTRAINT

**Feature F531**: Temporary tables
- 🟡 Planned for future release

**Feature F812**: Basic flagging
- ✅ SQL injection prevention
- ✅ Input validation

**Feature T321**: Basic SQL-invoked routines
- ✅ CREATE PROCEDURE
- ✅ CALL/EXEC procedure

---

### Oracle Compatibility

RustyDB provides extensive Oracle Database compatibility:

**SQL Syntax**:
- ✅ PL/SQL-style procedures
- ✅ Oracle outer join syntax `(+)`
- ✅ DUAL table
- ✅ SYSDATE, SYSTIMESTAMP
- ✅ NVL, NVL2, DECODE
- ✅ ROWNUM (via LIMIT)

**Data Types**:
- ✅ NUMBER(p,s)
- ✅ VARCHAR2
- ✅ NVARCHAR2
- ✅ CLOB, NCLOB, BLOB
- ✅ DATE, TIMESTAMP

**Functions**:
- ✅ String functions (SUBSTR, INSTR, LENGTH, etc.)
- ✅ Date functions (TRUNC, ADD_MONTHS, etc.)
- ✅ Conversion functions (TO_CHAR, TO_DATE, TO_NUMBER)
- ✅ Analytic functions (ROW_NUMBER, RANK, DENSE_RANK)

**Enterprise Features**:
- ✅ Flashback Query
- ✅ Virtual Private Database (VPD)
- ✅ Transparent Data Encryption (TDE)
- ✅ Data Masking
- ✅ Real Application Clusters (RAC)
- ✅ Advanced Replication

---

### PostgreSQL Compatibility

**SQL Syntax**:
- ✅ PostgreSQL wire protocol
- ✅ Recursive CTEs
- ✅ RETURNING clause
- ✅ ILIKE operator
- ✅ SIMILAR TO pattern matching

**Data Types**:
- ✅ SERIAL, BIGSERIAL
- ✅ BYTEA
- ✅ JSONB
- ✅ Array types
- ✅ UUID
- ✅ INET, CIDR, MACADDR
- ✅ Geometric types

**Functions**:
- ✅ String functions (POSITION, SUBSTRING)
- ✅ Array functions
- ✅ JSON functions (jsonb_*)
- ✅ Statistical aggregates

**Extensions**:
- ✅ Full-text search
- ✅ PostGIS-compatible spatial operations
- ✅ Logical replication

---

### Feature Comparison Matrix

| Feature | SQL:2016 | Oracle | PostgreSQL | RustyDB |
|---------|----------|--------|------------|---------|
| **Core SQL** |
| SELECT/FROM/WHERE | ✅ | ✅ | ✅ | ✅ |
| JOINs (INNER/LEFT/RIGHT/FULL) | ✅ | ✅ | ✅ | ✅ |
| Subqueries | ✅ | ✅ | ✅ | ✅ |
| CTEs | ✅ | ✅ | ✅ | ✅ |
| Window Functions | ✅ | ✅ | ✅ | ✅ |
| Recursive CTEs | ✅ | ✅ | ✅ | ✅ |
| **Data Types** |
| Numeric | ✅ | ✅ | ✅ | ✅ |
| Character | ✅ | ✅ | ✅ | ✅ |
| Date/Time | ✅ | ✅ | ✅ | ✅ |
| JSON | ✅ | ✅ | ✅ | ✅ |
| Arrays | 🟡 | 🟡 | ✅ | ✅ |
| UUID | ❌ | ❌ | ✅ | ✅ |
| **Constraints** |
| PRIMARY KEY | ✅ | ✅ | ✅ | ✅ |
| FOREIGN KEY | ✅ | ✅ | ✅ | ✅ |
| UNIQUE | ✅ | ✅ | ✅ | ✅ |
| CHECK | ✅ | ✅ | ✅ | ✅ |
| NOT NULL | ✅ | ✅ | ✅ | ✅ |
| **Transactions** |
| ACID Compliance | ✅ | ✅ | ✅ | ✅ |
| MVCC | ❌ | ✅ | ✅ | ✅ |
| Isolation Levels | ✅ | ✅ | ✅ | ✅ |
| Savepoints | ✅ | ✅ | ✅ | ✅ |
| **Advanced** |
| Stored Procedures | ✅ | ✅ | ✅ | ✅ |
| Triggers | ✅ | ✅ | ✅ | ✅ |
| Full-Text Search | 🟡 | ✅ | ✅ | ✅ |
| Spatial Data | ❌ | ✅ | ✅ | ✅ |
| Replication | ❌ | ✅ | ✅ | ✅ |
| Partitioning | 🟡 | ✅ | ✅ | ✅ |

**Legend**: ✅ Fully Supported | 🟡 Partially Supported | ❌ Not in Standard

---

## Best Practices

### Query Optimization

1. **Use Indexes Wisely**
```sql
-- Create indexes on columns used in WHERE, JOIN, ORDER BY
CREATE INDEX idx_customer_email ON customers(email);
CREATE INDEX idx_order_date ON orders(order_date);
```

2. **Avoid SELECT ***
```sql
-- Bad
SELECT * FROM large_table;

-- Good
SELECT id, name, email FROM large_table;
```

3. **Use LIMIT for Large Result Sets**
```sql
-- Pagination
SELECT * FROM products
ORDER BY product_id
LIMIT 20 OFFSET 0;
```

4. **Optimize JOINs**
```sql
-- Use INNER JOIN instead of WHERE when possible
-- Good
SELECT o.*, c.name
FROM orders o
INNER JOIN customers c ON o.customer_id = c.customer_id;

-- Avoid Cartesian products
```

5. **Use EXISTS Instead of IN for Large Subqueries**
```sql
-- Better performance
SELECT * FROM customers c
WHERE EXISTS (SELECT 1 FROM orders o WHERE o.customer_id = c.customer_id);

-- vs
SELECT * FROM customers
WHERE customer_id IN (SELECT customer_id FROM orders);
```

---

### Transaction Best Practices

1. **Keep Transactions Short**
```sql
-- Bad: Long transaction
BEGIN;
SELECT * FROM inventory WHERE product_id = 100;
-- ... user thinks for 5 minutes ...
UPDATE inventory SET quantity = quantity - 1 WHERE product_id = 100;
COMMIT;

-- Good: Short transaction
BEGIN;
UPDATE inventory SET quantity = quantity - 1 WHERE product_id = 100;
COMMIT;
```

2. **Use Appropriate Isolation Level**
```sql
-- Financial transactions
BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE;
-- ... critical operations ...
COMMIT;

-- Read-heavy operations
BEGIN TRANSACTION ISOLATION LEVEL READ COMMITTED;
-- ... queries ...
COMMIT;
```

3. **Handle Errors**
```sql
BEGIN;
SAVEPOINT before_critical_update;

UPDATE accounts SET balance = balance - 100 WHERE id = 1;

-- If error occurs
ROLLBACK TO SAVEPOINT before_critical_update;
-- Or continue
COMMIT;
```

---

### Security Best Practices

1. **Use Parameterized Queries** (in application code)
```rust
// Good - parameterized
let stmt = "SELECT * FROM users WHERE username = ? AND password = ?";
execute(stmt, &[username, password]);

// Bad - string concatenation (SQL injection risk)
let stmt = format!("SELECT * FROM users WHERE username = '{}' AND password = '{}'",
    username, password);
```

2. **Principle of Least Privilege**
```sql
-- Grant minimum necessary permissions
GRANT SELECT ON employees TO analyst_role;
GRANT SELECT, INSERT, UPDATE ON orders TO sales_role;
```

3. **Use Constraints for Data Integrity**
```sql
CREATE TABLE orders (
    order_id INTEGER PRIMARY KEY,
    customer_id INTEGER NOT NULL,
    total_amount DECIMAL(12,2) CHECK (total_amount >= 0),
    order_date DATE DEFAULT CURRENT_DATE,
    FOREIGN KEY (customer_id) REFERENCES customers(customer_id)
);
```

---

## Migration Guide

### From Oracle to RustyDB

**Connection String Changes**:
```
Oracle: jdbc:oracle:thin:@localhost:1521:ORCL
RustyDB: postgresql://localhost:5432/rustydb
```

**Data Type Mapping**:
```sql
-- Oracle               -> RustyDB
NUMBER(p,s)            -> DECIMAL(p,s) or NUMBER(p,s)
VARCHAR2(n)            -> VARCHAR(n)
CLOB                   -> TEXT or CLOB
DATE                   -> TIMESTAMP or DATE
TIMESTAMP              -> TIMESTAMP
RAW(n)                 -> BYTEA or VARBINARY(n)
```

**Function Mapping**:
```sql
-- Oracle               -> RustyDB
SYSDATE                -> CURRENT_TIMESTAMP
NVL(x,y)               -> COALESCE(x,y)
DECODE(...)            -> CASE WHEN ... END
ROWNUM <= 10           -> LIMIT 10
```

---

### From PostgreSQL to RustyDB

Most PostgreSQL queries work without modification due to wire protocol compatibility.

**Minor Differences**:
```sql
-- PostgreSQL array syntax works
SELECT ARRAY[1,2,3];

-- JSON operations compatible
SELECT metadata->>'key' FROM documents;
```

---

### From MySQL to RustyDB

**Data Type Mapping**:
```sql
-- MySQL                -> RustyDB
INT                    -> INTEGER
TINYINT(1)             -> BOOLEAN
DATETIME               -> TIMESTAMP
LONGTEXT               -> TEXT
```

**Function Mapping**:
```sql
-- MySQL                -> RustyDB
NOW()                  -> CURRENT_TIMESTAMP
CONCAT_WS()            -> CONCAT_WS()  (compatible)
LIMIT 10, 20           -> LIMIT 20 OFFSET 10
```

---

## Performance Tuning

### Index Strategies

```sql
-- B-Tree for sorted data and range queries
CREATE INDEX idx_salary ON employees(salary);

-- Hash index for equality lookups
CREATE INDEX idx_customer_id ON orders USING HASH(customer_id);

-- Partial index for filtered queries
CREATE INDEX idx_active ON users(user_id) WHERE status = 'active';

-- Composite index for multi-column queries
CREATE INDEX idx_name_dept ON employees(department_id, last_name);
```

### Query Analysis

```sql
-- Use EXPLAIN to analyze query plans
EXPLAIN SELECT * FROM orders WHERE customer_id = 100;

-- EXPLAIN ANALYZE for actual execution stats
EXPLAIN ANALYZE
SELECT o.*, c.name
FROM orders o
JOIN customers c ON o.customer_id = c.customer_id
WHERE o.order_date > '2025-01-01';
```

---

## Appendix

### Reserved Keywords

The following are reserved keywords in RustyDB and should be quoted if used as identifiers:

```
ADD, ALL, ALTER, AND, ANY, AS, ASC, BACKUP, BEGIN, BETWEEN, BY, CASCADE, CASE,
CAST, CHECK, COLUMN, COMMIT, CONSTRAINT, CREATE, CROSS, CURRENT_DATE,
CURRENT_TIME, CURRENT_TIMESTAMP, DATABASE, DATE, DEFAULT, DELETE, DESC,
DISTINCT, DROP, ELSE, END, EXCEPT, EXEC, EXISTS, FOREIGN, FROM, FULL, GRANT,
GROUP, HAVING, IN, INDEX, INNER, INSERT, INTERSECT, INTO, IS, JOIN, KEY, LEFT,
LIKE, LIMIT, NOT, NULL, OFFSET, ON, OR, ORDER, OUTER, PRIMARY, PROCEDURE,
REFERENCES, REVOKE, RIGHT, ROLLBACK, SAVEPOINT, SELECT, SET, TABLE, THEN,
TRANSACTION, TRUNCATE, UNION, UNIQUE, UPDATE, VALUES, VIEW, WHEN, WHERE
```

To use reserved keywords as identifiers, quote them:
```sql
CREATE TABLE "order" (  -- "order" is quoted
    "select" INTEGER     -- "select" is quoted
);
```

---

### System Catalogs

Query system metadata:

```sql
-- List all tables
SELECT table_name FROM information_schema.tables;

-- List columns in a table
SELECT column_name, data_type
FROM information_schema.columns
WHERE table_name = 'employees';

-- List indexes
SELECT indexname, tablename
FROM pg_indexes;

-- List constraints
SELECT constraint_name, constraint_type, table_name
FROM information_schema.table_constraints;
```

---

### Error Codes

Common SQL error codes:

| Code | Description |
|------|-------------|
| 23000 | Integrity constraint violation |
| 23502 | NOT NULL violation |
| 23503 | Foreign key violation |
| 23505 | Unique constraint violation |
| 23514 | Check constraint violation |
| 40001 | Serialization failure (deadlock) |
| 42000 | Syntax error |
| 42P01 | Undefined table |
| 42703 | Undefined column |

---

## Additional Resources

### Documentation

- **Architecture Guide**: `/release/docs/0.5.1/CORE_FOUNDATION.md`
- **API Reference**: `/release/docs/0.5.1/API_REFERENCE_SUMMARY.md`
- **Security Guide**: `/release/docs/0.5.1/SECURITY.md`
- **Deployment Guide**: `/release/docs/0.5.1/DEPLOYMENT_GUIDE.md`
- **Operations Guide**: `/release/docs/0.5.1/OPERATIONS.md`

### Online Resources

- **RustyDB Documentation**: https://rustydb.io/docs
- **SQL Standard**: ISO/IEC 9075:2016
- **Oracle Compatibility**: https://rustydb.io/oracle-compatibility
- **PostgreSQL Compatibility**: https://rustydb.io/postgresql-compatibility

### Support

- **Community Forum**: https://community.rustydb.io
- **Issue Tracker**: https://github.com/rustydb/rustydb/issues
- **Enterprise Support**: enterprise@rustydb.io

---

## Changelog

### v0.5.1 (2025-12-27)

**SQL Features Added**:
- ✅ Full UNION and UNION ALL support
- ✅ CREATE OR REPLACE VIEW
- ✅ SELECT INTO statement
- ✅ INSERT INTO SELECT
- ✅ All 32 SQL Server string functions
- ✅ Enhanced ALTER TABLE with all operations
- ✅ TRUNCATE TABLE support
- ✅ DROP INDEX and DROP VIEW
- ✅ Stored procedure CREATE and EXEC
- ✅ Database-level operations (CREATE/DROP DATABASE)
- ✅ BACKUP DATABASE support

**Improvements**:
- Enhanced expression evaluator with CASE, BETWEEN, IN, LIKE
- Improved constraint system with full referential integrity
- Advanced MVCC with nanosecond-precision timestamps
- Multi-layer SQL injection prevention
- Performance optimizations for string functions

**Bug Fixes**:
- Fixed DISTINCT clause execution
- Corrected LIMIT/OFFSET handling
- Improved NULL handling in expressions
- Enhanced error messages for constraint violations

---

## License

RustyDB v0.5.1 SQL Reference
Copyright (c) 2025 RustyDB Project

This documentation is licensed under CC BY-SA 4.0.
The RustyDB software is licensed under Apache 2.0 or MIT.

---

**Document Version**: 1.0
**Last Updated**: December 27, 2025
**Maintained By**: RustyDB Documentation Team
**Status**: Production Ready
