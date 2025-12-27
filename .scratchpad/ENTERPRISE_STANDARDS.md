# RustyDB Enterprise Coding Standards
## TypeScript, JavaScript, and Rust Best Practices

**Version:** 1.0
**Effective Date:** December 27, 2025
**Last Updated:** December 27, 2025
**Owner:** RustyDB Engineering Leadership
**Status:** Active

---

## Table of Contents

1. [Introduction](#introduction)
2. [TypeScript Best Practices](#typescript-best-practices)
3. [ESLint Configuration Standards](#eslint-configuration-standards)
4. [Code Quality Requirements](#code-quality-requirements)
5. [No-Any Policy](#no-any-policy)
6. [Unused Code Removal Policy](#unused-code-removal-policy)
7. [Hook Dependency Management](#hook-dependency-management)
8. [Rust Coding Standards](#rust-coding-standards)
9. [Documentation Requirements](#documentation-requirements)
10. [Enforcement and Compliance](#enforcement-and-compliance)

---

## Introduction

### Purpose

This document establishes enterprise-grade coding standards for the RustyDB project, a $350M database management system. These standards ensure code quality, maintainability, security, and performance across our TypeScript frontend, Node.js adapter, and Rust backend.

### Scope

These standards apply to:
- All TypeScript/JavaScript code in `/frontend/`
- All TypeScript code in `/nodejs-adapter/`
- All Rust code in `/src/`
- All configuration files (package.json, tsconfig.json, Cargo.toml)
- All test code and documentation

### Principles

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

### Interface Design

#### ✅ DO: Create Comprehensive Interfaces

```typescript
// Correct: Well-defined interface
interface DatabaseConfig {
  host: string;
  port: number;
  database: string;
  username: string;
  password: string;
  ssl?: {
    enabled: boolean;
    ca?: string;
    cert?: string;
    key?: string;
  };
  poolSize?: number;
  timeout?: number;
}

// Correct: Extend interfaces for specialization
interface EnterpriseConfig extends DatabaseConfig {
  clusterNodes: string[];
  replicationMode: 'sync' | 'async' | 'semi-sync';
  failoverEnabled: boolean;
}
```

#### ❌ DON'T: Use Inline Types Repeatedly

```typescript
// WRONG: Type definition duplicated
function connect(config: { host: string; port: number }): void {}
function disconnect(config: { host: string; port: number }): void {}

// Correct: Define once, use everywhere
interface ConnectionConfig {
  host: string;
  port: number;
}

function connect(config: ConnectionConfig): void {}
function disconnect(config: ConnectionConfig): void {}
```

### Null Safety

#### ✅ DO: Handle Null and Undefined Explicitly

```typescript
// Correct: Explicit null handling
function getUserName(user: User | null): string {
  return user?.name ?? 'Anonymous';
}

// Correct: Type guards
function isValidUser(user: unknown): user is User {
  return (
    typeof user === 'object' &&
    user !== null &&
    'id' in user &&
    'name' in user
  );
}

// Correct: Optional chaining
const email = user?.contact?.email?.toLowerCase();
```

#### ❌ DON'T: Ignore Null Checks

```typescript
// WRONG: Potential runtime error
function getUserName(user: User | null): string {
  return user.name; // Error if user is null
}

// WRONG: Non-null assertion without validation
function processUser(user: User | null): void {
  const name = user!.name; // Dangerous!
}
```

### Async/Await Patterns

#### ✅ DO: Proper Error Handling

```typescript
// Correct: Comprehensive error handling
async function fetchUserData(userId: string): Promise<User> {
  try {
    const response = await fetch(`/api/users/${userId}`);

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const data = await response.json();

    if (!isValidUser(data)) {
      throw new Error('Invalid user data received');
    }

    return data;
  } catch (error) {
    if (error instanceof Error) {
      console.error('Failed to fetch user:', error.message);
    }
    throw error;
  }
}

// Correct: Promise.all for parallel operations
async function loadDashboard(): Promise<Dashboard> {
  const [users, stats, alerts] = await Promise.all([
    fetchUsers(),
    fetchStats(),
    fetchAlerts()
  ]);

  return { users, stats, alerts };
}
```

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
    "@typescript-eslint/no-unsafe-assignment": "warn",
    "@typescript-eslint/no-unsafe-member-access": "warn",
    "@typescript-eslint/no-unsafe-call": "warn",
    "@typescript-eslint/no-unsafe-return": "warn",
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
    "noPropertyAccessFromIndexSignature": true,
    "allowUnusedLabels": false,
    "allowUnreachableCode": false,
    "exactOptionalPropertyTypes": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  }
}
```

### Exemption Process

If a rule must be disabled:

1. Document the reason with a detailed comment
2. Use inline disables, not file-level or global disables
3. Limit the scope to the minimum necessary
4. Create a technical debt ticket
5. Get approval from tech lead

```typescript
// Correct: Documented exemption with limited scope
// eslint-disable-next-line @typescript-eslint/no-explicit-any
// Reason: Third-party library returns untyped data; typed wrapper pending (JIRA: RUST-1234)
const rawData = await legacyApi.fetch() as any;
const typedData: KnownType = validateAndTransform(rawData);
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

## No-Any Policy

### Policy Statement

**The use of TypeScript's `any` type is strictly prohibited in production code.**

### Rationale

The `any` type defeats the entire purpose of TypeScript by:

1. **Eliminating Type Safety:** Compiler cannot catch type errors
2. **Breaking IntelliSense:** IDE cannot provide accurate autocomplete
3. **Hiding Bugs:** Runtime errors that should be caught at compile time
4. **Degrading Documentation:** Types serve as living documentation
5. **Technical Debt:** Future refactoring becomes dangerous
6. **Security Risks:** Unchecked data flows enable injection attacks

### Real-World Impact Example

```typescript
// DANGEROUS: Using 'any' type
function executeQuery(query: any): any {
  return database.execute(query);
}

// Vulnerability: SQL injection possible
const userInput = "'; DROP TABLE users; --";
executeQuery(userInput); // No compile-time protection!

// SAFE: Proper typing
interface Query {
  sql: string;
  params: unknown[];
}

interface QueryResult<T> {
  rows: T[];
  rowCount: number;
}

function executeQuery<T>(query: Query): Promise<QueryResult<T>> {
  // Parameterized query prevents SQL injection
  return database.execute(query.sql, query.params);
}
```

### Approved Alternatives

#### 1. Use `unknown` for Truly Unknown Types

```typescript
// Instead of 'any', use 'unknown'
function parseJSON(input: string): unknown {
  return JSON.parse(input);
}

// Force type validation before use
const data = parseJSON(rawInput);
if (isUser(data)) {
  console.log(data.name); // Type-safe after validation
}
```

#### 2. Create Proper Type Definitions

```typescript
// Instead of 'any', define the structure
interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
  };
}

async function callApi<T>(endpoint: string): Promise<ApiResponse<T>> {
  const response = await fetch(endpoint);
  return response.json();
}
```

#### 3. Use Generics for Flexibility

```typescript
// Instead of 'any', use generics
function cache<T>(key: string, value: T): void {
  localStorage.setItem(key, JSON.stringify(value));
}

function getCache<T>(key: string): T | null {
  const item = localStorage.getItem(key);
  return item ? JSON.parse(item) : null;
}
```

#### 4. Use Type Guards

```typescript
// Type guard for runtime validation
function isString(value: unknown): value is string {
  return typeof value === 'string';
}

function processInput(input: unknown): void {
  if (isString(input)) {
    console.log(input.toUpperCase()); // Safe!
  }
}
```

### Exemptions

The ONLY acceptable use of `any` is:

1. **Type Assertion Middleware:** When transforming truly untyped external data
2. **Must be followed immediately by type validation**
3. **Must be documented with ticket reference**
4. **Must be reviewed by tech lead**

```typescript
// Acceptable pattern (with validation)
function processExternalData(rawData: any): User {
  // Immediately validate and transform
  if (!isValidUser(rawData)) {
    throw new Error('Invalid user data');
  }
  // Now type-safe
  return rawData as User;
}
```

---

## Unused Code Removal Policy

### Policy Statement

**All unused variables, functions, imports, and code must be removed immediately.**

### Rationale

Unused code:

1. **Increases Bundle Size:** Shipped to production unnecessarily
2. **Confuses Maintainers:** Unclear what's actually used
3. **Security Risk:** Unused code may contain vulnerabilities
4. **Build Performance:** Slower compilation and bundling
5. **Code Review Burden:** Noise in diffs and reviews

### Detection and Removal

#### Automated Detection

```bash
# Frontend
npm run lint

# TypeScript compiler
npx tsc --noUnusedLocals --noUnusedParameters

# Rust
cargo clippy -- -W unused
```

#### ESLint Configuration

```json
{
  "rules": {
    "@typescript-eslint/no-unused-vars": [
      "error",
      {
        "vars": "all",
        "args": "after-used",
        "ignoreRestSiblings": false
      }
    ]
  }
}
```

### Exceptions

#### 1. Intentional Unused Parameters

```typescript
// Use underscore prefix for intentionally unused parameters
function onClick(_event: MouseEvent, data: Data): void {
  // 'event' not used but required by interface
  processData(data);
}

// Or use rest parameters
function logMessage(message: string, ..._metadata: unknown[]): void {
  console.log(message);
}
```

#### 2. Exported Public API

```typescript
// Exported but unused within file
export function publicApi(): void {
  // Used by consumers, not internally
}
```

#### 3. Type-Only Imports (Must use type keyword)

```typescript
// Correct: Type-only imports
import type { User, Session } from './types';

// WRONG: Regular import for types
import { User, Session } from './types';
```

---

## Hook Dependency Management

### Policy Statement

**All React hooks must declare complete and accurate dependency arrays.**

### Rationale

Missing dependencies cause:

1. **Stale Closures:** Variables have outdated values
2. **Memory Leaks:** Effects not cleaned up properly
3. **Infinite Loops:** Effects triggering themselves
4. **Incorrect Renders:** Components not updating when they should
5. **Debugging Nightmares:** Non-deterministic behavior

### Complete Dependency Rules

#### ✅ DO: Include All Dependencies

```typescript
// Correct: All dependencies listed
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

### useCallback Dependencies

```typescript
// Correct: Include callback dependencies
function SearchBox({ onSearch, category }: Props) {
  const handleSubmit = useCallback((query: string) => {
    onSearch(query, category); // Uses both onSearch and category
  }, [onSearch, category]); // Both must be in dependencies

  return <input onSubmit={handleSubmit} />;
}
```

### useMemo Dependencies

```typescript
// Correct: Memoize expensive calculations
function DataTable({ data, sortBy, filterBy }: Props) {
  const processedData = useMemo(() => {
    return data
      .filter(item => matches(item, filterBy))
      .sort((a, b) => compare(a, b, sortBy));
  }, [data, sortBy, filterBy]); // All referenced values

  return <Table data={processedData} />;
}
```

### Stable References

```typescript
// Problem: Object recreated every render
function Parent() {
  const config = { api: '/api', timeout: 5000 }; // New object each render!
  return <Child config={config} />;
}

// Solution: useMemo for stable reference
function Parent() {
  const config = useMemo(
    () => ({ api: '/api', timeout: 5000 }),
    [] // Only create once
  );
  return <Child config={config} />;
}

// Better: Define outside component
const DEFAULT_CONFIG = { api: '/api', timeout: 5000 };

function Parent() {
  return <Child config={DEFAULT_CONFIG} />;
}
```

### ESLint Enforcement

```json
{
  "rules": {
    "react-hooks/rules-of-hooks": "error",
    "react-hooks/exhaustive-deps": "error"
  }
}
```

### Documented Exceptions

```typescript
// Only disable with clear documentation
useEffect(() => {
  // This should only run once on mount
  initializeApp();

  // eslint-disable-next-line react-hooks/exhaustive-deps
  // Reason: Intentionally only run on mount, not when config changes
  // Changing config requires app restart
}, []);
```

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

## Documentation Requirements

### TypeScript Documentation

```typescript
/**
 * Fetches user data from the database
 *
 * @param userId - Unique identifier for the user
 * @returns Promise resolving to User object
 * @throws {DatabaseError} If database connection fails
 * @throws {NotFoundError} If user does not exist
 *
 * @example
 * ```typescript
 * const user = await getUser('user-123');
 * console.log(user.name);
 * ```
 */
async function getUser(userId: string): Promise<User> {
  // Implementation
}
```

### Public API Documentation

All exported functions, classes, and interfaces must have:
- Purpose description
- Parameter documentation
- Return value documentation
- Error conditions
- Usage examples

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

## Document Control

**Version History:**

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 1.0 | 2025-12-27 | Initial release | Enterprise Agent 12 |

**Review Schedule:** Quarterly

**Next Review:** 2026-03-27

---

*This document is authoritative for all RustyDB development.*
*Deviations require written approval from Engineering Leadership.*
