# GitHub Issues to Create - RustyDB

## Date: 2025-12-13
## Session: claude/enable-all-api-features-0136igGpj9vcQBJqoD7CuF9Q

---

## Issue #1: Optimizer Handler Return Type Incompatibility

**Title:** Fix optimizer_handlers return type to use ApiResult instead of Result<Response>

**Labels:** bug, api, high-priority

**Description:**
The optimizer handlers in `src/api/rest/handlers/optimizer_handlers.rs` use `Result<Response>` from `crate::error::Result` instead of the standard `ApiResult` type expected by Axum handlers.

**Impact:**
12 optimizer endpoints are currently disabled:
- `/api/v1/optimizer/hints` (GET/POST)
- `/api/v1/optimizer/hints/active`
- `/api/v1/optimizer/hints/apply`
- `/api/v1/optimizer/hints/{id}`
- `/api/v1/optimizer/baselines` (GET/POST)
- `/api/v1/optimizer/baselines/{id}` (GET/PUT/DELETE)
- `/api/v1/optimizer/baselines/{id}/evolve`
- `/api/v1/optimizer/explain`
- `/api/v1/optimizer/explain-analyze`

**Fix Required:**
Change all handler function return types from `Result<Response>` to `ApiResult<Json<...>>` to match the Axum handler trait requirements.

**File:** `src/api/rest/handlers/optimizer_handlers.rs`

---

## Issue #2: CRITICAL - SQL Injection in FGAC Security Module

**Title:** [SECURITY] SQL injection vulnerability in FGAC predicate handling

**Labels:** security, critical, vulnerability

**Description:**
Security predicates are directly concatenated into SQL queries without parameterization in the Fine-Grained Access Control (FGAC) module.

**Location:** `src/security/fgac.rs:640-649`

**Risk:** CRITICAL - Attackers could bypass security controls and execute arbitrary SQL.

**Fix Required:**
- Use parameterized queries
- Validate predicates with `InjectionPreventionGuard`
- Implement whitelist-based predicate validation

---

## Issue #3: CRITICAL - SQL Injection in VPD Security Module

**Title:** [SECURITY] SQL injection vulnerability in VPD policy predicates

**Labels:** security, critical, vulnerability

**Description:**
Virtual Private Database (VPD) policy predicates are injected via `format!` macro without proper validation or sanitization.

**Location:** `src/security_vault/vpd.rs:438-462`

**Risk:** CRITICAL - Attackers could bypass row-level security and access unauthorized data.

**Fix Required:**
- Validate all predicates with `InjectionPreventionGuard`
- Use parameterized query construction
- Implement predicate syntax validation

---

## Issue #4: HIGH - Command Injection in DBMS_SCHEDULER

**Title:** [SECURITY] Command injection risk in DBMS_SCHEDULER argument parsing

**Labels:** security, high, vulnerability

**Description:**
The DBMS_SCHEDULER built-in procedure uses `split_whitespace()` for argument parsing, which is unsafe and could allow command injection.

**Location:** `src/procedures/builtins.rs:1444-1491`

**Risk:** HIGH - Attackers could execute arbitrary system commands.

**Fix Required:**
- Implement proper argument parsing with shell escaping
- Add command whitelisting
- Validate all arguments before execution

---

## Issue #5: GraphQL Interfaces Disabled Due to Trait Bounds

**Title:** Enable async_graphql Interface implementations for Node, Timestamped, Auditable

**Labels:** bug, graphql, enhancement

**Description:**
Three GraphQL interfaces are commented out due to trait bound issues with the `async_graphql` Interface derive macro.

**Location:** `src/api/graphql/types.rs:226-257`

**Disabled Interfaces:**
- `Node` - Base interface for all queryable nodes
- `Timestamped` - Interface for objects with timestamps
- `Auditable` - Interface for audit trail support

**Fix Required:**
Resolve trait bound conflicts with async_graphql's Interface derive macro. May require:
- Custom resolver implementations
- Type coercion adjustments
- async_graphql version update

---

## Issue #6: Feature Flags Declared But Not Gated

**Title:** Implement proper feature flag gating for simd and iocp features

**Labels:** enhancement, build-system

**Description:**
The `simd` and `iocp` features are declared in `Cargo.toml` but the code doesn't properly gate functionality behind these flags.

**Location:** `Cargo.toml`

**Impact:**
- Cannot selectively enable/disable SIMD optimizations
- Cannot selectively enable/disable Windows IOCP support
- Build size and dependencies not optimized

**Fix Required:**
Add `#[cfg(feature = "simd")]` and `#[cfg(feature = "iocp")]` attributes to relevant code sections.

---

## Issue #7: Pre-existing Test Compilation Errors (93 errors)

**Title:** Fix test compilation errors - missing types and trait bounds

**Labels:** bug, tests, tech-debt

**Description:**
93 test compilation errors exist in the test suite.

**Error Categories:**
1. **Missing Types:**
   - `SlotName`
   - `SlotType`
   - `SnapshotType`
   - Various other undefined types

2. **Trait Bound Issues:**
   - `bincode::Encode` not implemented
   - `bincode::Decode` not implemented

3. **Missing Struct Fields:**
   - `ColumnMetadata::index` field missing

**Fix Required:**
- Define missing types or import them correctly
- Implement missing trait bounds
- Add missing struct fields

---

## Issue #8: Default Authentication Disabled in Production Config

**Title:** [SECURITY] Enable authentication by default in production configuration

**Labels:** security, configuration, high

**Description:**
The default configuration has `enable_auth: false`, which means the database runs without authentication by default.

**Risk:** HIGH - Unauthorized access to database in production deployments.

**Fix Required:**
- Change default to `enable_auth: true`
- Require explicit opt-out for development environments
- Add startup warning when auth is disabled

---

## Issue #9: Hardcoded Admin Credentials

**Title:** [SECURITY] Remove hardcoded admin/admin credentials

**Labels:** security, critical, vulnerability

**Description:**
Default admin credentials are hardcoded in the authentication module.

**Location:** `src/security/auth.rs` (or similar)

**Risk:** CRITICAL - Default credentials are a well-known attack vector.

**Fix Required:**
- Remove hardcoded credentials
- Require admin password setup on first run
- Implement secure credential generation

---

## Issue #10: JWT Signature Validation Not Enforced

**Title:** [SECURITY] Implement proper JWT signature validation

**Labels:** security, high, authentication

**Description:**
JWT tokens may not be properly validated for signature authenticity.

**Risk:** HIGH - Token forgery could allow unauthorized access.

**Fix Required:**
- Implement proper JWT signature validation
- Use strong signing algorithms (RS256 or ES256)
- Validate all claims including expiration

---

## Summary

| Priority | Count | Issues |
|----------|-------|--------|
| CRITICAL | 3 | #2, #3, #9 |
| HIGH | 3 | #4, #8, #10 |
| MEDIUM | 4 | #1, #5, #6, #7 |

**Total Issues:** 10

---

*Generated: 2025-12-13*
