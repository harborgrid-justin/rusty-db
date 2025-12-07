#!/bin/bash
# Demo script for RustyDB

echo "╔══════════════════════════════════════════════════════════╗"
echo "║          RustyDB - Enterprise Database Demo             ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Build the project
echo "Building RustyDB..."
cargo build --release --quiet
echo "✓ Build complete"
echo ""

# Show project statistics
echo "Project Statistics:"
echo "-------------------"
find src -name "*.rs" | wc -l | xargs echo "Rust source files:"
find src -name "*.rs" -exec cat {} \; | wc -l | xargs echo "Total lines of code:"
echo ""

# Run tests
echo "Running test suite..."
cargo test --quiet 2>&1 | grep "test result"
echo ""

# Show features
echo "RustyDB Features:"
echo "-----------------"
echo "✓ Page-based Storage Engine (4KB pages)"
echo "✓ Buffer Pool Manager with LRU replacement"
echo "✓ SQL Parser (CREATE, SELECT, INSERT, UPDATE, DELETE, DROP)"
echo "✓ ACID Transaction Support"
echo "✓ Two-Phase Locking (2PL)"
echo "✓ B-Tree and Hash Indexes"
echo "✓ Catalog System for Metadata"
echo "✓ Async TCP Server"
echo "✓ Interactive CLI Client"
echo ""

# Show example SQL
echo "Example SQL Queries:"
echo "-------------------"
cat <<'EOF'
-- Create a table
CREATE TABLE users (
    id INT,
    name VARCHAR(255),
    email VARCHAR(255)
);

-- Insert data
INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.com');

-- Query data
SELECT id, name, email FROM users;
SELECT * FROM users;

-- Update data
UPDATE users SET name = 'Alice Smith' WHERE id = 1;

-- Delete data
DELETE FROM users WHERE id = 1;

-- Drop table
DROP TABLE users;

-- Transaction example
BEGIN;
INSERT INTO users VALUES (1, 'Alice', 'alice@example.com');
INSERT INTO users VALUES (2, 'Bob', 'bob@example.com');
COMMIT;
EOF

echo ""
echo "To start the database server:"
echo "  cargo run --bin rusty-db-server"
echo ""
echo "To start the CLI client (in another terminal):"
echo "  cargo run --bin rusty-db-cli"
echo ""
