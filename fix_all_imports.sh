#!/bin/bash

# Script to fix all missing imports in RustyDB files

# Function to add import if not present
add_import_if_needed() {
    local file=$1
    local search_pattern=$2
    local import_line=$3
    local insert_after=$4

    if grep -q "$search_pattern" "$file" && ! grep -q "$import_line" "$file"; then
        # Add the import after the specified line
        sed -i "/$insert_after/a $import_line" "$file"
        echo "Added '$import_line' to $file"
    fi
}

# Fix files that use Mutex but don't import it
echo "Fixing Mutex imports..."

# Files using parking_lot - add Mutex to parking_lot imports
for file in src/**/*.rs; do
    if [ -f "$file" ]; then
        # Check if file uses Mutex:: and has parking_lot but not parking_lot::Mutex
        if grep -q "Mutex::" "$file" && grep -q "use parking_lot::" "$file" && ! grep -q "use parking_lot::.*Mutex" "$file"; then
            # Try to add Mutex to existing parking_lot::RwLock import
            if grep -q "use parking_lot::RwLock;" "$file"; then
                sed -i 's/use parking_lot::RwLock;/use parking_lot::{RwLock, Mutex};/' "$file"
                echo "✓ Fixed $file (added Mutex to parking_lot)"
            elif grep -q "use parking_lot::{" "$file"; then
                # Add Mutex to existing multi-import
                sed -i '/use parking_lot::{/ s/}/,Mutex}/' "$file"
                echo "✓ Fixed $file (added Mutex to existing parking_lot multi-import)"
            else
                # Add new parking_lot::Mutex import
                sed -i '/use parking_lot::/a use parking_lot::Mutex;' "$file"
                echo "✓ Fixed $file (added new parking_lot::Mutex import)"
            fi
        # Check if file uses Mutex:: but doesn't have any Mutex import
        elif grep -q "Mutex::" "$file" && ! grep -q "use.*::Mutex" "$file"; then
            # Add std::sync::Mutex
            if grep -q "use std::sync::{.*Arc" "$file"; then
                sed -i '/use std::sync::{/ s/}/,Mutex}/' "$file"
                echo "✓ Fixed $file (added Mutex to std::sync)"
            else
                sed -i '/use std::sync::Arc/a use std::sync::Mutex;' "$file"
                echo "✓ Fixed $file (added std::sync::Mutex)"
            fi
        fi
    fi
done

# Fix files that use sleep but don't import it
echo -e "\nFixing sleep imports..."
for file in src/**/*.rs; do
    if [ -f "$file" ]; then
        if grep -q "sleep(" "$file" && ! grep -q "use tokio::time::.*sleep" "$file"; then
            if grep -q "use tokio::time::{" "$file"; then
                # Add to existing tokio::time multi-import
                sed -i '/use tokio::time::{/ s/}/,sleep}/' "$file"
                echo "✓ Fixed $file (added sleep to tokio::time)"
            else
                # Add new import after use tokio:: lines
                sed -i '/use tokio::/a use tokio::time::sleep;' "$file"
                echo "✓ Fixed $file (added tokio::time::sleep)"
            fi
        fi
    fi
done

# Fix files that use interval but don't import it
echo -e "\nFixing interval imports..."
for file in src/**/*.rs; do
    if [ -f "$file" ]; then
        if grep -q "interval(" "$file" && ! grep -q "use tokio::time::.*interval" "$file"; then
            if grep -q "use tokio::time::{" "$file"; then
                # Add to existing tokio::time multi-import
                sed -i '/use tokio::time::{/ s/}/,interval}/' "$file"
                echo "✓ Fixed $file (added interval to tokio::time)"
            elif grep -q "use tokio::time::sleep;" "$file"; then
                # Change sleep to multi-import
                sed -i 's/use tokio::time::sleep;/use tokio::time::{sleep, interval};/' "$file"
                echo "✓ Fixed $file (added interval with sleep)"
            else
                # Add new import
                sed -i '/use tokio::/a use tokio::time::interval;' "$file"
                echo "✓ Fixed $file (added tokio::time::interval)"
            fi
        fi
    fi
done

echo -e "\nDone! Running cargo check to verify..."
cargo check 2>&1 | grep -E "error|warning" | head -20
