#!/bin/bash
# Quick check for concurrent and bench module compilation

echo "=== Checking Rust compilation for concurrent and bench modules ==="
echo ""

# Try to compile just the library
echo "Running: cargo check --lib"
cargo check --lib 2>&1 | tee /tmp/cargo_check.log

echo ""
echo "=== Filtering for concurrent and bench errors ==="
grep -E "(concurrent|bench)" /tmp/cargo_check.log | grep -E "error" || echo "No errors found in concurrent/bench modules!"

echo ""
echo "=== Error summary ==="
ERROR_COUNT=$(grep -c "^error\[E" /tmp/cargo_check.log 2>/dev/null || echo "0")
echo "Total compilation errors: $ERROR_COUNT"

if [ "$ERROR_COUNT" = "0" ]; then
    echo "✓ Compilation successful!"
    exit 0
else
    echo "✗ Compilation has errors"
    exit 1
fi
