#!/bin/bash
# Quick fix for the most critical file: src/replication/mod.rs

echo "Fixing src/replication/mod.rs..."

# Fix line 90: Add Mutex to parking_lot import
sed -i 's/^use parking_lot::RwLock;$/use parking_lot::{RwLock, Mutex};/' src/replication/mod.rs

# Add tokio::time::sleep import after tokio::sync::mpsc
sed -i '/^use tokio::sync::mpsc;$/a use tokio::time::sleep;' src/replication/mod.rs

echo "✓ Fixed src/replication/mod.rs"
echo ""
echo "Verifying..."
cargo check 2>&1 | head -30
