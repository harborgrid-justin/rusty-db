# RustyDB v0.6.0 - Development Overview

**Version**: 0.6.0
**Release**: $856M Enterprise Server
**Last Updated**: 2025-12-28
**Target Audience**: Engineering Teams

---

## Table of Contents

1. [Introduction](#introduction)
2. [Prerequisites](#prerequisites)
3. [Environment Setup](#environment-setup)
4. [Project Architecture](#project-architecture)
5. [Development Workflow](#development-workflow)
6. [IDE Configuration](#ide-configuration)
7. [Quick Start Guide](#quick-start-guide)
8. [Common Commands](#common-commands)
9. [Troubleshooting](#troubleshooting)

---

## Introduction

RustyDB is an enterprise-grade, Oracle-compatible database management system written in Rust. This guide provides comprehensive instructions for setting up your development environment and contributing to the project.

### What You'll Build

- **Database Engine**: Page-based storage, MVCC transactions, query optimizer
- **Enterprise Features**: Clustering, replication, security, backup/recovery
- **Specialized Engines**: Graph, document, spatial, ML, in-memory analytics
- **APIs**: REST, GraphQL, Node.js adapter
- **Tools**: CLI client, web frontend, monitoring dashboards

### Release v0.6.0 Highlights

- 14-agent parallel campaign completion
- Enterprise database deployment
- Complete audit backend linting
- Parallel agents v0.6 integration
- Production-ready security and performance

---

## Prerequisites

### Required Software

| Software | Version | Purpose |
|----------|---------|---------|
| **Rust** | 1.70+ (1.92.0 recommended) | Primary programming language |
| **Cargo** | Included with Rust | Package manager and build tool |
| **Git** | Latest stable | Version control |
| **Linux/macOS/Windows** | - | Cross-platform support |

### Recommended Tools

| Tool | Purpose | Installation |
|------|---------|--------------|
| **rust-analyzer** | LSP for IDE support | Via IDE extension |
| **cargo-watch** | Auto-recompile on changes | `cargo install cargo-watch` |
| **cargo-audit** | Security vulnerability scanning | `cargo install cargo-audit` |
| **cargo-flamegraph** | Performance profiling | `cargo install flamegraph` |
| **cargo-edit** | Dependency management | `cargo install cargo-edit` |
| **cargo-tarpaulin** | Code coverage | `cargo install cargo-tarpaulin` |

### System Requirements

**Minimum**:
- CPU: 4 cores
- RAM: 8 GB
- Disk: 20 GB free space

**Recommended**:
- CPU: 8+ cores
- RAM: 16+ GB
- Disk: 50+ GB SSD
- Network: High-speed for cluster testing

---

## Environment Setup

### 1. Install Rust

**Linux/macOS**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

**Windows**:
Download and run: https://rustup.rs/

**Verify Installation**:
```bash
rustc --version  # Should show 1.70 or higher
cargo --version  # Should match rustc version
```

### 2. Clone Repository

```bash
git clone https://github.com/harborgrid-justin/rusty-db.git
cd rusty-db
```

**Current Branch**: `claude/centralize-enterprise-docs-JECKH`
**Main Branch**: (to be determined)

### 3. Environment Configuration

Create a `.env` file (optional):
```bash
# Database configuration
RUSTYDB_DATA_DIR=./data
RUSTYDB_PORT=5432
RUSTYDB_LOG_LEVEL=info

# Development settings
RUST_BACKTRACE=1
RUST_LOG=rusty_db=debug
```

### 4. Verify Setup

```bash
# Check compilation without building
cargo check

# Run quick test
cargo test --lib common

# Build debug version
cargo build

# Verify binaries
ls -la target/debug/rusty-db-*
```

---

## Project Architecture

### Directory Structure

```
rusty-db/
├── src/                      # Source code
│   ├── lib.rs               # Library entry point
│   ├── main.rs              # Server binary
│   ├── cli.rs               # CLI client binary
│   ├── error.rs             # Unified error types
│   ├── common.rs            # Shared types and traits
│   │
│   ├── Core Foundation Layer
│   ├── storage/             # Storage engine
│   ├── buffer/              # Buffer pool manager
│   ├── memory/              # Memory management
│   ├── io/                  # Async I/O layer
│   │
│   ├── Transaction Layer
│   ├── transaction/         # MVCC, WAL, locks
│   │
│   ├── Query Processing
│   ├── parser/              # SQL parsing
│   ├── execution/           # Query execution
│   ├── optimizer_pro/       # Advanced optimization
│   ├── index/               # Index structures
│   ├── simd/                # SIMD operations
│   │
│   ├── Networking & API
│   ├── network/             # TCP server
│   ├── api/                 # REST/GraphQL APIs
│   ├── pool/                # Connection pooling
│   │
│   ├── Enterprise Features
│   ├── security/            # 10 security modules
│   ├── security_vault/      # TDE, masking, VPD
│   ├── clustering/          # Raft, sharding
│   ├── rac/                 # Cache Fusion
│   ├── replication/         # Sync/async replication
│   ├── backup/              # Backup/recovery
│   ├── monitoring/          # Metrics, health checks
│   │
│   └── Specialized Engines
│       ├── graph/           # Graph database
│       ├── document_store/  # JSON/BSON store
│       ├── spatial/         # Geospatial
│       ├── ml/              # Machine learning
│       └── inmemory/        # In-memory analytics
│
├── tests/                   # Integration tests
├── benches/                 # Benchmarks
├── examples/                # Example code
├── docs/                    # Documentation
├── release/                 # Release documentation
├── .scratchpad/             # Development coordination
├── Cargo.toml               # Project manifest
└── README.md                # Project overview
```

### Key Modules

**Core Foundation**:
- `error.rs`: Unified `DbError` enum with `thiserror`
- `common.rs`: Type aliases (`TransactionId`, `PageId`) and core traits

**Storage Layer**:
- Page-based storage (4KB pages)
- LSM trees, columnar storage
- Buffer pool with pluggable eviction (CLOCK, LRU, 2Q, LRU-K, LIRS, ARC)

**Transaction Layer**:
- MVCC with snapshot isolation
- Two-phase locking with deadlock detection
- Write-Ahead Logging (WAL)

**Query Processing**:
- SQL parser using `sqlparser` crate
- Query optimizer (cost-based)
- Parallel and vectorized execution

---

## Development Workflow

### Standard Workflow

1. **Create Feature Branch**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Write Code**
   - Follow Rust API guidelines
   - Write tests alongside code
   - Update documentation

3. **Run Checks**
   ```bash
   cargo fmt       # Format code
   cargo clippy    # Lint code
   cargo test      # Run tests
   cargo check     # Verify compilation
   ```

4. **Commit Changes**
   ```bash
   git add .
   git commit -m "[Module] Brief description"
   ```

5. **Push and Create PR**
   ```bash
   git push origin feature/my-feature
   ```

### Module Development

When adding a new module:

```bash
# 1. Create module directory
mkdir src/my_module

# 2. Create mod.rs
cat > src/my_module/mod.rs << 'EOF'
//! My module for doing X.

mod core;
mod types;
mod operations;

pub use core::*;
pub use types::*;
pub use operations::*;
EOF

# 3. Declare in lib.rs
echo "pub mod my_module;" >> src/lib.rs

# 4. Add tests
mkdir tests
touch tests/my_module_test.rs
```

### Module Refactoring

**Target Size**: <500 lines per file

**Process**:
1. Identify logical groupings
2. Create submodules
3. Preserve public APIs with re-exports
4. Update module-level documentation
5. Ensure tests still pass

---

## IDE Configuration

### VS Code

**Extensions**:
- rust-analyzer
- Even Better TOML
- Error Lens
- CodeLLDB (debugging)

**Settings** (`.vscode/settings.json`):
```json
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "rust-lang.rust-analyzer"
}
```

**Tasks** (`.vscode/tasks.json`):
Pre-configured tasks available:
- cargo build
- cargo build (release)
- cargo check
- cargo clippy
- cargo test
- cargo run
- cargo doc (open)
- Full CI Check

Run with: `Ctrl+Shift+P` → "Tasks: Run Task"

### IntelliJ IDEA / CLion

1. Install Rust plugin
2. Enable Clippy integration
3. Configure formatter on save
4. Set up run configurations

---

## Quick Start Guide

### 1. Build the Project

```bash
# Debug build (fast compilation, no optimization)
cargo build

# Release build (slow compilation, full optimization)
cargo build --release
```

**Output**:
- `target/debug/rusty-db-server` - Server binary (debug)
- `target/debug/rusty-db-cli` - CLI client (debug)
- `target/release/*` - Optimized binaries

### 2. Run the Database Server

```bash
# Debug mode
cargo run --bin rusty-db-server

# Release mode (production)
cargo run --release --bin rusty-db-server

# Custom configuration
RUSTYDB_PORT=5433 cargo run --bin rusty-db-server
```

**Default Configuration**:
- Port: 5432
- Data directory: `./data`
- Page size: 4096 bytes (4 KB)
- Buffer pool: 1000 pages (~4 MB)
- Max connections: 100

### 3. Run the CLI Client

```bash
# In a separate terminal
cargo run --bin rusty-db-cli

# Or connect to specific server
cargo run --bin rusty-db-cli -- --host localhost --port 5432
```

**Example Session**:
```sql
rusty-db> CREATE TABLE users (id INT, name VARCHAR(100), email VARCHAR(255));
rusty-db> INSERT INTO users VALUES (1, 'Alice', 'alice@example.com');
rusty-db> SELECT * FROM users;
rusty-db> UPDATE users SET name = 'Alice Smith' WHERE id = 1;
rusty-db> DELETE FROM users WHERE id = 1;
rusty-db> \q  -- Quit
```

### 4. Run Tests

```bash
# All tests
cargo test

# Specific module
cargo test storage::
cargo test transaction::
cargo test security::

# With output
cargo test -- --nocapture

# Single test
cargo test test_buffer_pool

# Integration tests only
cargo test --test integration_*
```

### 5. Run Benchmarks

```bash
# All benchmarks
cargo bench

# Specific benchmark
cargo bench buffer_pool
```

---

## Common Commands

### Build Commands

```bash
# Fast syntax check (no binary)
cargo check

# Build debug
cargo build

# Build release (optimized)
cargo build --release

# Build specific binary
cargo build --bin rusty-db-server
cargo build --bin rusty-db-cli

# Build with features
cargo build --features simd
cargo build --features io_uring
```

### Test Commands

```bash
# All tests
cargo test

# Module tests
cargo test storage::
cargo test transaction::mvcc

# Single test
cargo test test_create_table

# Tests with output
cargo test -- --nocapture

# Tests in parallel (default)
cargo test

# Tests serially
cargo test -- --test-threads=1

# Ignored tests
cargo test -- --ignored

# Documentation tests
cargo test --doc
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy

# Fail on warnings
cargo clippy -- -D warnings

# Auto-fix issues
cargo clippy --fix

# Security audit
cargo audit
```

### Documentation

```bash
# Generate and open docs
cargo doc --open

# Generate without opening
cargo doc

# Document private items
cargo doc --document-private-items
```

### Maintenance

```bash
# Clean build artifacts
cargo clean

# Update dependencies
cargo update

# Add dependency
cargo add tokio

# Remove dependency
cargo rm old-crate

# Upgrade dependencies
cargo upgrade
```

### Development Utilities

```bash
# Watch and auto-rebuild on changes
cargo watch -x check
cargo watch -x test
cargo watch -x run

# Profile with flamegraph
cargo flamegraph --bin rusty-db-server

# Code coverage
cargo tarpaulin --out Html
open tarpaulin-report.html
```

---

## Troubleshooting

### Build Errors

**Missing dependencies**:
```bash
cargo update
cargo clean && cargo build
```

**Conflicting versions**:
```bash
cargo clean
rm Cargo.lock
cargo build
```

**Out of memory**:
```bash
# Use fewer parallel jobs
cargo build -j 2
```

### Test Failures

**Flaky tests**:
- Check for race conditions
- Use deterministic randomness (seeded)
- Ensure proper cleanup

**Timeout issues**:
- Increase timeout in test
- Check for deadlocks
- Run with `--test-threads=1`

### Performance Issues

**Slow compilation**:
- Use `cargo check` instead of `cargo build`
- Reduce generic complexity
- Split large files
- Enable incremental compilation

**Runtime performance**:
- Always use `--release` for benchmarks
- Profile first (flamegraph)
- Check for allocations (heaptrack)

### IDE Issues

**rust-analyzer slow**:
- Exclude `target/` directory
- Disable unused features
- Increase memory limit
- Restart language server

**Outdated completions**:
```bash
cargo clean
# Restart rust-analyzer
```

---

## Environment Information

### Current Setup (Dev Container)

- **Rust Version**: 1.92.0 (stable)
- **Cargo Version**: 1.92.0
- **Platform**: x86_64-unknown-linux-gnu
- **OS**: Ubuntu 24.04.3 LTS

### Installed Components

- ✅ rustc (Rust compiler)
- ✅ cargo (Rust package manager)
- ✅ clippy (Rust linter)
- ✅ rustfmt (Rust formatter)
- ✅ rust-docs (Offline documentation)

### Additional Tools Installed

- **cargo-edit** - Dependency management
- **cargo-watch** - File watching and auto-rebuild
- **cargo-audit** - Security scanning

---

## Next Steps

1. Review [BUILD_INSTRUCTIONS.md](./BUILD_INSTRUCTIONS.md) for detailed build procedures
2. Study [CODE_STANDARDS.md](./CODE_STANDARDS.md) for coding conventions
3. Check [SQL_COMPLIANCE.md](./SQL_COMPLIANCE.md) for SQL feature support
4. Explore [CONTRIBUTING.md](./CONTRIBUTING.md) for contribution guidelines

---

## Resources

### Official Documentation

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)

### Database Resources

- [CMU Database Systems Course](https://15445.courses.cs.cmu.edu/)
- [PostgreSQL Internals](https://www.postgresql.org/docs/current/internals.html)
- Database Internals (book by Alex Petrov)

### RustyDB Documentation

- Architecture: `docs/ARCHITECTURE.md`
- Security: `docs/SECURITY_ARCHITECTURE.md`
- AI Assistant Guidance: `CLAUDE.md`

---

**Welcome to the RustyDB development team!**

For questions or issues, please check the GitHub Issues or Discussions.
