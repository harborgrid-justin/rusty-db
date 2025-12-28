# RustyDB v0.6.0 - CLI Quick Reference

**Version**: 0.6.0 | **Updated**: December 28, 2025

---

## Server Commands

### Start Server
```bash
# Development mode (foreground)
cd /home/user/rusty-db
./builds/linux/rusty-db-server

# With custom config
./builds/linux/rusty-db-server --config /path/to/config.toml

# Production mode (systemd)
sudo systemctl start rustydb
```

### Stop Server
```bash
# Development mode
Ctrl+C

# Production mode
sudo systemctl stop rustydb
```

### Restart Server
```bash
sudo systemctl restart rustydb
```

### Check Server Status
```bash
# Check process
ps aux | grep rusty-db-server

# Check systemd status
sudo systemctl status rustydb

# Check ports
netstat -tulpn | grep -E '5432|8080'
nc -zv localhost 5432
nc -zv localhost 8080
```

---

## CLI Client Commands

### Connect to Database
```bash
# Connect to local server
./builds/linux/rusty-db-cli

# Connect to remote server
./builds/linux/rusty-db-cli --host 192.168.1.100 --port 5432

# Connect with connection string
./builds/linux/rusty-db-cli --url rustydb://localhost:5432/mydb
```

### Execute Single Command
```bash
# Run single query
./builds/linux/rusty-db-cli --command "SELECT version();"

# Run query and exit
./builds/linux/rusty-db-cli -c "SELECT * FROM users LIMIT 10;"
```

### Execute SQL File
```bash
# Run SQL script
./builds/linux/rusty-db-cli --file /path/to/script.sql

# Shorter form
./builds/linux/rusty-db-cli -f script.sql
```

### Interactive Mode
```bash
# Start interactive session
./builds/linux/rusty-db-cli

# At prompt
rusty-db> SELECT * FROM users;
rusty-db> \q  # Quit
```

---

## CLI Client Options

```bash
./builds/linux/rusty-db-cli [OPTIONS]

Options:
  -h, --host <HOST>         Server host (default: localhost)
  -p, --port <PORT>         Server port (default: 5432)
  -u, --url <URL>           Connection URL
  -c, --command <COMMAND>   Execute command and exit
  -f, --file <FILE>         Execute SQL file
  -o, --output <FORMAT>     Output format: table|json|csv (default: table)
  -v, --verbose             Verbose output
  --help                    Show help message
  --version                 Show version
```

---

## Build Commands

### Build Project
```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Build specific binary
cargo build --release --bin rusty-db-server
cargo build --release --bin rusty-db-cli
```

### Check Compilation
```bash
# Check without building
cargo check

# Check all targets
cargo check --all-targets
```

### Clean Build Artifacts
```bash
cargo clean
```

---

## Test Commands

### Run All Tests
```bash
cargo test

# With output
cargo test -- --nocapture

# Parallel execution (default)
cargo test

# Single-threaded
cargo test -- --test-threads=1
```

### Run Specific Tests
```bash
# Run storage tests
cargo test storage::

# Run transaction tests
cargo test transaction::

# Run security tests
cargo test security::

# Run specific test
cargo test test_mvcc_basic

# Run tests matching pattern
cargo test mvcc
```

### Run Benchmarks
```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench storage_bench
```

---

## Code Quality Commands

### Format Code
```bash
# Format all code
cargo fmt

# Check formatting (CI)
cargo fmt -- --check
```

### Run Linter
```bash
# Run clippy
cargo clippy

# Fix issues automatically
cargo clippy --fix

# Strict mode
cargo clippy -- -D warnings
```

### Documentation
```bash
# Generate documentation
cargo doc

# Open in browser
cargo doc --open

# Generate and open
cargo doc --open --no-deps
```

---

## Database Maintenance

### Backup Database
```bash
# Using REST API
curl -X POST http://localhost:8080/api/v1/admin/backup \
  -H "Content-Type: application/json" \
  -d '{"path": "/backups/backup-2025-12-28.db"}'

# Manual file copy (server must be stopped)
sudo systemctl stop rustydb
cp -r /var/lib/rusty-db/data /backups/data-2025-12-28
sudo systemctl start rustydb
```

### Restore Database
```bash
# Stop server
sudo systemctl stop rustydb

# Restore data
cp -r /backups/data-2025-12-28 /var/lib/rusty-db/data

# Start server
sudo systemctl start rustydb
```

### View Database Size
```bash
du -sh /var/lib/rusty-db/data
du -sh /var/lib/rusty-db/wal
```

---

## Log Management

### View Logs
```bash
# Development mode
# Logs appear in terminal

# Production mode (systemd)
sudo journalctl -u rustydb

# Follow logs (tail -f)
sudo journalctl -u rustydb -f

# Last 100 lines
sudo journalctl -u rustydb -n 100

# Since timestamp
sudo journalctl -u rustydb --since "2025-12-28 10:00:00"
sudo journalctl -u rustydb --since "1 hour ago"
sudo journalctl -u rustydb --since today

# Filter by priority
sudo journalctl -u rustydb -p err  # errors only
sudo journalctl -u rustydb -p warning  # warnings and above
```

### Log Rotation
```bash
# Configure in /etc/systemd/journald.conf
SystemMaxUse=1G
MaxRetentionSec=30day

# Apply changes
sudo systemctl restart systemd-journald
```

---

## Health Checks

### Server Health
```bash
# REST API
curl http://localhost:8080/api/v1/health

# Expected: {"status":"healthy","version":"0.6.0"}
```

### Memory Usage
```bash
# Using REST API
curl http://localhost:8080/api/v1/stats/performance | jq '.memory_usage_bytes'

# System memory
ps aux | grep rusty-db-server | awk '{print $6}'  # RSS in KB
```

### Connection Count
```bash
curl http://localhost:8080/api/v1/pools/default/stats | jq '{active: .active_connections, idle: .idle_connections}'
```

### Database Stats
```bash
# Table count
curl http://localhost:8080/api/v1/stats/queries | jq

# Metrics
curl http://localhost:8080/api/v1/metrics
```

---

## Performance Monitoring

### Metrics Endpoint
```bash
# JSON format
curl http://localhost:8080/api/v1/metrics | jq

# Prometheus format
curl http://localhost:8080/api/v1/metrics/prometheus
```

### Query Statistics
```bash
curl http://localhost:8080/api/v1/stats/queries | jq
```

### Session Statistics
```bash
curl http://localhost:8080/api/v1/stats/sessions | jq
```

### Performance Stats
```bash
curl http://localhost:8080/api/v1/stats/performance | jq
```

---

## Configuration Management

### View Configuration
```bash
curl http://localhost:8080/api/v1/admin/config | jq
```

### Update Configuration
```bash
# Edit config file
sudo nano /etc/rusty-db/config.toml

# Restart server
sudo systemctl restart rustydb
```

---

## Network Diagnostics

### Test Connectivity
```bash
# Test PostgreSQL port
nc -zv localhost 5432
telnet localhost 5432

# Test API port
nc -zv localhost 8080
curl http://localhost:8080/api/v1/health
```

### Check Listening Ports
```bash
# All ports
sudo netstat -tulpn | grep rusty

# Specific ports
sudo lsof -i :5432
sudo lsof -i :8080
```

### Firewall Configuration
```bash
# Allow ports
sudo ufw allow 5432/tcp
sudo ufw allow 8080/tcp

# Check status
sudo ufw status
```

---

## System Service Management

### Install Service
```bash
# Copy service file
sudo cp deploy/systemd/rustydb-single.service /etc/systemd/system/rustydb.service

# Reload systemd
sudo systemctl daemon-reload

# Enable on boot
sudo systemctl enable rustydb

# Start service
sudo systemctl start rustydb
```

### Uninstall Service
```bash
# Stop and disable
sudo systemctl stop rustydb
sudo systemctl disable rustydb

# Remove service file
sudo rm /etc/systemd/system/rustydb.service

# Reload systemd
sudo systemctl daemon-reload
```

---

## Troubleshooting Commands

### Port Already in Use
```bash
# Find process using port
sudo lsof -i :5432
sudo lsof -i :8080

# Kill process
sudo kill -9 <PID>

# Or stop conflicting service
sudo systemctl stop postgresql
```

### Permission Issues
```bash
# Make binary executable
chmod +x builds/linux/rusty-db-server
chmod +x builds/linux/rusty-db-cli

# Fix data directory permissions
sudo chown -R rustydb:rustydb /var/lib/rusty-db
sudo chmod 700 /var/lib/rusty-db/data
```

### Connection Refused
```bash
# Check server is running
ps aux | grep rusty-db-server

# Check ports are listening
sudo netstat -tulpn | grep -E '5432|8080'

# Check firewall
sudo ufw status
sudo iptables -L
```

### Database Corruption
```bash
# Stop server
sudo systemctl stop rustydb

# Check data integrity
cargo run --bin rusty-db-cli -- --file check_integrity.sql

# Restore from backup
cp -r /backups/data-2025-12-28 /var/lib/rusty-db/data

# Start server
sudo systemctl start rustydb
```

---

## Environment Variables

```bash
# Set data directory
export RUSTY_DB_DATA_DIR=/custom/data/path

# Set log level
export RUST_LOG=debug
export RUST_LOG=info
export RUST_LOG=error

# Set server port
export RUSTY_DB_PORT=5433

# Set API port
export RUSTY_DB_API_PORT=8081
```

---

## Quick Reference Card

```
SERVER COMMANDS
---------------
Start:   ./builds/linux/rusty-db-server
Stop:    Ctrl+C (dev) | sudo systemctl stop rustydb (prod)
Status:  sudo systemctl status rustydb

CLIENT COMMANDS
---------------
Connect: ./builds/linux/rusty-db-cli
Query:   ./builds/linux/rusty-db-cli -c "SELECT * FROM users;"
File:    ./builds/linux/rusty-db-cli -f script.sql

BUILD COMMANDS
--------------
Build:   cargo build --release
Test:    cargo test
Lint:    cargo clippy
Format:  cargo fmt

HEALTH CHECKS
-------------
Health:  curl http://localhost:8080/api/v1/health
Metrics: curl http://localhost:8080/api/v1/metrics
Logs:    sudo journalctl -u rustydb -f

PORTS
-----
PostgreSQL: 5432
REST API:   8080
GraphQL:    8080/graphql
WebSocket:  8080/ws
```

---

## Common Workflows

### Development Workflow
```bash
# 1. Build
cargo build --release

# 2. Run tests
cargo test

# 3. Start server
./builds/linux/rusty-db-server

# 4. Test connection
./builds/linux/rusty-db-cli -c "SELECT version();"
```

### Production Deployment
```bash
# 1. Build release binary
cargo build --release

# 2. Copy binaries
sudo cp target/release/rusty-db-server /usr/local/bin/
sudo cp target/release/rusty-db-cli /usr/local/bin/

# 3. Install service
sudo cp deploy/systemd/rustydb-single.service /etc/systemd/system/rustydb.service
sudo systemctl daemon-reload

# 4. Start service
sudo systemctl enable rustydb
sudo systemctl start rustydb

# 5. Verify
sudo systemctl status rustydb
curl http://localhost:8080/api/v1/health
```

### Debugging Workflow
```bash
# 1. Check logs
sudo journalctl -u rustydb -n 100

# 2. Check process
ps aux | grep rusty-db-server

# 3. Check ports
sudo netstat -tulpn | grep -E '5432|8080'

# 4. Test connectivity
nc -zv localhost 5432
curl http://localhost:8080/api/v1/health

# 5. Check configuration
curl http://localhost:8080/api/v1/admin/config | jq
```

---

**CLI Reference** | RustyDB v0.6.0 | Enterprise Database Server
