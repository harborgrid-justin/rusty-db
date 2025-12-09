#!/bin/bash
# Script to add std::time imports where SystemTime or UNIX_EPOCH are used but not imported

echo "=== Fixing std::time imports ==="

# Find all Rust files
find /workspaces/rusty-db/src -name "*.rs" | while read file; do
    # Check if file uses SystemTime or UNIX_EPOCH
    if grep -q '\bSystemTime\b\|\bUNIX_EPOCH\b' "$file"; then
        # Check if already has the import
        if ! grep -q 'use std::time::{.*SystemTime.*}' "$file" && \
           ! grep -q 'use std::time::SystemTime' "$file" && \
           ! grep -q 'std::time::SystemTime' "$file"; then

            # Determine what's needed
            needs_systemtime=false
            needs_unix_epoch=false
            needs_duration=false
            needs_instant=false

            if grep -q '\bSystemTime\b' "$file"; then
                needs_systemtime=true
            fi
            if grep -q '\bUNIX_EPOCH\b' "$file"; then
                needs_unix_epoch=true
            fi
            if grep -q '\bDuration\b' "$file" && ! grep -q 'use.*Duration' "$file"; then
                needs_duration=true
            fi
            if grep -q '\bInstant\b' "$file" && ! grep -q 'use.*Instant' "$file"; then
                needs_instant=true
            fi

            # Build import statement
            imports=""
            if $needs_systemtime; then
                imports="SystemTime"
            fi
            if $needs_unix_epoch; then
                if [ -n "$imports" ]; then
                    imports="$imports, UNIX_EPOCH"
                else
                    imports="UNIX_EPOCH"
                fi
            fi
            if $needs_duration; then
                if [ -n "$imports" ]; then
                    imports="$imports, Duration"
                else
                    imports="Duration"
                fi
            fi
            if $needs_instant; then
                if [ -n "$imports" ]; then
                    imports="$imports, Instant"
                else
                    imports="Instant"
                fi
            fi

            if [ -n "$imports" ]; then
                echo "Adding 'use std::time::{$imports};' to $file"

                # Add import after first use statement or at start of file after comments
                if grep -q '^use ' "$file"; then
                    # Add after last existing use statement in the header
                    sed -i "0,/^use /{s/^use /use std::time::{$imports};\nuse /}" "$file"
                else
                    # Add after any initial comments/attributes
                    # Find first non-comment, non-attribute, non-empty line
                    sed -i "1i use std::time::{$imports};" "$file"
                fi
            fi
        fi
    fi
done

echo ""
echo "=== Fixing Duration imports ==="

find /workspaces/rusty-db/src -name "*.rs" | while read file; do
    if grep -q '\bDuration\b' "$file"; then
        # Check various import patterns
        if ! grep -q 'use std::time::Duration' "$file" && \
           ! grep -q 'use std::time::{.*Duration.*}' "$file" && \
           ! grep -q 'use tokio::time::Duration' "$file" && \
           ! grep -q 'std::time::Duration' "$file"; then

            echo "Adding Duration import to $file"

            # Check if there's already a std::time import we can extend
            if grep -q 'use std::time::{' "$file"; then
                # Add Duration to existing import
                sed -i 's/use std::time::{/use std::time::{Duration, /' "$file"
            elif grep -q '^use ' "$file"; then
                sed -i "0,/^use /{s/^use /use std::time::Duration;\nuse /}" "$file"
            else
                sed -i "1i use std::time::Duration;" "$file"
            fi
        fi
    fi
done

echo ""
echo "=== Fixing Instant imports ==="

find /workspaces/rusty-db/src -name "*.rs" | while read file; do
    if grep -q '\bInstant\b' "$file"; then
        if ! grep -q 'use std::time::Instant' "$file" && \
           ! grep -q 'use std::time::{.*Instant.*}' "$file" && \
           ! grep -q 'use tokio::time::Instant' "$file" && \
           ! grep -q 'std::time::Instant' "$file"; then

            echo "Adding Instant import to $file"

            if grep -q 'use std::time::{' "$file"; then
                sed -i 's/use std::time::{/use std::time::{Instant, /' "$file"
            elif grep -q '^use ' "$file"; then
                sed -i "0,/^use /{s/^use /use std::time::Instant;\nuse /}" "$file"
            else
                sed -i "1i use std::time::Instant;" "$file"
            fi
        fi
    fi
done

echo ""
echo "=== Cleaning up duplicate imports ==="

# Remove duplicate consecutive import lines
find /workspaces/rusty-db/src -name "*.rs" | while read file; do
    # Remove exact duplicate consecutive lines for imports only
    awk '
    /^use std::time/ {
        if (prev == $0) next
        prev = $0
    }
    { print; prev = $0 }
    ' "$file" > "${file}.tmp" && mv "${file}.tmp" "$file"
done

echo ""
echo "=== Done! ==="
echo "Run 'cargo check 2>&1 | head -50' to verify"
