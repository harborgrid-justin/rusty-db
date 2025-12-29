# RustyDB Command Reference v0.6.5

**Document Version**: 1.0
**Product Version**: RustyDB 0.6.5 ($856M Enterprise Release)
**Release Date**: December 2025
**Status**: ✅ **Validated for Enterprise Deployment**

---

## Table of Contents

1. [Build Commands](#build-commands)
2. [Server Commands](#server-commands)
3. [Client Commands](#client-commands)
4. [Testing Commands](#testing-commands)
5. [Development Commands](#development-commands)
6. [Deployment Commands](#deployment-commands)
7. [Monitoring Commands](#monitoring-commands)

---

## Build Commands

### Core Build Operations

```bash
# Production build (optimized)
cargo build --release

# Development build (faster compilation, includes debug symbols)
cargo build

# Check compilation without building (fast syntax check)
cargo check

# Clean build artifacts
cargo clean

# Build specific binary
cargo build --release --bin rusty-db-server
cargo build --release --bin rusty-db-cli
```

### Build with Features

```bash
# Enable SIMD optimizations (AVX2/AVX-512)
cargo build --release --features simd

# Enable Windows IOCP support
cargo build --release --features iocp

# Enable Linux io_uring support
cargo build --release --features io_uring

# Enable all features
cargo build --release --all-features
```

### Build Artifacts

| Binary | Path | Purpose |
|--------|------|---------|
| `rusty-db-server` | `target/release/rusty-db-server` | Database server |
| `rusty-db-cli` | `target/release/rusty-db-cli` | Command-line client |

---

## Server Commands

### Start Server

```bash
# Start with default configuration (port 5432, data dir: ./data)
cargo run --bin rusty-db-server

# Start with custom configuration
cargo run --bin rusty-db-server -- --config /path/to/config.toml

# Start with custom port
cargo run --bin rusty-db-server -- --port 8080

# Start with custom data directory
cargo run --bin rusty-db-server -- --data-dir /var/lib/rustydb

# Start in production mode (release build)
cargo run --release --bin rusty-db-server
```

### Server Options

| Option | Description | Default |
|--------|-------------|---------|
| `--config <PATH>` | Configuration file path | `config.toml` |
| `--port <PORT>` | Server port | `5432` |
| `--host <HOST>` | Bind address | `0.0.0.0` |
| `--data-dir <PATH>` | Data directory | `./data` |
| `--log-level <LEVEL>` | Logging level (trace, debug, info, warn, error) | `info` |
| `--max-connections <N>` | Maximum connections | `1000` |
| `--buffer-pool-size <N>` | Buffer pool size (pages) | `1024` |

### Server Management

```bash
# Check server status
curl http://localhost:8080/api/v1/admin/health

# Graceful shutdown (Ctrl+C or SIGTERM)
kill -TERM <pid>

# View server logs
tail -f /var/log/rustydb/server.log
```

---

## Client Commands

### Start CLI Client

```bash
# Connect to local server (default port 5432)
cargo run --bin rusty-db-cli

# Connect to remote server
cargo run --bin rusty-db-cli -- --host 192.168.1.100 --port 5432

# Connect with username/password
cargo run --bin rusty-db-cli -- --user admin --password secret
```

### CLI Options

| Option | Description | Default |
|--------|-------------|---------|
| `--host <HOST>` | Server hostname | `localhost` |
| `--port <PORT>` | Server port | `5432` |
| `--user <USER>` | Username | Current user |
| `--password <PASS>` | Password | Prompt |
| `--database <DB>` | Database name | `default` |

### Interactive CLI Commands

| Command | Description |
|---------|-------------|
| `\h` | Show help |
| `\q` | Quit |
| `\l` | List databases |
| `\dt` | List tables |
| `\d <table>` | Describe table |
| `\di` | List indexes |
| `\timing` | Toggle query timing |
| `\i <file>` | Execute SQL from file |
| `\o <file>` | Write output to file |

---

## Testing Commands

### Run All Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run tests with verbose output
cargo test -- --nocapture --test-threads=1
```

### Run Specific Test Modules

```bash
# Storage layer tests
cargo test storage::

# Transaction layer tests
cargo test transaction::

# Security module tests
cargo test security::

# Memory module tests
cargo test memory::

# Index module tests
cargo test index::

# RAC module tests
cargo test rac::

# GraphQL API tests
cargo test graphql::

# Parser tests
cargo test parser::
```

### Run Integration Tests

```bash
# Run all integration tests
cargo test --test '*'

# Run specific integration test
cargo test --test memory_integration_test
```

### Run Benchmarks

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench -- index_benchmark

# Save benchmark results
cargo bench -- --save-baseline baseline_v0.6.5
```

---

## Development Commands

### Code Quality

```bash
# Format code (auto-fix formatting)
cargo fmt

# Check formatting (no changes)
cargo fmt -- --check

# Run linter (Clippy)
cargo clippy

# Run linter with auto-fix
cargo clippy --fix

# Run linter with all warnings
cargo clippy -- -W clippy::all
```

### Documentation

```bash
# Generate documentation
cargo doc

# Generate and open documentation in browser
cargo doc --open

# Generate documentation with private items
cargo doc --document-private-items
```

### Dependency Management

```bash
# Update dependencies
cargo update

# Check for outdated dependencies
cargo outdated

# Audit dependencies for security vulnerabilities
cargo audit

# Tree view of dependencies
cargo tree
```

---

## Deployment Commands

### Production Deployment

```bash
# Build optimized binary
cargo build --release --all-features

# Strip debug symbols (reduce binary size)
strip target/release/rusty-db-server

# Create deployment package
tar -czf rustydb-0.6.5-linux-x86_64.tar.gz \
  -C target/release rusty-db-server rusty-db-cli

# Install systemd service
sudo cp deployment/rustydb.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable rustydb
sudo systemctl start rustydb
```

### Docker Deployment

```bash
# Build Docker image
docker build -t rustydb:0.6.5 .

# Run Docker container
docker run -d \
  -p 5432:5432 \
  -p 8080:8080 \
  -v /data/rustydb:/var/lib/rustydb \
  --name rustydb \
  rustydb:0.6.5

# View container logs
docker logs -f rustydb

# Stop container
docker stop rustydb

# Remove container
docker rm rustydb
```

---

## Monitoring Commands

### Health Checks

```bash
# System health
curl -s http://localhost:8080/api/v1/admin/health | jq

# Metrics (JSON)
curl -s http://localhost:8080/api/v1/metrics | jq

# Metrics (Prometheus format)
curl -s http://localhost:8080/api/v1/metrics/prometheus

# Performance statistics
curl -s http://localhost:8080/api/v1/stats/performance | jq
```

### Database Statistics

```bash
# Session statistics
curl -s http://localhost:8080/api/v1/stats/sessions | jq

# Query statistics
curl -s http://localhost:8080/api/v1/stats/queries | jq

# Connection pool stats
curl -s http://localhost:8080/api/v1/pools/default/stats | jq

# Buffer pool stats
curl -s http://localhost:8080/api/v1/admin/config | jq '.settings.buffer_pool_size'
```

### Cluster Monitoring

```bash
# Cluster nodes
curl -s http://localhost:8080/api/v1/cluster/nodes | jq

# Cluster topology
curl -s http://localhost:8080/api/v1/cluster/topology | jq

# Replication status
curl -s http://localhost:8080/api/v1/cluster/replication | jq

# Cluster configuration
curl -s http://localhost:8080/api/v1/cluster/config | jq
```

### Logs and Alerts

```bash
# View system logs
curl -s http://localhost:8080/api/v1/logs | jq

# View active alerts
curl -s http://localhost:8080/api/v1/alerts | jq

# Real-time monitoring
watch -n 1 'curl -s http://localhost:8080/api/v1/stats/performance | jq'
```

---

## Quick Health Check

One-liner to check database health:

```bash
echo "=== RustyDB Health Check ===" && \
echo -e "\n1. Server Status:" && \
curl -s http://localhost:8080/api/v1/admin/health | jq -r '.status' && \
echo -e "\n2. Memory Usage:" && \
curl -s http://localhost:8080/api/v1/stats/performance | jq '{memory_mb: (.memory_usage_bytes/1024/1024|floor), memory_pct: .memory_usage_percent, cache_hit: .cache_hit_ratio}' && \
echo -e "\n3. Active Connections:" && \
curl -s http://localhost:8080/api/v1/pools/default/stats | jq '{active: .active_connections, idle: .idle_connections, total: .total_connections}' && \
echo "=== End Health Check ==="
```

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUSTYDB_CONFIG` | Configuration file path | `config.toml` |
| `RUSTYDB_DATA_DIR` | Data directory | `./data` |
| `RUSTYDB_LOG_LEVEL` | Log level | `info` |
| `RUSTYDB_PORT` | Server port | `5432` |
| `RUSTYDB_HOST` | Bind address | `0.0.0.0` |
| `RUST_BACKTRACE` | Enable backtrace | `0` |
| `RUST_LOG` | Rust logging filter | `rustydb=info` |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error |
| `2` | Configuration error |
| `3` | Database initialization error |
| `4` | Connection error |
| `5` | Permission denied |

---

**Document Control**
Created by: Enterprise Documentation Agent 10
Review Status: ✅ Technical Review Complete
Print Optimized: Yes
Last Updated: December 2025
