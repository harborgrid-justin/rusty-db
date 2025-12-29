# RustyDB v0.6.5 - DDL Reference

**Version**: 0.6.5 | **Release**: Enterprise ($856M) | **Updated**: December 29, 2025

**✅ Validated for Enterprise Deployment**

---

## Table of Contents

1. [Overview](#overview)
2. [Database Operations](#database-operations)
3. [Table Operations](#table-operations)
4. [Index Operations](#index-operations)
5. [View Operations](#view-operations)
6. [Constraint Operations](#constraint-operations)
7. [Complete Syntax Reference](#complete-syntax-reference)

---

## Overview

Data Definition Language (DDL) statements define and modify database structure. RustyDB v0.6.5 provides comprehensive DDL support with **85% standard compliance**.

### Supported DDL Statements

| Statement | Purpose | Status |
|-----------|---------|--------|
| `CREATE DATABASE` | Create new database | ✅ Fully Supported |
| `DROP DATABASE` | Delete database | ✅ Fully Supported |
| `CREATE TABLE` | Create new table | ✅ Fully Supported |
| `ALTER TABLE` | Modify table structure | ✅ Fully Supported |
| `DROP TABLE` | Delete table | ✅ Fully Supported |
| `TRUNCATE TABLE` | Remove all table rows | ✅ Fully Supported |
| `CREATE INDEX` | Create index | ✅ Fully Supported |
| `CREATE UNIQUE INDEX` | Create unique index | ✅ Fully Supported |
| `DROP INDEX` | Delete index | ✅ Fully Supported |
| `CREATE VIEW` | Create view | ✅ Fully Supported |
| `CREATE OR REPLACE VIEW` | Create/replace view | ✅ Fully Supported |
| `DROP VIEW` | Delete view | ✅ Fully Supported |

---

## Database Operations

### CREATE DATABASE

Create a new database with default configuration.

**Syntax:**
```sql
CREATE DATABASE database_name;
```

**Examples:**
```sql
-- Create production database
CREATE DATABASE production_db;

-- Create sales database
CREATE DATABASE sales_db;

-- Create analytics database
CREATE DATABASE analytics_warehouse;
```

**Notes:**
- Database names must be valid identifiers (alphanumeric, underscores)
- Default configuration: 4KB page size, MVCC transactions
- See CONFIG_REFERENCE.md for database settings

**REST API Equivalent:**
```bash
POST /api/v1/sql/databases
{
  "name": "production_db"
}
```

---

### DROP DATABASE

Permanently delete a database and all its contents.

**Syntax:**
```sql
DROP DATABASE database_name;
```

**Examples:**
```sql
-- Drop test database
DROP DATABASE test_db;

-- Drop old archive database
DROP DATABASE archive_2024;
```

**⚠️ Warning:**
- This operation is **irreversible**
- All tables, views, indexes, and data are permanently deleted
- Active connections to the database will be terminated
- Ensure proper backups exist before dropping

**REST API Equivalent:**
```bash
DELETE /api/v1/sql/databases/production_db
```

---

## Table Operations

### CREATE TABLE

Create a new table with specified columns and data types.

**Syntax:**
```sql
CREATE TABLE table_name (
    column_name data_type [constraints],
    column_name data_type [constraints],
    ...
    [table_constraints]
);
```

**Data Types:**
- **Numeric**: `INTEGER`, `BIGINT`, `SMALLINT`, `FLOAT`, `DOUBLE`, `DECIMAL(p,s)`, `NUMBER(p,s)`
- **String**: `TEXT`, `VARCHAR(n)`, `VARCHAR2(n)`, `CHAR(n)`
- **Boolean**: `BOOLEAN`
- **Date/Time**: `DATE`, `TIMESTAMP`, `TIME`
- **Large Objects**: `BLOB`, `CLOB`
- **Structured**: `JSON`

**Column Constraints:**
- `PRIMARY KEY` - Unique identifier for row
- `NOT NULL` - Column cannot be NULL
- `UNIQUE` - All values must be unique
- `DEFAULT value` - Default value if not specified
- `CHECK (condition)` - Value must satisfy condition

**Examples:**

```sql
-- Simple table
CREATE TABLE employees (
    employee_id INTEGER,
    first_name VARCHAR2(50),
    last_name VARCHAR2(50),
    email VARCHAR(100),
    hire_date DATE,
    salary NUMBER(10, 2),
    active BOOLEAN
);

-- Table with constraints
CREATE TABLE departments (
    dept_id INTEGER PRIMARY KEY,
    dept_name VARCHAR2(100) NOT NULL,
    location VARCHAR2(100) DEFAULT 'Headquarters',
    budget NUMBER(12, 2) CHECK (budget > 0),
    active BOOLEAN DEFAULT true
);

-- Table with all data types
CREATE TABLE data_types_demo (
    id INTEGER PRIMARY KEY,
    small_num SMALLINT,
    big_num BIGINT,
    decimal_num DECIMAL(10, 2),
    oracle_num NUMBER(10, 2),
    float_num FLOAT,
    double_num DOUBLE,
    text_col TEXT,
    varchar_col VARCHAR(255),
    varchar2_col VARCHAR2(255),
    char_col CHAR(10),
    bool_col BOOLEAN,
    date_col DATE,
    timestamp_col TIMESTAMP,
    time_col TIME,
    blob_col BLOB,
    clob_col CLOB,
    json_col JSON
);

-- Table with CHECK constraints
CREATE TABLE products (
    product_id INTEGER PRIMARY KEY,
    product_name VARCHAR2(200) NOT NULL,
    price NUMBER(10, 2) NOT NULL CHECK (price >= 0),
    quantity INTEGER CHECK (quantity >= 0),
    category VARCHAR2(50),
    status VARCHAR2(20) CHECK (status IN ('active', 'inactive', 'discontinued'))
);

-- Table with composite primary key
CREATE TABLE order_items (
    order_id INTEGER,
    product_id INTEGER,
    quantity INTEGER NOT NULL,
    unit_price NUMBER(10, 2) NOT NULL,
    PRIMARY KEY (order_id, product_id)
);
```

**REST API Equivalent:**
```bash
POST /api/v1/tables/employees
{
  "columns": [
    {"name": "employee_id", "data_type": "INTEGER"},
    {"name": "first_name", "data_type": "VARCHAR2(50)"},
    {"name": "last_name", "data_type": "VARCHAR2(50)"}
  ]
}
```

---

### ALTER TABLE

Modify existing table structure by adding/dropping columns, constraints, or defaults.

**Syntax:**
```sql
ALTER TABLE table_name action;
```

**Available Actions:**

#### ADD COLUMN
```sql
ALTER TABLE table_name ADD COLUMN column_name data_type [constraints];
```

**Examples:**
```sql
-- Add single column
ALTER TABLE employees ADD COLUMN phone VARCHAR2(20);

-- Add column with constraint
ALTER TABLE employees ADD COLUMN manager_id INTEGER NOT NULL;

-- Add column with default
ALTER TABLE employees ADD COLUMN status VARCHAR2(20) DEFAULT 'active';
```

#### DROP COLUMN
```sql
ALTER TABLE table_name DROP COLUMN column_name;
```

**Examples:**
```sql
-- Drop single column
ALTER TABLE employees DROP COLUMN phone;

-- Drop multiple columns (sequential)
ALTER TABLE employees DROP COLUMN temp_field;
ALTER TABLE employees DROP COLUMN old_field;
```

#### MODIFY COLUMN
```sql
ALTER TABLE table_name MODIFY COLUMN column_name new_data_type;
```

**Examples:**
```sql
-- Change column type
ALTER TABLE employees MODIFY COLUMN email VARCHAR2(200);

-- Modify to allow NULL
ALTER TABLE employees MODIFY COLUMN phone VARCHAR2(20) NULL;

-- Modify to NOT NULL
ALTER TABLE employees MODIFY COLUMN email VARCHAR2(100) NOT NULL;
```

#### ALTER COLUMN
```sql
ALTER TABLE table_name ALTER COLUMN column_name new_data_type;
```

**Examples:**
```sql
-- Change column data type
ALTER TABLE products ALTER COLUMN description TEXT;

-- Change precision
ALTER TABLE products ALTER COLUMN price NUMBER(12, 2);
```

#### ADD CONSTRAINT
```sql
ALTER TABLE table_name ADD CONSTRAINT constraint_name constraint_definition;
```

**Examples:**
```sql
-- Add primary key
ALTER TABLE employees ADD CONSTRAINT pk_employee PRIMARY KEY (employee_id);

-- Add unique constraint
ALTER TABLE employees ADD CONSTRAINT uk_email UNIQUE (email);

-- Add check constraint
ALTER TABLE employees ADD CONSTRAINT chk_salary CHECK (salary > 0);

-- Add foreign key
ALTER TABLE employees ADD CONSTRAINT fk_dept
    FOREIGN KEY (dept_id) REFERENCES departments(dept_id);

-- Foreign key with cascade
ALTER TABLE order_items ADD CONSTRAINT fk_order
    FOREIGN KEY (order_id) REFERENCES orders(order_id)
    ON DELETE CASCADE ON UPDATE CASCADE;
```

#### DROP CONSTRAINT
```sql
ALTER TABLE table_name DROP CONSTRAINT constraint_name;
```

**Examples:**
```sql
-- Drop unique constraint
ALTER TABLE employees DROP CONSTRAINT uk_email;

-- Drop foreign key
ALTER TABLE employees DROP CONSTRAINT fk_dept;

-- Drop check constraint
ALTER TABLE employees DROP CONSTRAINT chk_salary;
```

#### DROP DEFAULT
```sql
ALTER TABLE table_name DROP DEFAULT column_name;
```

**Examples:**
```sql
-- Remove default value
ALTER TABLE departments DROP DEFAULT location;

-- Remove default from status column
ALTER TABLE employees DROP DEFAULT status;
```

**Complete ALTER TABLE Examples:**
```sql
-- Add employee department
ALTER TABLE employees ADD COLUMN dept_id INTEGER;
ALTER TABLE employees ADD CONSTRAINT fk_dept
    FOREIGN KEY (dept_id) REFERENCES departments(dept_id);

-- Restructure product table
ALTER TABLE products ADD COLUMN sku VARCHAR2(50);
ALTER TABLE products ADD CONSTRAINT uk_sku UNIQUE (sku);
ALTER TABLE products MODIFY COLUMN description TEXT;
ALTER TABLE products DROP COLUMN old_code;

-- Add audit fields
ALTER TABLE employees ADD COLUMN created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;
ALTER TABLE employees ADD COLUMN updated_at TIMESTAMP;
ALTER TABLE employees ADD COLUMN created_by VARCHAR2(50);
```

**REST API Equivalent:**
```bash
PATCH /api/v1/sql/tables/employees/alter
{
  "operation": "add_column",
  "column_definition": {
    "name": "phone",
    "data_type": "VARCHAR2(20)",
    "nullable": true
  }
}
```

---

### DROP TABLE

Permanently delete a table and all its data.

**Syntax:**
```sql
DROP TABLE table_name;
```

**Examples:**
```sql
-- Drop single table
DROP TABLE temp_employees;

-- Drop archive table
DROP TABLE sales_2023_archive;

-- Drop test table
DROP TABLE test_data;
```

**⚠️ Warning:**
- This operation is **irreversible**
- All data in the table is permanently deleted
- All indexes on the table are dropped
- All foreign keys referencing this table must be dropped first
- Views depending on this table may become invalid

**REST API Equivalent:**
```bash
DELETE /api/v1/tables/temp_employees
```

---

### TRUNCATE TABLE

Remove all rows from a table while keeping table structure.

**Syntax:**
```sql
TRUNCATE TABLE table_name;
```

**Examples:**
```sql
-- Clear all employee data
TRUNCATE TABLE employees;

-- Clear staging table
TRUNCATE TABLE staging_data;

-- Clear log table
TRUNCATE TABLE audit_log;
```

**Characteristics:**
- **Fast**: More efficient than `DELETE FROM table_name`
- **Non-transactional**: May not be rollback-able in all configurations
- **Resets**: May reset auto-increment/sequence counters
- **Keeps**: Table structure, indexes, and constraints remain intact

**TRUNCATE vs DELETE:**
```sql
-- TRUNCATE - faster, may not be rollback-able
TRUNCATE TABLE temp_data;

-- DELETE - slower, always transactional
DELETE FROM temp_data;
```

**REST API Equivalent:**
```bash
POST /api/v1/sql/tables/employees/truncate
```

---

## Index Operations

### CREATE INDEX

Create an index to improve query performance.

**Syntax:**
```sql
CREATE [UNIQUE] [BITMAP] INDEX index_name
ON table_name (column1 [, column2, ...]);
```

**Index Types:**
- **B-Tree Index** (default) - General purpose, balanced tree
- **Unique Index** - Ensures unique values
- **Bitmap Index** - Efficient for low-cardinality columns
- **Composite Index** - Multiple columns
- **Expression Index** - Index on computed expression

**Examples:**

```sql
-- Simple single-column index
CREATE INDEX idx_last_name ON employees (last_name);

-- Multi-column composite index
CREATE INDEX idx_name ON employees (last_name, first_name);

-- Unique index (enforces uniqueness)
CREATE UNIQUE INDEX idx_employee_id ON employees (employee_id);

-- Unique constraint on email
CREATE UNIQUE INDEX idx_email ON employees (email);

-- Bitmap index (for low cardinality)
CREATE BITMAP INDEX idx_active ON employees (active);
CREATE BITMAP INDEX idx_dept ON employees (dept_id);

-- Expression-based index
CREATE INDEX idx_upper_email ON employees (UPPER(email));
CREATE INDEX idx_full_name ON employees (first_name || ' ' || last_name);

-- Index on date column
CREATE INDEX idx_hire_date ON employees (hire_date);

-- Index for frequently joined columns
CREATE INDEX idx_dept_id ON employees (dept_id);

-- Composite index for common WHERE clause
CREATE INDEX idx_active_dept ON employees (active, dept_id);
```

**Index Naming Convention:**
- `idx_` prefix for regular indexes
- `uk_` prefix for unique constraints
- `bmp_` prefix for bitmap indexes
- Descriptive name including table and column(s)

**Performance Considerations:**
- Index columns used in WHERE, JOIN, ORDER BY
- Create indexes on foreign key columns
- Don't over-index (slows INSERT/UPDATE/DELETE)
- Monitor index usage and drop unused indexes
- Composite index column order matters (most selective first)

**REST API Equivalent:**
```bash
POST /api/v1/sql/indexes
{
  "name": "idx_email",
  "table": "employees",
  "columns": ["email"],
  "unique": false
}
```

---

### CREATE UNIQUE INDEX

Create an index that enforces unique values.

**Syntax:**
```sql
CREATE UNIQUE INDEX index_name ON table_name (column1 [, column2, ...]);
```

**Examples:**
```sql
-- Unique index on single column
CREATE UNIQUE INDEX uk_ssn ON employees (ssn);

-- Unique index on multiple columns
CREATE UNIQUE INDEX uk_first_last_birth ON employees (first_name, last_name, birth_date);

-- Unique email address
CREATE UNIQUE INDEX uk_email ON employees (email);

-- Unique product SKU
CREATE UNIQUE INDEX uk_product_sku ON products (sku);
```

**Notes:**
- Ensures all values in indexed column(s) are unique
- NULL values are allowed (multiple NULLs permitted)
- Automatically created for PRIMARY KEY constraints
- Can be created after table creation
- Violations cause INSERT/UPDATE to fail

---

### DROP INDEX

Remove an existing index.

**Syntax:**
```sql
DROP INDEX index_name;
```

**Examples:**
```sql
-- Drop regular index
DROP INDEX idx_last_name;

-- Drop unique index
DROP INDEX uk_email;

-- Drop bitmap index
DROP INDEX bmp_active;

-- Drop composite index
DROP INDEX idx_active_dept;
```

**Notes:**
- Does not affect table data
- Cannot drop indexes created by PRIMARY KEY constraints directly
- Queries may become slower after dropping frequently-used indexes
- Can be recreated with CREATE INDEX if needed

**REST API Equivalent:**
```bash
DELETE /api/v1/sql/indexes/idx_email
```

---

## View Operations

### CREATE VIEW

Create a virtual table based on a SELECT query.

**Syntax:**
```sql
CREATE VIEW view_name AS
SELECT_statement;
```

**Examples:**

```sql
-- Simple view
CREATE VIEW active_employees AS
SELECT employee_id, first_name, last_name, email
FROM employees
WHERE active = true;

-- View with join
CREATE VIEW employee_departments AS
SELECT
    e.employee_id,
    e.first_name,
    e.last_name,
    d.dept_name,
    d.location
FROM employees e
JOIN departments d ON e.dept_id = d.dept_id;

-- View with aggregation
CREATE VIEW department_summary AS
SELECT
    d.dept_id,
    d.dept_name,
    COUNT(e.employee_id) AS employee_count,
    AVG(e.salary) AS avg_salary,
    SUM(e.salary) AS total_salary
FROM departments d
LEFT JOIN employees e ON d.dept_id = e.dept_id
GROUP BY d.dept_id, d.dept_name;

-- View with calculated columns
CREATE VIEW employee_compensation AS
SELECT
    employee_id,
    first_name,
    last_name,
    salary,
    salary * 0.15 AS bonus,
    salary * 1.15 AS total_comp
FROM employees;

-- View filtering high earners
CREATE VIEW high_earners AS
SELECT employee_id, first_name, last_name, salary
FROM employees
WHERE salary > 100000
ORDER BY salary DESC;

-- Complex view with multiple joins
CREATE VIEW order_details AS
SELECT
    o.order_id,
    o.order_date,
    c.customer_name,
    p.product_name,
    oi.quantity,
    oi.unit_price,
    oi.quantity * oi.unit_price AS line_total
FROM orders o
JOIN customers c ON o.customer_id = c.customer_id
JOIN order_items oi ON o.order_id = oi.order_id
JOIN products p ON oi.product_id = p.product_id;
```

**View Benefits:**
- **Simplify complex queries** - Hide JOIN complexity
- **Security** - Expose only specific columns
- **Consistency** - Ensure same query logic across applications
- **Abstraction** - Change underlying tables without affecting applications

**View Limitations:**
- Views cannot be updated directly (in most cases)
- Performance depends on underlying query
- No indexes on views (index base tables instead)

---

### CREATE OR REPLACE VIEW

Create a view or replace it if it already exists.

**Syntax:**
```sql
CREATE OR REPLACE VIEW view_name AS
SELECT_statement;
```

**Examples:**

```sql
-- Create or replace simple view
CREATE OR REPLACE VIEW active_employees AS
SELECT employee_id, first_name, last_name, email, salary
FROM employees
WHERE active = true;

-- Update view definition (add columns)
CREATE OR REPLACE VIEW employee_summary AS
SELECT
    employee_id,
    first_name || ' ' || last_name AS full_name,
    email,
    dept_id,
    salary,
    hire_date
FROM employees;

-- Replace view with new filter
CREATE OR REPLACE VIEW high_earners AS
SELECT employee_id, first_name, last_name, salary
FROM employees
WHERE salary > 150000  -- Changed threshold
ORDER BY salary DESC;
```

**Benefits over CREATE VIEW:**
- No need to DROP VIEW first
- Maintains view dependencies
- Atomic operation (replace is transactional)
- Simplifies view updates

**REST API Equivalent:**
```bash
POST /api/v1/sql/views
{
  "name": "active_employees",
  "query": "SELECT * FROM employees WHERE active = true",
  "or_replace": true
}
```

---

### DROP VIEW

Remove an existing view.

**Syntax:**
```sql
DROP VIEW view_name;
```

**Examples:**
```sql
-- Drop simple view
DROP VIEW active_employees;

-- Drop summary view
DROP VIEW department_summary;

-- Drop multiple views (sequential)
DROP VIEW temp_view1;
DROP VIEW temp_view2;
DROP VIEW temp_view3;
```

**Notes:**
- Does not affect underlying tables or data
- Applications using the view will fail
- Can be recreated with CREATE VIEW
- Dependent views may become invalid

**REST API Equivalent:**
```bash
DELETE /api/v1/sql/views/active_employees
```

---

## Constraint Operations

### Constraint Types

RustyDB supports the following constraints:

#### PRIMARY KEY
Uniquely identifies each row in a table.

```sql
-- Column-level primary key
CREATE TABLE employees (
    employee_id INTEGER PRIMARY KEY,
    name VARCHAR2(100)
);

-- Table-level primary key
CREATE TABLE employees (
    employee_id INTEGER,
    name VARCHAR2(100),
    PRIMARY KEY (employee_id)
);

-- Composite primary key
CREATE TABLE order_items (
    order_id INTEGER,
    product_id INTEGER,
    quantity INTEGER,
    PRIMARY KEY (order_id, product_id)
);

-- Add primary key to existing table
ALTER TABLE employees ADD CONSTRAINT pk_employee PRIMARY KEY (employee_id);
```

**Characteristics:**
- Ensures uniqueness
- Cannot be NULL
- Only one per table
- Automatically creates unique index

---

#### FOREIGN KEY
Enforces referential integrity between tables.

```sql
-- Simple foreign key
ALTER TABLE employees ADD CONSTRAINT fk_dept
    FOREIGN KEY (dept_id) REFERENCES departments(dept_id);

-- Foreign key with cascade delete
ALTER TABLE order_items ADD CONSTRAINT fk_order
    FOREIGN KEY (order_id) REFERENCES orders(order_id)
    ON DELETE CASCADE;

-- Foreign key with cascade update
ALTER TABLE employees ADD CONSTRAINT fk_manager
    FOREIGN KEY (manager_id) REFERENCES employees(employee_id)
    ON UPDATE CASCADE;

-- Foreign key with SET NULL
ALTER TABLE employees ADD CONSTRAINT fk_dept
    FOREIGN KEY (dept_id) REFERENCES departments(dept_id)
    ON DELETE SET NULL;

-- Foreign key with RESTRICT (default)
ALTER TABLE order_items ADD CONSTRAINT fk_product
    FOREIGN KEY (product_id) REFERENCES products(product_id)
    ON DELETE RESTRICT;
```

**Referential Actions:**
- `CASCADE` - Delete/update child rows when parent changes
- `SET NULL` - Set child foreign key to NULL when parent deleted
- `RESTRICT` - Prevent parent deletion if children exist (default)
- `NO ACTION` - Similar to RESTRICT

---

#### UNIQUE
Ensures all values in column(s) are unique.

```sql
-- Column-level unique
CREATE TABLE employees (
    employee_id INTEGER PRIMARY KEY,
    email VARCHAR2(100) UNIQUE
);

-- Table-level unique
CREATE TABLE employees (
    employee_id INTEGER,
    email VARCHAR2(100),
    UNIQUE (email)
);

-- Composite unique constraint
CREATE TABLE products (
    product_id INTEGER PRIMARY KEY,
    supplier_id INTEGER,
    sku VARCHAR2(50),
    UNIQUE (supplier_id, sku)
);

-- Add unique constraint to existing table
ALTER TABLE employees ADD CONSTRAINT uk_email UNIQUE (email);
ALTER TABLE employees ADD CONSTRAINT uk_ssn UNIQUE (ssn);
```

---

#### CHECK
Ensures column values meet specified condition.

```sql
-- Column-level check
CREATE TABLE employees (
    employee_id INTEGER PRIMARY KEY,
    salary NUMBER(10, 2) CHECK (salary > 0),
    age INTEGER CHECK (age >= 18 AND age <= 100)
);

-- Table-level check
CREATE TABLE products (
    product_id INTEGER PRIMARY KEY,
    price NUMBER(10, 2),
    discount_price NUMBER(10, 2),
    CHECK (discount_price < price)
);

-- Named check constraint
CREATE TABLE employees (
    employee_id INTEGER PRIMARY KEY,
    salary NUMBER(10, 2),
    CONSTRAINT chk_salary_positive CHECK (salary > 0)
);

-- Add check constraint to existing table
ALTER TABLE employees ADD CONSTRAINT chk_salary CHECK (salary > 0);
ALTER TABLE products ADD CONSTRAINT chk_price CHECK (price >= 0);
ALTER TABLE employees ADD CONSTRAINT chk_email_format
    CHECK (email LIKE '%@%.%');
```

---

#### NOT NULL
Ensures column cannot contain NULL values.

```sql
-- Column definition with NOT NULL
CREATE TABLE employees (
    employee_id INTEGER NOT NULL,
    first_name VARCHAR2(50) NOT NULL,
    last_name VARCHAR2(50) NOT NULL,
    email VARCHAR2(100),  -- Nullable
    phone VARCHAR2(20)    -- Nullable
);

-- Modify existing column to NOT NULL
ALTER TABLE employees MODIFY COLUMN email VARCHAR2(100) NOT NULL;

-- Modify to allow NULL
ALTER TABLE employees MODIFY COLUMN phone VARCHAR2(20) NULL;
```

---

#### DEFAULT
Specifies default value for column when not provided.

```sql
-- Column with default value
CREATE TABLE employees (
    employee_id INTEGER PRIMARY KEY,
    first_name VARCHAR2(50) NOT NULL,
    status VARCHAR2(20) DEFAULT 'active',
    hire_date DATE DEFAULT CURRENT_DATE,
    is_manager BOOLEAN DEFAULT false
);

-- Add default to existing column
ALTER TABLE employees ADD COLUMN created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP;

-- Remove default
ALTER TABLE employees DROP DEFAULT status;
```

---

## Complete Syntax Reference

### CREATE DATABASE
```sql
CREATE DATABASE database_name;
```

### DROP DATABASE
```sql
DROP DATABASE database_name;
```

### CREATE TABLE
```sql
CREATE TABLE table_name (
    column1 datatype [constraint],
    column2 datatype [constraint],
    ...
    [table_constraint]
);
```

### ALTER TABLE
```sql
ALTER TABLE table_name ADD COLUMN column_name datatype [constraint];
ALTER TABLE table_name DROP COLUMN column_name;
ALTER TABLE table_name MODIFY COLUMN column_name new_datatype;
ALTER TABLE table_name ALTER COLUMN column_name new_datatype;
ALTER TABLE table_name ADD CONSTRAINT name constraint_definition;
ALTER TABLE table_name DROP CONSTRAINT constraint_name;
ALTER TABLE table_name DROP DEFAULT column_name;
```

### DROP TABLE
```sql
DROP TABLE table_name;
```

### TRUNCATE TABLE
```sql
TRUNCATE TABLE table_name;
```

### CREATE INDEX
```sql
CREATE [UNIQUE] [BITMAP] INDEX index_name ON table_name (column1 [, column2, ...]);
```

### DROP INDEX
```sql
DROP INDEX index_name;
```

### CREATE VIEW
```sql
CREATE VIEW view_name AS SELECT_statement;
```

### CREATE OR REPLACE VIEW
```sql
CREATE OR REPLACE VIEW view_name AS SELECT_statement;
```

### DROP VIEW
```sql
DROP VIEW view_name;
```

---

## Best Practices

### Table Design
1. Always define PRIMARY KEY for tables
2. Use appropriate data types (smallest that fits)
3. Add NOT NULL to required columns
4. Use CHECK constraints for data validation
5. Define FOREIGN KEYs for referential integrity
6. Consider normalization (avoid redundancy)

### Indexing Strategy
1. Index foreign key columns
2. Index columns used in WHERE clauses
3. Index columns used in JOIN conditions
4. Create composite indexes for multi-column queries
5. Don't over-index (affects INSERT/UPDATE performance)
6. Monitor and drop unused indexes

### View Usage
1. Use views to simplify complex queries
2. Use views for security (hide sensitive columns)
3. Avoid complex views in performance-critical paths
4. Index underlying tables, not views
5. Use CREATE OR REPLACE for view updates

### Constraint Guidelines
1. Always use PRIMARY KEY constraints
2. Define FOREIGN KEYs with appropriate cascade actions
3. Use CHECK constraints for business rules
4. Use NOT NULL for required fields
5. Name constraints explicitly for maintainability

---

**RustyDB v0.6.5** | DDL Reference | **✅ Validated for Enterprise Deployment**
