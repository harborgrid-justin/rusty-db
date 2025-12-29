# RustyDB v0.6.5 - Transaction Control Reference

**Version**: 0.6.5 | **Release**: Enterprise ($856M) | **Updated**: December 29, 2025

**✅ Validated for Enterprise Deployment** | **✅ MVCC with ACID Guarantees**

---

## Table of Contents

1. [Overview](#overview)
2. [Transaction Basics](#transaction-basics)
3. [Transaction Statements](#transaction-statements)
4. [Isolation Levels](#isolation-levels)
5. [Savepoints](#savepoints)
6. [MVCC Architecture](#mvcc-architecture)
7. [Concurrency Control](#concurrency-control)
8. [Best Practices](#best-practices)

---

## Overview

RustyDB v0.6.5 provides enterprise-grade transaction management with **Multi-Version Concurrency Control (MVCC)** and full ACID compliance.

### ACID Properties

| Property | Implementation | Status |
|----------|---------------|--------|
| **Atomicity** | All-or-nothing operations | ✅ Fully Supported |
| **Consistency** | Constraint enforcement | ✅ Fully Supported |
| **Isolation** | MVCC with 5 isolation levels | ✅ Fully Supported |
| **Durability** | Write-Ahead Logging (WAL) | ✅ Fully Supported |

### Transaction Features

- **MVCC**: Multi-Version Concurrency Control for high concurrency
- **UUID Transaction IDs**: Globally unique transaction identifiers
- **Two-Phase Locking**: Deadlock detection and prevention
- **Write-Ahead Logging**: Crash recovery and durability
- **Multiple Isolation Levels**: READ UNCOMMITTED to SERIALIZABLE
- **Savepoints**: Partial rollback within transactions
- **Deadlock Detection**: Automatic deadlock resolution

---

## Transaction Basics

### Implicit Transactions

Transactions begin automatically with the first DML statement (INSERT, UPDATE, DELETE, SELECT FOR UPDATE).

```sql
-- Transaction starts implicitly
UPDATE employees SET salary = 80000 WHERE employee_id = 101;

-- Transaction continues
INSERT INTO audit_log VALUES (101, 'Salary updated', CURRENT_TIMESTAMP);

-- Commit or rollback required
COMMIT;
```

---

### Explicit Transactions

**Syntax:**
```sql
BEGIN;  -- or START TRANSACTION;
-- DML statements
COMMIT; -- or ROLLBACK;
```

**Example:**
```sql
BEGIN;

INSERT INTO employees (employee_id, first_name, last_name)
VALUES (201, 'John', 'Doe');

UPDATE departments SET employee_count = employee_count + 1
WHERE dept_id = 10;

COMMIT;
```

---

### Auto-Commit Mode

By default, each statement is auto-committed unless in an explicit transaction.

**Disable auto-commit:**
```sql
SET AUTOCOMMIT = OFF;

-- Now transactions are explicit
UPDATE employees SET salary = 85000 WHERE employee_id = 101;
COMMIT;  -- Must commit manually
```

**Enable auto-commit:**
```sql
SET AUTOCOMMIT = ON;

-- Each statement commits automatically
UPDATE employees SET salary = 85000 WHERE employee_id = 101;
-- Automatically committed
```

---

## Transaction Statements

### BEGIN / START TRANSACTION

Start an explicit transaction.

**Syntax:**
```sql
BEGIN [TRANSACTION];
-- or
START TRANSACTION;
```

**Examples:**
```sql
-- Simple transaction
BEGIN;
UPDATE employees SET active = true WHERE dept_id = 10;
COMMIT;

-- Transaction with multiple operations
START TRANSACTION;
INSERT INTO orders (order_id, customer_id) VALUES (1001, 501);
INSERT INTO order_items (order_id, product_id, quantity) VALUES (1001, 201, 5);
UPDATE inventory SET quantity = quantity - 5 WHERE product_id = 201;
COMMIT;
```

---

### COMMIT

Permanently save all changes made in the current transaction.

**Syntax:**
```sql
COMMIT;
```

**Examples:**
```sql
-- Simple commit
BEGIN;
UPDATE employees SET salary = salary * 1.1 WHERE dept_id = 10;
COMMIT;

-- Commit after multiple operations
BEGIN;
DELETE FROM temp_data WHERE created_at < CURRENT_DATE - 30;
INSERT INTO archive SELECT * FROM temp_data;
COMMIT;

-- Commit in stored procedure
CREATE PROCEDURE update_salary(p_emp_id INTEGER, p_salary NUMBER) AS
BEGIN
    UPDATE employees SET salary = p_salary WHERE employee_id = p_emp_id;
    COMMIT;
END;
```

**Characteristics:**
- Makes all changes permanent
- Releases all locks held by transaction
- Increments transaction ID
- Writes to Write-Ahead Log (WAL)
- Cannot be undone after commit

---

### ROLLBACK

Undo all changes made in the current transaction.

**Syntax:**
```sql
ROLLBACK [TO SAVEPOINT savepoint_name];
```

**Examples:**
```sql
-- Rollback entire transaction
BEGIN;
UPDATE employees SET salary = 0;  -- Oops, mistake!
ROLLBACK;  -- Undo changes

-- Rollback on error
BEGIN;
UPDATE inventory SET quantity = quantity - 10 WHERE product_id = 101;

-- Check if sufficient quantity
DECLARE v_qty INTEGER;
SELECT quantity INTO v_qty FROM inventory WHERE product_id = 101;

IF v_qty < 0 THEN
    ROLLBACK;
ELSE
    COMMIT;
END IF;

-- Rollback in exception handler
BEGIN
    UPDATE employees SET salary = salary * 1.1;
    UPDATE departments SET budget = budget * 1.1;
EXCEPTION
    WHEN OTHERS THEN
        ROLLBACK;
        RAISE;
END;
```

**Characteristics:**
- Undoes all changes since BEGIN
- Releases all locks
- Returns database to pre-transaction state
- Cannot be undone after rollback

---

### SAVEPOINT

Create a named point within a transaction for partial rollback.

**Syntax:**
```sql
SAVEPOINT savepoint_name;
ROLLBACK TO savepoint_name;
```

**Examples:**

```sql
-- Simple savepoint usage
BEGIN;

UPDATE employees SET salary = salary * 1.05 WHERE dept_id = 10;
SAVEPOINT after_dept_10;

UPDATE employees SET salary = salary * 1.05 WHERE dept_id = 20;
SAVEPOINT after_dept_20;

UPDATE employees SET salary = salary * 1.05 WHERE dept_id = 30;

-- Rollback only dept 30 changes
ROLLBACK TO after_dept_20;

-- Commit dept 10 and 20 changes
COMMIT;
```

**Complex Example:**
```sql
BEGIN;

-- Insert main order
INSERT INTO orders (order_id, customer_id, order_date)
VALUES (1001, 501, CURRENT_DATE);
SAVEPOINT order_created;

-- Insert order items
INSERT INTO order_items VALUES (1001, 201, 5, 19.99);
SAVEPOINT item_1_added;

INSERT INTO order_items VALUES (1001, 202, 3, 29.99);
SAVEPOINT item_2_added;

-- Update inventory
UPDATE inventory SET quantity = quantity - 5 WHERE product_id = 201;
UPDATE inventory SET quantity = quantity - 3 WHERE product_id = 202;

-- Check inventory levels
DECLARE v_qty INTEGER;
SELECT quantity INTO v_qty FROM inventory WHERE product_id = 201;

IF v_qty < 0 THEN
    -- Not enough inventory, rollback to after order creation
    ROLLBACK TO order_created;
    -- Order exists but no items
ELSE
    -- Everything OK, commit
    COMMIT;
END IF;
```

**Savepoint Characteristics:**
- Multiple savepoints allowed in one transaction
- Can rollback to any savepoint
- Earlier savepoints released when rolling back to later point
- Savepoints released on COMMIT or full ROLLBACK

---

### RELEASE SAVEPOINT

Release a savepoint without rolling back.

**Syntax:**
```sql
RELEASE SAVEPOINT savepoint_name;
```

**Example:**
```sql
BEGIN;

UPDATE employees SET salary = salary * 1.1;
SAVEPOINT salary_updated;

-- More operations
UPDATE departments SET budget = budget * 1.1;

-- No longer need this savepoint
RELEASE SAVEPOINT salary_updated;

COMMIT;
```

---

## Isolation Levels

RustyDB supports 5 isolation levels following SQL standard and Oracle extensions.

### Isolation Level Summary

| Level | Dirty Read | Non-Repeatable Read | Phantom Read | Performance | Use Case |
|-------|-----------|---------------------|--------------|-------------|----------|
| **READ UNCOMMITTED** | ⚠️ Yes | ⚠️ Yes | ⚠️ Yes | Highest | Analytics, approximate queries |
| **READ COMMITTED** | ✅ No | ⚠️ Yes | ⚠️ Yes | High | **Default**, general use |
| **REPEATABLE READ** | ✅ No | ✅ No | ⚠️ Yes | Medium | Reports, consistent reads |
| **SERIALIZABLE** | ✅ No | ✅ No | ✅ No | Low | Financial, critical operations |
| **SNAPSHOT ISOLATION** | ✅ No | ✅ No | ✅ No | Medium-High | Long-running transactions |

---

### READ UNCOMMITTED

**Description:** Lowest isolation level. Allows reading uncommitted changes from other transactions (dirty reads).

**Syntax:**
```sql
SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED;
```

**Example:**
```sql
-- Session 1
BEGIN;
SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED;

-- Can see uncommitted changes from Session 2
SELECT * FROM employees WHERE employee_id = 101;
```

**Characteristics:**
- ⚠️ Allows dirty reads
- ⚠️ Allows non-repeatable reads
- ⚠️ Allows phantom reads
- ✅ Highest concurrency
- ✅ No read locks

**Use Cases:**
- Analytics and reporting (approximate data acceptable)
- Dashboard queries
- Data exploration

---

### READ COMMITTED (Default)

**Description:** Prevents dirty reads. Each query sees committed data as of query start.

**Syntax:**
```sql
SET TRANSACTION ISOLATION LEVEL READ COMMITTED;
```

**Example:**
```sql
BEGIN;
SET TRANSACTION ISOLATION LEVEL READ COMMITTED;

-- Only sees committed data
SELECT salary FROM employees WHERE employee_id = 101;
-- Returns: 50000

-- Another transaction commits salary change to 60000

-- New query sees the committed change
SELECT salary FROM employees WHERE employee_id = 101;
-- Returns: 60000 (non-repeatable read)

COMMIT;
```

**Characteristics:**
- ✅ Prevents dirty reads
- ⚠️ Allows non-repeatable reads
- ⚠️ Allows phantom reads
- ✅ Good concurrency
- ✅ **Default isolation level**

**Use Cases:**
- General application queries
- Most OLTP operations
- Web applications

---

### REPEATABLE READ

**Description:** Prevents dirty reads and non-repeatable reads. Same query returns same results throughout transaction.

**Syntax:**
```sql
SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;
```

**Example:**
```sql
BEGIN;
SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;

-- First read
SELECT salary FROM employees WHERE employee_id = 101;
-- Returns: 50000

-- Another transaction commits salary change to 60000

-- Same query returns same value (repeatable read)
SELECT salary FROM employees WHERE employee_id = 101;
-- Still returns: 50000

COMMIT;
```

**Characteristics:**
- ✅ Prevents dirty reads
- ✅ Prevents non-repeatable reads
- ⚠️ May allow phantom reads (new rows)
- ⚠️ Lower concurrency
- ⚠️ More locks held

**Use Cases:**
- Reports requiring consistent data
- Batch processing
- Data validation

---

### SERIALIZABLE

**Description:** Highest isolation level. Transactions appear to execute serially.

**Syntax:**
```sql
SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
```

**Example:**
```sql
BEGIN;
SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;

-- First query
SELECT COUNT(*) FROM employees WHERE dept_id = 10;
-- Returns: 50

-- Another transaction inserts new employee in dept 10 and commits

-- Same query returns same count (no phantoms)
SELECT COUNT(*) FROM employees WHERE dept_id = 10;
-- Still returns: 50

COMMIT;
```

**Characteristics:**
- ✅ Prevents dirty reads
- ✅ Prevents non-repeatable reads
- ✅ Prevents phantom reads
- ⚠️ Lowest concurrency
- ⚠️ Most locks held
- ⚠️ Higher risk of deadlocks

**Use Cases:**
- Financial transactions
- Inventory management
- Critical operations requiring perfect consistency

---

### SNAPSHOT ISOLATION

**Description:** Each transaction sees a consistent snapshot of the database as of transaction start. Oracle-style implementation.

**Syntax:**
```sql
SET TRANSACTION ISOLATION LEVEL SNAPSHOT;
```

**Example:**
```sql
-- Transaction 1
BEGIN;
SET TRANSACTION ISOLATION LEVEL SNAPSHOT;

SELECT * FROM employees WHERE dept_id = 10;
-- Sees snapshot as of transaction start

-- Transaction 2 commits changes

-- Transaction 1 still sees original snapshot
SELECT * FROM employees WHERE dept_id = 10;
-- Same results as before

-- Updates based on snapshot
UPDATE employees SET salary = salary * 1.1 WHERE dept_id = 10;

COMMIT;
```

**Characteristics:**
- ✅ Prevents dirty reads
- ✅ Prevents non-repeatable reads
- ✅ Prevents phantom reads
- ✅ Good concurrency for read-heavy workloads
- ✅ No read locks needed
- ⚠️ Write-write conflicts possible

**Use Cases:**
- Long-running reports
- Data warehousing queries
- Read-heavy applications

---

## MVCC Architecture

### How MVCC Works

RustyDB uses **Multi-Version Concurrency Control** to provide high concurrency with strong isolation guarantees.

**Key Concepts:**

1. **Version Numbers**: Each transaction gets a unique ID
2. **Row Versions**: Each row stores multiple versions
3. **Visibility Rules**: Each transaction sees appropriate version based on isolation level
4. **No Read Locks**: Readers don't block writers, writers don't block readers
5. **Garbage Collection**: Old versions cleaned up automatically

**Row Version Structure:**
```
Row Data:
  - Actual column values
  - xmin: Transaction ID that created this version
  - xmax: Transaction ID that deleted/updated this version (or NULL)
  - Version pointer: Link to previous version
```

---

### Transaction ID Management

```sql
-- Get current transaction ID
SELECT TXN_ID();

-- View transaction information
SELECT * FROM pg_stat_activity;

-- Long-running transactions
SELECT pid, state, transaction_started, query
FROM pg_stat_activity
WHERE state = 'active'
  AND transaction_started < NOW() - INTERVAL '5 minutes';
```

---

## Concurrency Control

### Locking

RustyDB uses **two-phase locking** with automatic deadlock detection.

**Lock Types:**
- **Shared Lock (S)**: Multiple readers
- **Exclusive Lock (X)**: Single writer
- **Update Lock (U)**: Intent to update
- **Row-level locks**: Fine-grained locking

**Lock Compatibility:**
```
       S    X    U
S     Yes  No   Yes
X     No   No   No
U     Yes  No   No
```

---

### SELECT FOR UPDATE

Acquire exclusive lock on selected rows.

**Syntax:**
```sql
SELECT columns
FROM table
WHERE condition
FOR UPDATE [NOWAIT];
```

**Examples:**

```sql
-- Lock row for update
BEGIN;

SELECT salary FROM employees
WHERE employee_id = 101
FOR UPDATE;

-- No other transaction can modify this row
UPDATE employees SET salary = 80000 WHERE employee_id = 101;

COMMIT;
```

**With NOWAIT:**
```sql
-- Don't wait if row is locked
BEGIN;

SELECT * FROM inventory
WHERE product_id = 201
FOR UPDATE NOWAIT;

-- Raises error if row is locked
-- Otherwise, row is locked and can be updated

COMMIT;
```

---

### Deadlock Detection

RustyDB automatically detects deadlocks and aborts one transaction.

**Example Deadlock:**
```sql
-- Session 1
BEGIN;
UPDATE employees SET salary = 80000 WHERE employee_id = 101;
-- Waits for Session 2's lock on emp 102...

-- Session 2
BEGIN;
UPDATE employees SET salary = 85000 WHERE employee_id = 102;
-- Waits for Session 1's lock on emp 101...

-- DEADLOCK! One transaction will be aborted
-- ERROR: deadlock detected
```

**Prevention:**
- Always acquire locks in same order
- Keep transactions short
- Use timeout settings
- Use NOWAIT to fail fast

---

### Lock Timeout

**Set lock timeout:**
```sql
SET LOCK_TIMEOUT = 30000;  -- 30 seconds

-- Query will fail if can't acquire lock within 30s
UPDATE employees SET salary = 80000 WHERE employee_id = 101;
```

---

## Best Practices

### Transaction Design

1. **Keep transactions short**
   ```sql
   -- Good: Short transaction
   BEGIN;
   UPDATE inventory SET quantity = quantity - 1 WHERE product_id = 101;
   COMMIT;

   -- Avoid: Long transaction
   BEGIN;
   -- ... lots of processing ...
   -- ... user interaction ...
   COMMIT;
   ```

2. **Commit or rollback promptly**
   ```sql
   -- Don't leave transactions hanging
   BEGIN;
   UPDATE employees SET active = true;
   -- Don't forget COMMIT or ROLLBACK!
   ```

3. **Use appropriate isolation level**
   ```sql
   -- Default READ COMMITTED for most operations
   BEGIN;
   -- queries...
   COMMIT;

   -- SERIALIZABLE only when necessary
   BEGIN;
   SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
   -- critical operations...
   COMMIT;
   ```

---

### Error Handling

**Always handle errors:**
```sql
BEGIN
    BEGIN;

    UPDATE accounts SET balance = balance - 100 WHERE account_id = 1;
    UPDATE accounts SET balance = balance + 100 WHERE account_id = 2;

    COMMIT;

EXCEPTION
    WHEN OTHERS THEN
        ROLLBACK;
        RAISE;
END;
```

**Use savepoints for partial rollback:**
```sql
BEGIN;

-- Critical operation
UPDATE critical_data SET value = new_value;
SAVEPOINT critical_done;

-- Non-critical operation
BEGIN
    UPDATE non_critical_data SET value = new_value;
EXCEPTION
    WHEN OTHERS THEN
        -- Rollback only non-critical part
        ROLLBACK TO critical_done;
END;

COMMIT;
```

---

### Performance Optimization

1. **Batch operations**
   ```sql
   -- Instead of many small transactions
   FOR i IN 1..1000 LOOP
       BEGIN;
       INSERT INTO logs VALUES (i, 'msg');
       COMMIT;
   END LOOP;

   -- Use single transaction
   BEGIN;
   FOR i IN 1..1000 LOOP
       INSERT INTO logs VALUES (i, 'msg');
   END LOOP;
   COMMIT;
   ```

2. **Use read-only transactions for reports**
   ```sql
   SET TRANSACTION READ ONLY;
   SELECT * FROM large_table;
   ```

3. **Avoid long-running transactions**
   - Block other transactions
   - Hold locks longer
   - Prevent MVCC cleanup
   - Increase deadlock risk

---

### Monitoring Transactions

**View active transactions:**
```sql
SELECT * FROM v$transaction;
```

**View locks:**
```sql
SELECT * FROM v$lock;
```

**View waiting transactions:**
```sql
SELECT * FROM v$lock_waits;
```

---

## Transaction Examples

### Bank Transfer
```sql
BEGIN;

-- Debit from account 1
UPDATE accounts SET balance = balance - 100
WHERE account_id = 1 AND balance >= 100;

IF SQL%ROWCOUNT = 0 THEN
    ROLLBACK;
    RAISE_APPLICATION_ERROR(-20001, 'Insufficient funds');
END IF;

-- Credit to account 2
UPDATE accounts SET balance = balance + 100
WHERE account_id = 2;

-- Log transaction
INSERT INTO transaction_log VALUES (txn_id(), 1, 2, 100, CURRENT_TIMESTAMP);

COMMIT;
```

---

### Inventory Management
```sql
BEGIN;
SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;

-- Check inventory
SELECT quantity INTO v_qty FROM inventory
WHERE product_id = 101
FOR UPDATE;

IF v_qty < 10 THEN
    ROLLBACK;
    RAISE_APPLICATION_ERROR(-20002, 'Insufficient inventory');
END IF;

-- Reduce inventory
UPDATE inventory SET quantity = quantity - 10
WHERE product_id = 101;

-- Create order
INSERT INTO orders VALUES (order_id, customer_id, CURRENT_TIMESTAMP);
INSERT INTO order_items VALUES (order_id, 101, 10, price);

COMMIT;
```

---

### Batch Processing with Savepoints
```sql
BEGIN;

FOR rec IN (SELECT * FROM import_data) LOOP
    SAVEPOINT before_record;

    BEGIN
        -- Process record
        INSERT INTO target_table VALUES (rec.id, rec.data);

    EXCEPTION
        WHEN OTHERS THEN
            -- Rollback just this record
            ROLLBACK TO before_record;

            -- Log error
            INSERT INTO error_log VALUES (rec.id, SQLERRM);
    END;
END LOOP;

COMMIT;
```

---

## Quick Reference

### Transaction Control
```sql
BEGIN;                                    -- Start transaction
START TRANSACTION;                        -- Start transaction (alternative)
COMMIT;                                   -- Commit changes
ROLLBACK;                                 -- Rollback changes
SAVEPOINT name;                           -- Create savepoint
ROLLBACK TO name;                         -- Rollback to savepoint
RELEASE SAVEPOINT name;                   -- Release savepoint
```

### Isolation Levels
```sql
SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED;
SET TRANSACTION ISOLATION LEVEL READ COMMITTED;
SET TRANSACTION ISOLATION LEVEL REPEATABLE READ;
SET TRANSACTION ISOLATION LEVEL SERIALIZABLE;
SET TRANSACTION ISOLATION LEVEL SNAPSHOT;
```

### Locking
```sql
SELECT ... FOR UPDATE;                    -- Exclusive lock
SELECT ... FOR UPDATE NOWAIT;             -- Fail if locked
SET LOCK_TIMEOUT = milliseconds;          -- Lock timeout
```

---

**RustyDB v0.6.5** | Transaction Control Reference | **✅ Validated for Enterprise Deployment** | **✅ MVCC with ACID Guarantees**
