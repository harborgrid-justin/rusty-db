# RustyDB v0.6.0 - Stored Procedures Quick Reference

**Version**: 0.6.0 | **Oracle PL/SQL Compatible** | **Updated**: December 28, 2025

---

## Quick Start

### Create Procedure
```sql
CREATE PROCEDURE hello_world AS
BEGIN
    DBMS_OUTPUT.PUT_LINE('Hello, World!');
END;
```

### Execute Procedure
```sql
EXECUTE hello_world;
-- or
BEGIN
    hello_world;
END;
```

---

## Basic Syntax

### Simple Procedure
```sql
CREATE PROCEDURE procedure_name AS
BEGIN
    -- PL/SQL statements
    NULL;
END;
```

### Procedure with Parameters
```sql
CREATE PROCEDURE update_salary(
    p_emp_id IN INTEGER,
    p_new_salary IN NUMBER,
    p_result OUT VARCHAR2
) AS
BEGIN
    UPDATE employees
    SET salary = p_new_salary
    WHERE employee_id = p_emp_id;

    p_result := 'Success';
END;
```

### Parameter Modes
- `IN` - Input only (default)
- `OUT` - Output only
- `IN OUT` - Both input and output

---

## Control Structures

### IF-THEN-ELSE
```sql
IF age >= 18 THEN
    status := 'Adult';
ELSIF age >= 13 THEN
    status := 'Teen';
ELSE
    status := 'Child';
END IF;
```

### LOOP
```sql
LOOP
    counter := counter + 1;
    EXIT WHEN counter > 10;
END LOOP;
```

### WHILE LOOP
```sql
WHILE counter <= 10 LOOP
    counter := counter + 1;
END LOOP;
```

### FOR LOOP (Numeric)
```sql
FOR i IN 1..10 LOOP
    DBMS_OUTPUT.PUT_LINE('Iteration: ' || i);
END LOOP;

-- Reverse
FOR i IN REVERSE 1..10 LOOP
    DBMS_OUTPUT.PUT_LINE('Countdown: ' || i);
END LOOP;
```

### FOR LOOP (Cursor)
```sql
FOR emp_rec IN (SELECT * FROM employees) LOOP
    DBMS_OUTPUT.PUT_LINE('Name: ' || emp_rec.name);
END LOOP;
```

### CASE Statement
```sql
CASE status
    WHEN 'A' THEN result := 'Active';
    WHEN 'I' THEN result := 'Inactive';
    ELSE result := 'Unknown';
END CASE;

-- Searched CASE
CASE
    WHEN age < 18 THEN category := 'Minor';
    WHEN age < 65 THEN category := 'Adult';
    ELSE category := 'Senior';
END CASE;
```

---

## Variables

### Declaration
```sql
DECLARE
    v_name VARCHAR2(100);
    v_age INTEGER := 25;
    v_salary NUMBER(10,2) := 50000.00;
    v_active BOOLEAN := TRUE;
    v_date DATE;
    v_timestamp TIMESTAMP;

    -- NOT NULL constraint
    v_id INTEGER NOT NULL := 1;

    -- Constants
    c_tax_rate CONSTANT NUMBER := 0.20;
BEGIN
    -- Use variables
    v_name := 'John Doe';
END;
```

### Supported Types
- `INTEGER` / `INT`
- `NUMBER(p,s)` / `NUMERIC`
- `VARCHAR2(n)` / `VARCHAR(n)`
- `CHAR(n)`
- `BOOLEAN`
- `DATE`
- `TIMESTAMP`
- `CLOB` / `BLOB`

---

## Exception Handling

### Basic Exception Handling
```sql
BEGIN
    -- Operations that may fail
    v_result := v_numerator / v_denominator;
EXCEPTION
    WHEN ZERO_DIVIDE THEN
        DBMS_OUTPUT.PUT_LINE('Error: Division by zero');
    WHEN NO_DATA_FOUND THEN
        DBMS_OUTPUT.PUT_LINE('Error: No data found');
    WHEN TOO_MANY_ROWS THEN
        DBMS_OUTPUT.PUT_LINE('Error: Too many rows');
    WHEN OTHERS THEN
        DBMS_OUTPUT.PUT_LINE('Error: ' || SQLERRM);
END;
```

### Predefined Exceptions
- `ZERO_DIVIDE` - Division by zero
- `NO_DATA_FOUND` - SELECT INTO returned no rows
- `TOO_MANY_ROWS` - SELECT INTO returned multiple rows
- `VALUE_ERROR` - Type conversion error
- `INVALID_CURSOR` - Invalid cursor operation
- `DUP_VAL_ON_INDEX` - Duplicate key

### Raising Exceptions
```sql
BEGIN
    IF v_age < 0 THEN
        RAISE VALUE_ERROR;
    END IF;

    -- Custom error
    IF v_balance < 0 THEN
        RAISE_APPLICATION_ERROR(-20001, 'Insufficient balance');
    END IF;
END;
```

---

## Cursors

### Explicit Cursor
```sql
DECLARE
    CURSOR emp_cursor IS
        SELECT employee_id, name, salary
        FROM employees
        WHERE department_id = 10;

    v_emp_id INTEGER;
    v_name VARCHAR2(100);
    v_salary NUMBER;
BEGIN
    OPEN emp_cursor;

    LOOP
        FETCH emp_cursor INTO v_emp_id, v_name, v_salary;
        EXIT WHEN emp_cursor%NOTFOUND;

        DBMS_OUTPUT.PUT_LINE(v_name || ': ' || v_salary);
    END LOOP;

    CLOSE emp_cursor;
END;
```

### Cursor with Parameters
```sql
DECLARE
    CURSOR dept_cursor(p_dept_id INTEGER) IS
        SELECT * FROM employees WHERE department_id = p_dept_id;
BEGIN
    FOR emp IN dept_cursor(10) LOOP
        DBMS_OUTPUT.PUT_LINE(emp.name);
    END LOOP;
END;
```

### Cursor Attributes
- `%ISOPEN` - TRUE if cursor is open
- `%FOUND` - TRUE if last fetch returned a row
- `%NOTFOUND` - TRUE if last fetch didn't return a row
- `%ROWCOUNT` - Number of rows fetched

### Cursor FOR Loop (Implicit)
```sql
BEGIN
    FOR emp IN (SELECT * FROM employees WHERE active = TRUE) LOOP
        DBMS_OUTPUT.PUT_LINE(emp.name);
    END LOOP;
END;
```

---

## Built-in Packages

### DBMS_OUTPUT
```sql
BEGIN
    DBMS_OUTPUT.ENABLE(20000);
    DBMS_OUTPUT.PUT_LINE('Message');
    DBMS_OUTPUT.PUT('Partial ');
    DBMS_OUTPUT.PUT('line');
    DBMS_OUTPUT.NEW_LINE();
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
    -- Write
    v_file := UTL_FILE.FOPEN('TEMP', 'output.txt', 'W');
    UTL_FILE.PUT_LINE(v_file, 'Hello, File!');
    UTL_FILE.FCLOSE(v_file);

    -- Read
    v_file := UTL_FILE.FOPEN('TEMP', 'input.txt', 'R');
    UTL_FILE.GET_LINE(v_file, v_line);
    UTL_FILE.FCLOSE(v_file);
END;
```

### DBMS_LOCK
```sql
DECLARE
    v_result INTEGER;
BEGIN
    -- Acquire lock
    v_result := DBMS_LOCK.REQUEST('my_lock', DBMS_LOCK.X_MODE, 10);

    -- Critical section
    -- ...

    -- Release lock
    v_result := DBMS_LOCK.RELEASE('my_lock');

    -- Sleep
    DBMS_LOCK.SLEEP(5);  -- 5 seconds
END;
```

---

## Built-in Functions

### String Functions
```sql
-- Convert case
UPPER('hello')              -- 'HELLO'
LOWER('WORLD')              -- 'world'

-- Length and substring
LENGTH('test')              -- 4
SUBSTR('hello', 2, 3)       -- 'ell'

-- Trim
TRIM('  hello  ')           -- 'hello'
LTRIM('  hello')            -- 'hello'
RTRIM('hello  ')            -- 'hello'

-- Replace
REPLACE('hello', 'l', 'r')  -- 'herro'

-- Concatenation
'hello' || ' ' || 'world'   -- 'hello world'
CONCAT('hello', 'world')    -- 'helloworld'
```

### Numeric Functions
```sql
ABS(-10)                    -- 10
CEIL(3.2)                   -- 4
FLOOR(3.8)                  -- 3
ROUND(3.14159, 2)           -- 3.14
TRUNC(3.14159, 2)           -- 3.14
POWER(2, 3)                 -- 8
SQRT(16)                    -- 4
MOD(10, 3)                  -- 1
SIGN(-5)                    -- -1
```

### Conversion Functions
```sql
TO_CHAR(123)                -- '123'
TO_NUMBER('123')            -- 123
TO_DATE('2025-12-28')       -- DATE
```

### NULL Functions
```sql
NVL(null_value, default_value)
NVL2(value, if_not_null, if_null)
COALESCE(val1, val2, val3)
```

---

## DML Operations

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
    WHERE department_id = 10;
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
    WHERE employee_id = 100;

    DBMS_OUTPUT.PUT_LINE('Name: ' || v_name);
EXCEPTION
    WHEN NO_DATA_FOUND THEN
        DBMS_OUTPUT.PUT_LINE('Employee not found');
    WHEN TOO_MANY_ROWS THEN
        DBMS_OUTPUT.PUT_LINE('Multiple employees found');
END;
```

---

## Transaction Control

```sql
BEGIN
    -- Start implicit transaction
    INSERT INTO logs VALUES (1, 'Start');

    -- Create savepoint
    SAVEPOINT sp1;

    UPDATE logs SET status = 'processing';

    -- Rollback to savepoint
    ROLLBACK TO sp1;

    -- Commit changes
    COMMIT;

    -- Or rollback all
    -- ROLLBACK;
END;
```

---

## User-Defined Functions

### Scalar Function
```sql
CREATE FUNCTION calculate_tax(p_salary NUMBER)
RETURN NUMBER AS
BEGIN
    RETURN p_salary * 0.20;
END;

-- Usage
DECLARE
    v_tax NUMBER;
BEGIN
    v_tax := calculate_tax(50000);
    DBMS_OUTPUT.PUT_LINE('Tax: ' || v_tax);
END;
```

### Table Function
```sql
CREATE TYPE employee_record AS (
    emp_id INTEGER,
    name VARCHAR2(100)
);

CREATE TYPE employee_table AS TABLE OF employee_record;

CREATE FUNCTION get_high_earners
RETURN employee_table AS
    v_result employee_table;
BEGIN
    SELECT emp_id, name
    BULK COLLECT INTO v_result
    FROM employees
    WHERE salary > 100000;

    RETURN v_result;
END;

-- Usage
SELECT * FROM TABLE(get_high_earners());
```

---

## Complete Examples

### Example 1: Update Employee Salary
```sql
CREATE PROCEDURE raise_salary(
    p_emp_id IN INTEGER,
    p_percentage IN NUMBER,
    p_result OUT VARCHAR2
) AS
    v_current_salary NUMBER;
    v_new_salary NUMBER;
BEGIN
    -- Get current salary
    SELECT salary INTO v_current_salary
    FROM employees
    WHERE employee_id = p_emp_id;

    -- Calculate new salary
    v_new_salary := v_current_salary * (1 + p_percentage / 100);

    -- Update
    UPDATE employees
    SET salary = v_new_salary
    WHERE employee_id = p_emp_id;

    p_result := 'Success: ' || v_current_salary || ' -> ' || v_new_salary;
    COMMIT;
EXCEPTION
    WHEN NO_DATA_FOUND THEN
        p_result := 'Error: Employee not found';
        ROLLBACK;
    WHEN OTHERS THEN
        p_result := 'Error: ' || SQLERRM;
        ROLLBACK;
END;
```

### Example 2: Process All Employees
```sql
CREATE PROCEDURE process_department(p_dept_id IN INTEGER) AS
    v_count INTEGER := 0;
BEGIN
    FOR emp IN (SELECT * FROM employees WHERE department_id = p_dept_id) LOOP
        -- Process each employee
        UPDATE employees
        SET last_processed = SYSDATE
        WHERE employee_id = emp.employee_id;

        v_count := v_count + 1;
    END LOOP;

    DBMS_OUTPUT.PUT_LINE('Processed ' || v_count || ' employees');
    COMMIT;
EXCEPTION
    WHEN OTHERS THEN
        DBMS_OUTPUT.PUT_LINE('Error: ' || SQLERRM);
        ROLLBACK;
END;
```

### Example 3: Safe Division
```sql
CREATE FUNCTION safe_divide(
    p_numerator NUMBER,
    p_denominator NUMBER
) RETURN NUMBER AS
BEGIN
    IF p_denominator = 0 THEN
        RETURN NULL;
    END IF;

    RETURN p_numerator / p_denominator;
EXCEPTION
    WHEN ZERO_DIVIDE THEN
        RETURN NULL;
END;
```

---

## Best Practices

### 1. Always Handle Exceptions
```sql
BEGIN
    -- Your code
EXCEPTION
    WHEN OTHERS THEN
        DBMS_OUTPUT.PUT_LINE('Error: ' || SQLERRM);
        RAISE;
END;
```

### 2. Close Cursors
```sql
BEGIN
    OPEN my_cursor;
    -- fetch rows
    CLOSE my_cursor;
EXCEPTION
    WHEN OTHERS THEN
        IF my_cursor%ISOPEN THEN
            CLOSE my_cursor;
        END IF;
        RAISE;
END;
```

### 3. Use Bulk Operations
```sql
-- Instead of row-by-row
FOR i IN 1..10000 LOOP
    UPDATE table SET col = i WHERE id = i;
END LOOP;

-- Use FORALL
FORALL i IN 1..v_array.COUNT
    UPDATE table SET col = v_array(i) WHERE id = i;
```

### 4. Use Meaningful Names
```sql
-- ✅ Good
v_employee_name VARCHAR2(100);
v_total_salary NUMBER;

-- ❌ Bad
x VARCHAR2(100);
y NUMBER;
```

### 5. Add Comments
```sql
BEGIN
    -- Calculate quarterly bonus
    v_bonus := v_salary * v_performance * 0.05;

    -- Apply cap of $10,000
    IF v_bonus > 10000 THEN
        v_bonus := 10000;
    END IF;
END;
```

---

## Common Patterns

### Pattern 1: Conditional Insert/Update
```sql
DECLARE
    v_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO v_count
    FROM employees WHERE emp_id = p_emp_id;

    IF v_count > 0 THEN
        UPDATE employees SET name = p_name WHERE emp_id = p_emp_id;
    ELSE
        INSERT INTO employees VALUES (p_emp_id, p_name);
    END IF;
END;
```

### Pattern 2: Process All Rows
```sql
DECLARE
    CURSOR emp_cursor IS SELECT * FROM employees;
BEGIN
    FOR emp_rec IN emp_cursor LOOP
        process_employee(emp_rec.emp_id);
    END LOOP;
END;
```

### Pattern 3: Logging with Autonomous Transaction
```sql
CREATE PROCEDURE log_message(p_message VARCHAR2) AS
    PRAGMA AUTONOMOUS_TRANSACTION;
BEGIN
    INSERT INTO log_table VALUES (SYSDATE, p_message);
    COMMIT;  -- Independent transaction
END;
```

---

## Debugging

### Enable Output
```sql
SET SERVEROUTPUT ON;
```

### Print Debug Messages
```sql
BEGIN
    DBMS_OUTPUT.PUT_LINE('Debug: Variable value = ' || v_value);
    DBMS_OUTPUT.PUT_LINE('Debug: Counter = ' || v_counter);
END;
```

### Check Cursor State
```sql
IF my_cursor%ISOPEN THEN
    DBMS_OUTPUT.PUT_LINE('Cursor is open');
END IF;

DBMS_OUTPUT.PUT_LINE('Rows fetched: ' || my_cursor%ROWCOUNT);
```

---

## Error Codes

| Code | Exception | Description |
|------|-----------|-------------|
| 0 | Success | Operation completed |
| -1403 | NO_DATA_FOUND | SELECT INTO returned no rows |
| -1422 | TOO_MANY_ROWS | SELECT INTO returned multiple rows |
| -1476 | ZERO_DIVIDE | Division by zero |
| -6502 | VALUE_ERROR | Type conversion error |
| -1001 | INVALID_CURSOR | Invalid cursor operation |
| -1 | DUP_VAL_ON_INDEX | Duplicate key |
| -20000 to -20999 | User-defined | Application errors |

---

## Performance Tips

1. **Use BULK COLLECT** for fetching multiple rows
2. **Use FORALL** for bulk DML
3. **Close cursors** to free resources
4. **Use bind variables** instead of literals
5. **Avoid SELECT COUNT(\*)** when checking existence
6. **Use LIMIT** with BULK COLLECT
7. **Create indexes** on WHERE columns
8. **Use static SQL** over dynamic SQL

---

## API Integration

### Execute Procedure via GraphQL
```graphql
mutation {
  executeProcedure(
    name: "update_salary"
    parameters: [
      { name: "p_emp_id", value: "100", type: INTEGER }
      { name: "p_percentage", value: "10", type: NUMBER }
    ]
  ) {
    success
    output {
      name
      value
    }
  }
}
```

### Execute via REST API
```bash
curl -X POST http://localhost:8080/api/v1/procedures/execute \
  -H "Content-Type: application/json" \
  -d '{
    "name": "update_salary",
    "parameters": {
      "p_emp_id": 100,
      "p_percentage": 10
    }
  }'
```

---

**Procedures Reference** | RustyDB v0.6.0 | Oracle PL/SQL Compatible
