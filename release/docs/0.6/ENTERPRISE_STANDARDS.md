# RustyDB Enterprise Coding Standards
## TypeScript, JavaScript, and Rust Best Practices

**Version:** 1.0
**Effective Date:** December 27, 2025
**Applies To:** RustyDB v0.6.0 Enterprise Release
**Owner:** RustyDB Engineering Leadership
**Status:** Active

---

## Executive Summary

This document establishes enterprise-grade coding standards for the RustyDB project, an $856M enterprise database management system. These standards ensure code quality, maintainability, security, and performance across our TypeScript frontend, Node.js adapter, and Rust backend.

### Scope

These standards apply to:
- All TypeScript/JavaScript code in `/frontend/`
- All TypeScript code in `/nodejs-adapter/`
- All Rust code in `/src/`
- All configuration files (package.json, tsconfig.json, Cargo.toml)
- All test code and documentation

### Core Principles

1. **Type Safety First:** Strong typing prevents bugs
2. **Zero Tolerance for Technical Debt:** Fix issues immediately
3. **Performance Matters:** Every allocation and render counts
4. **Security by Default:** Assume all input is malicious
5. **Maintainability:** Code is read 10x more than written
6. **Documentation:** If it's not documented, it doesn't exist

---

## TypeScript Best Practices

### Type Safety Standards

#### ✅ DO: Use Explicit Types

```typescript
// Correct: Explicit type definitions
interface User {
  id: string;
  name: string;
  email: string;
  role: 'admin' | 'user' | 'guest';
}

function getUser(id: string): Promise<User> {
  return fetchUser(id);
}

// Correct: Generic types for flexibility
function processItems<T extends { id: string }>(items: T[]): Map<string, T> {
  return new Map(items.map(item => [item.id, item]));
}
```

#### ❌ DON'T: Use 'any' Type

```typescript
// WRONG: Defeats TypeScript's purpose
function processData(data: any): any {
  return data.something;
}

// WRONG: Implicit any
function handleEvent(event) {
  console.log(event.target.value);
}
```

### No-Any Policy

**Policy Statement:** The use of TypeScript's `any` type is strictly prohibited in production code.

**Rationale:**

The `any` type defeats the entire purpose of TypeScript by:
1. **Eliminating Type Safety:** Compiler cannot catch type errors
2. **Breaking IntelliSense:** IDE cannot provide accurate autocomplete
3. **Hiding Bugs:** Runtime errors that should be caught at compile time
4. **Degrading Documentation:** Types serve as living documentation
5. **Technical Debt:** Future refactoring becomes dangerous
6. **Security Risks:** Unchecked data flows enable injection attacks

**Approved Alternatives:**

1. **Use `unknown` for Truly Unknown Types:**
```typescript
function parseJSON(input: string): unknown {
  return JSON.parse(input);
}

const data = parseJSON(rawInput);
if (isUser(data)) {
  console.log(data.name); // Type-safe after validation
}
```

2. **Create Proper Type Definitions:**
```typescript
interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
  };
}
```

3. **Use Generics for Flexibility:**
```typescript
function cache<T>(key: string, value: T): void {
  localStorage.setItem(key, JSON.stringify(value));
}
```

4. **Use Type Guards:**
```typescript
function isString(value: unknown): value is string {
  return typeof value === 'string';
}
```

---

## React Best Practices

### Hook Dependency Management

**Policy Statement:** All React hooks must declare complete and accurate dependency arrays.

**Rationale:**

Missing dependencies cause:
1. **Stale Closures:** Variables have outdated values
2. **Memory Leaks:** Effects not cleaned up properly
3. **Infinite Loops:** Effects triggering themselves
4. **Incorrect Renders:** Components not updating when they should
5. **Debugging Nightmares:** Non-deterministic behavior

#### ✅ DO: Include All Dependencies

```typescript
function UserProfile({ userId }: Props) {
  const [user, setUser] = useState<User | null>(null);

  useEffect(() => {
    async function loadUser() {
      const data = await fetchUser(userId);
      setUser(data);
    }
    loadUser();
  }, [userId]); // userId is a dependency

  return <div>{user?.name}</div>;
}
```

#### ❌ DON'T: Omit Dependencies

```typescript
// WRONG: userId not in dependencies
function UserProfile({ userId }: Props) {
  const [user, setUser] = useState<User | null>(null);

  useEffect(() => {
    async function loadUser() {
      const data = await fetchUser(userId); // Uses userId
      setUser(data);
    }
    loadUser();
  }, []); // BUG: Won't update when userId changes!

  return <div>{user?.name}</div>;
}
```

### Unused Code Removal Policy

**Policy Statement:** All unused variables, functions, imports, and code must be removed immediately.

**Rationale:**

Unused code:
1. **Increases Bundle Size:** Shipped to production unnecessarily
2. **Confuses Maintainers:** Unclear what's actually used
3. **Security Risk:** Unused code may contain vulnerabilities
4. **Build Performance:** Slower compilation and bundling
5. **Code Review Burden:** Noise in diffs and reviews

---

## Rust Coding Standards

### Clippy Configuration

Required `Cargo.toml` configuration:

```toml
[lints.clippy]
# Deny (block compilation)
all = "deny"
pedantic = "deny"
nursery = "warn"

# Allow specific cases
missing_errors_doc = "allow"
missing_panics_doc = "allow"
module_name_repetitions = "allow"
```

### Performance Standards

#### Minimize Clones

```rust
// WRONG: Unnecessary clone
fn process_name(user: &User) -> String {
    let name = user.name.clone(); // Unnecessary!
    name.to_uppercase()
}

// Correct: Borrow instead
fn process_name(user: &User) -> String {
    user.name.to_uppercase()
}
```

#### Use References

```rust
// WRONG: Takes ownership unnecessarily
fn log_user(user: User) {
    println!("{}", user.name);
    // user is dropped here!
}

// Correct: Borrow
fn log_user(user: &User) {
    println!("{}", user.name);
    // user still available to caller
}
```

### Error Handling

```rust
// Use Result type consistently
use crate::error::{DbError, Result};

pub fn connect_database(config: &Config) -> Result<Connection> {
    let conn = TcpStream::connect(&config.host)
        .map_err(|e| DbError::Connection(e.to_string()))?;

    Ok(Connection::new(conn))
}
```

### Documentation

```rust
/// Executes a SQL query against the database.
///
/// # Arguments
///
/// * `sql` - The SQL statement to execute
/// * `params` - Query parameters for prepared statements
///
/// # Returns
///
/// A `Result` containing the query results or a `DbError`
///
/// # Errors
///
/// Returns `DbError::InvalidSql` if SQL is malformed
/// Returns `DbError::Connection` if database is unreachable
///
/// # Examples
///
/// ```
/// let results = db.execute("SELECT * FROM users WHERE id = ?", &[1])?;
/// ```
pub fn execute(&self, sql: &str, params: &[Value]) -> Result<QueryResult> {
    // Implementation
}
```

---

## Code Quality Requirements

### Complexity Limits

| Metric | Limit | Enforcement |
|--------|-------|-------------|
| Cyclomatic Complexity | 15 per function | Warning at 10, Error at 15 |
| Function Length | 50 lines | Warning at 40, Error at 60 |
| File Length | 500 lines | Warning at 400, Error at 600 |
| Parameters per Function | 5 | Warning at 4, Error at 6 |
| Nesting Depth | 4 levels | Warning at 3, Error at 5 |

### Code Metrics Dashboard

All projects must track:
- Code coverage (minimum 80%)
- Type coverage (minimum 95%)
- Linting issues (zero tolerance)
- Bundle size (track trends)
- Performance metrics (Core Web Vitals)

### Refactoring Triggers

Refactor immediately when:
- Function exceeds complexity limits
- File exceeds 500 lines
- Duplicate code detected (DRY principle)
- Test coverage drops below 80%
- Performance regression detected

---

## ESLint Configuration Standards

### Required Configuration

All projects must use the following ESLint configuration as a baseline:

```json
{
  "extends": [
    "next/core-web-vitals",
    "plugin:@typescript-eslint/recommended",
    "plugin:@typescript-eslint/recommended-requiring-type-checking"
  ],
  "parser": "@typescript-eslint/parser",
  "parserOptions": {
    "project": "./tsconfig.json",
    "ecmaVersion": 2022,
    "sourceType": "module"
  },
  "plugins": ["@typescript-eslint", "react-hooks"],
  "rules": {
    "@typescript-eslint/no-explicit-any": "error",
    "@typescript-eslint/no-unused-vars": [
      "error",
      {
        "argsIgnorePattern": "^_",
        "varsIgnorePattern": "^_",
        "caughtErrorsIgnorePattern": "^_"
      }
    ],
    "@typescript-eslint/explicit-function-return-type": [
      "warn",
      {
        "allowExpressions": true,
        "allowTypedFunctionExpressions": true
      }
    ],
    "@typescript-eslint/no-floating-promises": "error",
    "@typescript-eslint/no-misused-promises": "error",
    "@typescript-eslint/await-thenable": "error",
    "react-hooks/rules-of-hooks": "error",
    "react-hooks/exhaustive-deps": "error",
    "no-console": ["warn", { "allow": ["warn", "error"] }],
    "prefer-const": "error",
    "no-var": "error",
    "eqeqeq": ["error", "always"],
    "curly": ["error", "all"]
  }
}
```

### TypeScript Compiler Options

Required `tsconfig.json` settings for strict type checking:

```json
{
  "compilerOptions": {
    "strict": true,
    "noImplicitAny": true,
    "strictNullChecks": true,
    "strictFunctionTypes": true,
    "strictBindCallApply": true,
    "strictPropertyInitialization": true,
    "noImplicitThis": true,
    "alwaysStrict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "exactOptionalPropertyTypes": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  }
}
```

---

## Enforcement and Compliance

### Pre-commit Hooks

```bash
#!/bin/sh
# .git/hooks/pre-commit

# Run linters
npm run lint --fix
cargo fmt --check
cargo clippy -- -D warnings

# Run tests
npm test
cargo test

# Exit on any failure
```

### CI/CD Pipeline

All PRs must pass:
- ✅ ESLint with zero errors
- ✅ TypeScript compilation with strict mode
- ✅ Clippy with no warnings
- ✅ All tests passing
- ✅ Code coverage >= 80%
- ✅ Bundle size within limits

### Code Review Checklist

- [ ] No `any` types used
- [ ] No unused variables or imports
- [ ] All hook dependencies declared
- [ ] Functions under complexity limits
- [ ] Proper error handling
- [ ] Documentation complete
- [ ] Tests added/updated
- [ ] Performance considered

### Violation Response

| Severity | Response | Timeline |
|----------|----------|----------|
| Critical (any, unsafe) | Block PR, immediate fix | Same day |
| High (unused code) | Block PR, fix required | 1-2 days |
| Medium (complexity) | Warning, refactor ticket | 1 week |
| Low (style) | Warning, fix in next PR | 2 weeks |

---

## Training and Resources

### Required Training
- TypeScript Advanced Types (4 hours)
- React Hooks Masterclass (3 hours)
- Rust Ownership and Borrowing (6 hours)
- Enterprise Security Practices (2 hours)

### Reference Materials
- TypeScript Handbook: https://www.typescriptlang.org/docs/
- React Docs: https://react.dev
- Rust Book: https://doc.rust-lang.org/book/
- RustyDB CLAUDE.md: `/home/user/rusty-db/CLAUDE.md`

---

## Quality Metrics from Linting Audit

**Audit Date:** December 27, 2025
**Audit ID:** LINT-2025-12-27-001

### Issues Identified

| Component | Total Issues | Critical | High | Medium | Low |
|-----------|--------------|----------|------|--------|-----|
| Frontend (TypeScript/React) | 475+ | 150 | 275 | 50 | 0 |
| Node.js Adapter | 120+ | 50 | 40 | 30 | 0 |
| Rust Backend (Clippy) | 250+ | 0 | 100 | 120 | 30 |
| **Total** | **845+** | **200** | **415** | **200** | **30** |

### Issue Categories

**Frontend:**
- Type Safety Violations: 150+ (Critical)
- Unused Variables/Imports: 200+ (High)
- React Hook Dependencies: 75+ (High)
- Console Statements: 50+ (Medium)

**Node.js Adapter:**
- Type Safety Issues: 50+ (Critical)
- Error Handling Gaps: 30+ (Critical)
- Unused Code: 40+ (High)

**Rust Backend:**
- Unnecessary Clones: 100+ (High)
- Complex Functions: 50+ (Medium)
- Deprecated APIs: 20+ (Medium)
- Other Clippy Warnings: 80+ (Medium)

### Remediation Status

**Phase 1 (Critical):** In Progress
**Phase 2 (High Priority):** Planned
**Phase 3 (Medium Priority):** Planned
**Phase 4 (Ongoing):** Framework established

---

## Document Control

**Version History:**

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 1.0 | 2025-12-27 | Initial release | Enterprise Agent 12 |
| 1.1 | 2025-12-28 | Added v0.6 metrics | Agent 12 |

**Review Schedule:** Quarterly

**Next Review:** 2026-03-28

---

*This document is authoritative for all RustyDB development.*
*Deviations require written approval from Engineering Leadership.*
