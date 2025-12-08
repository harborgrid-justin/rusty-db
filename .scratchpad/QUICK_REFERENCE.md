# Quick Reference Guide for Agents

**Last Updated**: 2025-12-08

---

## Common Error Patterns & Solutions

### 1. E0599: No method `clone` found for AtomicU64

**Error**:
```rust
let value = atomic.clone(); // ERROR
```

**Fix**:
```rust
let value = atomic.load(Ordering::SeqCst);
```

---

### 2. E0599: No method `clone` found for RwLockReadGuard

**Error**:
```rust
let data = lock.read().clone(); // ERROR
```

**Fix**:
```rust
let data = (*lock.read()).clone();
// OR
let data = lock.read().deref().clone();
// OR (if data is Copy)
let data = *lock.read();
```

---

### 3. E0277: can't compare `String` with `&String`

**Error**:
```rust
if my_string == &other_string { } // ERROR
```

**Fix**:
```rust
if my_string == other_string { } // Remove &
// OR
if my_string.as_str() == other_string.as_str() { }
// OR
if &my_string == other_string { } // Add & to left side
```

---

### 4. E0277: can't compare `str` with `String`

**Error**:
```rust
if "literal" == my_string { } // ERROR
```

**Fix**:
```rust
if "literal" == my_string.as_str() { }
// OR
if "literal" == &my_string { }
// OR
if my_string == "literal" { } // Reverse order
```

---

### 5. E0369: cannot subtract `&&f64` from `&&f64`

**Error**:
```rust
let diff = &&a - &&b; // ERROR
```

**Fix**:
```rust
let diff = **a - **b; // Double dereference
// OR
let diff = *a - *b; // If only single reference
```

---

### 6. E0034: multiple applicable items in scope (SIMD)

**Error**:
```rust
let result = min(a, b); // ERROR: multiple min functions
```

**Fix**:
```rust
let result = std::cmp::min(a, b); // Use full path
// OR
let result = a.min(b); // Use method syntax
// OR
let result: f64 = min(a, b); // Add explicit type
```

---

### 7. E0689: ambiguous numeric type `{float}`

**Error**:
```rust
let threshold = 0.5; // ERROR: is this f32 or f64?
```

**Fix**:
```rust
let threshold = 0.5_f64; // Explicit f64
// OR
let threshold: f64 = 0.5; // Type annotation
```

---

### 8. E0277: cannot add-assign `usize` to `u64`

**Error**:
```rust
my_u64 += my_usize; // ERROR
```

**Fix**:
```rust
my_u64 += my_usize as u64; // Cast to u64
```

---

### 9. E0600: cannot apply unary operator `-` to type `u64`

**Error**:
```rust
let negative = -my_u64; // ERROR
```

**Fix**:
```rust
let negative = -(my_u64 as i64); // Cast to signed first
// OR (if subtracting)
let result = my_u64.saturating_sub(other); // Use saturating_sub
```

---

### 10. E0616: field `sessions` is private

**Error**:
```rust
let sessions = auth_manager.sessions; // ERROR
```

**Fix Option A** (Add getter in struct definition):
```rust
impl AuthenticationManager {
    pub fn sessions(&self) -> &HashMap<...> {
        &self.sessions
    }
}
```

**Fix Option B** (Use in same module):
```rust
// In same file as struct definition
pub(crate) sessions: HashMap<...>,
```

---

### 11. E0282: type annotations needed

**Error**:
```rust
let mut list = Vec::new(); // ERROR: what type?
```

**Fix**:
```rust
let mut list: Vec<String> = Vec::new();
// OR
let mut list = Vec::<String>::new();
// OR (if inferrable from usage)
let mut list = Vec::new();
list.push("string"); // Type inferred from push
```

---

### 12. E0423: expected function, found type alias `TableId`

**Error**:
```rust
let id = TableId(123); // ERROR: TableId is type alias, not newtype
```

**Fix**:
```rust
// If TableId is defined as: type TableId = u64;
let id: TableId = 123;
// OR wrap in proper newtype
let id = TableId::new(123);
```

---

### 13. E0277: `?` couldn't convert the error

**Error**:
```rust
let result = some_function()?; // ERROR: wrong error type
```

**Fix**:
```rust
let result = some_function().map_err(|e| DbError::from(e))?;
// OR implement From trait
impl From<OtherError> for DbError {
    fn from(err: OtherError) -> Self {
        DbError::Other(err.to_string())
    }
}
```

---

### 14. E0505: cannot move out of borrowed value

**Error**:
```rust
let borrowed = &data;
some_function(data); // ERROR: data is borrowed
```

**Fix**:
```rust
let borrowed = data.clone(); // Clone before borrowing
some_function(data);
// OR
{
    let borrowed = &data;
    // use borrowed
} // borrowed dropped here
some_function(data); // Now data can move
```

---

### 15. E0119: conflicting implementations

**Error**:
```rust
impl Default for MyType { }
impl Default for MyType { } // ERROR: duplicate
```

**Fix**:
```rust
// Remove duplicate implementation
// OR use newtype pattern if needed
struct Wrapper(MyType);
impl Default for Wrapper { }
```

---

## Cargo Commands Quick Reference

```bash
# Check compilation without building
cargo check

# Check specific package
cargo check -p rusty-db

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run clippy linter
cargo clippy

# Auto-fix clippy warnings
cargo clippy --fix

# Format code
cargo fmt

# Clean build artifacts
cargo clean
```

---

## File Navigation Tips

```bash
# Find all errors in a file
cargo check 2>&1 | grep "src/path/to/file.rs"

# Count errors by type
cargo check 2>&1 | grep "error\[E" | cut -d':' -f1 | sort | uniq -c

# Find specific error type
cargo check 2>&1 | grep "error\[E0308\]"
```

---

## Agent Status File Template

```markdown
# Agent [N] Status

**Module**: [your modules]
**Assigned Errors**: [count]
**Fixed**: [count]
**In Progress**: [count]
**Blocked**: [count]
**Last Updated**: [timestamp]

## Completed ✅
- [x] file.rs:line - E0XXX - Description of fix

## In Progress 🔄
- [ ] file.rs:line - E0XXX - Working on it

## Blocked ⛔
- file.rs:line - E0XXX - Reason for blocker

## Notes
- Any important findings
- Questions for orchestrator
- Concerns or risks
```

---

## Testing Your Fixes

```bash
# Test compilation of your module
cargo check --lib

# Test specific file
cargo check --lib 2>&1 | grep "your_file.rs"

# Run tests for your module
cargo test --lib [module_name]

# Check for new warnings
cargo clippy -- -W clippy::all
```

---

## Common Pitfalls to Avoid

1. ❌ Don't remove security features to "fix" errors
2. ❌ Don't use `any` types or unsafe to bypass type system
3. ❌ Don't remove functions - implement them properly
4. ❌ Don't use type aliases for imports
5. ❌ Don't batch too many changes without testing
6. ❌ Don't forget to update your status file
7. ❌ Don't push fixes that introduce new errors

---

## When to Ask for Help

- Security-related errors you're unsure about
- Errors that seem to require architectural changes
- Multiple interconnected errors that affect other modules
- Errors that persist after multiple fix attempts
- Uncertainty about whether to remove or implement unused code

---

## Best Practices

1. ✅ Fix one error at a time and test
2. ✅ Update your status file after each fix
3. ✅ Use concrete types instead of generics when possible
4. ✅ Add comments explaining non-obvious fixes
5. ✅ Run `cargo fmt` before committing
6. ✅ Check for new errors your fix might introduce
7. ✅ Document any assumptions you make

---

## Communication Protocol

1. Create your status file immediately: `.scratchpad/AGENT_N_STATUS.md`
2. Update status every 15 minutes or after significant progress
3. Report blockers immediately in your status file
4. Mark critical findings with 🔴
5. Update completion percentage
6. Report when done and run final `cargo check`

---

*Quick Reference v1.0 - RustyDB Orchestrator*
