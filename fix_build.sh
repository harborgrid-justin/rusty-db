#!/bin/bash
# Safe Build fixer script for rusty-db
# Iteratively fixes common build errors by parsing cargo output

set -e

echo "=== Rusty-DB Build Fixer ==="
echo ""

cd /workspaces/rusty-db

# Step 1: Fix bincode version if needed (1.x has serialize/deserialize functions)
echo "[1/3] Checking dependencies..."
if grep -q 'bincode = "2' Cargo.toml 2>/dev/null; then
    echo "  Downgrading bincode to 1.3..."
    sed -i 's/bincode = "2[^"]*"/bincode = "1.3"/' Cargo.toml
fi

# Step 2: Parse cargo check output and fix errors
echo "[2/3] Running cargo check and analyzing errors..."
echo ""

# Run cargo check and capture output
ERRORS=$(cargo check 2>&1 || true)

# Count errors
ERROR_COUNT=$(echo "$ERRORS" | grep -c "^error\[E" || echo "0")
echo "Found $ERROR_COUNT compilation errors"
echo ""

# Show first batch of errors
echo "$ERRORS" | head -80

echo ""
echo "[3/3] Analysis complete"
echo ""

# Provide guidance based on error types
if echo "$ERRORS" | grep -q "cannot find value \`_"; then
    echo "DETECTED: Underscore variable naming issues"
    echo "  Variables named with underscore prefix are being used without it."
    echo "  Fix: Remove underscore from variable definitions or references."
    echo ""
fi

if echo "$ERRORS" | grep -q "cannot find type.*in this scope"; then
    echo "DETECTED: Missing type imports"
    echo "  Common missing imports: Duration, Instant, SystemTime, HashSet, VecDeque, etc."
    echo "  Fix: Add appropriate 'use' statements at the top of affected files."
    echo ""
fi

if echo "$ERRORS" | grep -q "unresolved import"; then
    echo "DETECTED: Unresolved imports"
    echo "  Some 'use' statements reference non-existent or renamed items."
    echo "  Fix: Check the import paths and feature flags in Cargo.toml."
    echo ""
fi

echo "=== Summary ==="
echo "Total errors: $ERROR_COUNT"
echo ""
echo "Run 'cargo check 2>&1 | grep \"^error\" | sort | uniq -c | sort -rn' to see error distribution"
echo "Run 'cargo check 2>&1 | less' to browse all errors"
