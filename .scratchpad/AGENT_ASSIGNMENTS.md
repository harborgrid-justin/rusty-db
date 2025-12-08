# Agent Assignment Instructions

## Overview
This document contains detailed instructions for each of the 10 parallel agents working on RustyDB compilation error fixes.

**Total Errors to Fix**: 159
**Coordination File**: ORCHESTRATOR_STATUS.md
**Error Details**: ERROR_BREAKDOWN.md

---

## Agent 1: Storage & Buffer Layer

**Files**: F:\temp\rusty-db\.scratchpad\AGENT_1_INSTRUCTIONS.md
**Modules**: storage/, buffer/
**Error Count**: 15 errors
**Priority**: HIGH

### Errors to Fix:

1. **src/storage/disk.rs:799** - E0599: no method `schedule` found for enum Result
2. **src/storage/disk.rs:806** - E0599: no method `schedule` found for enum Result
3. **src/storage/disk.rs:823** - E0599: no method `next_operation` found for enum Result
4. **src/buffer/manager.rs:687** - E0308: mismatched types
5. **src/buffer/manager.rs:707** - E0308: mismatched types
6. **src/buffer/manager.rs:799** - E0599: no method `clone` found for AtomicU64
7. **src/buffer/manager.rs:800** - E0599: no method `clone` found for AtomicU64

### Strategy:
- Check if Result type has schedule/next_operation methods in scope
- For AtomicU64 clone: Use .load(Ordering::SeqCst) to get value, then clone that
- Type mismatches: Read the full error context to understand expected vs actual types

---

## Agent 2: Transaction & Execution Engine

**Files**: F:\temp\rusty-db\.scratchpad\AGENT_2_INSTRUCTIONS.md
**Modules**: transaction/, execution/
**Error Count**: 13 errors
**Priority**: HIGH

### Errors to Fix:

1. **src/transaction/wal.rs:580** - E0107: type alias takes 1 generic argument but 2 supplied
2. **src/transaction/wal.rs:580** - E0277: cannot build Result from iterator
3. **src/transaction/locks.rs:461** - E0618: expected function, found DbError
4. **src/transaction/locks.rs:617** - E0599: no method `clone` for RwLockReadGuard
5. **src/transaction/locks.rs:784** - E0599: no method `clone` for RwLockReadGuard
6. **src/transaction/recovery.rs:536** - E0599: no method `clone` for RwLockReadGuard
7. **src/transaction/recovery.rs:626** - E0599: no method `clone` for RwLockReadGuard
8. **src/transaction/occ.rs:406** - E0599: no method `clone` for RwLockWriteGuard
9. **src/execution/expressions.rs:281** - E0308: `?` operator has incompatible types

### Strategy:
- RwLock guard cloning: Replace `guard.clone()` with `(*guard).clone()` or `guard.deref().clone()`
- Type alias: Check definition and ensure correct generic arguments
- DbError as function: Likely needs DbError::SomeVariant construction

---

## Agent 3: Security & Vault

**Files**: F:\temp\rusty-db\.scratchpad\AGENT_3_INSTRUCTIONS.md
**Modules**: security/, security_vault/
**Error Count**: 17 errors
**Priority**: CRITICAL (Security must not be compromised)

### Errors to Fix:

1. **src/security/encryption_engine.rs:668** - E0034: multiple applicable items in scope
2. **src/security/encryption_engine.rs:1016** - E0034: multiple applicable items in scope
3. **src/security/insider_threat.rs:774** - E0689: ambiguous numeric type {float}
4. **src/security/memory_hardening.rs:731** - E0308: mismatched types
5. **src/security/memory_hardening.rs:784** - E0308: mismatched types
6. **src/security/memory_hardening.rs:937** - E0308: mismatched types
7. **src/security/mod.rs:265** - E0308: mismatched types
8. **src/security/mod.rs:297** - E0308: mismatched types
9. **src/security/mod.rs:320** - E0308: mismatched types
10. **src/security/mod.rs:350** - E0308: mismatched types
11. **src/security/mod.rs:368** - E0308: mismatched types
12. **src/security/mod.rs:392** - E0616: field `sessions` of AuthenticationManager is private
13. **src/security/mod.rs:393** - E0616: field `users` of AuthenticationManager is private

### Strategy:
- Ambiguous float: Add explicit type annotation like `0.0_f64`
- Multiple applicable items: Use full path like `std::cmp::min` or module::function
- Private fields: Add getter methods in AuthenticationManager or make fields pub(crate)
- Type mismatches: Check function signatures and ensure types align

---

## Agent 4: Indexing & SIMD

**Files**: F:\temp\rusty-db\.scratchpad\AGENT_4_INSTRUCTIONS.md
**Modules**: index/, simd/
**Error Count**: 4 errors
**Priority**: MEDIUM

### Errors to Fix:

1. **src/index/swiss_table.rs:353** - E0277: trait bound `K: AsRef<[u8]>` not satisfied
2. **src/index/mod.rs:185** - E0308: `match` arms have incompatible types
3. **src/index/mod.rs:263** - E0277: trait bound `index::Index: Clone` not satisfied

### Strategy:
- Trait bound: Add where clause `K: AsRef<[u8]>` or implement trait
- Match arms: Ensure all arms return same type
- Clone for Index: Add #[derive(Clone)] to Index enum/struct

---

## Agent 5: Clustering & Replication

**Files**: F:\temp\rusty-db\.scratchpad\AGENT_5_INSTRUCTIONS.md
**Modules**: clustering/, rac/, replication/
**Error Count**: 3 errors
**Priority**: HIGH

### Errors to Fix:

1. **src/advanced_replication/conflicts.rs:493** - E0609: no field `pending_conflicts`
2. **src/advanced_replication/apply.rs:460** - E0505: cannot move out of `group` because borrowed
3. **src/rac/parallel_query.rs:314** - E0277: oneshot::Sender not Clone

### Strategy:
- Missing field: Check struct definition, may need to add field or use different accessor
- Move out of borrowed: Clone before moving, or restructure borrowing
- Sender not Clone: Use Arc<Mutex<Option<Sender>>> or channel multiple times

---

## Agent 6: Analytics & ML

**Files**: F:\temp\rusty-db\.scratchpad\AGENT_6_INSTRUCTIONS.md
**Modules**: analytics/, inmemory/, ml/, ml_engine/
**Error Count**: 28 errors
**Priority**: MEDIUM (but most errors)

### Key Errors:

1. **src/ml/engine.rs** - 12x E0034: Multiple applicable items (SIMD ambiguity)
2. **src/analytics/warehouse.rs:530-532** - 4x E0277: String comparison issues
3. **src/ml/quantization.rs** - 4x E0369: Cannot subtract &&f64 from &&f64
4. **src/ml/mod.rs:238** - E0119: Conflicting Default implementation
5. **src/analytics/approximate.rs:97** - E0600: Cannot apply unary `-` to u64

### Strategy:
- SIMD ambiguity: Add explicit type annotations or use full paths
- String comparisons: Use .as_str() or dereference properly
- &&f64 subtraction: Dereference: `**a - **b`
- Conflicting Default: Remove one implementation or use newtype pattern
- Unary `-` on u64: Cast to i64 first or use saturating_sub

---

## Agent 7: Backup & Monitoring

**Files**: F:\temp\rusty-db\.scratchpad\AGENT_7_INSTRUCTIONS.md
**Modules**: backup/, flashback/, monitoring/
**Error Count**: 5 errors
**Priority**: HIGH

### Errors to Fix:

1. **src/backup/catalog.rs:349** - E0282: type annotations needed for Option<_>
2. **src/backup/catalog.rs:357** - E0609: no field `scn_end` on type
3. **src/flashback/transaction.rs:162** - E0308: mismatched types
4. **src/flashback/transaction.rs:162** - E0277: cannot add-assign usize to u64

### Strategy:
- Type annotations: Add explicit type like `let var: Option<TypeName> = ...`
- Missing field: Check struct definition
- Add-assign: Cast usize to u64: `value += other as u64`

---

## Agent 8: Network & API

**Files**: F:\temp\rusty-db\.scratchpad\AGENT_8_INSTRUCTIONS.md
**Modules**: network/, api/, pool/
**Error Count**: 1 error
**Priority**: MEDIUM

### Errors to Fix:

1. **src/streams/integration.rs:479** - E0277: `?` couldn't convert error to DbError

### Strategy:
- Add .map_err(|e| DbError::from(e)) or implement From trait for error type

---

## Agent 9: Graph & Spatial

**Files**: F:\temp\rusty-db\.scratchpad\AGENT_9_INSTRUCTIONS.md
**Modules**: graph/, spatial/, document_store/
**Error Count**: 10 errors
**Priority**: MEDIUM

### Key Errors:

1. **src/graph/storage.rs:293** - E0373: closure may outlive function
2. **src/graph/property_graph.rs:1147** - E0599: method clone trait bounds not satisfied
3. **src/document_store/indexing.rs** - 2x E0423: Type alias used as constructor
4. **src/document_store/qbe.rs** - 2x E0599: no method `weekday` for DateTime

### Strategy:
- Closure lifetime: Add `move` keyword or restructure
- Clone bounds: Check inner type and add Clone derive
- Type alias: Use proper constructor syntax
- weekday: Use .date().weekday() or check chrono version

---

## Agent 10: Concurrency & Misc

**Files**: F:\temp\rusty-db\.scratchpad\AGENT_10_INSTRUCTIONS.md
**Modules**: concurrent/, compression/, procedures/, autonomous/, blockchain/, workload/, orchestration/, multitenancy/, multitenant/, event_processing/, enterprise/, io/
**Error Count**: 78 errors (LARGEST)
**Priority**: MIXED

### High Priority Errors:

1. **src/multitenancy/container.rs:40** - E0599: Typo "InsufficificientPrivileges" -> "InsufficientPrivileges"
2. **src/multitenant/cdb.rs:327** - E0277: PluggableDatabase not Debug
3. **src/multitenant/pdb.rs:369** - E0277: PluggableDatabase not Debug + not Clone
4. **src/concurrent/skiplist.rs:85** - E0015: non-const fn in constants
5. **src/io/mod.rs:150** - E0277: *mut c_void not Send

### Strategy:
- Quick win: Fix typo first
- PluggableDatabase: Add #[derive(Debug, Clone)] or implement manually
- Const fn: Move initialization to lazy_static or OnceCell
- *mut c_void: Wrap in struct with unsafe impl Send

---

## Coordination Protocol

Each agent should:

1. Create their status file: `.scratchpad/AGENT_N_STATUS.md`
2. Update status file with:
   - Errors fixed
   - Errors in progress
   - Blockers encountered
   - Estimated completion time
3. Run `cargo check` on their modules after fixes
4. Report completion to orchestrator

**Status File Format**:
```markdown
# Agent N Status

**Module**: [modules]
**Progress**: X/Y errors fixed
**Status**: IN_PROGRESS / COMPLETED / BLOCKED
**Last Updated**: [timestamp]

## Completed Fixes
- [x] File:line - Error type - Description

## In Progress
- [ ] File:line - Error type - Description

## Blockers
- Issue description

## Notes
- Any findings or concerns
```

---

## Success Criteria

- All 159 compilation errors resolved
- No new errors introduced
- All security features maintained
- No functions removed
- Proper concrete types (no `any`)
- No type aliases for imports
- Code compiles with `cargo check`
- Tests pass with `cargo test`
