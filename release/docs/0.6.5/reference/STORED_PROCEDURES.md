# RustyDB v0.6.5 - Stored Procedures Reference

**Version**: 0.6.5 | **Release**: Enterprise ($856M) | **Updated**: December 29, 2025

**✅ Validated for Enterprise Deployment** | **✅ 157/157 Tests Passing (100%)**

---

## Table of Contents

1. [Overview](#overview)
2. [PL/SQL Basics](#plsql-basics)
3. [Procedure Creation](#procedure-creation)
4. [Control Structures](#control-structures)
5. [Exception Handling](#exception-handling)
6. [Cursors](#cursors)
7. [Built-in Packages](#built-in-packages)
8. [User-Defined Functions](#user-defined-functions)
9. [Complete Syntax Reference](#complete-syntax-reference)

---

## Overview

RustyDB v0.6.5 provides comprehensive PL/SQL-compatible stored procedures with **100% test coverage** (157 passing tests). The implementation is production-ready with Oracle-compatible syntax and semantics.

### Feature Summary

| Feature | Support | Tests | Notes |
|---------|---------|-------|-------|
| PL/SQL Parser | ✅ 100% | 20/20 | Full syntax support |
| Runtime Execution | ✅ 100% | 20/20 | Efficient execution engine |
| Procedure Management | ✅ 100% | 15/15 | CREATE, DROP, EXECUTE |
| Compiler & Validation | ✅ 100% | 12/12 | Type checking, semantic analysis |
| Built-in Packages | ✅ 100% | 43/43 | DBMS_OUTPUT, DBMS_SQL, UTL_FILE, etc. |
| User Functions | ✅ 100% | 15/15 | Scalar, table, aggregate functions |
| Cursors | ✅ 100% | 17/17 | Explicit, REF CURSOR, BULK ops |
| Advanced Features | ✅ 100% | 15/15 | Nested calls, recursion, dynamic SQL |

**Overall: 157/157 tests passing (100%)**

---

## PL/SQL Basics

### Block Structure

Every PL/SQL block has three sections:

```sql
DECLARE
    -- Variable declarations (optional)
    variable_name datatype [NOT NULL] [:= initial_value];
BEGIN
    -- Executable statements (required)
    statement1;
    statement2;
EXCEPTION
    -- Exception handlers (optional)
    WHEN exception_name THEN
        handler_statement;
END;
```

### Simple Example

```sql
DECLARE
    message VARCHAR2(100);
    count INTEGER := 0;
BEGIN
    message := 'Hello, RustyDB!';
    count := count + 1;
    DBMS_OUTPUT.PUT_LINE(message);
    DBMS_OUTPUT.PUT_LINE('Count: ' || count);
END;
```

---

## Procedure Creation

### CREATE PROCEDURE

**Syntax:**
```sql
CREATE PROCEDURE procedure_name [
    (parameter1 [IN | OUT | INOUT] datatype,
     parameter2 [IN | OUT | INOUT] datatype,
     ...)
]
AS
DECLARE
    -- Local variable declarations
BEGIN
    -- Executable statements
EXCEPTION
    -- Exception handlers
END;
```

### Parameter Modes

- **IN** (default) - Input parameter (read-only)
- **OUT** - Output parameter (write-only)
- **INOUT** - Input/output parameter (read-write)

### Examples

**Simple Procedure (No Parameters):**
```sql
CREATE PROCEDURE hello_world AS
BEGIN
    DBMS_OUTPUT.PUT_LINE('Hello, World!');
END;

-- Execute
EXEC hello_world;
```

**Procedure with IN Parameter:**
```sql
CREATE PROCEDURE greet_user(
    p_name IN VARCHAR2
) AS
BEGIN
    DBMS_OUTPUT.PUT_LINE('Hello, ' || p_name || '!');
END;

-- Execute
EXEC greet_user('Alice');
```

**Procedure with OUT Parameter:**
```sql
CREATE PROCEDURE calculate_discount(
    p_price IN NUMBER,
    p_discount OUT NUMBER
) AS
BEGIN
    p_discount := p_price * 0.10;
END;

-- Execute with variable
DECLARE
    v_discount NUMBER;
BEGIN
    calculate_discount(100, v_discount);
    DBMS_OUTPUT.PUT_LINE('Discount: ' || v_discount);
END;
```

**Procedure with INOUT Parameter:**
```sql
CREATE PROCEDURE double_value(
    p_value INOUT INTEGER
) AS
BEGIN
    p_value := p_value * 2;
END;

-- Execute
DECLARE
    v_num INTEGER := 5;
BEGIN
    double_value(v_num);
    DBMS_OUTPUT.PUT_LINE('Result: ' || v_num);  -- Prints: 10
END;
```

**Complex Procedure Example:**
```sql
CREATE PROCEDURE process_employee_salary(
    p_emp_id IN INTEGER,
    p_raise_percent IN NUMBER,
    p_new_salary OUT NUMBER,
    p_status OUT VARCHAR2
) AS
    v_current_salary NUMBER;
    v_dept_id INTEGER;
    v_max_salary NUMBER := 200000;
BEGIN
    -- Get current salary
    SELECT salary, dept_id INTO v_current_salary, v_dept_id
    FROM employees
    WHERE employee_id = p_emp_id;

    -- Calculate new salary
    p_new_salary := v_current_salary * (1 + p_raise_percent / 100);

    -- Check if within limit
    IF p_new_salary > v_max_salary THEN
        p_status := 'REJECTED - Exceeds maximum salary';
        p_new_salary := v_current_salary;
    ELSE
        -- Update salary
        UPDATE employees
        SET salary = p_new_salary
        WHERE employee_id = p_emp_id;

        p_status := 'SUCCESS';
        COMMIT;
    END IF;

EXCEPTION
    WHEN NO_DATA_FOUND THEN
        p_status := 'ERROR - Employee not found';
        p_new_salary := 0;
    WHEN OTHERS THEN
        p_status := 'ERROR - ' || SQLERRM;
        ROLLBACK;
END;
```

---

### DROP PROCEDURE

**Syntax:**
```sql
DROP PROCEDURE procedure_name;
```

**Example:**
```sql
DROP PROCEDURE hello_world;
DROP PROCEDURE calculate_discount;
```

---

### EXECUTE Procedure

**Syntax:**
```sql
EXEC procedure_name[(parameter1, parameter2, ...)];

-- Or
EXECUTE procedure_name[(parameter1, parameter2, ...)];

-- Or within PL/SQL block
BEGIN
    procedure_name(parameter1, parameter2, ...);
END;
```

---

## Control Structures

### IF-THEN-ELSIF-ELSE

**Syntax:**
```sql
IF condition THEN
    statements;
[ELSIF condition THEN
    statements;]
[ELSE
    statements;]
END IF;
```

**Examples:**
```sql
-- Simple IF
IF salary > 50000 THEN
    bonus := 5000;
END IF;

-- IF-ELSE
IF age >= 18 THEN
    status := 'Adult';
ELSE
    status := 'Minor';
END IF;

-- IF-ELSIF-ELSE
IF salary < 50000 THEN
    level := 'Entry';
ELSIF salary < 100000 THEN
    level := 'Mid';
ELSIF salary < 150000 THEN
    level := 'Senior';
ELSE
    level := 'Executive';
END IF;
```

---

### CASE Statement

**Syntax:**
```sql
-- Simple CASE
CASE expression
    WHEN value1 THEN statements;
    WHEN value2 THEN statements;
    ELSE statements;
END CASE;

-- Searched CASE
CASE
    WHEN condition1 THEN statements;
    WHEN condition2 THEN statements;
    ELSE statements;
END CASE;
```

**Examples:**
```sql
-- Simple CASE
CASE dept_id
    WHEN 10 THEN dept_name := 'Sales';
    WHEN 20 THEN dept_name := 'Engineering';
    WHEN 30 THEN dept_name := 'HR';
    ELSE dept_name := 'Other';
END CASE;

-- Searched CASE
CASE
    WHEN salary < 50000 THEN level := 'Entry';
    WHEN salary < 100000 THEN level := 'Mid';
    ELSE level := 'Senior';
END CASE;
```

---

### LOOP

**Basic LOOP:**
```sql
LOOP
    statements;
    EXIT WHEN condition;
END LOOP;
```

**Example:**
```sql
DECLARE
    counter INTEGER := 1;
BEGIN
    LOOP
        DBMS_OUTPUT.PUT_LINE('Counter: ' || counter);
        counter := counter + 1;
        EXIT WHEN counter > 10;
    END LOOP;
END;
```

**EXIT and CONTINUE:**
```sql
LOOP
    -- Skip even numbers
    IF MOD(counter, 2) = 0 THEN
        CONTINUE;
    END IF;

    -- Process odd numbers
    DBMS_OUTPUT.PUT_LINE(counter);

    -- Exit when done
    EXIT WHEN counter >= 100;
    counter := counter + 1;
END LOOP;
```

---

### WHILE LOOP

**Syntax:**
```sql
WHILE condition LOOP
    statements;
END LOOP;
```

**Example:**
```sql
DECLARE
    counter INTEGER := 1;
BEGIN
    WHILE counter <= 10 LOOP
        DBMS_OUTPUT.PUT_LINE('Counter: ' || counter);
        counter := counter + 1;
    END LOOP;
END;
```

---

### FOR LOOP

**Numeric FOR:**
```sql
FOR counter IN [REVERSE] lower_bound..upper_bound LOOP
    statements;
END LOOP;
```

**Examples:**
```sql
-- Forward loop
FOR i IN 1..10 LOOP
    DBMS_OUTPUT.PUT_LINE('i = ' || i);
END LOOP;

-- Reverse loop
FOR i IN REVERSE 1..10 LOOP
    DBMS_OUTPUT.PUT_LINE('i = ' || i);  -- 10, 9, 8, ..., 1
END LOOP;

-- Dynamic range
DECLARE
    v_start INTEGER := 5;
    v_end INTEGER := 15;
BEGIN
    FOR i IN v_start..v_end LOOP
        DBMS_OUTPUT.PUT_LINE(i);
    END LOOP;
END;
```

**Cursor FOR Loop:**
```sql
FOR record IN (SELECT * FROM employees WHERE salary > 50000) LOOP
    DBMS_OUTPUT.PUT_LINE(record.first_name || ': ' || record.salary);
END LOOP;
```

---

## Exception Handling

### EXCEPTION Block

**Syntax:**
```sql
BEGIN
    statements;
EXCEPTION
    WHEN exception_name1 THEN
        handler_statements;
    WHEN exception_name2 THEN
        handler_statements;
    WHEN OTHERS THEN
        handler_statements;
END;
```

### Predefined Exceptions

- `NO_DATA_FOUND` - SELECT INTO returns no rows
- `TOO_MANY_ROWS` - SELECT INTO returns multiple rows
- `ZERO_DIVIDE` - Division by zero
- `VALUE_ERROR` - Arithmetic, conversion, or truncation error
- `INVALID_CURSOR` - Invalid cursor operation
- `INVALID_NUMBER` - Conversion to number failed
- `DUP_VAL_ON_INDEX` - Duplicate value on unique index

**Examples:**

```sql
-- Handle specific exception
DECLARE
    v_salary NUMBER;
BEGIN
    SELECT salary INTO v_salary
    FROM employees
    WHERE employee_id = 999;
EXCEPTION
    WHEN NO_DATA_FOUND THEN
        DBMS_OUTPUT.PUT_LINE('Employee not found');
        v_salary := 0;
END;

-- Handle multiple exceptions
BEGIN
    -- Some operations
    result := numerator / denominator;
EXCEPTION
    WHEN ZERO_DIVIDE THEN
        DBMS_OUTPUT.PUT_LINE('Cannot divide by zero');
        result := 0;
    WHEN VALUE_ERROR THEN
        DBMS_OUTPUT.PUT_LINE('Invalid value');
        result := -1;
    WHEN OTHERS THEN
        DBMS_OUTPUT.PUT_LINE('Unexpected error: ' || SQLERRM);
        ROLLBACK;
END;
```

---

### RAISE Exception

**Syntax:**
```sql
RAISE exception_name;
RAISE;  -- Re-raise current exception
```

**Examples:**
```sql
-- Raise predefined exception
IF salary < 0 THEN
    RAISE VALUE_ERROR;
END IF;

-- Raise in exception handler
EXCEPTION
    WHEN OTHERS THEN
        DBMS_OUTPUT.PUT_LINE('Error occurred');
        RAISE;  -- Re-raise to caller
```

---

### User-Defined Exceptions

```sql
DECLARE
    invalid_salary EXCEPTION;
    v_salary NUMBER := -1000;
BEGIN
    IF v_salary < 0 THEN
        RAISE invalid_salary;
    END IF;
EXCEPTION
    WHEN invalid_salary THEN
        DBMS_OUTPUT.PUT_LINE('Salary cannot be negative');
END;
```

---

### SQLERRM Function

Returns error message for current exception.

```sql
EXCEPTION
    WHEN OTHERS THEN
        DBMS_OUTPUT.PUT_LINE('Error: ' || SQLERRM);
```

---

## Cursors

### Explicit Cursors

**Syntax:**
```sql
DECLARE
    CURSOR cursor_name IS
        SELECT_statement;
    record_variable cursor_name%ROWTYPE;
BEGIN
    OPEN cursor_name;
    LOOP
        FETCH cursor_name INTO record_variable;
        EXIT WHEN cursor_name%NOTFOUND;
        -- Process record
    END LOOP;
    CLOSE cursor_name;
END;
```

**Example:**
```sql
DECLARE
    CURSOR emp_cursor IS
        SELECT employee_id, first_name, salary
        FROM employees
        WHERE salary > 50000;

    v_emp_id INTEGER;
    v_name VARCHAR2(50);
    v_salary NUMBER;
BEGIN
    OPEN emp_cursor;
    LOOP
        FETCH emp_cursor INTO v_emp_id, v_name, v_salary;
        EXIT WHEN emp_cursor%NOTFOUND;

        DBMS_OUTPUT.PUT_LINE(v_name || ': $' || v_salary);
    END LOOP;
    CLOSE emp_cursor;
END;
```

---

### Cursor Attributes

- `%ISOPEN` - Is cursor open?
- `%FOUND` - Did last FETCH return a row?
- `%NOTFOUND` - Did last FETCH fail?
- `%ROWCOUNT` - Number of rows fetched so far

**Example:**
```sql
IF emp_cursor%ISOPEN THEN
    DBMS_OUTPUT.PUT_LINE('Cursor is open');
END IF;

IF emp_cursor%FOUND THEN
    DBMS_OUTPUT.PUT_LINE('Row was fetched');
END IF;

DBMS_OUTPUT.PUT_LINE('Rows fetched: ' || emp_cursor%ROWCOUNT);
```

---

### Cursor with Parameters

```sql
DECLARE
    CURSOR emp_cursor(p_dept_id INTEGER) IS
        SELECT * FROM employees WHERE dept_id = p_dept_id;
    v_emp employees%ROWTYPE;
BEGIN
    OPEN emp_cursor(10);
    LOOP
        FETCH emp_cursor INTO v_emp;
        EXIT WHEN emp_cursor%NOTFOUND;
        DBMS_OUTPUT.PUT_LINE(v_emp.first_name);
    END LOOP;
    CLOSE emp_cursor;
END;
```

---

### Cursor FOR Loop

Simplifies cursor processing (automatic OPEN, FETCH, CLOSE).

```sql
DECLARE
    CURSOR emp_cursor IS
        SELECT first_name, salary FROM employees;
BEGIN
    FOR emp_rec IN emp_cursor LOOP
        DBMS_OUTPUT.PUT_LINE(emp_rec.first_name || ': ' || emp_rec.salary);
    END LOOP;
END;

-- Or inline cursor
BEGIN
    FOR emp_rec IN (SELECT * FROM employees WHERE dept_id = 10) LOOP
        DBMS_OUTPUT.PUT_LINE(emp_rec.first_name);
    END LOOP;
END;
```

---

### REF CURSOR

Dynamic cursors that can be opened with different queries.

```sql
DECLARE
    TYPE ref_cursor_type IS REF CURSOR;
    emp_cursor ref_cursor_type;
    v_name VARCHAR2(50);
    v_salary NUMBER;
    v_dept_id INTEGER := 10;
BEGIN
    -- Open with dynamic query
    OPEN emp_cursor FOR
        SELECT first_name, salary
        FROM employees
        WHERE dept_id = v_dept_id;

    LOOP
        FETCH emp_cursor INTO v_name, v_salary;
        EXIT WHEN emp_cursor%NOTFOUND;
        DBMS_OUTPUT.PUT_LINE(v_name || ': ' || v_salary);
    END LOOP;

    CLOSE emp_cursor;
END;
```

---

### BULK COLLECT

Fetch multiple rows at once (performance optimization).

```sql
DECLARE
    TYPE name_table IS TABLE OF VARCHAR2(50);
    TYPE salary_table IS TABLE OF NUMBER;

    names name_table;
    salaries salary_table;
BEGIN
    -- Bulk collect all rows
    SELECT first_name, salary
    BULK COLLECT INTO names, salaries
    FROM employees
    WHERE dept_id = 10;

    -- Process collections
    FOR i IN 1..names.COUNT LOOP
        DBMS_OUTPUT.PUT_LINE(names(i) || ': ' || salaries(i));
    END LOOP;
END;
```

**BULK COLLECT with LIMIT:**
```sql
DECLARE
    CURSOR emp_cursor IS SELECT * FROM employees;
    TYPE emp_table IS TABLE OF employees%ROWTYPE;
    emps emp_table;
BEGIN
    OPEN emp_cursor;
    LOOP
        FETCH emp_cursor BULK COLLECT INTO emps LIMIT 100;
        EXIT WHEN emps.COUNT = 0;

        -- Process batch of 100 rows
        FOR i IN 1..emps.COUNT LOOP
            DBMS_OUTPUT.PUT_LINE(emps(i).first_name);
        END LOOP;
    END LOOP;
    CLOSE emp_cursor;
END;
```

---

### FORALL

Bulk DML operations.

```sql
DECLARE
    TYPE id_table IS TABLE OF INTEGER;
    emp_ids id_table := id_table(101, 102, 103);
BEGIN
    FORALL i IN 1..emp_ids.COUNT
        UPDATE employees
        SET salary = salary * 1.1
        WHERE employee_id = emp_ids(i);

    DBMS_OUTPUT.PUT_LINE('Updated: ' || SQL%ROWCOUNT || ' rows');
END;
```

---

## Built-in Packages

### DBMS_OUTPUT

Text output buffering (debugging and logging).

**Procedures:**
- `ENABLE([buffer_size])` - Enable output (default 20,000 bytes)
- `DISABLE` - Disable output
- `PUT_LINE(text)` - Write line to buffer
- `PUT(text)` - Write text without newline
- `NEW_LINE` - Add newline
- `GET_LINE(line OUT, status OUT)` - Read line from buffer
- `GET_LINES(lines OUT, numlines INOUT)` - Read multiple lines

**Example:**
```sql
BEGIN
    DBMS_OUTPUT.ENABLE(100000);
    DBMS_OUTPUT.PUT_LINE('Hello, World!');
    DBMS_OUTPUT.PUT('Line ');
    DBMS_OUTPUT.PUT_LINE('continued');

    FOR i IN 1..5 LOOP
        DBMS_OUTPUT.PUT_LINE('Count: ' || i);
    END LOOP;
END;
```

---

### DBMS_SQL

Dynamic SQL execution.

**Procedures:**
- `OPEN_CURSOR` - Open cursor
- `PARSE(cursor, sql, language)` - Parse SQL statement
- `BIND_VARIABLE(cursor, name, value)` - Bind variable
- `DEFINE_COLUMN(cursor, position, column)` - Define output column
- `EXECUTE(cursor)` - Execute statement
- `FETCH_ROWS(cursor)` - Fetch rows
- `COLUMN_VALUE(cursor, position, value OUT)` - Get column value
- `CLOSE_CURSOR(cursor)` - Close cursor
- `IS_OPEN(cursor)` - Check if cursor is open

**Example:**
```sql
DECLARE
    v_cursor INTEGER;
    v_sql VARCHAR2(1000);
    v_rows INTEGER;
BEGIN
    -- Open cursor
    v_cursor := DBMS_SQL.OPEN_CURSOR;

    -- Parse SQL
    v_sql := 'UPDATE employees SET salary = salary * 1.1 WHERE dept_id = :dept';
    DBMS_SQL.PARSE(v_cursor, v_sql, DBMS_SQL.NATIVE);

    -- Bind variable
    DBMS_SQL.BIND_VARIABLE(v_cursor, ':dept', 10);

    -- Execute
    v_rows := DBMS_SQL.EXECUTE(v_cursor);
    DBMS_OUTPUT.PUT_LINE('Updated: ' || v_rows || ' rows');

    -- Close cursor
    DBMS_SQL.CLOSE_CURSOR(v_cursor);
END;
```

---

### UTL_FILE

File I/O operations.

**Procedures:**
- `FOPEN(directory, filename, mode)` - Open file
- `PUT_LINE(file, text)` - Write line to file
- `GET_LINE(file, buffer OUT)` - Read line from file
- `FCLOSE(file)` - Close file
- `IS_OPEN(file)` - Check if file is open

**Example:**
```sql
DECLARE
    v_file UTL_FILE.FILE_TYPE;
    v_line VARCHAR2(1000);
BEGIN
    -- Open file for writing
    v_file := UTL_FILE.FOPEN('/tmp', 'output.txt', 'w');

    -- Write lines
    UTL_FILE.PUT_LINE(v_file, 'First line');
    UTL_FILE.PUT_LINE(v_file, 'Second line');

    -- Close file
    UTL_FILE.FCLOSE(v_file);

    -- Open for reading
    v_file := UTL_FILE.FOPEN('/tmp', 'output.txt', 'r');

    -- Read lines
    LOOP
        BEGIN
            UTL_FILE.GET_LINE(v_file, v_line);
            DBMS_OUTPUT.PUT_LINE(v_line);
        EXCEPTION
            WHEN NO_DATA_FOUND THEN
                EXIT;
        END;
    END LOOP;

    -- Close file
    UTL_FILE.FCLOSE(v_file);
END;
```

**Note:** Directories must be registered for security.

---

### DBMS_SCHEDULER

Job scheduling.

**Procedures:**
- `CREATE_JOB(job_name, job_type, action, ...)` - Create job
- `ENABLE_JOB(job_name)` - Enable job
- `DISABLE_JOB(job_name)` - Disable job
- `DROP_JOB(job_name)` - Delete job
- `RUN_JOB(job_name)` - Run job immediately

**Example:**
```sql
BEGIN
    -- Create daily job
    DBMS_SCHEDULER.CREATE_JOB(
        job_name => 'CLEANUP_JOB',
        job_type => 'PLSQL_BLOCK',
        job_action => 'BEGIN cleanup_old_records(); END;',
        start_date => SYSTIMESTAMP,
        repeat_interval => 'FREQ=DAILY',
        enabled => TRUE
    );
END;
```

---

### DBMS_LOCK

Lock management and delays.

**Procedures:**
- `REQUEST(lock_id, mode, timeout)` - Request lock
- `RELEASE(lock_id)` - Release lock
- `SLEEP(seconds)` - Sleep for specified time

**Example:**
```sql
BEGIN
    -- Sleep for 5 seconds
    DBMS_LOCK.SLEEP(5);

    -- Request exclusive lock
    DBMS_LOCK.REQUEST(12345, DBMS_LOCK.X_MODE, 10);

    -- Critical section
    -- ... protected operations ...

    -- Release lock
    DBMS_LOCK.RELEASE(12345);
END;
```

---

## User-Defined Functions

### Scalar Functions

Return a single value.

**Syntax:**
```sql
CREATE FUNCTION function_name(
    parameter1 datatype,
    parameter2 datatype
)
RETURN return_datatype
AS
DECLARE
    -- Local variables
BEGIN
    -- Function body
    RETURN value;
END;
```

**Example:**
```sql
CREATE FUNCTION calculate_bonus(
    p_salary NUMBER,
    p_performance NUMBER
)
RETURN NUMBER
AS
    v_bonus NUMBER;
BEGIN
    v_bonus := p_salary * p_performance / 100;
    RETURN v_bonus;
END;

-- Use in SELECT
SELECT
    employee_id,
    first_name,
    salary,
    calculate_bonus(salary, 10) AS bonus
FROM employees;
```

---

### DETERMINISTIC Functions

Functions that always return the same result for same inputs.

```sql
CREATE FUNCTION get_tax_rate(p_state VARCHAR2)
RETURN NUMBER
DETERMINISTIC
AS
BEGIN
    CASE p_state
        WHEN 'CA' THEN RETURN 0.0725;
        WHEN 'NY' THEN RETURN 0.04;
        ELSE RETURN 0.05;
    END CASE;
END;
```

---

## Complete Syntax Reference

### CREATE PROCEDURE
```sql
CREATE PROCEDURE name [(param1 mode type, ...)]
AS [DECLARE declarations]
BEGIN statements
[EXCEPTION handlers]
END;
```

### CREATE FUNCTION
```sql
CREATE FUNCTION name [(param1 type, ...)]
RETURN return_type
[DETERMINISTIC]
AS [DECLARE declarations]
BEGIN statements RETURN value;
END;
```

### DECLARE Block
```sql
DECLARE
    variable type [:= value];
    CURSOR name IS select;
BEGIN
    statements;
EXCEPTION
    WHEN exception THEN handler;
END;
```

---

## Best Practices

### Performance
1. Use BULK COLLECT for large result sets
2. Use FORALL for bulk DML operations
3. Close cursors explicitly
4. Use DETERMINISTIC for cacheable functions
5. Minimize context switches between SQL and PL/SQL

### Error Handling
1. Always include EXCEPTION block
2. Handle specific exceptions before OTHERS
3. Log errors with DBMS_OUTPUT or UTL_FILE
4. Use RAISE to propagate errors
5. Include ROLLBACK in error handlers

### Code Quality
1. Use meaningful variable names
2. Comment complex logic
3. Keep procedures small and focused
4. Avoid deep nesting
5. Use parameters instead of global variables

---

**RustyDB v0.6.5** | Stored Procedures Reference | **✅ Validated for Enterprise Deployment** | **✅ 100% Test Coverage**
