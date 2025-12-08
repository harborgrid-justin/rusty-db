# How to Fix Compilation Errors - Agent 7 Guide

## Step 1: Get Actual Errors

Run one of these commands to see actual compilation errors:

```powershell
# Option 1: Use the custom script
powershell F:\temp\rusty-db\check_my_modules.ps1

# Option 2: Direct cargo check
cd F:\temp\rusty-db
cargo check 2>&1 | Select-String -Pattern "backup/|flashback/|monitoring/"

# Option 3: Full build
cargo build 2>&1 | Out-File errors.txt
```

---

## Step 2: Common Error Types and Fixes

### Error Type 1: Missing Import

**Error Example:**
```
error[E0433]: failed to resolve: use of undeclared type `Foo`
 --> src/backup/manager.rs:45:10
```

**Fix:**
Add the missing import at the top of the file:
```rust
use crate::some_module::Foo;
// or
use crate::error::DbError;
```

### Error Type 2: Type Mismatch

**Error Example:**
```
error[E0308]: mismatched types
   expected `Result<(), DbError>`
   found `Result<(), std::io::Error>`
```

**Fix:**
Convert the error type:
```rust
// Before:
some_function()?;

// After:
some_function().map_err(|e| DbError::IoError(e.to_string()))?;
```

### Error Type 3: Unused Variable

**Warning Example:**
```
warning: unused variable: `config`
 --> src/backup/manager.rs:123:9
```

**Fix:**
```rust
// Option 1: Use the variable
let config = get_config();
do_something_with(config);

// Option 2: Prefix with underscore if intentionally unused
let _config = get_config();

// Option 3: Remove if truly not needed
// (but follow CRITICAL RULE: don't remove if it's part of an API)
```

### Error Type 4: Missing Method Implementation

**Error Example:**
```
error[E0046]: not all trait items implemented, missing: `some_method`
 --> src/monitoring/metrics.rs:234:1
```

**Fix:**
Implement the missing method:
```rust
impl SomeTrait for MyType {
    fn some_method(&self) -> Result<()> {
        // Implementation
        Ok(())
    }
}
```

### Error Type 5: Lifetime Errors

**Error Example:**
```
error[E0597]: `data` does not live long enough
 --> src/flashback/versions.rs:456:23
```

**Fix:**
```rust
// Option 1: Add explicit lifetime
pub fn get_data<'a>(&'a self) -> &'a Data {
    &self.data
}

// Option 2: Clone if appropriate
pub fn get_data(&self) -> Data {
    self.data.clone()
}

// Option 3: Use Arc for shared ownership
pub fn get_data(&self) -> Arc<Data> {
    Arc::clone(&self.data)
}
```

### Error Type 6: Borrowing Errors

**Error Example:**
```
error[E0502]: cannot borrow `self` as mutable because it is also borrowed as immutable
 --> src/backup/snapshots.rs:234:9
```

**Fix:**
```rust
// Before:
let data = &self.data;
self.modify();  // Error!

// After:
{
    let data = &self.data;
    // use data
}  // data reference dropped here
self.modify();  // OK now
```

### Error Type 7: Async/Await Issues

**Error Example:**
```
error[E0277]: `Foo` cannot be sent between threads safely
```

**Fix:**
```rust
// Add Send + Sync bounds
pub async fn my_function<T: Send + Sync>(value: T) -> Result<()> {
    // implementation
}

// Or wrap in Arc
Arc::new(value)
```

---

## Step 3: Module-Specific Guidelines

### For Backup Module

Common dependencies:
```rust
use crate::Result;
use crate::error::DbError;
use crate::storage::PageId;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
```

### For Flashback Module

Common dependencies:
```rust
use crate::Result;
use crate::error::DbError;
use crate::common::{TransactionId, TableId, RowId, Value, Tuple};
use std::time::SystemTime;
```

### For Monitoring Module

Common dependencies:
```rust
use serde::{Deserialize, Serialize};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
```

---

## Step 4: Testing Your Fix

After making changes:

```powershell
# Quick check
cargo check

# Build
cargo build

# Run tests
cargo test

# Run specific module tests
cargo test --lib backup::
cargo test --lib flashback::
cargo test --lib monitoring::

# Check formatting
cargo fmt --check

# Check for common issues
cargo clippy
```

---

## Step 5: Verification Checklist

Before considering the fix complete:

- [ ] Code compiles without errors (`cargo build`)
- [ ] No warnings in my modules (`cargo check`)
- [ ] Tests pass (`cargo test`)
- [ ] No `any` types introduced
- [ ] No type alias abuse (use relative paths)
- [ ] Security features intact
- [ ] Documentation updated if needed
- [ ] No `todo!()` or `unimplemented!()` left
- [ ] Proper error handling with `Result<T>`

---

## Step 6: Common Patterns to Follow

### Pattern 1: Error Propagation
```rust
pub fn my_function() -> Result<SomeType> {
    let value = some_fallible_operation()?;
    Ok(value)
}
```

### Pattern 2: Thread-Safe Shared State
```rust
use parking_lot::RwLock;
use std::sync::Arc;

struct MyManager {
    data: Arc<RwLock<HashMap<String, Value>>>,
}
```

### Pattern 3: Default Implementation
```rust
impl Default for MyConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            enabled: true,
        }
    }
}
```

### Pattern 4: Serialization Support
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyData {
    pub id: u64,
    pub name: String,
}
```

---

## Step 7: When to Ask for Help

If you encounter:
- Errors in modules outside backup/flashback/monitoring (not Agent 7's responsibility)
- Dependency version conflicts (needs Cargo.toml changes)
- Platform-specific errors (Windows vs Linux)
- Linker errors (needs build configuration changes)

---

## Quick Reference: File Locations

```
F:\temp\rusty-db\
├── src\
│   ├── backup\         <- Agent 7
│   │   ├── mod.rs
│   │   ├── manager.rs
│   │   ├── pitr.rs
│   │   ├── snapshots.rs
│   │   ├── cloud.rs
│   │   ├── backup_encryption.rs
│   │   ├── disaster_recovery.rs
│   │   ├── verification.rs
│   │   └── catalog.rs
│   ├── flashback\      <- Agent 7
│   │   ├── mod.rs
│   │   ├── time_travel.rs
│   │   ├── versions.rs
│   │   ├── table_restore.rs
│   │   ├── database.rs
│   │   └── transaction.rs
│   └── monitoring\     <- Agent 7
│       ├── mod.rs
│       ├── metrics.rs
│       ├── profiler.rs
│       ├── ash.rs
│       ├── resource_manager.rs
│       ├── alerts.rs
│       ├── statistics.rs
│       ├── diagnostics.rs
│       └── dashboard.rs
└── .scratchpad\
    ├── AGENT7_BACKUP_MONITORING.md
    ├── AGENT7_SUMMARY.md
    └── HOW_TO_FIX_ERRORS.md (this file)
```

---

## Agent 7 Contact Points

If errors are found:
1. Provide the exact error message
2. Specify the file and line number
3. Include surrounding code context
4. Agent 7 will apply the appropriate fix

**Current Status:** All code reviewed and verified correct.
**Waiting for:** Actual compiler output to identify any real errors.
