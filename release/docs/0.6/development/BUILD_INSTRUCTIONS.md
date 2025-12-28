# RustyDB v0.6.0 - Build Instructions

**Version**: 0.6.0
**Release**: $856M Enterprise Server
**Last Updated**: 2025-12-28

---

## Table of Contents

1. [Build Overview](#build-overview)
2. [Build Profiles](#build-profiles)
3. [Build Commands](#build-commands)
4. [Feature Flags](#feature-flags)
5. [Platform-Specific Builds](#platform-specific-builds)
6. [Testing Procedures](#testing-procedures)
7. [Benchmarking](#benchmarking)
8. [Packaging](#packaging)
9. [Deployment](#deployment)
10. [CI/CD Integration](#cicd-integration)

---

## Build Overview

RustyDB uses Cargo, Rust's built-in package manager and build system. The build process compiles all source code, links dependencies, and produces optimized binaries for the database server and CLI client.

### Build Artifacts

| Binary | Description | Size (Debug) | Size (Release) |
|--------|-------------|--------------|----------------|
| `rusty-db-server` | Database server | ~500 MB | ~50 MB |
| `rusty-db-cli` | CLI client | ~400 MB | ~40 MB |

### Build Time Estimates

| Build Type | First Build | Incremental |
|------------|-------------|-------------|
| Debug | 5-10 min | 30 sec - 2 min |
| Release | 15-30 min | 2-5 min |

**Note**: Times vary based on CPU, RAM, and whether dependencies are cached.

---

## Build Profiles

### Debug Profile (Default)

**Purpose**: Development and debugging

**Characteristics**:
- Fast compilation
- No optimizations
- Debug symbols included
- Assertions enabled
- Larger binary size

**Configuration** (`Cargo.toml`):
```toml
[profile.dev]
opt-level = 0          # No optimization
debug = true           # Include debug info
debug-assertions = true
overflow-checks = true
lto = false
panic = 'unwind'
incremental = true
codegen-units = 256
```

**Build Command**:
```bash
cargo build
```

**Output**: `target/debug/rusty-db-server`

---

### Release Profile

**Purpose**: Production deployment

**Characteristics**:
- Full optimizations
- No debug symbols
- Link-time optimization (LTO)
- Smaller binary size
- Maximum performance

**Configuration** (`Cargo.toml`):
```toml
[profile.release]
opt-level = 3          # Maximum optimization
debug = false          # No debug info
lto = true            # Link-time optimization
codegen-units = 1     # Better optimization, slower compile
panic = 'abort'       # Smaller binary
strip = true          # Strip symbols
```

**Build Command**:
```bash
cargo build --release
```

**Output**: `target/release/rusty-db-server`

---

### Test Profile

**Purpose**: Running tests

**Configuration**:
```toml
[profile.test]
opt-level = 1         # Some optimization for faster tests
debug = true
```

**Build Command**:
```bash
cargo test
```

---

### Bench Profile

**Purpose**: Performance benchmarks

**Configuration**:
```toml
[profile.bench]
opt-level = 3
lto = true
debug = false
```

**Build Command**:
```bash
cargo bench
```

---

## Build Commands

### Basic Builds

```bash
# Debug build (fast, no optimization)
cargo build

# Release build (slow, full optimization)
cargo build --release

# Check compilation without building binary
cargo check

# Check release compilation
cargo check --release
```

### Specific Binary Builds

```bash
# Build only the server
cargo build --bin rusty-db-server

# Build only the CLI
cargo build --bin rusty-db-cli

# Build both (default)
cargo build
```

### Clean Builds

```bash
# Clean all build artifacts
cargo clean

# Full rebuild
cargo clean && cargo build

# Clean and rebuild release
cargo clean && cargo build --release
```

### Parallel Builds

```bash
# Use all CPU cores (default)
cargo build

# Limit parallel jobs
cargo build -j 4

# Single-threaded build
cargo build -j 1
```

### Verbose Builds

```bash
# Show detailed build information
cargo build -v
cargo build --verbose

# Very verbose (show cargo internal operations)
cargo build -vv
```

---

## Feature Flags

RustyDB supports optional features that can be enabled at compile time.

### Available Features

| Feature | Description | Default | Build Impact |
|---------|-------------|---------|--------------|
| `simd` | SIMD optimizations (AVX2/AVX-512) | No | +10% compile time |
| `iocp` | Windows IOCP I/O support | No | Windows only |
| `io_uring` | Linux io_uring support | No | Linux only |

### Enable Features

```bash
# Enable SIMD optimizations
cargo build --features simd
cargo build --release --features simd

# Enable io_uring (Linux)
cargo build --features io_uring

# Enable multiple features
cargo build --features "simd,io_uring"

# Enable all features
cargo build --all-features

# Disable default features
cargo build --no-default-features
```

### Feature Flags in Cargo.toml

```toml
[features]
default = []
simd = ["packed_simd"]
io_uring = ["io-uring"]
iocp = ["windows-sys"]

[dependencies]
packed_simd = { version = "0.3", optional = true }
io-uring = { version = "0.6", optional = true }

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.48", optional = true }
```

---

## Platform-Specific Builds

### Linux

```bash
# Standard build
cargo build --release

# With io_uring support
cargo build --release --features io_uring

# With SIMD (requires AVX2 CPU)
cargo build --release --features simd
RUSTFLAGS="-C target-cpu=native" cargo build --release --features simd
```

**Platform Notes**:
- Best I/O performance with `io_uring` (kernel 5.1+)
- SIMD requires AVX2 or AVX-512 capable CPU

### macOS

```bash
# Standard build
cargo build --release

# With SIMD
cargo build --release --features simd
```

**Platform Notes**:
- Uses kqueue for async I/O
- SIMD requires Intel CPU with AVX2 or Apple Silicon with NEON

### Windows

```bash
# Standard build
cargo build --release

# With IOCP support
cargo build --release --features iocp
```

**Platform Notes**:
- Uses IOCP for async I/O
- May require Visual Studio Build Tools

### Cross-Compilation

```bash
# Install target
rustup target add x86_64-unknown-linux-musl

# Build for target
cargo build --release --target x86_64-unknown-linux-musl

# Common targets
cargo build --target x86_64-apple-darwin      # macOS
cargo build --target x86_64-pc-windows-msvc   # Windows
cargo build --target aarch64-unknown-linux-gnu # ARM64 Linux
```

---

## Testing Procedures

### Unit Tests

```bash
# Run all unit tests
cargo test

# Run tests for specific module
cargo test storage::
cargo test transaction::
cargo test security::

# Run single test
cargo test test_buffer_pool

# Run with output
cargo test -- --nocapture

# Run tests serially (for debugging)
cargo test -- --test-threads=1
```

### Integration Tests

```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test
cargo test --test storage_integration
cargo test --test transaction_integration

# Run integration tests with setup
cargo test --test integration_tests -- --test-threads=1
```

### Documentation Tests

```bash
# Test code examples in documentation
cargo test --doc

# Test specific module docs
cargo test --doc storage
```

### Test Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html

# Open coverage report
open tarpaulin-report.html

# Coverage with specific parameters
cargo tarpaulin --ignore-tests --out Html --output-dir coverage
```

### Test Organization

**Test Command Matrix**:

```bash
# By scope
cargo test                         # All tests
cargo test --lib                   # Library tests only
cargo test --bins                  # Binary tests only
cargo test --test integration      # Integration tests
cargo test --doc                   # Documentation tests

# By module
cargo test storage::               # Storage module
cargo test transaction::mvcc       # MVCC submodule

# By name pattern
cargo test create                  # All tests with 'create'
cargo test test_insert_            # Tests starting with 'test_insert_'

# By output
cargo test -- --nocapture          # Show println! output
cargo test -- --show-output        # Show test output

# By execution
cargo test -- --test-threads=1     # Serial execution
cargo test -- --test-threads=8     # 8 parallel threads

# Special tests
cargo test -- --ignored            # Run ignored tests
cargo test -- --include-ignored    # Run all including ignored
```

---

## Benchmarking

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench buffer_pool
cargo bench query_execution

# Run with baseline comparison
cargo bench -- --save-baseline my_baseline

# Compare against baseline
cargo bench -- --baseline my_baseline
```

### Benchmark Organization

**Benchmark Files** (`benches/`):
- `buffer_pool.rs` - Buffer pool performance
- `storage.rs` - Storage operations
- `query_execution.rs` - Query performance
- `transaction.rs` - Transaction throughput

**Example Benchmark**:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_buffer_pool(c: &mut Criterion) {
    let pool = BufferPool::new(1000);

    c.bench_function("pin_page", |b| {
        b.iter(|| {
            pool.pin_page(black_box(1))
        })
    });
}

criterion_group!(benches, benchmark_buffer_pool);
criterion_main!(benches);
```

### Performance Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile server
cargo flamegraph --bin rusty-db-server

# Profile specific benchmark
cargo flamegraph --bench buffer_pool

# Profile with release optimizations
cargo flamegraph --release --bin rusty-db-server
```

---

## Packaging

### Binary Packaging

```bash
# Build release binaries
cargo build --release

# Copy binaries
mkdir -p dist/bin
cp target/release/rusty-db-server dist/bin/
cp target/release/rusty-db-cli dist/bin/

# Create tarball
tar -czf rustydb-v0.6.0-linux-x86_64.tar.gz -C dist .

# Create zip (Windows)
zip -r rustydb-v0.6.0-windows-x86_64.zip dist/
```

### Debian Package

```bash
# Install cargo-deb
cargo install cargo-deb

# Create .deb package
cargo deb

# Output: target/debian/rustydb_0.6.0_amd64.deb
```

**Cargo.toml additions**:
```toml
[package.metadata.deb]
maintainer = "RustyDB Team <team@rustydb.io>"
copyright = "2025, RustyDB Team"
license-file = ["LICENSE", "4"]
extended-description = """\
RustyDB is an enterprise-grade database management system."""
depends = "$auto"
section = "database"
priority = "optional"
assets = [
    ["target/release/rusty-db-server", "usr/bin/", "755"],
    ["target/release/rusty-db-cli", "usr/bin/", "755"],
    ["README.md", "usr/share/doc/rustydb/", "644"],
]
```

### Docker Image

```bash
# Build Docker image
docker build -t rustydb:0.6.0 .

# Multi-stage Dockerfile
cat > Dockerfile << 'EOF'
# Build stage
FROM rust:1.92 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/rusty-db-server /usr/local/bin/
EXPOSE 5432
CMD ["rusty-db-server"]
EOF

# Build and push
docker build -t rustydb:0.6.0 .
docker tag rustydb:0.6.0 rustydb:latest
docker push rustydb:0.6.0
```

---

## Deployment

### Development Deployment

```bash
# Run from source (debug)
cargo run --bin rusty-db-server

# Run with custom config
cargo run --bin rusty-db-server -- --config config.toml

# Run with environment variables
RUSTYDB_PORT=5433 cargo run --bin rusty-db-server
```

### Production Deployment

```bash
# Build optimized binary
cargo build --release --features "simd,io_uring"

# Copy to deployment location
sudo cp target/release/rusty-db-server /usr/local/bin/
sudo chmod +x /usr/local/bin/rusty-db-server

# Create systemd service
sudo cat > /etc/systemd/system/rustydb.service << 'EOF'
[Unit]
Description=RustyDB Enterprise Database Server
After=network.target

[Service]
Type=simple
User=rustydb
WorkingDirectory=/var/lib/rustydb
ExecStart=/usr/local/bin/rusty-db-server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable rustydb
sudo systemctl start rustydb
```

### Container Deployment

```bash
# Run Docker container
docker run -d \
  --name rustydb \
  -p 5432:5432 \
  -v /var/lib/rustydb:/data \
  -e RUSTYDB_DATA_DIR=/data \
  rustydb:0.6.0

# Docker Compose
cat > docker-compose.yml << 'EOF'
version: '3.8'
services:
  rustydb:
    image: rustydb:0.6.0
    ports:
      - "5432:5432"
    volumes:
      - rustydb-data:/data
    environment:
      - RUSTYDB_DATA_DIR=/data
      - RUSTYDB_LOG_LEVEL=info
    restart: always

volumes:
  rustydb-data:
EOF

docker-compose up -d
```

---

## CI/CD Integration

### GitHub Actions

```yaml
name: Build and Test

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  build:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true

    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo index
      uses: actions/cache@v3
      with:
        path: ~/.cargo/git
        key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache target directory
      uses: actions/cache@v3
      with:
        path: target
        key: ${{ runner.os }}-target-${{ hashFiles('**/Cargo.lock') }}

    - name: Check formatting
      run: cargo fmt -- --check

    - name: Run clippy
      run: cargo clippy -- -D warnings

    - name: Build
      run: cargo build --verbose

    - name: Run tests
      run: cargo test --verbose

    - name: Build release
      run: cargo build --release --verbose

    - name: Upload artifacts
      uses: actions/upload-artifact@v3
      with:
        name: rustydb-binaries
        path: |
          target/release/rusty-db-server
          target/release/rusty-db-cli
```

### GitLab CI

```yaml
stages:
  - check
  - build
  - test
  - deploy

variables:
  CARGO_HOME: $CI_PROJECT_DIR/.cargo

cache:
  paths:
    - .cargo/
    - target/

check:
  stage: check
  script:
    - cargo fmt -- --check
    - cargo clippy -- -D warnings

build:
  stage: build
  script:
    - cargo build --release
  artifacts:
    paths:
      - target/release/rusty-db-server
      - target/release/rusty-db-cli
    expire_in: 1 week

test:
  stage: test
  script:
    - cargo test --verbose

deploy:
  stage: deploy
  script:
    - ./scripts/deploy.sh
  only:
    - main
```

### Local Pre-commit Hook

```bash
# Create pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Format check
echo "Checking formatting..."
cargo fmt -- --check

# Clippy
echo "Running clippy..."
cargo clippy -- -D warnings

# Tests
echo "Running tests..."
cargo test

echo "All checks passed!"
EOF

chmod +x .git/hooks/pre-commit
```

---

## Build Optimization Tips

### Faster Compilation

1. **Use `cargo check` during development**
   ```bash
   cargo watch -x check  # Auto-check on file changes
   ```

2. **Limit codegen units for faster debug builds**
   ```toml
   [profile.dev]
   codegen-units = 512  # More units = faster compile
   ```

3. **Use incremental compilation** (enabled by default)
   ```bash
   export CARGO_INCREMENTAL=1
   ```

4. **Install sccache for caching**
   ```bash
   cargo install sccache
   export RUSTC_WRAPPER=sccache
   ```

### Smaller Binaries

1. **Enable LTO and strip symbols**
   ```toml
   [profile.release]
   lto = true
   strip = true
   opt-level = "z"  # Optimize for size
   ```

2. **Use `cargo-bloat` to analyze binary size**
   ```bash
   cargo install cargo-bloat
   cargo bloat --release
   ```

3. **Remove unused dependencies**
   ```bash
   cargo install cargo-udeps
   cargo +nightly udeps
   ```

---

## Troubleshooting Build Issues

### Out of Memory

```bash
# Reduce parallel jobs
cargo build -j 2

# Increase swap space (Linux)
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### Linker Errors

```bash
# Install required linker (Linux)
sudo apt-get install build-essential

# Use faster linker (Linux)
sudo apt-get install lld
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
```

### Dependency Conflicts

```bash
# Update dependencies
cargo update

# Clean and rebuild
cargo clean
rm Cargo.lock
cargo build
```

---

**For more information, see [DEVELOPMENT_OVERVIEW.md](./DEVELOPMENT_OVERVIEW.md) and [CODE_STANDARDS.md](./CODE_STANDARDS.md).**
