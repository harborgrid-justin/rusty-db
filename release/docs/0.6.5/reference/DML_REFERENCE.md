# RustyDB v0.6.5 - DML Reference

**Version**: 0.6.5 | **Release**: Enterprise ($856M) | **Updated**: December 29, 2025

**✅ Validated for Enterprise Deployment**

---

## Table of Contents

1. [Overview](#overview)
2. [SELECT Statement](#select-statement)
3. [INSERT Statement](#insert-statement)
4. [UPDATE Statement](#update-statement)
5. [DELETE Statement](#delete-statement)
6. [Advanced Query Techniques](#advanced-query-techniques)
7. [Complete Syntax Reference](#complete-syntax-reference)

---

## Overview

Data Manipulation Language (DML) statements query and modify data in database tables. RustyDB v0.6.5 provides **90% DML compliance** with comprehensive support for:

- **SELECT**: Query data with full predicate support, JOINs, aggregates, subqueries
- **INSERT**: Insert single or multiple rows, insert from SELECT
- **UPDATE**: Update rows with complex predicates and expressions
- **DELETE**: Delete rows with conditions

### DML Feature Support

| Feature | Support | Notes |
|---------|---------|-------|
| SELECT | ✅ 95% | Full support with JOINs, aggregates, subqueries |
| INSERT | ✅ 90% | Single/multiple rows, INSERT INTO SELECT |
| UPDATE | ✅ 90% | Complex predicates, CASE expressions |
| DELETE | ✅ 90% | WHERE filters, subqueries |
| DISTINCT | ✅ 100% | Remove duplicate rows |
| WHERE predicates | ✅ 100% | =, !=, <, >, BETWEEN, IN, LIKE, IS NULL |
| JOINs | ✅ 100% | INNER, LEFT, RIGHT, FULL OUTER, CROSS |
| Aggregates | ✅ 100% | COUNT, SUM, AVG, MIN, MAX with DISTINCT |
| GROUP BY | ✅ 100% | Grouping with HAVING clause |
| ORDER BY | ✅ 100% | ASC/DESC on multiple columns |
| LIMIT/OFFSET | ✅ 100% | Result limiting and pagination |
| Subqueries | ✅ 90% | Correlated and non-correlated |
| UNION | ✅ 100% | UNION and UNION ALL |

---

## SELECT Statement

### Basic SELECT

**Syntax:**
```sql
SELECT [DISTINCT] column1 [, column2, ...]
FROM table_name
[WHERE condition]
[GROUP BY column1 [, column2, ...]]
[HAVING condition]
[ORDER BY column1 [ASC|DESC] [, column2 [ASC|DESC], ...]]
[LIMIT count [OFFSET offset]];
```

**Examples:**

```sql
-- Select all columns
SELECT * FROM employees;

-- Select specific columns
SELECT employee_id, first_name, last_name FROM employees;

-- Select with column aliases
SELECT
    employee_id AS id,
    first_name AS fname,
    last_name AS lname,
    salary AS annual_salary
FROM employees;

-- Select with expressions
SELECT
    employee_id,
    first_name || ' ' || last_name AS full_name,
    salary * 12 AS annual_salary,
    salary * 0.15 AS annual_bonus
FROM employees;

-- Select distinct values
SELECT DISTINCT dept_id FROM employees;
SELECT DISTINCT last_name FROM employees;
SELECT DISTINCT dept_id, active FROM employees;

-- Select with literals
SELECT
    employee_id,
    first_name,
    'Active' AS status,
    100 AS score
FROM employees;
```

---

### WHERE Clause

Filter rows based on conditions.

**Comparison Operators:**
```sql
-- Equality
SELECT * FROM employees WHERE employee_id = 100;
SELECT * FROM employees WHERE first_name = 'John';
SELECT * FROM employees WHERE active = true;

-- Inequality
SELECT * FROM employees WHERE dept_id != 10;
SELECT * FROM employees WHERE status <> 'inactive';

-- Comparison
SELECT * FROM employees WHERE salary > 50000;
SELECT * FROM employees WHERE salary >= 60000;
SELECT * FROM employees WHERE age < 65;
SELECT * FROM employees WHERE age <= 30;
```

**Logical Operators:**
```sql
-- AND
SELECT * FROM employees
WHERE salary > 50000 AND dept_id = 10;

SELECT * FROM employees
WHERE active = true AND salary > 75000 AND dept_id IN (10, 20);

-- OR
SELECT * FROM employees
WHERE dept_id = 10 OR dept_id = 20;

SELECT * FROM employees
WHERE salary < 40000 OR salary > 150000;

-- NOT
SELECT * FROM employees WHERE NOT (active = false);
SELECT * FROM employees WHERE NOT dept_id = 10;

-- Complex combinations
SELECT * FROM employees
WHERE (salary > 50000 AND dept_id = 10)
   OR (salary > 75000 AND dept_id = 20)
   AND active = true;
```

**BETWEEN Operator:**
```sql
-- Numeric range
SELECT * FROM employees WHERE salary BETWEEN 50000 AND 100000;

-- Date range
SELECT * FROM employees WHERE hire_date BETWEEN '2020-01-01' AND '2025-12-31';

-- Age range
SELECT * FROM employees WHERE age BETWEEN 25 AND 45;

-- Negated BETWEEN
SELECT * FROM employees WHERE salary NOT BETWEEN 40000 AND 60000;
```

**IN Operator:**
```sql
-- List of values
SELECT * FROM employees WHERE dept_id IN (10, 20, 30);
SELECT * FROM employees WHERE status IN ('active', 'on-leave', 'probation');

-- String values
SELECT * FROM employees WHERE last_name IN ('Smith', 'Johnson', 'Williams');

-- Negated IN
SELECT * FROM employees WHERE dept_id NOT IN (10, 20);

-- Subquery with IN
SELECT * FROM employees
WHERE dept_id IN (SELECT dept_id FROM departments WHERE location = 'New York');
```

**LIKE Operator:**
```sql
-- Starts with
SELECT * FROM employees WHERE last_name LIKE 'S%';
SELECT * FROM employees WHERE first_name LIKE 'Jo%';

-- Ends with
SELECT * FROM employees WHERE email LIKE '%@gmail.com';
SELECT * FROM employees WHERE last_name LIKE '%son';

-- Contains
SELECT * FROM employees WHERE last_name LIKE '%mit%';
SELECT * FROM employees WHERE email LIKE '%@%';

-- Single character wildcard (_)
SELECT * FROM employees WHERE first_name LIKE 'J_hn';    -- John, Joan, etc.
SELECT * FROM employees WHERE phone LIKE '555-____';

-- Negated LIKE
SELECT * FROM employees WHERE email NOT LIKE '%@test.com';
SELECT * FROM employees WHERE last_name NOT LIKE '%temp%';

-- Case-insensitive search (using UPPER/LOWER)
SELECT * FROM employees WHERE UPPER(last_name) LIKE 'SMITH%';
```

**NULL Checks:**
```sql
-- IS NULL
SELECT * FROM employees WHERE email IS NULL;
SELECT * FROM employees WHERE phone IS NULL;
SELECT * FROM employees WHERE manager_id IS NULL;

-- IS NOT NULL
SELECT * FROM employees WHERE email IS NOT NULL;
SELECT * FROM employees WHERE phone IS NOT NULL;
SELECT * FROM employees WHERE dept_id IS NOT NULL;

-- Combine with other conditions
SELECT * FROM employees
WHERE email IS NOT NULL AND active = true;
```

---

### ORDER BY Clause

Sort results by one or more columns.

**Examples:**
```sql
-- Single column ascending (default)
SELECT * FROM employees ORDER BY last_name;
SELECT * FROM employees ORDER BY last_name ASC;

-- Single column descending
SELECT * FROM employees ORDER BY salary DESC;
SELECT * FROM employees ORDER BY hire_date DESC;

-- Multiple columns
SELECT * FROM employees
ORDER BY dept_id ASC, salary DESC;

SELECT * FROM employees
ORDER BY last_name ASC, first_name ASC;

-- Order by expression
SELECT * FROM employees
ORDER BY (salary * 1.1) DESC;

SELECT * FROM employees
ORDER BY LENGTH(last_name) DESC;

-- Order by column position
SELECT employee_id, first_name, last_name, salary
FROM employees
ORDER BY 4 DESC, 2 ASC;  -- 4=salary DESC, 2=first_name ASC

-- Order by calculated column
SELECT
    employee_id,
    first_name,
    salary,
    salary * 12 AS annual_salary
FROM employees
ORDER BY annual_salary DESC;
```

---

### LIMIT and OFFSET

Control result set size and implement pagination.

**Examples:**
```sql
-- Limit to first 10 rows
SELECT * FROM employees LIMIT 10;

-- Limit with ORDER BY (top N)
SELECT * FROM employees
ORDER BY salary DESC
LIMIT 10;

-- Skip first 20 rows, return next 10 (pagination)
SELECT * FROM employees LIMIT 10 OFFSET 20;

-- Page 1 (rows 1-10)
SELECT * FROM employees ORDER BY employee_id LIMIT 10 OFFSET 0;

-- Page 2 (rows 11-20)
SELECT * FROM employees ORDER BY employee_id LIMIT 10 OFFSET 10;

-- Page 3 (rows 21-30)
SELECT * FROM employees ORDER BY employee_id LIMIT 10 OFFSET 20;

-- Top 5 highest salaries
SELECT employee_id, first_name, last_name, salary
FROM employees
ORDER BY salary DESC
LIMIT 5;

-- Rows 11-15 (2nd set of 5)
SELECT * FROM employees
ORDER BY hire_date DESC
LIMIT 5 OFFSET 10;
```

---

### GROUP BY and Aggregates

Group rows and calculate aggregates.

**Aggregate Functions:**
- `COUNT(*)` - Count all rows
- `COUNT(column)` - Count non-NULL values
- `COUNT(DISTINCT column)` - Count unique values
- `SUM(column)` - Sum values
- `AVG(column)` - Average value
- `MIN(column)` - Minimum value
- `MAX(column)` - Maximum value

**Examples:**

```sql
-- Count all rows
SELECT COUNT(*) FROM employees;

-- Count by department
SELECT dept_id, COUNT(*) AS employee_count
FROM employees
GROUP BY dept_id;

-- Multiple aggregates
SELECT
    dept_id,
    COUNT(*) AS count,
    AVG(salary) AS avg_salary,
    MIN(salary) AS min_salary,
    MAX(salary) AS max_salary,
    SUM(salary) AS total_salary
FROM employees
GROUP BY dept_id;

-- Group by multiple columns
SELECT
    dept_id,
    active,
    COUNT(*) AS count
FROM employees
GROUP BY dept_id, active;

-- Count distinct
SELECT
    dept_id,
    COUNT(DISTINCT job_title) AS unique_jobs
FROM employees
GROUP BY dept_id;

-- HAVING clause (filter groups)
SELECT dept_id, COUNT(*) AS employee_count
FROM employees
GROUP BY dept_id
HAVING COUNT(*) > 10;

-- HAVING with multiple conditions
SELECT
    dept_id,
    AVG(salary) AS avg_salary,
    COUNT(*) AS count
FROM employees
GROUP BY dept_id
HAVING AVG(salary) > 60000 AND COUNT(*) > 5;

-- Complex grouping
SELECT
    dept_id,
    CASE
        WHEN salary < 50000 THEN 'Low'
        WHEN salary < 100000 THEN 'Medium'
        ELSE 'High'
    END AS salary_band,
    COUNT(*) AS count
FROM employees
GROUP BY dept_id, salary_band;
```

---

### JOINs

Combine data from multiple tables.

**INNER JOIN:**
```sql
-- Basic inner join
SELECT e.employee_id, e.first_name, d.dept_name
FROM employees e
INNER JOIN departments d ON e.dept_id = d.dept_id;

-- Join with WHERE
SELECT e.employee_id, e.first_name, d.dept_name
FROM employees e
INNER JOIN departments d ON e.dept_id = d.dept_id
WHERE e.salary > 50000;

-- Multiple columns in join
SELECT *
FROM order_items oi
INNER JOIN products p ON oi.product_id = p.product_id
    AND oi.supplier_id = p.supplier_id;
```

**LEFT JOIN (LEFT OUTER JOIN):**
```sql
-- Left join (all employees, even without department)
SELECT e.employee_id, e.first_name, d.dept_name
FROM employees e
LEFT JOIN departments d ON e.dept_id = d.dept_id;

-- Left join with NULL check
SELECT e.employee_id, e.first_name
FROM employees e
LEFT JOIN departments d ON e.dept_id = d.dept_id
WHERE d.dept_id IS NULL;  -- Employees without department
```

**RIGHT JOIN (RIGHT OUTER JOIN):**
```sql
-- Right join (all departments, even without employees)
SELECT e.employee_id, e.first_name, d.dept_name
FROM employees e
RIGHT JOIN departments d ON e.dept_id = d.dept_id;

-- Departments with no employees
SELECT d.dept_id, d.dept_name
FROM employees e
RIGHT JOIN departments d ON e.dept_id = d.dept_id
WHERE e.employee_id IS NULL;
```

**FULL OUTER JOIN:**
```sql
-- Full outer join (all employees and all departments)
SELECT e.employee_id, e.first_name, d.dept_name
FROM employees e
FULL OUTER JOIN departments d ON e.dept_id = d.dept_id;
```

**CROSS JOIN:**
```sql
-- Cartesian product (every employee with every department)
SELECT e.first_name, d.dept_name
FROM employees e
CROSS JOIN departments d;

-- Cross join for combinations
SELECT s.size, c.color
FROM sizes s
CROSS JOIN colors c;
```

**Multiple JOINs:**
```sql
-- Three-table join
SELECT
    e.first_name,
    e.last_name,
    d.dept_name,
    l.city
FROM employees e
INNER JOIN departments d ON e.dept_id = d.dept_id
INNER JOIN locations l ON d.location_id = l.location_id;

-- Join with aggregation
SELECT
    d.dept_name,
    COUNT(e.employee_id) AS employee_count,
    AVG(e.salary) AS avg_salary
FROM departments d
LEFT JOIN employees e ON d.dept_id = e.dept_id
GROUP BY d.dept_name;
```

**Self JOIN:**
```sql
-- Find employees and their managers
SELECT
    e1.employee_id,
    e1.first_name AS employee,
    e2.first_name AS manager
FROM employees e1
LEFT JOIN employees e2 ON e1.manager_id = e2.employee_id;

-- Find employees in same department
SELECT
    e1.first_name AS emp1,
    e2.first_name AS emp2
FROM employees e1
INNER JOIN employees e2 ON e1.dept_id = e2.dept_id
WHERE e1.employee_id < e2.employee_id;  -- Avoid duplicates
```

---

### Subqueries

Nested queries for complex data retrieval.

**Subquery in WHERE:**
```sql
-- Employees in specific departments
SELECT * FROM employees
WHERE dept_id IN (
    SELECT dept_id FROM departments WHERE location = 'New York'
);

-- Employees with above-average salary
SELECT * FROM employees
WHERE salary > (SELECT AVG(salary) FROM employees);

-- Employees in departments with >10 employees
SELECT * FROM employees
WHERE dept_id IN (
    SELECT dept_id FROM employees GROUP BY dept_id HAVING COUNT(*) > 10
);
```

**Correlated Subquery:**
```sql
-- Employees earning more than dept average
SELECT employee_id, first_name, salary
FROM employees e1
WHERE salary > (
    SELECT AVG(salary)
    FROM employees e2
    WHERE e2.dept_id = e1.dept_id
);

-- Departments with employees
SELECT * FROM departments d
WHERE EXISTS (
    SELECT 1 FROM employees e WHERE e.dept_id = d.dept_id
);

-- Departments without employees
SELECT * FROM departments d
WHERE NOT EXISTS (
    SELECT 1 FROM employees e WHERE e.dept_id = d.dept_id
);
```

**Subquery in SELECT:**
```sql
-- Include aggregate in SELECT
SELECT
    employee_id,
    first_name,
    salary,
    (SELECT AVG(salary) FROM employees) AS company_avg
FROM employees;

-- Department employee count in SELECT
SELECT
    d.dept_id,
    d.dept_name,
    (SELECT COUNT(*) FROM employees e WHERE e.dept_id = d.dept_id) AS emp_count
FROM departments d;
```

---

### UNION Operations

Combine results from multiple SELECT statements.

**UNION (removes duplicates):**
```sql
-- Combine employees and contractors
SELECT first_name, last_name, 'Employee' AS type
FROM employees
UNION
SELECT first_name, last_name, 'Contractor' AS type
FROM contractors;

-- Combine different tables
SELECT email FROM employees
UNION
SELECT email FROM customers;
```

**UNION ALL (keeps duplicates):**
```sql
-- Keep all rows including duplicates
SELECT first_name, last_name FROM employees
UNION ALL
SELECT first_name, last_name FROM contractors;

-- Faster than UNION (no duplicate check)
SELECT product_id FROM inventory_warehouse_a
UNION ALL
SELECT product_id FROM inventory_warehouse_b;
```

**UNION with ORDER BY:**
```sql
-- Sort combined results
SELECT first_name, last_name, hire_date, 'Employee' AS type
FROM employees
UNION ALL
SELECT first_name, last_name, start_date, 'Contractor' AS type
FROM contractors
ORDER BY last_name, first_name;
```

---

### CASE Expressions

Conditional logic in SELECT.

**Simple CASE:**
```sql
SELECT
    employee_id,
    first_name,
    dept_id,
    CASE dept_id
        WHEN 10 THEN 'Sales'
        WHEN 20 THEN 'Engineering'
        WHEN 30 THEN 'HR'
        WHEN 40 THEN 'Finance'
        ELSE 'Other'
    END AS department
FROM employees;
```

**Searched CASE:**
```sql
SELECT
    employee_id,
    first_name,
    salary,
    CASE
        WHEN salary < 50000 THEN 'Entry'
        WHEN salary < 75000 THEN 'Junior'
        WHEN salary < 100000 THEN 'Mid'
        WHEN salary < 150000 THEN 'Senior'
        ELSE 'Executive'
    END AS level
FROM employees;

-- Multiple conditions
SELECT
    employee_id,
    first_name,
    salary,
    CASE
        WHEN salary > 100000 AND dept_id = 10 THEN 'Sales Executive'
        WHEN salary > 100000 AND dept_id = 20 THEN 'Engineering Lead'
        WHEN salary > 50000 THEN 'Professional'
        ELSE 'Staff'
    END AS category
FROM employees;
```

---

## INSERT Statement

Add new rows to a table.

**Syntax:**
```sql
INSERT INTO table_name (column1, column2, ...)
VALUES (value1, value2, ...);
```

**Examples:**

```sql
-- Insert single row with explicit columns
INSERT INTO employees (employee_id, first_name, last_name, email)
VALUES (101, 'John', 'Doe', 'john.doe@example.com');

-- Insert all columns (must match table order)
INSERT INTO employees
VALUES (102, 'Jane', 'Smith', 'jane@example.com', '2025-01-15', 75000, true);

-- Insert with default values
INSERT INTO employees (employee_id, first_name, last_name)
VALUES (103, 'Bob', 'Johnson');  -- Other columns get DEFAULT or NULL

-- Insert multiple rows
INSERT INTO employees (employee_id, first_name, last_name, salary)
VALUES
    (104, 'Alice', 'Williams', 65000),
    (105, 'Charlie', 'Brown', 70000),
    (106, 'Diana', 'Davis', 72000);

-- Insert with expressions
INSERT INTO employees (employee_id, first_name, last_name, email, hire_date)
VALUES (107, 'Eve', 'Miller', LOWER('EVE.MILLER@EXAMPLE.COM'), CURRENT_DATE);

-- Insert with NULL
INSERT INTO employees (employee_id, first_name, last_name, email)
VALUES (108, 'Frank', 'Wilson', NULL);
```

**INSERT INTO SELECT:**
```sql
-- Copy all rows
INSERT INTO archive_employees
SELECT * FROM employees WHERE hire_date < '2020-01-01';

-- Copy specific columns
INSERT INTO employee_summary (emp_id, full_name, salary)
SELECT employee_id, first_name || ' ' || last_name, salary
FROM employees;

-- Insert with transformation
INSERT INTO high_earners (employee_id, name, annual_comp)
SELECT
    employee_id,
    first_name || ' ' || last_name,
    salary * 12
FROM employees
WHERE salary > 100000;
```

**REST API Equivalent:**
```bash
POST /api/v1/query
{
  "sql": "INSERT INTO employees (employee_id, first_name, last_name) VALUES (?, ?, ?)",
  "params": [101, "John", "Doe"]
}
```

---

## UPDATE Statement

Modify existing rows in a table.

**Syntax:**
```sql
UPDATE table_name
SET column1 = value1 [, column2 = value2, ...]
[WHERE condition];
```

**Examples:**

```sql
-- Update single column
UPDATE employees
SET salary = 80000
WHERE employee_id = 101;

-- Update multiple columns
UPDATE employees
SET salary = 85000, active = true, updated_at = CURRENT_TIMESTAMP
WHERE employee_id = 101;

-- Update with expression
UPDATE employees
SET salary = salary * 1.10
WHERE dept_id = 10;

-- Update with calculation
UPDATE products
SET price = cost * 1.5,
    margin = (cost * 1.5) - cost
WHERE category = 'Electronics';

-- Update all rows (use with caution!)
UPDATE employees SET active = true;

-- Update with CASE
UPDATE employees
SET salary = CASE
    WHEN salary < 50000 THEN salary * 1.15
    WHEN salary < 100000 THEN salary * 1.10
    ELSE salary * 1.05
END
WHERE active = true;

-- Update based on another column
UPDATE employees
SET status = CASE
    WHEN age < 18 THEN 'minor'
    WHEN age >= 65 THEN 'senior'
    ELSE 'adult'
END;

-- Update with subquery
UPDATE employees
SET salary = (SELECT AVG(salary) FROM employees WHERE dept_id = 10)
WHERE employee_id = 999;

-- Update with join (using subquery)
UPDATE employees
SET dept_id = (
    SELECT dept_id FROM departments WHERE dept_name = 'Engineering'
)
WHERE job_title = 'Software Engineer';
```

**REST API Equivalent:**
```bash
POST /api/v1/query
{
  "sql": "UPDATE employees SET salary = ? WHERE employee_id = ?",
  "params": [80000, 101]
}
```

---

## DELETE Statement

Remove rows from a table.

**Syntax:**
```sql
DELETE FROM table_name
[WHERE condition];
```

**Examples:**

```sql
-- Delete single row
DELETE FROM employees WHERE employee_id = 101;

-- Delete with conditions
DELETE FROM employees
WHERE active = false AND hire_date < '2015-01-01';

-- Delete with AND/OR
DELETE FROM employees
WHERE (dept_id = 10 AND salary < 40000)
   OR (dept_id = 20 AND salary < 50000);

-- Delete with BETWEEN
DELETE FROM employees
WHERE employee_id BETWEEN 1000 AND 2000;

-- Delete with IN
DELETE FROM employees
WHERE dept_id IN (90, 91, 92);

-- Delete with LIKE
DELETE FROM employees
WHERE email LIKE '%@test.com';

-- Delete with subquery
DELETE FROM employees
WHERE dept_id IN (
    SELECT dept_id FROM departments WHERE location = 'Closed Office'
);

-- Delete with NOT EXISTS
DELETE FROM products p
WHERE NOT EXISTS (
    SELECT 1 FROM order_items oi WHERE oi.product_id = p.product_id
);

-- Delete all rows (use with caution!)
DELETE FROM temp_table;
```

**⚠️ Warning:**
- `DELETE FROM table` without WHERE deletes ALL rows
- Use `TRUNCATE TABLE` for faster deletion of all rows
- Always test with SELECT first:
  ```sql
  -- Test first
  SELECT * FROM employees WHERE active = false;
  -- Then delete
  DELETE FROM employees WHERE active = false;
  ```

**REST API Equivalent:**
```bash
POST /api/v1/query
{
  "sql": "DELETE FROM employees WHERE employee_id = ?",
  "params": [101]
}
```

---

## Advanced Query Techniques

### Window Functions (Future)
```sql
-- Planned for future release
-- ROW_NUMBER(), RANK(), DENSE_RANK(), etc.
```

### Common Table Expressions (CTEs)
```sql
-- Use subqueries or views as alternative
-- WITH clause planned for future release
```

### Recursive Queries
```sql
-- Planned for future release
-- Use procedures for complex hierarchical queries
```

---

## Complete Syntax Reference

### SELECT
```sql
SELECT [DISTINCT] column_list
FROM table_name
[WHERE condition]
[GROUP BY column_list]
[HAVING condition]
[ORDER BY column_list [ASC|DESC]]
[LIMIT count [OFFSET offset]];
```

### INSERT
```sql
-- Single row
INSERT INTO table_name (column1, column2, ...)
VALUES (value1, value2, ...);

-- Multiple rows
INSERT INTO table_name (column1, column2, ...)
VALUES
    (value1a, value2a, ...),
    (value1b, value2b, ...);

-- From SELECT
INSERT INTO table_name (column1, column2, ...)
SELECT column1, column2, ... FROM source_table;
```

### UPDATE
```sql
UPDATE table_name
SET column1 = value1 [, column2 = value2, ...]
[WHERE condition];
```

### DELETE
```sql
DELETE FROM table_name
[WHERE condition];
```

---

## Performance Best Practices

### Query Optimization
1. **Use indexes** on columns in WHERE, JOIN, ORDER BY
2. **Specify columns** instead of SELECT *
3. **Filter early** with WHERE clauses
4. **Use LIMIT** for large result sets
5. **Avoid functions** in WHERE (prevents index use)
6. **Use EXISTS** instead of IN for large subqueries

### Indexing for DML
```sql
-- Index foreign keys for JOINs
CREATE INDEX idx_dept_id ON employees (dept_id);

-- Index frequently queried columns
CREATE INDEX idx_email ON employees (email);
CREATE INDEX idx_active ON employees (active);

-- Composite index for multi-column WHERE
CREATE INDEX idx_active_dept ON employees (active, dept_id);
```

### Batch Operations
```sql
-- Instead of multiple INSERTs
INSERT INTO employees VALUES (1, 'A');
INSERT INTO employees VALUES (2, 'B');
INSERT INTO employees VALUES (3, 'C');

-- Use multi-row INSERT
INSERT INTO employees VALUES (1, 'A'), (2, 'B'), (3, 'C');
```

---

## Common Query Patterns

### Pagination
```sql
-- Page 1
SELECT * FROM employees ORDER BY employee_id LIMIT 20 OFFSET 0;

-- Page 2
SELECT * FROM employees ORDER BY employee_id LIMIT 20 OFFSET 20;

-- Page N
SELECT * FROM employees ORDER BY employee_id LIMIT 20 OFFSET (N-1)*20;
```

### Top N
```sql
-- Top 10 highest salaries
SELECT * FROM employees ORDER BY salary DESC LIMIT 10;

-- Top 5 recent hires
SELECT * FROM employees ORDER BY hire_date DESC LIMIT 5;
```

### Find Duplicates
```sql
SELECT email, COUNT(*) AS count
FROM employees
GROUP BY email
HAVING COUNT(*) > 1;
```

### Conditional Aggregation
```sql
SELECT
    dept_id,
    COUNT(CASE WHEN active = true THEN 1 END) AS active_count,
    COUNT(CASE WHEN active = false THEN 1 END) AS inactive_count
FROM employees
GROUP BY dept_id;
```

---

**RustyDB v0.6.5** | DML Reference | **✅ Validated for Enterprise Deployment**
