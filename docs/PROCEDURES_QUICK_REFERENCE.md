# RustyDB Stored Procedures - Quick Reference Guide

## Overview
RustyDB provides enterprise-grade PL/SQL-compatible stored procedures with full Oracle compatibility.

---

## Procedure Syntax

### Basic Procedure
```sql
CREATE PROCEDURE procedure_name AS
BEGIN
    -- statements
END;
```

### Procedure with Parameters
```sql
CREATE PROCEDURE procedure_name(
    p_in_param IN VARCHAR2,
    p_out_param OUT NUMBER,
    p_inout_param IN OUT INTEGER
) AS
BEGIN
    -- statements
END;
```

---

## Control Structures

### IF-THEN-ELSIF-ELSE
```sql
IF condition THEN
    -- statements
ELSIF condition THEN
    -- statements
ELSE
    -- statements
END IF;
```

### LOOP
```sql
LOOP
    -- statements
    EXIT WHEN condition;
END LOOP;
```

### WHILE LOOP
```sql
WHILE condition LOOP
    -- statements
END LOOP;
```

### FOR Loop (Numeric)
```sql
FOR i IN 1..10 LOOP
    -- i is automatically declared
    -- statements
END LOOP;

-- Reverse
FOR i IN REVERSE 1..10 LOOP
    -- statements
END LOOP;
```

### FOR Loop (Cursor)
```sql
FOR record IN cursor_name LOOP
    -- record.field_name
    -- statements
END LOOP;
```

### CASE Statement
```sql
CASE
    WHEN condition1 THEN
        -- statements
    WHEN condition2 THEN
        -- statements
    ELSE
        -- statements
END CASE;

-- Simple CASE
CASE variable
    WHEN value1 THEN
        -- statements
    WHEN value2 THEN
        -- statements
    ELSE
        -- statements
END CASE;
```

---

## Variable Declarations

```sql
DECLARE
    v_integer INTEGER := 10;
    v_number NUMBER(10,2) := 3.14;
    v_varchar VARCHAR2(100) := 'hello';
    v_char CHAR(10);
    v_date DATE;
    v_timestamp TIMESTAMP;
    v_boolean BOOLEAN := TRUE;
    v_clob CLOB;
    v_blob BLOB;

    -- NOT NULL constraint
    v_required INTEGER NOT NULL := 0;

    -- Constants
    c_tax_rate CONSTANT NUMBER := 0.08;
BEGIN
    NULL;
END;
```

---

## Exception Handling

### Basic Exception Handling
```sql
BEGIN
    -- statements
EXCEPTION
    WHEN ZERO_DIVIDE THEN
        -- handle division by zero
    WHEN NO_DATA_FOUND THEN
        -- handle no rows returned
    WHEN TOO_MANY_ROWS THEN
        -- handle multiple rows returned
    WHEN VALUE_ERROR THEN
        -- handle value conversion error
    WHEN INVALID_CURSOR THEN
        -- handle cursor error
    WHEN DUP_VAL_ON_INDEX THEN
        -- handle duplicate key error
    WHEN OTHERS THEN
        -- handle all other exceptions
END;
```

### Raising Exceptions
```sql
BEGIN
    RAISE ZERO_DIVIDE;  -- Raise predefined exception

    -- Raise application error
    RAISE_APPLICATION_ERROR(-20001, 'Custom error message');
END;
```

---

## Cursors

### Explicit Cursor
```sql
DECLARE
    CURSOR emp_cursor IS
        SELECT employee_id, name, salary FROM employees WHERE dept_id = 10;

    v_emp_id INTEGER;
    v_name VARCHAR2(100);
    v_salary NUMBER;
BEGIN
    OPEN emp_cursor;

    LOOP
        FETCH emp_cursor INTO v_emp_id, v_name, v_salary;
        EXIT WHEN emp_cursor%NOTFOUND;

        -- Process row
        DBMS_OUTPUT.PUT_LINE('Employee: ' || v_name);
    END LOOP;

    CLOSE emp_cursor;
END;
```

### Cursor with Parameters
```sql
DECLARE
    CURSOR dept_cursor(p_dept_id INTEGER) IS
        SELECT * FROM employees WHERE dept_id = p_dept_id;
BEGIN
    OPEN dept_cursor(10);
    -- fetch rows
    CLOSE dept_cursor;
END;
```

### Cursor Attributes
- `%ISOPEN` - TRUE if cursor is open
- `%FOUND` - TRUE if last fetch returned a row
- `%NOTFOUND` - TRUE if last fetch didn't return a row
- `%ROWCOUNT` - Number of rows fetched so far

### REF CURSOR
```sql
DECLARE
    TYPE ref_cursor_type IS REF CURSOR;
    v_cursor ref_cursor_type;
    v_name VARCHAR2(100);
BEGIN
    OPEN v_cursor FOR 'SELECT name FROM employees';

    LOOP
        FETCH v_cursor INTO v_name;
        EXIT WHEN v_cursor%NOTFOUND;
        DBMS_OUTPUT.PUT_LINE(v_name);
    END LOOP;

    CLOSE v_cursor;
END;
```

### BULK COLLECT
```sql
DECLARE
    TYPE name_array IS TABLE OF VARCHAR2(100);
    v_names name_array;
BEGIN
    SELECT name BULK COLLECT INTO v_names FROM employees;

    FOR i IN 1..v_names.COUNT LOOP
        DBMS_OUTPUT.PUT_LINE(v_names(i));
    END LOOP;
END;
```

### FORALL (Bulk DML)
```sql
DECLARE
    TYPE id_array IS TABLE OF INTEGER;
    v_ids id_array;
BEGIN
    v_ids := id_array(1, 2, 3, 4, 5);

    FORALL i IN v_ids.FIRST..v_ids.LAST
        UPDATE employees SET bonus = 1000 WHERE emp_id = v_ids(i);

    DBMS_OUTPUT.PUT_LINE('Updated ' || SQL%ROWCOUNT || ' rows');
END;
```

---

## Built-in Packages

### DBMS_OUTPUT
```sql
BEGIN
    DBMS_OUTPUT.ENABLE(20000);                    -- Enable with buffer size
    DBMS_OUTPUT.PUT_LINE('Hello, World!');        -- Write line
    DBMS_OUTPUT.PUT('Partial ');                  -- Write without newline
    DBMS_OUTPUT.PUT('line');
    DBMS_OUTPUT.NEW_LINE();                       -- Add newline
    DBMS_OUTPUT.GET_LINE(v_line, v_status);      -- Read line
    DBMS_OUTPUT.GET_LINES(v_lines, v_numlines);  -- Read multiple lines
    DBMS_OUTPUT.DISABLE();                        -- Disable output
END;
```

### DBMS_SQL (Dynamic SQL)
```sql
DECLARE
    v_cursor INTEGER;
    v_sql VARCHAR2(1000);
    v_result INTEGER;
BEGIN
    v_cursor := DBMS_SQL.OPEN_CURSOR;

    v_sql := 'SELECT * FROM employees WHERE dept_id = :dept';
    DBMS_SQL.PARSE(v_cursor, v_sql, DBMS_SQL.NATIVE);
    DBMS_SQL.BIND_VARIABLE(v_cursor, 'dept', 10);

    v_result := DBMS_SQL.EXECUTE(v_cursor);

    DBMS_SQL.CLOSE_CURSOR(v_cursor);
END;
```

### UTL_FILE (File I/O)
```sql
DECLARE
    v_file UTL_FILE.FILE_TYPE;
    v_line VARCHAR2(1000);
BEGIN
    -- Write to file
    v_file := UTL_FILE.FOPEN('TEMP', 'output.txt', 'W');
    UTL_FILE.PUT_LINE(v_file, 'Hello, File!');
    UTL_FILE.FCLOSE(v_file);

    -- Read from file
    v_file := UTL_FILE.FOPEN('TEMP', 'input.txt', 'R');
    UTL_FILE.GET_LINE(v_file, v_line);
    UTL_FILE.FCLOSE(v_file);
END;
```

### DBMS_SCHEDULER (Job Scheduling)
```sql
BEGIN
    -- Create job
    DBMS_SCHEDULER.CREATE_JOB(
        job_name        => 'nightly_cleanup',
        job_type        => 'PLSQL_BLOCK',
        job_action      => 'BEGIN cleanup_procedure; END;',
        start_date      => SYSTIMESTAMP,
        repeat_interval => 'FREQ=DAILY; BYHOUR=2',
        enabled         => TRUE
    );

    -- Run job immediately
    DBMS_SCHEDULER.RUN_JOB('nightly_cleanup');

    -- Disable job
    DBMS_SCHEDULER.DISABLE('nightly_cleanup');

    -- Drop job
    DBMS_SCHEDULER.DROP_JOB('nightly_cleanup');
END;
```

### DBMS_LOCK
```sql
BEGIN
    -- Request lock
    v_result := DBMS_LOCK.REQUEST('my_lock', DBMS_LOCK.X_MODE, 10);

    -- Critical section
    -- ... protected code ...

    -- Release lock
    v_result := DBMS_LOCK.RELEASE('my_lock');

    -- Sleep
    DBMS_LOCK.SLEEP(5);  -- Sleep for 5 seconds
END;
```

---

## Built-in Functions

### String Functions
```sql
UPPER('hello')                  -- 'HELLO'
LOWER('WORLD')                  -- 'world'
LENGTH('test')                  -- 4
SUBSTR('hello', 2, 3)           -- 'ell'
TRIM('  hello  ')               -- 'hello'
LTRIM('  hello')                -- 'hello'
RTRIM('hello  ')                -- 'hello'
REPLACE('hello', 'l', 'r')      -- 'herro'
'hello' || ' ' || 'world'       -- 'hello world' (concatenation)
```

### Numeric Functions
```sql
ABS(-10)                        -- 10
CEIL(3.2)                       -- 4
FLOOR(3.8)                      -- 3
ROUND(3.14159, 2)               -- 3.14
TRUNC(3.14159, 2)               -- 3.14
POWER(2, 3)                     -- 8
SQRT(16)                        -- 4
MOD(10, 3)                      -- 1
SIGN(-5)                        -- -1
```

### Conversion Functions
```sql
TO_CHAR(123)                    -- '123'
TO_NUMBER('123')                -- 123
TO_DATE('2024-01-01')           -- DATE
```

### NULL Functions
```sql
NVL(null_value, default_value)                -- Return default if null
NVL2(value, if_not_null, if_null)            -- Choose based on null
COALESCE(val1, val2, val3, ...)              -- First non-null value
```

### Conditional Functions
```sql
DECODE(expr, search1, result1, search2, result2, default)

GREATEST(val1, val2, val3, ...)              -- Maximum value
LEAST(val1, val2, val3, ...)                 -- Minimum value
```

---

## Transaction Control

```sql
BEGIN
    -- DML operations
    INSERT INTO logs VALUES (1, 'message');

    -- Create savepoint
    SAVEPOINT sp1;

    UPDATE logs SET status = 'processed';

    -- Rollback to savepoint
    ROLLBACK TO sp1;

    -- Commit changes
    COMMIT;

    -- Or rollback all
    -- ROLLBACK;
END;
```

---

## DML Operations in Procedures

### INSERT
```sql
BEGIN
    INSERT INTO employees (emp_id, name, salary)
    VALUES (100, 'John Doe', 50000);
END;
```

### UPDATE
```sql
BEGIN
    UPDATE employees
    SET salary = salary * 1.10
    WHERE dept_id = 10;
END;
```

### DELETE
```sql
BEGIN
    DELETE FROM employees
    WHERE hire_date < DATE '2020-01-01';
END;
```

### SELECT INTO
```sql
DECLARE
    v_name VARCHAR2(100);
    v_salary NUMBER;
BEGIN
    SELECT name, salary
    INTO v_name, v_salary
    FROM employees
    WHERE emp_id = 100;

    DBMS_OUTPUT.PUT_LINE('Name: ' || v_name || ', Salary: ' || v_salary);
END;
```

---

## User-Defined Functions

### Scalar Function
```sql
CREATE FUNCTION calculate_tax(p_salary NUMBER) RETURN NUMBER AS
BEGIN
    RETURN p_salary * 0.20;
END;

-- Usage
DECLARE
    v_tax NUMBER;
BEGIN
    v_tax := calculate_tax(50000);
END;
```

### Table Function
```sql
CREATE FUNCTION get_high_earners RETURN employee_table AS
    v_result employee_table;
BEGIN
    SELECT * BULK COLLECT INTO v_result
    FROM employees
    WHERE salary > 100000;

    RETURN v_result;
END;

-- Usage
SELECT * FROM TABLE(get_high_earners());
```

---

## Best Practices

### 1. Always Use Exception Handling
```sql
BEGIN
    -- Your code
EXCEPTION
    WHEN OTHERS THEN
        -- Log error
        DBMS_OUTPUT.PUT_LINE('Error: ' || SQLERRM);
        RAISE;  -- Re-raise exception
END;
```

### 2. Close Cursors
```sql
BEGIN
    OPEN my_cursor;
    -- fetch rows
    CLOSE my_cursor;  -- Always close!
EXCEPTION
    WHEN OTHERS THEN
        IF my_cursor%ISOPEN THEN
            CLOSE my_cursor;
        END IF;
        RAISE;
END;
```

### 3. Use BULK Operations for Performance
```sql
-- Instead of row-by-row processing:
FORALL i IN 1..v_array.COUNT
    UPDATE table SET col = v_array(i) WHERE id = i;

-- Instead of:
FOR i IN 1..v_array.COUNT LOOP
    UPDATE table SET col = v_array(i) WHERE id = i;
END LOOP;
```

### 4. Use Meaningful Variable Names
```sql
-- Good
v_employee_name VARCHAR2(100);
v_total_salary NUMBER;

-- Bad
x VARCHAR2(100);
y NUMBER;
```

### 5. Comment Your Code
```sql
BEGIN
    -- Calculate quarterly bonus based on performance metrics
    v_bonus := v_salary * v_performance_rating * 0.05;

    -- Apply cap of $10,000
    IF v_bonus > 10000 THEN
        v_bonus := 10000;
    END IF;
END;
```

---

## Common Patterns

### Pattern 1: Safe Division
```sql
DECLARE
    v_result NUMBER;
BEGIN
    IF v_divisor != 0 THEN
        v_result := v_numerator / v_divisor;
    ELSE
        v_result := NULL;
    END IF;
EXCEPTION
    WHEN ZERO_DIVIDE THEN
        v_result := NULL;
END;
```

### Pattern 2: Processing All Rows
```sql
DECLARE
    CURSOR emp_cursor IS SELECT * FROM employees;
BEGIN
    FOR emp_rec IN emp_cursor LOOP
        -- Process emp_rec.emp_id, emp_rec.name, etc.
        process_employee(emp_rec.emp_id);
    END LOOP;
END;
```

### Pattern 3: Conditional Update
```sql
DECLARE
    v_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO v_count
    FROM employees
    WHERE emp_id = p_emp_id;

    IF v_count > 0 THEN
        UPDATE employees SET salary = p_new_salary WHERE emp_id = p_emp_id;
    ELSE
        INSERT INTO employees VALUES (p_emp_id, p_name, p_new_salary);
    END IF;
END;
```

### Pattern 4: Logging with Autonomous Transaction
```sql
CREATE PROCEDURE log_message(p_message VARCHAR2) AS
    PRAGMA AUTONOMOUS_TRANSACTION;
BEGIN
    INSERT INTO log_table VALUES (SYSDATE, p_message);
    COMMIT;  -- Independent transaction
END;
```

---

## Error Codes

| Code | Exception | Description |
|------|-----------|-------------|
| 0 | Success | Operation completed successfully |
| -20000 to -20999 | User-defined | Application-specific errors |
| -1403 | NO_DATA_FOUND | SELECT INTO returned no rows |
| -1422 | TOO_MANY_ROWS | SELECT INTO returned multiple rows |
| -1476 | ZERO_DIVIDE | Division by zero |
| -6502 | VALUE_ERROR | Type conversion or constraint error |
| -1001 | INVALID_CURSOR | Invalid cursor operation |
| -1 | DUP_VAL_ON_INDEX | Duplicate key on unique index |

---

## Performance Tips

1. **Use BULK COLLECT** for fetching multiple rows
2. **Use FORALL** for bulk DML operations
3. **Close cursors** to free resources
4. **Use bind variables** instead of literals
5. **Avoid SELECT COUNT(*)** when you only need to know if rows exist
6. **Use LIMIT clause** with BULK COLLECT to control memory usage
7. **Create indexes** on columns used in WHERE clauses
8. **Use EXECUTE IMMEDIATE** sparingly (prefer static SQL)

---

## Debugging Tips

1. Use `DBMS_OUTPUT.PUT_LINE()` to print debug messages
2. Check cursor attributes (`%FOUND`, `%NOTFOUND`, `%ROWCOUNT`)
3. Use exception handlers with `SQLERRM` to see error messages
4. Test procedures in isolation before integration
5. Use meaningful variable names for clarity
6. Add comments to complex logic
7. Break complex procedures into smaller procedures

---

## Module Information

- **Location:** `/home/user/rusty-db/src/procedures/`
- **Files:**
  - `mod.rs` - Main procedure manager
  - `parser/` - PL/SQL parser (lexer, AST, parser)
  - `runtime.rs` - Execution engine
  - `compiler.rs` - Semantic analysis
  - `builtins.rs` - Built-in packages
  - `functions.rs` - User-defined functions
  - `cursors.rs` - Cursor management
  - `triggers.rs` - Trigger support
  - `packages.rs` - Package support

---

**Version:** Latest (commit: claude/docs-review-testing-018A3aqsKMtRP6vV91JUHCEo)
**Last Updated:** 2025-12-11
