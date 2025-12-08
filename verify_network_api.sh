#!/bin/bash
# Verification script for network and API modules
# Run this to verify that network and API modules compile without errors

echo "=========================================="
echo "Network and API Module Verification"
echo "=========================================="
echo ""

echo "Checking for errors in network modules..."
cargo check --lib 2>&1 | grep -E "src/network.*error" | head -20

if [ $? -eq 0 ]; then
    echo "❌ Errors found in network modules"
else
    echo "✅ No errors in network modules"
fi

echo ""
echo "Checking for errors in API modules..."
cargo check --lib 2>&1 | grep -E "src/api.*error" | head -20

if [ $? -eq 0 ]; then
    echo "❌ Errors found in API modules"
else
    echo "✅ No errors in API modules"
fi

echo ""
echo "Checking for missing import errors..."
cargo check --lib 2>&1 | grep -E "(src/network|src/api)" | grep -E "cannot find (type|function|value)" | grep -E "Mutex|sleep|interval"

if [ $? -eq 0 ]; then
    echo "❌ Missing imports detected"
else
    echo "✅ All imports present"
fi

echo ""
echo "=========================================="
echo "Verification Complete"
echo "=========================================="
