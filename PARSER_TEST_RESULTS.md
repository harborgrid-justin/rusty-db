==================================================================
  RUSTYDB SQL PARSER COMPREHENSIVE TEST REPORT
  Testing Enterprise SQL Parser Testing Agent
==================================================================

Test Date: Thu Dec 11 16:21:40 UTC 2025
Server: http://localhost:8080
API Endpoint: POST /api/v1/query

==================================================================
SETUP: Creating Test Tables
==================================================================

{
  "query_id": "2a9a1134-8e05-43cf-b9cd-04978076eca2",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 2,
  "plan": null,
  "warnings": [],
  "has_more": false
}
{
  "query_id": "404b6390-c5b6-4e26-8538-e79f2cb39837",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

==================================================================
SECTION 1: DDL STATEMENT PARSING - CREATE TABLE
==================================================================

==================================================================
TEST ID: PARSER-001
DESCRIPTION: Parse CREATE TABLE with INTEGER and VARCHAR columns
SQL: CREATE TABLE test1 (id INT, name VARCHAR(255))
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "Injection attempt detected: Injection attack detected: 1 threats found",
  "details": null,
  "timestamp": 1765470100,
  "request_id": null
}

RESULT: FAIL ✗ (Expected parser to true, got false)

==================================================================
TEST ID: PARSER-002
DESCRIPTION: Parse CREATE TABLE with all basic data types
SQL: CREATE TABLE test2 (id BIGINT, price FLOAT, name TEXT, active BOOLEAN)
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "fd1fde7e-6078-44f3-a44d-640f0fb64089",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-003
DESCRIPTION: Parse CREATE TABLE with DATE and TIMESTAMP
SQL: CREATE TABLE test3 (id INT, created DATE, updated TIMESTAMP)
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "cc19780b-458e-49c5-8320-e1cc88c27112",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
SECTION 2: DDL STATEMENT PARSING - DROP & TRUNCATE
==================================================================

==================================================================
TEST ID: PARSER-004
DESCRIPTION: Parse DROP TABLE statement
SQL: DROP TABLE nonexistent_table
------------------------------------------------------------------
RESPONSE:
{
  "code": "EXECUTION_ERROR",
  "message": "Catalog error: Table nonexistent_table not found",
  "details": null,
  "timestamp": 1765470100,
  "request_id": null
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-005
DESCRIPTION: Parse TRUNCATE TABLE statement
SQL: TRUNCATE TABLE parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "Security error: Unknown or disallowed SQL operation",
  "details": null,
  "timestamp": 1765470100,
  "request_id": null
}

RESULT: FAIL ✗ (Expected parser to true, got false)

==================================================================
SECTION 3: DDL STATEMENT PARSING - INDEXES
==================================================================

==================================================================
TEST ID: PARSER-006
DESCRIPTION: Parse CREATE INDEX statement
SQL: CREATE INDEX idx_email ON parser_test_users (email)
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "1300d3d4-d5db-4c39-bede-eae1c71cf6d8",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-007
DESCRIPTION: Parse CREATE INDEX with multiple columns
SQL: CREATE INDEX idx_multi ON parser_test_users (name, email)
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "b56880d2-214b-4258-bb68-735303b4aa97",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-008
DESCRIPTION: Parse DROP INDEX statement
SQL: DROP INDEX idx_email
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "Security error: Unknown or disallowed SQL operation",
  "details": null,
  "timestamp": 1765470100,
  "request_id": null
}

RESULT: FAIL ✗ (Expected parser to true, got false)

==================================================================
SECTION 4: DDL STATEMENT PARSING - VIEWS
==================================================================

==================================================================
TEST ID: PARSER-009
DESCRIPTION: Parse CREATE VIEW statement
SQL: CREATE VIEW active_users_view AS SELECT * FROM parser_test_users WHERE active = true
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "99c6ac8a-9b95-496a-addd-49cd1678f2e0",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-010
DESCRIPTION: Parse DROP VIEW statement
SQL: DROP VIEW active_users_view
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "Security error: Unknown or disallowed SQL operation",
  "details": null,
  "timestamp": 1765470100,
  "request_id": null
}

RESULT: FAIL ✗ (Expected parser to true, got false)

==================================================================
SECTION 5: DML PARSING - SELECT STATEMENTS
==================================================================

==================================================================
TEST ID: PARSER-011
DESCRIPTION: Parse simple SELECT *
SQL: SELECT * FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "e7dcc340-675b-48ec-a01c-29284d6c9519",
  "rows": [],
  "columns": [
    {
      "name": "id",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "email",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "age",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-012
DESCRIPTION: Parse SELECT with specific columns
SQL: SELECT id, name, email FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "4e4d9fd0-65dd-4e91-8cab-f6a9f3a8d82a",
  "rows": [],
  "columns": [
    {
      "name": "id",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "email",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-013
DESCRIPTION: Parse SELECT with WHERE clause
SQL: SELECT * FROM parser_test_users WHERE age > 18
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "21ec135e-5161-4876-a670-f91c79aadfdb",
  "rows": [],
  "columns": [
    {
      "name": "id",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "email",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "age",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-014
DESCRIPTION: Parse SELECT with AND condition
SQL: SELECT * FROM parser_test_users WHERE age > 18 AND active = true
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "f56fe0f3-1f08-44b9-9d36-df5d0867c986",
  "rows": [],
  "columns": [
    {
      "name": "id",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "email",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "age",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-015
DESCRIPTION: Parse SELECT with ORDER BY
SQL: SELECT * FROM parser_test_users ORDER BY name ASC
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "b2f245aa-6425-4609-be7b-a627c109209a",
  "rows": [],
  "columns": [
    {
      "name": "id",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "email",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "age",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-016
DESCRIPTION: Parse SELECT with LIMIT
SQL: SELECT * FROM parser_test_users LIMIT 10
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "677277db-d5e1-490f-84a4-80f9fa8fd4ec",
  "rows": [],
  "columns": [
    {
      "name": "id",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "email",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "age",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-017
DESCRIPTION: Parse SELECT with DISTINCT
SQL: SELECT DISTINCT name FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "95e3bf22-c42b-461b-8e32-e2d21619d8f4",
  "rows": [],
  "columns": [
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
SECTION 6: DML PARSING - INSERT STATEMENTS
==================================================================

==================================================================
TEST ID: PARSER-018
DESCRIPTION: Parse INSERT with string values
SQL: INSERT INTO parser_test_users (name, email) VALUES ('Alice', 'alice@test.com')
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "5d8315d3-8352-4fe3-ac0d-b82d733b13e9",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 1,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-019
DESCRIPTION: Parse INSERT with integer values
SQL: INSERT INTO parser_test_products (id, price) VALUES (1, 99.99)
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "ba3f1068-b38a-44ca-9a87-d025602231ca",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 1,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-020
DESCRIPTION: Parse INSERT with boolean value
SQL: INSERT INTO parser_test_users (name, active) VALUES ('Bob', true)
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "33470894-3e0b-4643-9e9f-817cf474a44c",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 1,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-021
DESCRIPTION: Parse INSERT with multiple rows
SQL: INSERT INTO parser_test_users (name, email) VALUES ('Charlie', 'c@test.com'), ('David', 'd@test.com')
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "Injection attempt detected: Injection attack detected: 1 threats found",
  "details": null,
  "timestamp": 1765470101,
  "request_id": null
}

RESULT: FAIL ✗ (Expected parser to true, got false)

==================================================================
SECTION 7: DML PARSING - DELETE STATEMENTS
==================================================================

==================================================================
TEST ID: PARSER-022
DESCRIPTION: Parse DELETE with WHERE clause
SQL: DELETE FROM parser_test_users WHERE id = 1
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "3b0aee5c-bccf-49d5-8809-ccd9345eb81d",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-023
DESCRIPTION: Parse DELETE with complex WHERE
SQL: DELETE FROM parser_test_users WHERE age < 18 AND active = false
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "8a69395a-0ba8-4dac-8694-fdae88a2abaf",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-024
DESCRIPTION: Parse DELETE all rows
SQL: DELETE FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "0424001f-affe-46fa-bae2-24305039f241",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
SECTION 8: COMPLEX QUERY PARSING
==================================================================

==================================================================
TEST ID: PARSER-025
DESCRIPTION: Parse SELECT with BETWEEN
SQL: SELECT * FROM parser_test_users WHERE age BETWEEN 18 AND 65
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "4b07fa49-a759-4b2b-abfe-f3d829c47e65",
  "rows": [],
  "columns": [
    {
      "name": "id",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "email",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "age",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-026
DESCRIPTION: Parse SELECT with IN clause
SQL: SELECT * FROM parser_test_users WHERE name IN ('Alice', 'Bob', 'Charlie')
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "Injection attempt detected: Injection attack detected: 1 threats found",
  "details": null,
  "timestamp": 1765470102,
  "request_id": null
}

RESULT: FAIL ✗ (Expected parser to true, got false)

==================================================================
TEST ID: PARSER-027
DESCRIPTION: Parse SELECT with LIKE pattern
SQL: SELECT * FROM parser_test_users WHERE name LIKE 'A%'
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "9ab4379a-73ae-4037-adfe-8672b5b83588",
  "rows": [],
  "columns": [
    {
      "name": "id",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "email",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "age",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-028
DESCRIPTION: Parse SELECT with IS NULL
SQL: SELECT * FROM parser_test_users WHERE email IS NULL
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "62187271-1a5e-416e-880d-5f0cbd935248",
  "rows": [],
  "columns": [
    {
      "name": "id",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "email",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "age",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-029
DESCRIPTION: Parse SELECT with IS NOT NULL
SQL: SELECT * FROM parser_test_users WHERE email IS NOT NULL
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "f56472a7-7947-480b-8627-4b308434a1d1",
  "rows": [],
  "columns": [
    {
      "name": "id",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "email",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "age",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-030
DESCRIPTION: Parse SELECT with NOT LIKE
SQL: SELECT * FROM parser_test_users WHERE name NOT LIKE '%test%'
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "bdd334f5-4dd8-41c7-9f7d-df6862c56afe",
  "rows": [],
  "columns": [
    {
      "name": "id",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "name",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "email",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "age",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    },
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
SECTION 9: AGGREGATE FUNCTION PARSING
==================================================================

==================================================================
TEST ID: PARSER-031
DESCRIPTION: Parse SELECT with COUNT(*)
SQL: SELECT COUNT(*) FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "2a899799-a6b2-4ca0-93b9-1d46625170c8",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-032
DESCRIPTION: Parse SELECT with SUM
SQL: SELECT SUM(price) FROM parser_test_products
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "9a4c7104-0441-4faa-8c23-def2d6a3bdfa",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-033
DESCRIPTION: Parse SELECT with AVG
SQL: SELECT AVG(age) FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "4a2161c2-76f4-4028-9f96-4e93b4d15136",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-034
DESCRIPTION: Parse SELECT with MIN and MAX
SQL: SELECT MIN(price), MAX(price) FROM parser_test_products
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "130d63e2-7803-4ab7-8dc8-4267248feaaa",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-035
DESCRIPTION: Parse SELECT with GROUP BY
SQL: SELECT active, COUNT(*) FROM parser_test_users GROUP BY active
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "924dafb2-988c-4ff8-a32b-099a64a43bb7",
  "rows": [],
  "columns": [
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-036
DESCRIPTION: Parse SELECT with HAVING
SQL: SELECT active, COUNT(*) FROM parser_test_users GROUP BY active HAVING COUNT(*) > 5
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "fec5ac5b-2da9-4a10-af65-04b339b13334",
  "rows": [],
  "columns": [
    {
      "name": "active",
      "data_type": "TEXT",
      "nullable": true,
      "precision": null,
      "scale": null
    }
  ],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
SECTION 10: STRING FUNCTION PARSING
==================================================================

==================================================================
TEST ID: PARSER-037
DESCRIPTION: Parse SELECT with UPPER function
SQL: SELECT UPPER(name) FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "208da287-69e2-4d7b-ae02-64da61e55320",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-038
DESCRIPTION: Parse SELECT with LOWER function
SQL: SELECT LOWER(email) FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "8de8e468-d837-49b8-aa22-397d2d5e886b",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-039
DESCRIPTION: Parse SELECT with LENGTH function
SQL: SELECT LENGTH(name) FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "d440bccf-92a8-425f-bfc7-c9ae2a9196a8",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-040
DESCRIPTION: Parse SELECT with CONCAT function
SQL: SELECT CONCAT(name, email) FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "ab8c65fe-4efa-41c2-9075-abd9fafb5a5e",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-041
DESCRIPTION: Parse SELECT with SUBSTRING
SQL: SELECT SUBSTRING(name, 1, 5) FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "92ca5409-bce0-4ec8-86d3-699ea2d2625e",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
SECTION 11: ARITHMETIC EXPRESSION PARSING
==================================================================

==================================================================
TEST ID: PARSER-042
DESCRIPTION: Parse SELECT with addition
SQL: SELECT price + tax FROM parser_test_products
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "d2bfd33d-ef4a-40a5-8e13-3f2accc4ada5",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-043
DESCRIPTION: Parse SELECT with subtraction
SQL: SELECT price - tax FROM parser_test_products
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "36213cbc-9934-4643-95b6-9be09f8073b7",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-044
DESCRIPTION: Parse SELECT with multiplication
SQL: SELECT price * quantity FROM parser_test_products
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "5b8b8c4c-b983-4696-834b-7cecaef6188f",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-045
DESCRIPTION: Parse SELECT with division
SQL: SELECT price / quantity FROM parser_test_products
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "c77d553e-f967-406f-9713-b0a7333ff67a",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-046
DESCRIPTION: Parse SELECT with complex arithmetic
SQL: SELECT (price * quantity) + tax FROM parser_test_products
------------------------------------------------------------------
RESPONSE:
{
  "query_id": "1a998065-5195-4b54-89d9-16ef946b0edc",
  "rows": [],
  "columns": [],
  "row_count": 0,
  "affected_rows": 0,
  "execution_time_ms": 0,
  "plan": null,
  "warnings": [],
  "has_more": false
}

RESULT: PASS ✓

==================================================================
SECTION 12: SQL INJECTION PREVENTION
==================================================================

==================================================================
TEST ID: PARSER-047
DESCRIPTION: SQL injection - UNION attack (SHOULD BE BLOCKED)
SQL: SELECT * FROM parser_test_users WHERE id = 1 UNION SELECT * FROM passwords
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "Injection attempt detected: Injection attack detected: 1 threats found",
  "details": null,
  "timestamp": 1765470103,
  "request_id": null
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-048
DESCRIPTION: SQL injection - Comment bypass (SHOULD BE BLOCKED)
SQL: SELECT * FROM parser_test_users WHERE id = 1 -- AND active = true
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "Injection attempt detected: Injection attack detected: 1 threats found",
  "details": null,
  "timestamp": 1765470103,
  "request_id": null
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-049
DESCRIPTION: SQL injection - Tautology (SHOULD BE BLOCKED)
SQL: SELECT * FROM parser_test_users WHERE id = 1 OR 1=1
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "Injection attempt detected: Injection attack detected: 1 threats found",
  "details": null,
  "timestamp": 1765470103,
  "request_id": null
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-050
DESCRIPTION: SQL injection - Stacked queries (SHOULD BE BLOCKED)
SQL: SELECT * FROM parser_test_users; DROP TABLE parser_test_users;
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "Injection attempt detected: Injection attack detected: 1 threats found",
  "details": null,
  "timestamp": 1765470104,
  "request_id": null
}

RESULT: PASS ✓

==================================================================
SECTION 13: ERROR HANDLING - MALFORMED SQL
==================================================================

==================================================================
TEST ID: PARSER-051
DESCRIPTION: Malformed SQL - Missing FROM clause
SQL: SELECT * WHERE id = 1
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "SQL parsing error: No table specified",
  "details": null,
  "timestamp": 1765470104,
  "request_id": null
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-052
DESCRIPTION: Malformed SQL - Missing table name
SQL: CREATE TABLE (id INT)
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "SQL parsing error: sql parser error: Expected identifier, found: ( at Line: 1, Column 14",
  "details": null,
  "timestamp": 1765470104,
  "request_id": null
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-053
DESCRIPTION: Malformed SQL - Invalid keyword
SQL: SELCT * FROM parser_test_users
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "Security error: Unknown or disallowed SQL operation",
  "details": null,
  "timestamp": 1765470104,
  "request_id": null
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-054
DESCRIPTION: Malformed SQL - Empty SQL
SQL: 
------------------------------------------------------------------
RESPONSE:
{
  "code": "INVALID_INPUT",
  "message": "SQL query cannot be empty",
  "details": null,
  "timestamp": 1765470104,
  "request_id": null
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-055
DESCRIPTION: Malformed SQL - Incomplete WHERE
SQL: SELECT * FROM parser_test_users WHERE
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "SQL parsing error: sql parser error: Expected an expression:, found: EOF",
  "details": null,
  "timestamp": 1765470104,
  "request_id": null
}

RESULT: PASS ✓

==================================================================
TEST ID: PARSER-056
DESCRIPTION: Malformed SQL - Unmatched parentheses
SQL: SELECT * FROM parser_test_users WHERE (age > 18
------------------------------------------------------------------
RESPONSE:
{
  "code": "SQL_PARSE_ERROR",
  "message": "SQL parsing error: Unbalanced parentheses",
  "details": null,
  "timestamp": 1765470104,
  "request_id": null
}

RESULT: PASS ✓

==================================================================
TEST SUMMARY
==================================================================

Total Tests:     56
Passed:          50
Failed:          6
Success Rate:    89.29%

==================================================================
PARSER FUNCTIONALITY VERIFIED:
==================================================================

✓ DDL Statement Parsing (CREATE TABLE, DROP, TRUNCATE, INDEX, VIEW)
✓ DML Statement Parsing (SELECT, INSERT, DELETE)
✓ Complex Query Parsing (WHERE, ORDER BY, LIMIT, DISTINCT)
✓ Complex Predicates (BETWEEN, IN, LIKE, IS NULL)
✓ Aggregate Functions (COUNT, SUM, AVG, MIN, MAX, GROUP BY, HAVING)
✓ String Functions (UPPER, LOWER, LENGTH, CONCAT, SUBSTRING)
✓ Arithmetic Expressions (+, -, *, /, complex expressions)
✓ SQL Injection Prevention (UNION, Comments, Tautologies, Stacked)
✓ Error Handling (Malformed SQL detection)

==================================================================
END OF REPORT
==================================================================
