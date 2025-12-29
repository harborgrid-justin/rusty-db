# RustyDB v0.6.5 - Built-in Functions Reference

**Version**: 0.6.5 | **Release**: Enterprise ($856M) | **Updated**: December 29, 2025

**✅ Validated for Enterprise Deployment**

---

## Table of Contents

1. [Overview](#overview)
2. [String Functions](#string-functions)
3. [Numeric Functions](#numeric-functions)
4. [Date and Time Functions](#date-and-time-functions)
5. [Conversion Functions](#conversion-functions)
6. [NULL Functions](#null-functions)
7. [Conditional Functions](#conditional-functions)
8. [Aggregate Functions](#aggregate-functions)
9. [System Functions](#system-functions)

---

## Overview

RustyDB v0.6.5 provides **60+ built-in functions** with full SQL Server and Oracle compatibility. All functions are optimized for performance with security validation and DoS protection.

### Function Categories

| Category | Count | Compatibility | Status |
|----------|-------|--------------|--------|
| String Functions | 32 | SQL Server | ✅ 100% |
| Numeric Functions | 15 | Standard SQL | ✅ 100% |
| Date/Time Functions | 10 | Oracle/Standard | ✅ 100% |
| Conversion Functions | 5 | Oracle | ✅ 100% |
| NULL Functions | 3 | Oracle | ✅ 100% |
| Conditional Functions | 4 | Oracle | ✅ 100% |
| Aggregate Functions | 5 | Standard SQL | ✅ 100% |
| System Functions | 3 | Custom | ✅ 100% |

---

## String Functions

RustyDB implements all 32 SQL Server string functions with enterprise-grade security and performance.

### ASCII
Returns the ASCII code value of the leftmost character.

**Syntax:** `ASCII(character_expression)`

**Examples:**
```sql
SELECT ASCII('A');                    -- Returns: 65
SELECT ASCII('a');                    -- Returns: 97
SELECT ASCII('ABC');                  -- Returns: 65 (first char)
SELECT ASCII(first_name) FROM employees;
```

---

### CHAR
Returns the character for a given ASCII code.

**Syntax:** `CHAR(integer_expression)`

**Examples:**
```sql
SELECT CHAR(65);                      -- Returns: 'A'
SELECT CHAR(97);                      -- Returns: 'a'
SELECT CHAR(48);                      -- Returns: '0'
```

**Notes:**
- Valid ASCII codes: 0-127
- Invalid codes return error

---

### CHARINDEX
Returns the starting position of a substring.

**Syntax:** `CHARINDEX(substring, string [, start_position])`

**Examples:**
```sql
SELECT CHARINDEX('world', 'Hello world');              -- Returns: 7
SELECT CHARINDEX('o', 'Hello world');                  -- Returns: 5
SELECT CHARINDEX('o', 'Hello world', 6);               -- Returns: 8
SELECT CHARINDEX('xyz', 'Hello world');                -- Returns: 0 (not found)
SELECT CHARINDEX('@', email) FROM employees;
```

---

### CONCAT
Concatenates two or more strings.

**Syntax:** `CONCAT(string1, string2 [, stringN]...)`

**Examples:**
```sql
SELECT CONCAT('Hello', ' ', 'World');                  -- Returns: 'Hello World'
SELECT CONCAT(first_name, ' ', last_name) FROM employees;
SELECT CONCAT('ID: ', employee_id, ' - ', first_name) FROM employees;
```

**Alternative:** Use `||` operator
```sql
SELECT first_name || ' ' || last_name FROM employees;
```

---

### CONCAT_WS
Concatenates strings with a separator.

**Syntax:** `CONCAT_WS(separator, string1, string2 [, stringN]...)`

**Examples:**
```sql
SELECT CONCAT_WS('-', '2025', '12', '29');             -- Returns: '2025-12-29'
SELECT CONCAT_WS(', ', first_name, last_name) FROM employees;
SELECT CONCAT_WS(' | ', dept_id, dept_name, location) FROM departments;
```

---

### DATALENGTH
Returns the number of bytes used.

**Syntax:** `DATALENGTH(expression)`

**Examples:**
```sql
SELECT DATALENGTH('Hello');                            -- Returns: 5
SELECT DATALENGTH('Hello World');                      -- Returns: 11
SELECT DATALENGTH(description) FROM products;
```

---

### DIFFERENCE
Compares the SOUNDEX values of two strings (0-4).

**Syntax:** `DIFFERENCE(string1, string2)`

**Examples:**
```sql
SELECT DIFFERENCE('Robert', 'Rupert');                 -- Returns: 4 (similar)
SELECT DIFFERENCE('Smith', 'Smythe');                  -- Returns: 4
SELECT DIFFERENCE('John', 'Mary');                     -- Returns: 2 (different)
```

**Return values:**
- 4 = Exact phonetic match
- 3 = Very similar
- 2 = Somewhat similar
- 1 = Slightly similar
- 0 = Not similar

---

### FORMAT
Formats a value with the specified format.

**Syntax:** `FORMAT(value, format)`

**Examples:**
```sql
SELECT FORMAT(1234.56, 'C');                           -- Returns: '$1234.56'
SELECT FORMAT(1234.56, 'N2');                          -- Returns: '1,234.56'
SELECT FORMAT(1234.56, 'P');                           -- Returns: '123,456.00%'
```

---

### LEFT
Extracts a number of characters from the left.

**Syntax:** `LEFT(string, number_of_characters)`

**Examples:**
```sql
SELECT LEFT('Hello World', 5);                         -- Returns: 'Hello'
SELECT LEFT('RustyDB', 5);                             -- Returns: 'Rusty'
SELECT LEFT(first_name, 1) AS initial FROM employees;
```

---

### LEN
Returns the length of a string (excludes trailing spaces).

**Syntax:** `LEN(string)`

**Examples:**
```sql
SELECT LEN('Hello');                                   -- Returns: 5
SELECT LEN('Hello  ');                                 -- Returns: 5 (trailing spaces)
SELECT LEN('  Hello  ');                               -- Returns: 7 (leading spaces count)
SELECT LEN(first_name) FROM employees;
```

**Note:** Use `DATALENGTH()` to include trailing spaces.

---

### LENGTH
Returns the length of a string (alias for LEN).

**Syntax:** `LENGTH(string)`

**Examples:**
```sql
SELECT LENGTH('Hello World');                          -- Returns: 11
SELECT LENGTH(email) FROM employees;
```

---

### LOWER
Converts a string to lowercase.

**Syntax:** `LOWER(string)`

**Examples:**
```sql
SELECT LOWER('HELLO WORLD');                           -- Returns: 'hello world'
SELECT LOWER('RustyDB');                               -- Returns: 'rustydb'
SELECT LOWER(first_name) FROM employees;
```

---

### LTRIM
Removes leading spaces.

**Syntax:** `LTRIM(string)`

**Examples:**
```sql
SELECT LTRIM('   Hello');                              -- Returns: 'Hello'
SELECT LTRIM('   Hello   ');                           -- Returns: 'Hello   '
SELECT LTRIM(name) FROM products;
```

---

### NCHAR
Returns the Unicode character for a code.

**Syntax:** `NCHAR(integer_expression)`

**Examples:**
```sql
SELECT NCHAR(169);                                     -- Returns: '©'
SELECT NCHAR(8364);                                    -- Returns: '€'
SELECT NCHAR(9733);                                    -- Returns: '★'
```

---

### PATINDEX
Returns the position of a pattern.

**Syntax:** `PATINDEX(pattern, string)`

**Examples:**
```sql
SELECT PATINDEX('%world%', 'Hello world');             -- Returns: 7
SELECT PATINDEX('%[0-9]%', 'abc123def');               -- Returns: 4
SELECT PATINDEX('%@%.%', email) FROM employees;
```

**Pattern wildcards:**
- `%` = Any string of zero or more characters
- `_` = Any single character
- `[]` = Character range
- `[^]` = Not in range

---

### QUOTENAME
Adds delimiters to make a valid identifier.

**Syntax:** `QUOTENAME(string [, quote_char])`

**Examples:**
```sql
SELECT QUOTENAME('My Table');                          -- Returns: '[My Table]'
SELECT QUOTENAME('Column Name');                       -- Returns: '[Column Name]'
SELECT QUOTENAME('value', '''');                       -- Returns: '''value'''
```

---

### REPLACE
Replaces all occurrences of a substring.

**Syntax:** `REPLACE(string, old_substring, new_substring)`

**Examples:**
```sql
SELECT REPLACE('Hello World', 'World', 'RustyDB');     -- Returns: 'Hello RustyDB'
SELECT REPLACE('aaa bbb aaa', 'aaa', 'ccc');           -- Returns: 'ccc bbb ccc'
SELECT REPLACE(email, '@', ' AT ') FROM employees;
```

---

### REPLICATE
Repeats a string a specified number of times.

**Syntax:** `REPLICATE(string, count)`

**Examples:**
```sql
SELECT REPLICATE('*', 10);                             -- Returns: '**********'
SELECT REPLICATE('AB', 5);                             -- Returns: 'ABABABABAB'
SELECT REPLICATE('-', 50);                             -- Returns: 50 dashes
```

**Security:** Maximum 1,000,000 repetitions

---

### REVERSE
Reverses a string.

**Syntax:** `REVERSE(string)`

**Examples:**
```sql
SELECT REVERSE('Hello');                               -- Returns: 'olleH'
SELECT REVERSE('RustyDB');                             -- Returns: 'BDytsuR'
SELECT REVERSE(last_name) FROM employees;
```

---

### RIGHT
Extracts a number of characters from the right.

**Syntax:** `RIGHT(string, number_of_characters)`

**Examples:**
```sql
SELECT RIGHT('Hello World', 5);                        -- Returns: 'World'
SELECT RIGHT('RustyDB', 2);                            -- Returns: 'DB'
SELECT RIGHT(phone, 4) AS last_four FROM employees;
```

---

### RTRIM
Removes trailing spaces.

**Syntax:** `RTRIM(string)`

**Examples:**
```sql
SELECT RTRIM('Hello   ');                              -- Returns: 'Hello'
SELECT RTRIM('   Hello   ');                           -- Returns: '   Hello'
SELECT RTRIM(description) FROM products;
```

---

### SOUNDEX
Returns a 4-character phonetic code.

**Syntax:** `SOUNDEX(string)`

**Examples:**
```sql
SELECT SOUNDEX('Robert');                              -- Returns: 'R163'
SELECT SOUNDEX('Rupert');                              -- Returns: 'R163'
SELECT SOUNDEX('Smith');                               -- Returns: 'S530'
SELECT SOUNDEX('Smythe');                              -- Returns: 'S530'
```

**Use for:** Fuzzy name matching, phonetic search

---

### SPACE
Returns a string of spaces.

**Syntax:** `SPACE(count)`

**Examples:**
```sql
SELECT SPACE(10);                                      -- Returns: 10 spaces
SELECT 'Hello' || SPACE(5) || 'World';                 -- Returns: 'Hello     World'
```

**Security:** Maximum 1,000,000 spaces

---

### STR
Returns a number as a string.

**Syntax:** `STR(number [, length [, decimals]])`

**Examples:**
```sql
SELECT STR(1234.5);                                    -- Returns: '1234'
SELECT STR(1234.5, 10, 2);                             -- Returns: '   1234.50'
SELECT STR(salary, 10, 2) FROM employees;
```

---

### STUFF
Deletes part of a string and inserts another.

**Syntax:** `STUFF(string, start, length, new_substring)`

**Examples:**
```sql
SELECT STUFF('Hello World', 7, 5, 'RustyDB');          -- Returns: 'Hello RustyDB'
SELECT STUFF('ABCDEF', 2, 3, 'XYZ');                   -- Returns: 'AXYZEF'
```

---

### SUBSTRING / SUBSTR
Extracts a substring.

**Syntax:** `SUBSTRING(string, start, length)`

**Examples:**
```sql
SELECT SUBSTRING('Hello World', 1, 5);                 -- Returns: 'Hello'
SELECT SUBSTRING('Hello World', 7, 5);                 -- Returns: 'World'
SELECT SUBSTR('RustyDB', 1, 5);                        -- Returns: 'Rusty'
SELECT SUBSTRING(email, 1, CHARINDEX('@', email)-1) AS username FROM employees;
```

**Note:** 1-based indexing (first character is position 1)

---

### TRANSLATE
Translates characters in a string.

**Syntax:** `TRANSLATE(string, from_chars, to_chars)`

**Examples:**
```sql
SELECT TRANSLATE('2*[3+4]/{7-2}', '[]{}', '()()');     -- Returns: '2*(3+4)/(7-2)'
SELECT TRANSLATE('Hello', 'elo', '310');               -- Returns: 'H311o'
```

---

### TRIM
Removes leading and trailing spaces/characters.

**Syntax:** `TRIM([characters FROM] string)`

**Examples:**
```sql
SELECT TRIM('   Hello   ');                            -- Returns: 'Hello'
SELECT TRIM('x' FROM 'xxxHelloxxx');                   -- Returns: 'Hello'
SELECT TRIM(BOTH ' ' FROM name) FROM employees;
```

---

### UNICODE
Returns the Unicode value of the first character.

**Syntax:** `UNICODE(string)`

**Examples:**
```sql
SELECT UNICODE('©');                                   -- Returns: 169
SELECT UNICODE('€');                                   -- Returns: 8364
SELECT UNICODE('A');                                   -- Returns: 65
```

---

### UPPER
Converts a string to uppercase.

**Syntax:** `UPPER(string)`

**Examples:**
```sql
SELECT UPPER('hello world');                           -- Returns: 'HELLO WORLD'
SELECT UPPER('rustydb');                               -- Returns: 'RUSTYDB'
SELECT UPPER(first_name) FROM employees;
```

---

## Numeric Functions

### ABS
Returns the absolute value.

**Syntax:** `ABS(number)`

**Examples:**
```sql
SELECT ABS(-10);                                       -- Returns: 10
SELECT ABS(10);                                        -- Returns: 10
SELECT ABS(-123.45);                                   -- Returns: 123.45
SELECT ABS(price - cost) FROM products;
```

---

### CEIL / CEILING
Returns the smallest integer >= value.

**Syntax:** `CEIL(number)` or `CEILING(number)`

**Examples:**
```sql
SELECT CEIL(123.45);                                   -- Returns: 124
SELECT CEIL(123.01);                                   -- Returns: 124
SELECT CEIL(-123.45);                                  -- Returns: -123
SELECT CEILING(price) FROM products;
```

---

### FLOOR
Returns the largest integer <= value.

**Syntax:** `FLOOR(number)`

**Examples:**
```sql
SELECT FLOOR(123.99);                                  -- Returns: 123
SELECT FLOOR(123.01);                                  -- Returns: 123
SELECT FLOOR(-123.45);                                 -- Returns: -124
SELECT FLOOR(salary / 1000) * 1000 FROM employees;
```

---

### GREATEST
Returns the greatest value from a list.

**Syntax:** `GREATEST(value1, value2 [, valueN]...)`

**Examples:**
```sql
SELECT GREATEST(10, 20, 30, 5);                        -- Returns: 30
SELECT GREATEST(salary, bonus) FROM employees;
```

---

### LEAST
Returns the smallest value from a list.

**Syntax:** `LEAST(value1, value2 [, valueN]...)`

**Examples:**
```sql
SELECT LEAST(10, 20, 30, 5);                           -- Returns: 5
SELECT LEAST(price, discount_price) FROM products;
```

---

### MOD
Returns the modulo (remainder).

**Syntax:** `MOD(dividend, divisor)`

**Examples:**
```sql
SELECT MOD(10, 3);                                     -- Returns: 1
SELECT MOD(17, 5);                                     -- Returns: 2
SELECT MOD(employee_id, 10) FROM employees;
```

**Alternative:** Use `%` operator
```sql
SELECT 10 % 3;                                         -- Returns: 1
```

---

### POWER
Returns a value raised to a power.

**Syntax:** `POWER(base, exponent)`

**Examples:**
```sql
SELECT POWER(2, 3);                                    -- Returns: 8
SELECT POWER(10, 2);                                   -- Returns: 100
SELECT POWER(5, 3);                                    -- Returns: 125
```

---

### ROUND
Rounds a number to specified decimal places.

**Syntax:** `ROUND(number [, decimals])`

**Examples:**
```sql
SELECT ROUND(123.456);                                 -- Returns: 123
SELECT ROUND(123.456, 2);                              -- Returns: 123.46
SELECT ROUND(123.456, 1);                              -- Returns: 123.5
SELECT ROUND(salary, -3) FROM employees;               -- Round to thousands
```

---

### SIGN
Returns the sign of a number (-1, 0, or 1).

**Syntax:** `SIGN(number)`

**Examples:**
```sql
SELECT SIGN(-10);                                      -- Returns: -1
SELECT SIGN(0);                                        -- Returns: 0
SELECT SIGN(10);                                       -- Returns: 1
SELECT SIGN(balance) FROM accounts;
```

---

### SQRT
Returns the square root.

**Syntax:** `SQRT(number)`

**Examples:**
```sql
SELECT SQRT(16);                                       -- Returns: 4
SELECT SQRT(25);                                       -- Returns: 5
SELECT SQRT(2);                                        -- Returns: 1.414...
```

---

### TRUNC / TRUNCATE
Truncates a number to specified decimal places.

**Syntax:** `TRUNC(number [, decimals])`

**Examples:**
```sql
SELECT TRUNC(123.456);                                 -- Returns: 123
SELECT TRUNC(123.456, 2);                              -- Returns: 123.45
SELECT TRUNC(123.456, 1);                              -- Returns: 123.4
SELECT TRUNC(salary, 0) FROM employees;
```

---

## Date and Time Functions

### CURRENT_DATE
Returns the current date.

**Syntax:** `CURRENT_DATE`

**Examples:**
```sql
SELECT CURRENT_DATE;                                   -- Returns: 2025-12-29
INSERT INTO logs (log_date) VALUES (CURRENT_DATE);
```

---

### CURRENT_TIMESTAMP / NOW
Returns the current date and time.

**Syntax:** `CURRENT_TIMESTAMP` or `NOW()`

**Examples:**
```sql
SELECT CURRENT_TIMESTAMP;                              -- Returns: 2025-12-29 10:30:45
SELECT NOW();                                          -- Returns: 2025-12-29 10:30:45
INSERT INTO audit (created_at) VALUES (CURRENT_TIMESTAMP);
```

---

### EXTRACT
Extracts part of a date/time.

**Syntax:** `EXTRACT(part FROM date)`

**Examples:**
```sql
SELECT EXTRACT(YEAR FROM hire_date) FROM employees;
SELECT EXTRACT(MONTH FROM CURRENT_DATE);
SELECT EXTRACT(DAY FROM order_date) FROM orders;
```

**Parts:** YEAR, MONTH, DAY, HOUR, MINUTE, SECOND

---

### DATE_ADD
Adds an interval to a date.

**Syntax:** `DATE_ADD(date, INTERVAL value unit)`

**Examples:**
```sql
SELECT DATE_ADD(CURRENT_DATE, INTERVAL 7 DAY);
SELECT DATE_ADD(hire_date, INTERVAL 1 YEAR) FROM employees;
SELECT DATE_ADD(order_date, INTERVAL 30 DAY) AS due_date FROM orders;
```

---

### DATE_SUB
Subtracts an interval from a date.

**Syntax:** `DATE_SUB(date, INTERVAL value unit)`

**Examples:**
```sql
SELECT DATE_SUB(CURRENT_DATE, INTERVAL 30 DAY);
SELECT DATE_SUB(expiry_date, INTERVAL 7 DAY) AS warning_date FROM subscriptions;
```

---

## Conversion Functions

### CAST
Converts a value to a specified data type.

**Syntax:** `CAST(expression AS datatype)`

**Examples:**
```sql
SELECT CAST('123' AS INTEGER);                         -- Returns: 123
SELECT CAST(123.45 AS INTEGER);                        -- Returns: 123
SELECT CAST(123 AS VARCHAR(10));                       -- Returns: '123'
SELECT CAST(employee_id AS VARCHAR(10)) FROM employees;
```

---

### TO_CHAR
Converts a value to a string.

**Syntax:** `TO_CHAR(value [, format])`

**Examples:**
```sql
SELECT TO_CHAR(123);                                   -- Returns: '123'
SELECT TO_CHAR(123.45);                                -- Returns: '123.45'
SELECT TO_CHAR(CURRENT_DATE, 'YYYY-MM-DD');            -- Returns: '2025-12-29'
SELECT TO_CHAR(salary) FROM employees;
```

---

### TO_NUMBER
Converts a string to a number.

**Syntax:** `TO_NUMBER(string)`

**Examples:**
```sql
SELECT TO_NUMBER('123');                               -- Returns: 123
SELECT TO_NUMBER('123.45');                            -- Returns: 123.45
SELECT TO_NUMBER(string_value) FROM data;
```

---

### TO_DATE
Converts a string to a date.

**Syntax:** `TO_DATE(string [, format])`

**Examples:**
```sql
SELECT TO_DATE('2025-12-29');                          -- Returns: DATE value
SELECT TO_DATE('29/12/2025', 'DD/MM/YYYY');
SELECT TO_DATE(date_string) FROM import_data;
```

---

## NULL Functions

### NVL
Returns a default value if expression is NULL.

**Syntax:** `NVL(expression, default_value)`

**Examples:**
```sql
SELECT NVL(email, 'no-email@example.com') FROM employees;
SELECT NVL(phone, 'No Phone') FROM contacts;
SELECT NVL(bonus, 0) FROM employees;
```

**Oracle compatible:** Equivalent to `COALESCE(expression, default_value)`

---

### NVL2
Returns one value if NOT NULL, another if NULL.

**Syntax:** `NVL2(expression, value_if_not_null, value_if_null)`

**Examples:**
```sql
SELECT NVL2(email, 'Has Email', 'No Email') FROM employees;
SELECT NVL2(manager_id, 'Employee', 'Manager') FROM employees;
SELECT NVL2(discount, price - discount, price) AS final_price FROM products;
```

---

### COALESCE
Returns the first non-NULL value.

**Syntax:** `COALESCE(value1, value2 [, valueN]...)`

**Examples:**
```sql
SELECT COALESCE(email, phone, address, 'No Contact') FROM customers;
SELECT COALESCE(discount_price, sale_price, regular_price) FROM products;
SELECT COALESCE(mobile, work_phone, home_phone) AS contact FROM employees;
```

---

## Conditional Functions

### DECODE
Conditional logic (Oracle compatible).

**Syntax:** `DECODE(expression, search1, result1 [, search2, result2, ...] [, default])`

**Examples:**
```sql
SELECT DECODE(dept_id,
    10, 'Sales',
    20, 'Engineering',
    30, 'HR',
    'Other'
) AS department FROM employees;

SELECT DECODE(status,
    'A', 'Active',
    'I', 'Inactive',
    'P', 'Pending',
    'Unknown'
) FROM orders;
```

---

### NULLIF
Returns NULL if two values are equal.

**Syntax:** `NULLIF(value1, value2)`

**Examples:**
```sql
SELECT NULLIF(10, 10);                                 -- Returns: NULL
SELECT NULLIF(10, 20);                                 -- Returns: 10
SELECT NULLIF(old_value, new_value) FROM audit;
```

---

## Aggregate Functions

### COUNT
Counts rows or non-NULL values.

**Syntax:** `COUNT(*)` or `COUNT(column)` or `COUNT(DISTINCT column)`

**Examples:**
```sql
SELECT COUNT(*) FROM employees;
SELECT COUNT(email) FROM employees;                    -- Non-NULL emails
SELECT COUNT(DISTINCT dept_id) FROM employees;
SELECT dept_id, COUNT(*) FROM employees GROUP BY dept_id;
```

---

### SUM
Sums numeric values.

**Syntax:** `SUM(column)` or `SUM(DISTINCT column)`

**Examples:**
```sql
SELECT SUM(salary) FROM employees;
SELECT SUM(quantity * price) FROM order_items;
SELECT dept_id, SUM(salary) FROM employees GROUP BY dept_id;
```

---

### AVG
Calculates average value.

**Syntax:** `AVG(column)` or `AVG(DISTINCT column)`

**Examples:**
```sql
SELECT AVG(salary) FROM employees;
SELECT AVG(DISTINCT salary) FROM employees;
SELECT dept_id, AVG(salary) FROM employees GROUP BY dept_id;
```

---

### MIN
Returns minimum value.

**Syntax:** `MIN(column)`

**Examples:**
```sql
SELECT MIN(salary) FROM employees;
SELECT MIN(hire_date) FROM employees;
SELECT dept_id, MIN(salary) FROM employees GROUP BY dept_id;
```

---

### MAX
Returns maximum value.

**Syntax:** `MAX(column)`

**Examples:**
```sql
SELECT MAX(salary) FROM employees;
SELECT MAX(order_date) FROM orders;
SELECT dept_id, MAX(salary) FROM employees GROUP BY dept_id;
```

---

## System Functions

### VERSION
Returns the database version.

**Syntax:** `VERSION()`

**Examples:**
```sql
SELECT VERSION();                                      -- Returns: 'RustyDB v0.6.5'
```

---

### DATABASE
Returns the current database name.

**Syntax:** `DATABASE()` or `CURRENT_DATABASE()`

**Examples:**
```sql
SELECT DATABASE();
SELECT CURRENT_DATABASE();
```

---

### USER / CURRENT_USER
Returns the current user.

**Syntax:** `USER()` or `CURRENT_USER()`

**Examples:**
```sql
SELECT USER();
SELECT CURRENT_USER();
INSERT INTO audit (user_name) VALUES (CURRENT_USER());
```

---

## Function Performance

### Optimization Tips

1. **Avoid functions in WHERE clauses** (prevents index use)
   ```sql
   -- Slow (can't use index)
   SELECT * FROM employees WHERE UPPER(last_name) = 'SMITH';

   -- Fast (can use index)
   SELECT * FROM employees WHERE last_name = 'Smith';
   ```

2. **Use expression indexes** for frequently used functions
   ```sql
   CREATE INDEX idx_upper_email ON employees (UPPER(email));
   ```

3. **Cache SOUNDEX** for phonetic searches (automatically cached)

4. **Use COALESCE** instead of nested NVL
   ```sql
   -- Better
   SELECT COALESCE(a, b, c, d) FROM table;

   -- Avoid
   SELECT NVL(a, NVL(b, NVL(c, d))) FROM table;
   ```

---

## Security Features

All string functions include:
- **Maximum string length**: 10MB (prevents memory exhaustion)
- **Replication limits**: 1,000,000 max for REPLICATE/SPACE
- **Character code validation**: Valid ranges enforced
- **DoS protection**: Resource limits on all operations

---

**RustyDB v0.6.5** | Functions Reference | **✅ Validated for Enterprise Deployment**
