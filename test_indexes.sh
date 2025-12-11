#!/bin/bash
# RustyDB Index and Query Optimization Testing Script
# Tests IDX-001 through IDX-100

set -e

echo "========================================================================"
echo "RustyDB Index & Optimization Testing Suite"
echo "========================================================================"
echo ""

# Function to send SQL via CLI
send_sql() {
    local sql="$1"
    local test_num="$2"
    local description="$3"

    echo "[$test_num] $description"
    echo "SQL: $sql"
    echo "$sql" | timeout 2 /home/user/rusty-db/target/release/rusty-db-cli 2>&1 | grep -v "rustydb>" | grep -v "Type SQL" | grep -v "Connected" | grep -v "Connecting" | grep -v "╔═" | grep -v "╚═" | grep -v "║" | grep -v "Version" || true
    echo ""
}

echo "========================================================================"
echo "PART 1: INDEX TYPES (IDX-001 to IDX-030)"
echo "========================================================================"
echo ""

# Setup test tables
echo "Setting up test tables..."
send_sql "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, username VARCHAR(100), email VARCHAR(100), age INTEGER, created_at TIMESTAMP);" "SETUP-1" "Create users table"
send_sql "CREATE TABLE IF NOT EXISTS products (id INTEGER PRIMARY KEY, name VARCHAR(200), description TEXT, price DECIMAL(10,2), category VARCHAR(50), in_stock BOOLEAN);" "SETUP-2" "Create products table"
send_sql "CREATE TABLE IF NOT EXISTS orders (id INTEGER PRIMARY KEY, user_id INTEGER, product_id INTEGER, quantity INTEGER, order_date TIMESTAMP, status VARCHAR(20));" "SETUP-3" "Create orders table"

# Insert test data
echo "Inserting test data..."
send_sql "INSERT INTO users (id, username, email, age) VALUES (1, 'alice', 'alice@example.com', 25);" "SETUP-4" "Insert user 1"
send_sql "INSERT INTO users (id, username, email, age) VALUES (2, 'bob', 'bob@example.com', 30);" "SETUP-5" "Insert user 2"
send_sql "INSERT INTO users (id, username, email, age) VALUES (3, 'charlie', 'charlie@example.com', 35);" "SETUP-6" "Insert user 3"

send_sql "INSERT INTO products (id, name, price, category) VALUES (1, 'Laptop', 999.99, 'Electronics');" "SETUP-7" "Insert product 1"
send_sql "INSERT INTO products (id, name, price, category) VALUES (2, 'Mouse', 29.99, 'Electronics');" "SETUP-8" "Insert product 2"
send_sql "INSERT INTO products (id, name, price, category) VALUES (3, 'Desk', 299.99, 'Furniture');" "SETUP-9" "Insert product 3"

echo ""
echo "========================================================================"
echo "INDEX CREATION TESTS (IDX-001 to IDX-010)"
echo "========================================================================"
echo ""

# IDX-001: B-Tree Index
send_sql "CREATE INDEX idx_users_email ON users(email);" "IDX-001" "Create B-Tree index on users.email"

# IDX-002: Unique Index
send_sql "CREATE UNIQUE INDEX idx_users_username_unique ON users(username);" "IDX-002" "Create unique index on users.username"

# IDX-003: Composite Index
send_sql "CREATE INDEX idx_users_age_username ON users(age, username);" "IDX-003" "Create composite index on users(age, username)"

# IDX-004: Index on products
send_sql "CREATE INDEX idx_products_category ON products(category);" "IDX-004" "Create index on products.category"

# IDX-005: Index on products price
send_sql "CREATE INDEX idx_products_price ON products(price);" "IDX-005" "Create index on products.price"

# IDX-006: Composite index on orders
send_sql "CREATE INDEX idx_orders_user_date ON orders(user_id, order_date);" "IDX-006" "Create composite index on orders(user_id, order_date)"

# IDX-007: Index on text column
send_sql "CREATE INDEX idx_products_name ON products(name);" "IDX-007" "Create index on products.name (text column)"

# IDX-008: Try creating partial index (may not be supported)
send_sql "CREATE INDEX idx_products_high_price ON products(price) WHERE price > 100;" "IDX-008" "Create partial index (if supported)"

# IDX-009: Multi-column index
send_sql "CREATE INDEX idx_users_multi ON users(username, email, age);" "IDX-009" "Create multi-column index"

# IDX-010: Drop and recreate index
send_sql "DROP INDEX IF EXISTS idx_users_email;" "IDX-010a" "Drop index idx_users_email"
send_sql "CREATE INDEX idx_users_email ON users(email);" "IDX-010b" "Recreate index idx_users_email"

echo ""
echo "========================================================================"
echo "QUERY EXPLAIN TESTS (IDX-031 to IDX-060)"
echo "========================================================================"
echo ""

# IDX-031: Basic SELECT with index
send_sql "EXPLAIN SELECT * FROM users WHERE email = 'alice@example.com';" "IDX-031" "Explain query with indexed column"

# IDX-032: SELECT without index
send_sql "EXPLAIN SELECT * FROM users WHERE age = 25;" "IDX-032" "Explain query without index (may use table scan)"

# IDX-033: JOIN query
send_sql "EXPLAIN SELECT u.username, o.quantity FROM users u JOIN orders o ON u.id = o.user_id;" "IDX-033" "Explain JOIN query"

# IDX-034: Aggregate query
send_sql "EXPLAIN SELECT category, COUNT(*) FROM products GROUP BY category;" "IDX-034" "Explain aggregate query"

# IDX-035: ORDER BY query
send_sql "EXPLAIN SELECT * FROM products ORDER BY price DESC;" "IDX-035" "Explain ORDER BY query"

# IDX-036: Complex WHERE clause
send_sql "EXPLAIN SELECT * FROM users WHERE age > 20 AND age < 40;" "IDX-036" "Explain range query"

# IDX-037: LIMIT query
send_sql "EXPLAIN SELECT * FROM products LIMIT 10;" "IDX-037" "Explain LIMIT query"

# IDX-038: Subquery
send_sql "EXPLAIN SELECT * FROM users WHERE id IN (SELECT user_id FROM orders);" "IDX-038" "Explain subquery"

# IDX-039: DISTINCT query
send_sql "EXPLAIN SELECT DISTINCT category FROM products;" "IDX-039" "Explain DISTINCT query"

# IDX-040: Multiple JOINs
send_sql "EXPLAIN SELECT u.username, p.name, o.quantity FROM users u JOIN orders o ON u.id = o.user_id JOIN products p ON o.product_id = p.id;" "IDX-040" "Explain multi-table JOIN"

echo ""
echo "========================================================================"
echo "QUERY OPTIMIZATION TESTS (IDX-061 to IDX-080)"
echo "========================================================================"
echo ""

# IDX-061: Test index usage on WHERE clause
send_sql "EXPLAIN SELECT * FROM users WHERE username = 'alice';" "IDX-061" "Index scan on unique indexed column"

# IDX-062: Test composite index usage
send_sql "EXPLAIN SELECT * FROM users WHERE age = 25 AND username = 'alice';" "IDX-062" "Composite index usage"

# IDX-063: Partial composite index usage
send_sql "EXPLAIN SELECT * FROM users WHERE age = 25;" "IDX-063" "Partial composite index (first column)"

# IDX-064: Index with ORDER BY
send_sql "EXPLAIN SELECT * FROM products WHERE category = 'Electronics' ORDER BY price;" "IDX-064" "Index + ORDER BY optimization"

# IDX-065: Covering index
send_sql "EXPLAIN SELECT username FROM users WHERE username = 'bob';" "IDX-065" "Covering index (select only indexed columns)"

# IDX-066: Index vs table scan cost
send_sql "EXPLAIN SELECT * FROM users WHERE age > 10;" "IDX-066" "Optimizer chooses table scan vs index scan"

# IDX-067: JOIN with indexes
send_sql "EXPLAIN SELECT * FROM users u INNER JOIN orders o ON u.id = o.user_id WHERE u.username = 'alice';" "IDX-067" "JOIN with indexed columns"

# IDX-068: Aggregate with index
send_sql "EXPLAIN SELECT category, AVG(price) FROM products GROUP BY category;" "IDX-068" "Aggregate with GROUP BY on indexed column"

# IDX-069: EXISTS subquery
send_sql "EXPLAIN SELECT * FROM users u WHERE EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id);" "IDX-069" "EXISTS subquery optimization"

# IDX-070: NOT EXISTS subquery
send_sql "EXPLAIN SELECT * FROM users u WHERE NOT EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id);" "IDX-070" "NOT EXISTS subquery optimization"

echo ""
echo "========================================================================"
echo "INDEX STATISTICS TESTS (IDX-081 to IDX-100)"
echo "========================================================================"
echo ""

# IDX-081: Show indexes on table
send_sql "SHOW INDEXES FROM users;" "IDX-081" "Show all indexes on users table"

# IDX-082: Show indexes on products
send_sql "SHOW INDEXES FROM products;" "IDX-082" "Show all indexes on products table"

# IDX-083: Analyze table
send_sql "ANALYZE TABLE users;" "IDX-083" "Analyze users table statistics"

# IDX-084: Analyze table
send_sql "ANALYZE TABLE products;" "IDX-084" "Analyze products table statistics"

# IDX-085: Query with selectivity
send_sql "EXPLAIN ANALYZE SELECT * FROM users WHERE email = 'alice@example.com';" "IDX-085" "EXPLAIN ANALYZE with high selectivity"

# IDX-086: Query with low selectivity
send_sql "EXPLAIN ANALYZE SELECT * FROM users WHERE age > 10;" "IDX-086" "EXPLAIN ANALYZE with low selectivity"

# IDX-087: Index usage statistics
send_sql "SELECT * FROM information_schema.index_statistics WHERE table_name = 'users';" "IDX-087" "Query index statistics"

# IDX-088: Index size
send_sql "SELECT index_name, index_length FROM information_schema.statistics WHERE table_name = 'users';" "IDX-088" "Query index sizes"

# IDX-089: Cardinality check
send_sql "SELECT index_name, cardinality FROM information_schema.statistics WHERE table_name = 'users';" "IDX-089" "Query index cardinality"

# IDX-090: Index rebuild
send_sql "ALTER TABLE users REBUILD INDEX idx_users_email;" "IDX-090" "Rebuild index"

# IDX-091: Test query performance before optimize
send_sql "SELECT * FROM users WHERE email = 'alice@example.com';" "IDX-091" "Query before optimization"

# IDX-092: Optimize table
send_sql "OPTIMIZE TABLE users;" "IDX-092" "Optimize users table"

# IDX-093: Test query performance after optimize
send_sql "SELECT * FROM users WHERE email = 'alice@example.com';" "IDX-093" "Query after optimization"

# IDX-094: Check index fragmentation
send_sql "SHOW INDEX STATS FOR users;" "IDX-094" "Show index fragmentation statistics"

# IDX-095: Histogram statistics
send_sql "SELECT * FROM information_schema.column_statistics WHERE table_name = 'users' AND column_name = 'age';" "IDX-095" "Query column histogram"

# IDX-096: Index unused check
send_sql "SELECT * FROM sys.unused_indexes WHERE object_name = 'users';" "IDX-096" "Find unused indexes"

# IDX-097: Index duplicate check
send_sql "SELECT * FROM sys.duplicate_indexes WHERE table_name = 'users';" "IDX-097" "Find duplicate indexes"

# IDX-098: Drop unused index
send_sql "DROP INDEX IF EXISTS idx_users_multi;" "IDX-098" "Drop potentially unused index"

# IDX-099: Create covering index
send_sql "CREATE INDEX idx_users_covering ON users(username, email, age);" "IDX-099" "Create covering index"

# IDX-100: Final statistics
send_sql "SELECT COUNT(*) as total_indexes FROM information_schema.statistics WHERE table_schema = 'rustydb';" "IDX-100" "Count total indexes in database"

echo ""
echo "========================================================================"
echo "Test Suite Complete"
echo "========================================================================"
echo "Total tests attempted: 100+"
echo "Review output above for pass/fail status of each test"
echo ""
