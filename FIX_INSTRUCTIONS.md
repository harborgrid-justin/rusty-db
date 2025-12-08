# Instructions to Fix All Missing Imports in RustyDB

## Problem
The codebase has missing imports causing compilation errors:
- `Mutex` type not found
- `sleep` function not found
- `interval` function not found

## Solution Files Created

I've created multiple automated fix scripts for you to choose from:

### 1. PowerShell Script (Recommended for Windows)
**File:** `fix-imports.ps1`

**Run:**
```powershell
cd F:\temp\rusty-db
PowerShell -ExecutionPolicy Bypass -File .\fix-imports.ps1
```

### 2. Python Script
**File:** `fix_imports.py`

**Run:**
```bash
cd F:\temp\rusty-db
python fix_imports.py
```

### 3. Bash Script (For WSL/Linux/Git Bash)
**File:** `fix_all_imports.sh`

**Run:**
```bash
cd /f/temp/rusty-db
chmod +x fix_all_imports.sh
./fix_all_imports.sh
```

## Known Files Requiring Fixes

### Definitely Needs Fixing:

**src/replication/mod.rs**
- Line 90: Change `use parking_lot::RwLock;` → `use parking_lot::{RwLock, Mutex};`
- After line 92: Add `use tokio::time::sleep;`

### Likely Need Checking:

Files that may need interval imports (need to verify):
- src/buffer/manager.rs
- src/buffer/mod.rs
- src/autonomous/mod.rs
- src/autonomous/self_healing.rs
- src/autonomous/self_tuning.rs
- src/monitoring/ash.rs
- src/ml_engine/mod.rs
- src/ml_engine/scoring.rs
- src/pool/connection_pool.rs
- src/pool/session_manager.rs
- src/streams/cdc.rs
- src/streams/integration.rs
- src/streams/replication.rs
- src/streams/subscriber.rs
- src/orchestration/health.rs
- src/network/cluster_network.rs
- src/rac/mod.rs
- src/rac/recovery.rs
- src/rac/interconnect.rs
- src/analytics/approximate.rs
- src/analytics/timeseries.rs
- src/analytics/mod.rs
- src/api/graphql_api.rs

## Manual Fix Guide

If the automated scripts don't work, here's how to fix manually:

### For Mutex Imports:

1. **If file uses `parking_lot`:**
   - Find: `use parking_lot::RwLock;`
   - Replace with: `use parking_lot::{RwLock, Mutex};`

   OR if it already has multi-import:
   - Find: `use parking_lot::{RwLock};`
   - Replace with: `use parking_lot::{RwLock, Mutex};`

2. **If file uses `std::sync`:**
   - Find: `use std::sync::Arc;`
   - Add after it: `use std::sync::Mutex;`

   OR modify existing:
   - Find: `use std::sync::{Arc};`
   - Replace with: `use std::sync::{Arc, Mutex};`

### For sleep Imports:

Add after other tokio imports:
```rust
use tokio::time::sleep;
```

Or if tokio::time is already imported:
- Find: `use tokio::time::interval;`
- Replace with: `use tokio::time::{interval, sleep};`

### For interval Imports:

Add after other tokio imports:
```rust
use tokio::time::interval;
```

Or if tokio::time is already imported:
- Find: `use tokio::time::sleep;`
- Replace with: `use tokio::time::{sleep, interval};`

## Verification

After applying fixes, verify with:

```bash
cargo check 2>&1 | grep -E "cannot find (type|function)"
```

If this command returns no output, all errors are fixed!

Alternatively, just run:
```bash
cargo build
```

And check if it compiles successfully.

## Important Notes

- **NEVER remove existing code** - only ADD imports
- **Use parking_lot::Mutex** for files that already import from parking_lot
- **Use std::sync::Mutex** for files that use std::sync
- **Some files use fully qualified paths** (like `tokio::time::sleep(...)`) which don't need imports
- **Close your IDE** before running fix scripts to avoid file locking issues

## Troubleshooting

### File Locking Issues
If you get "file is being modified" errors:
1. Close your IDE (VS Code, IntelliJ, etc.)
2. Stop any running cargo processes: `taskkill /F /IM cargo.exe` (Windows) or `pkill cargo` (Linux)
3. Run the fix script again

### Permission Issues (PowerShell)
If PowerShell blocks the script:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Script Not Found (Bash)
Make sure you're in the right directory:
```bash
cd /f/temp/rusty-db
ls -la fix_all_imports.sh
```

## Summary

The easiest approach:
1. Close all editors/IDEs
2. Run the PowerShell script: `PowerShell -ExecutionPolicy Bypass -File .\fix-imports.ps1`
3. Run `cargo check` to verify
4. Done!
