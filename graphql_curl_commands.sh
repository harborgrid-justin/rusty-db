#!/bin/bash
# RustyDB GraphQL API - Curl Command Reference
# Usage: ./graphql_curl_commands.sh [test_name]
# Example: ./graphql_curl_commands.sh schema_introspection

GRAPHQL_ENDPOINT="http://localhost:8080/graphql"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper function
execute_query() {
    local name=$1
    local query=$2
    echo -e "${BLUE}==== $name ====${NC}"
    curl -s -X POST "$GRAPHQL_ENDPOINT" \
        -H "Content-Type: application/json" \
        -d "$query" | jq .
    echo ""
}

# Schema Introspection Tests
schema_introspection() {
    execute_query "Get Schema Types" \
        '{"query":"{ __schema { queryType { name } mutationType { name } subscriptionType { name } } }"}'

    execute_query "Get All Types" \
        '{"query":"{ __schema { types { name kind } } }"}'

    execute_query "Get All Queries" \
        '{"query":"{ __type(name: \"QueryRoot\") { fields { name args { name type { name } } } } }"}'

    execute_query "Get All Mutations" \
        '{"query":"{ __type(name: \"MutationRoot\") { fields { name args { name type { name } } } } }"}'

    execute_query "Get All Subscriptions" \
        '{"query":"{ __type(name: \"SubscriptionRoot\") { fields { name args { name type { name } } } } }"}'
}

# Query Tests
query_tests() {
    execute_query "List Schemas" \
        '{"query":"{ schemas { name } }"}'

    execute_query "Get Schema Details" \
        '{"query":"{ schema(name: \"public\") { name tables { name } } }"}'

    execute_query "List All Tables" \
        '{"query":"{ tables { name rowCount columns { name dataType nullable } } }"}'

    execute_query "Get Table Details" \
        '{"query":"{ table(name: \"users\") { name rowCount columns { name dataType nullable primaryKey } indexes { name columns unique } } }"}'

    execute_query "Count Rows" \
        '{"query":"{ count(table: \"users\") }"}'
}

# Query with Filters
query_filters() {
    execute_query "Query with Simple Filter" \
        '{"query":"{ queryTable(table: \"users\", whereClause: { condition: { field: \"age\", operator: GT, value: \"25\" } }, limit: 10) { __typename ... on QuerySuccess { rows { id fields } totalCount } ... on QueryError { message code } } }"}'

    execute_query "Query with AND Filter" \
        '{"query":"{ queryTable(table: \"users\", whereClause: { and: [{ condition: { field: \"age\", operator: GE, value: \"18\" } }, { condition: { field: \"active\", operator: EQ, value: \"true\" } }] }, orderBy: [{ field: \"name\", order: ASC }], limit: 20) { __typename ... on QuerySuccess { rows { id fields } totalCount } ... on QueryError { message code } } }"}'

    execute_query "Query with OR Filter" \
        '{"query":"{ queryTable(table: \"users\", whereClause: { or: [{ condition: { field: \"role\", operator: EQ, value: \"admin\" } }, { condition: { field: \"role\", operator: EQ, value: \"moderator\" } }] }, limit: 50) { __typename ... on QuerySuccess { rows { id fields } totalCount } ... on QueryError { message code } } }"}'
}

# Aggregation Tests
aggregation_tests() {
    execute_query "Count Aggregation" \
        '{"query":"{ aggregate(table: \"orders\", aggregates: [{ func: COUNT, field: \"id\", alias: \"total\" }]) { results { aggregates } totalCount executionTimeMs } }"}'

    execute_query "Multiple Aggregations" \
        '{"query":"{ aggregate(table: \"orders\", aggregates: [{ func: COUNT, field: \"id\" }, { func: SUM, field: \"amount\" }, { func: AVG, field: \"amount\" }, { func: MIN, field: \"amount\" }, { func: MAX, field: \"amount\" }]) { results { aggregates } executionTimeMs } }"}'

    execute_query "Group By Aggregation" \
        '{"query":"{ aggregate(table: \"orders\", aggregates: [{ func: COUNT, field: \"id\" }, { func: SUM, field: \"amount\" }], groupBy: [\"customer_id\"]) { results { groupValues aggregates } totalCount } }"}'
}

# Search and Explain
search_tests() {
    execute_query "Full-Text Search" \
        '{"query":"{ search(query: \"john\", tables: [\"users\", \"customers\"], limit: 10) { results { table rowId score matchedFields } totalCount executionTimeMs } }"}'

    execute_query "Explain Query Plan" \
        '{"query":"{ explain(table: \"users\", whereClause: { condition: { field: \"email\", operator: LIKE, value: \"%@example.com\" } }) { planText estimatedCost estimatedRows } }"}'
}

# Mutation Tests
mutation_tests() {
    execute_query "Insert One Row" \
        '{"query":"mutation { insertOne(table: \"users\", data: { id: 100, name: \"Test User\", email: \"test@example.com\", age: 25 }) { __typename ... on MutationSuccess { affectedRows executionTimeMs } ... on MutationError { message code } } }"}'

    execute_query "Insert Many Rows" \
        '{"query":"mutation { insertMany(table: \"users\", data: [{ id: 101, name: \"Alice\" }, { id: 102, name: \"Bob\" }, { id: 103, name: \"Charlie\" }]) { __typename ... on MutationSuccess { affectedRows executionTimeMs } ... on MutationError { message code } } }"}'

    execute_query "Update One Row" \
        '{"query":"mutation { updateOne(table: \"users\", id: \"100\", data: { name: \"Updated Name\", email: \"updated@example.com\" }) { __typename ... on MutationSuccess { affectedRows executionTimeMs } ... on MutationError { message code } } }"}'

    execute_query "Update Many Rows" \
        '{"query":"mutation { updateMany(table: \"users\", whereClause: { condition: { field: \"active\", operator: EQ, value: \"false\" } }, data: { status: \"archived\" }) { __typename ... on MutationSuccess { affectedRows executionTimeMs } ... on MutationError { message code } } }"}'

    execute_query "Delete One Row" \
        '{"query":"mutation { deleteOne(table: \"users\", id: \"100\") { __typename ... on MutationSuccess { affectedRows executionTimeMs } ... on MutationError { message code } } }"}'

    execute_query "Delete Many Rows" \
        '{"query":"mutation { deleteMany(table: \"users\", whereClause: { condition: { field: \"last_login\", operator: LT, value: \"2024-01-01\" } }) { __typename ... on MutationSuccess { affectedRows executionTimeMs } ... on MutationError { message code } } }"}'
}

# Transaction Tests
transaction_tests() {
    execute_query "Begin Transaction" \
        '{"query":"mutation { beginTransaction(isolationLevel: READ_COMMITTED) { transactionId status timestamp } }"}'

    execute_query "Commit Transaction" \
        '{"query":"mutation { commitTransaction(transactionId: \"txn_123\") { transactionId status timestamp } }"}'

    execute_query "Rollback Transaction" \
        '{"query":"mutation { rollbackTransaction(transactionId: \"txn_123\") { transactionId status timestamp } }"}'

    execute_query "Execute Transaction (Atomic)" \
        '{"query":"mutation { executeTransaction(isolationLevel: SERIALIZABLE, operations: [{ opType: INSERT, table: \"accounts\", data: { id: 1, balance: 1000 } }, { opType: UPDATE, table: \"accounts\", whereClause: { condition: { field: \"id\", operator: EQ, value: \"2\" } }, data: { balance: 500 } }]) { transactionId status timestamp } }"}'
}

# DDL Tests (Require Authentication)
ddl_tests() {
    execute_query "Create Database (Auth Required)" \
        '{"query":"mutation { createDatabase(name: \"testdb\") { __typename ... on DdlSuccess { message affectedObjects executionTimeMs } ... on DdlError { message code details } } }"}'

    execute_query "Create Index" \
        '{"query":"mutation { createIndex(table: \"users\", indexName: \"idx_email\", columns: [\"email\"], unique: true, ifNotExists: true) { __typename ... on DdlSuccess { message affectedObjects executionTimeMs } ... on DdlError { message code } } }"}'

    execute_query "Alter Table - Add Column" \
        '{"query":"mutation { alterTableAddColumn(table: \"users\", columnDefinition: { name: \"phone\", dataType: VARCHAR, nullable: true }) { __typename ... on DdlSuccess { message affectedObjects } ... on DdlError { message code } } }"}'

    execute_query "Create View" \
        '{"query":"mutation { createView(name: \"active_users\", query: \"SELECT id, name FROM users WHERE active = true\", orReplace: true) { __typename ... on DdlSuccess { message affectedObjects } ... on DdlError { message code } } }"}'

    execute_query "Truncate Table" \
        '{"query":"mutation { truncateTable(table: \"logs\") { __typename ... on DdlSuccess { message affectedObjects } ... on DdlError { message code } } }"}'
}

# Enum Values Tests
enum_tests() {
    execute_query "Get AggregateFunc Enum" \
        '{"query":"{ __type(name: \"AggregateFunc\") { enumValues { name } } }"}'

    execute_query "Get DataType Enum" \
        '{"query":"{ __type(name: \"DataType\") { enumValues { name } } }"}'

    execute_query "Get FilterOp Enum" \
        '{"query":"{ __type(name: \"FilterOp\") { enumValues { name } } }"}'

    execute_query "Get IsolationLevel Enum" \
        '{"query":"{ __type(name: \"IsolationLevel\") { enumValues { name } } }"}'

    execute_query "Get JoinType Enum" \
        '{"query":"{ __type(name: \"JoinType\") { enumValues { name } } }"}'

    execute_query "Get SortOrder Enum" \
        '{"query":"{ __type(name: \"SortOrder\") { enumValues { name } } }"}'
}

# Type Details Tests
type_tests() {
    execute_query "QuerySuccess Type" \
        '{"query":"{ __type(name: \"QuerySuccess\") { fields { name type { name kind } } } }"}'

    execute_query "MutationSuccess Type" \
        '{"query":"{ __type(name: \"MutationSuccess\") { fields { name type { name kind } } } }"}'

    execute_query "DdlSuccess Type" \
        '{"query":"{ __type(name: \"DdlSuccess\") { fields { name type { name kind } } } }"}'

    execute_query "TransactionResult Type" \
        '{"query":"{ __type(name: \"TransactionResult\") { fields { name type { name kind } } } }"}'

    execute_query "RowType Type" \
        '{"query":"{ __type(name: \"RowType\") { fields { name type { name kind } } } }"}'

    execute_query "TableType Type" \
        '{"query":"{ __type(name: \"TableType\") { fields { name type { name kind } } } }"}'
}

# All Tests
all_tests() {
    echo -e "${GREEN}=== Running All GraphQL Tests ===${NC}\n"
    schema_introspection
    query_tests
    query_filters
    aggregation_tests
    search_tests
    mutation_tests
    transaction_tests
    ddl_tests
    enum_tests
    type_tests
    echo -e "${GREEN}=== All Tests Complete ===${NC}"
}

# Main
case "$1" in
    "schema_introspection"|"schema")
        schema_introspection
        ;;
    "query_tests"|"queries")
        query_tests
        ;;
    "query_filters"|"filters")
        query_filters
        ;;
    "aggregation_tests"|"aggregations")
        aggregation_tests
        ;;
    "search_tests"|"search")
        search_tests
        ;;
    "mutation_tests"|"mutations")
        mutation_tests
        ;;
    "transaction_tests"|"transactions")
        transaction_tests
        ;;
    "ddl_tests"|"ddl")
        ddl_tests
        ;;
    "enum_tests"|"enums")
        enum_tests
        ;;
    "type_tests"|"types")
        type_tests
        ;;
    "all"|"")
        all_tests
        ;;
    *)
        echo "Usage: $0 [test_suite]"
        echo ""
        echo "Available test suites:"
        echo "  schema_introspection (schema) - Schema introspection tests"
        echo "  query_tests (queries)          - Basic query tests"
        echo "  query_filters (filters)        - Query filtering tests"
        echo "  aggregation_tests (aggregations) - Aggregation query tests"
        echo "  search_tests (search)          - Search and explain tests"
        echo "  mutation_tests (mutations)     - Mutation tests"
        echo "  transaction_tests (transactions) - Transaction tests"
        echo "  ddl_tests (ddl)                - DDL operation tests"
        echo "  enum_tests (enums)             - Enum value tests"
        echo "  type_tests (types)             - Type structure tests"
        echo "  all                            - Run all tests (default)"
        echo ""
        exit 1
        ;;
esac
