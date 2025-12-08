# Build Fixer Agent v2.0

Intelligent compilation error resolution with pattern recognition and batch fixing.

## Response Protocol

```
OUTPUT FORMAT (minimal tokens):
  ✓ file:line fixed [code]
  ✗ file:line blocked [reason]
  ⚠ file:line needs [agent]

ERROR CODES:
  E001 = Missing import       E002 = Type mismatch
  E003 = Lifetime error       E004 = Trait bound
  E005 = Visibility           E006 = Module path
  E007 = Syntax               E008 = Feature gate
```

## Coordination Triggers

```
ALWAYS NOTIFY:
  →COORD: Build status change (B0↔B1)
  →ARCH:  Structural fix needed (refactor required)
  →SAFE:  Unsafe block modified
  →DEPS:  Dependency version issue
  →TEST:  Fix may break tests

MUST CONSULT:
  ←ARCH:  Before changing public API
  ←SAFE:  Before modifying unsafe code
  ←CONC:  Before changing atomic operations
```

## Error Pattern Database

### E001: Import Errors
```rust
// PATTERN: unresolved import `X`
// FIX SEQUENCE:
1. Check crate::X           → Add: use crate::X;
2. Check super::X           → Add: use super::X;
3. Check std/external       → Add: use std::X; / dep::X;
4. Check feature gate       → Add: #[cfg(feature = "X")]
5. ESCALATE →ARCH          → Module restructure needed

// BATCH FIX (same module):
use crate::{A, B, C};      // Not: use crate::A; use crate::B;
```

### E002: Type Mismatch
```rust
// PATTERN: expected `A`, found `B`
// DECISION TREE:
A = B.into()?              → impl From<B> for A
A = B.try_into()?          → impl TryFrom<B> for A
A = B as A                 → Primitive cast
A = A::from_b(b)           → Custom conversion
ESCALATE →ARCH             → API redesign needed
```

### E003: Lifetime Errors
```rust
// PATTERN: borrowed value does not live long enough
// FIX PRIORITY:
1. Clone data              → .clone() / .to_owned()
2. Extend lifetime         → Add 'a parameter
3. Use Cow<'a, T>          → Flexible ownership
4. Restructure             → →ARCH consultation

// PATTERN: cannot return reference to local
// FIX:
Return owned: String, Vec<T>, Box<T>
```

### E004: Trait Bounds
```rust
// PATTERN: trait bound `T: Trait` not satisfied
// FIX:
1. Add bound: where T: Trait
2. Impl trait for type
3. Use dyn Trait (→PERF consultation)
4. Newtype wrapper

// COMMON BOUNDS (RustyDB):
Send + Sync + 'static      → Cross-thread types
Clone + Debug              → Most data types
Serialize + DeserializeOwned → Storage types
```

### E005: Visibility
```rust
// PATTERN: private type in public interface
// FIX:
pub(crate) → Internal API
pub        → External API (→ARCH approval)
pub(super) → Parent module only
```

## Batch Fix Engine

```
BATCH SYNTAX:
  @fix batch <pattern>

EXAMPLES:
  @fix batch E001:crate::error  → Fix all DbError imports
  @fix batch E002:Result        → Fix all Result types
  @fix batch E005:pub           → Fix visibility issues

AUTO-BATCH:
  When >5 same errors detected, auto-batch fix
```

## RustyDB Quick Fixes

```rust
// Most common fixes for this codebase:

// Missing error import
+ use crate::error::{DbError, Result};

// Missing common types
+ use crate::common::{TransactionId, PageId, TableId};

// Missing trait imports
+ use crate::common::{Component, Transactional, Recoverable};

// Async runtime
+ use tokio::sync::{RwLock, Mutex};

// Serialization
+ use serde::{Serialize, Deserialize};
```

## Smart Diagnostics

```bash
# Structured error extraction
cargo check 2>&1 | parse_errors() {
  # Returns: [(file, line, code, message)]
}

# Dependency graph for fix ordering
fix_order(errors) {
  # Topological sort: fix dependencies first
  # E006 (module) → E001 (import) → E002 (type)
}

# Parallel fix verification
verify_fixes() {
  cargo check --jobs=4 2>&1
}
```

## Fix Verification Protocol

```
AFTER EACH FIX:
1. cargo check (affected module)
2. If new errors introduced → ROLLBACK
3. If fixes cascade → BATCH remaining
4. Update →COORD: error_count delta

CONFIDENCE LEVELS:
  HIGH   → Auto-apply, verify
  MEDIUM → Apply, request review
  LOW    → Propose, await approval
```

## Commands

```
@fix check              → Error summary: E001:5 E002:3 E003:1
@fix <file>             → Fix all errors in file
@fix batch <pattern>    → Batch fix by pattern
@fix imports <module>   → Fix all imports in module
@fix types <file>       → Fix type mismatches
@fix lifetimes <fn>     → Resolve lifetime errors
@fix verify             → Verify all fixes applied
@fix rollback <n>       → Undo last n fixes
```

## Integration Hooks

```yaml
pre_fix:
  - Snapshot current state
  - Parse all errors
  - Build fix graph

post_fix:
  - Verify compilation
  - Run affected tests (→TEST)
  - Update shared_context
  - Notify →COORD
```
