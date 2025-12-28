# RustyDB v0.6.0 - Quick Troubleshooting Guide

**Version**: 0.6.0 | **Updated**: December 28, 2025

---

## Quick Diagnostics

### 1-Minute Health Check
```bash
# Run this script for instant diagnosis
echo "=== RustyDB Health Check ==="
echo "1. Process Status:"
ps aux | grep rusty-db-server | grep -v grep || echo "❌ Server not running"

echo -e "\n2. Ports:"
nc -zv localhost 5432 2>&1 | grep -q succeeded && echo "✅ PostgreSQL port open" || echo "❌ PostgreSQL port closed"
nc -zv localhost 8080 2>&1 | grep -q succeeded && echo "✅ API port open" || echo "❌ API port closed"

echo -e "\n3. API Health:"
curl -s http://localhost:8080/api/v1/health | jq -r '.status' 2>/dev/null || echo "❌ API not responding"

echo -e "\n4. Memory Usage:"
curl -s http://localhost:8080/api/v1/stats/performance 2>/dev/null | jq '{memory_mb: (.memory_usage_bytes/1024/1024|floor), memory_pct: .memory_usage_percent}' || echo "❌ Cannot get memory stats"

echo "=== End Health Check ==="
```

---

## Common Issues

### Server Won't Start

**Issue**: Port 5432 already in use
```bash
# Diagnose
sudo lsof -i :5432

# Solutions
# Option 1: Stop conflicting service (e.g., PostgreSQL)
sudo systemctl stop postgresql

# Option 2: Change RustyDB port
export RUSTY_DB_PORT=5433
./builds/linux/rusty-db-server
```

**Issue**: Port 8080 already in use
```bash
# Diagnose
sudo lsof -i :8080

# Solutions
# Option 1: Kill process using port
sudo kill -9 $(sudo lsof -t -i:8080)

# Option 2: Change API port
export RUSTY_DB_API_PORT=8081
./builds/linux/rusty-db-server
```

**Issue**: Permission denied
```bash
# Make binary executable
chmod +x builds/linux/rusty-db-server
chmod +x builds/linux/rusty-db-cli

# Fix data directory permissions
sudo chown -R rustydb:rustydb /var/lib/rusty-db
sudo chmod 700 /var/lib/rusty-db/data
```

**Issue**: Cannot create data directory
```bash
# Create directories manually
mkdir -p data wal backups

# Or for production
sudo mkdir -p /var/lib/rusty-db/{data,wal,backups}
sudo chown -R rustydb:rustydb /var/lib/rusty-db
```

---

### Cannot Connect

**Issue**: Connection refused
```bash
# Check server is running
ps aux | grep rusty-db-server

# Check ports are listening
sudo netstat -tulpn | grep -E '5432|8080'

# Check firewall
sudo ufw status
sudo ufw allow 5432/tcp
sudo ufw allow 8080/tcp

# Test connectivity
nc -zv localhost 5432
curl http://localhost:8080/api/v1/health
```

**Issue**: Cannot connect from remote host
```bash
# Check server is binding to all interfaces
curl http://localhost:8080/api/v1/admin/config | jq '.settings.host'

# Should be "0.0.0.0" for remote access, not "127.0.0.1"

# Update config
[server]
host = "0.0.0.0"  # Listen on all interfaces

# Restart server
sudo systemctl restart rustydb
```

**Issue**: Timeout connecting
```bash
# Check network connectivity
ping <server_ip>

# Check firewall rules
sudo iptables -L -n

# Check server logs
sudo journalctl -u rustydb -n 50
```

---

### Performance Issues

**Issue**: Slow queries
```bash
# Check memory usage
curl http://localhost:8080/api/v1/stats/performance | jq '{memory_mb: (.memory_usage_bytes/1024/1024), cache_hit: .cache_hit_ratio}'

# Check buffer pool size
curl http://localhost:8080/api/v1/admin/config | jq '.settings.buffer_pool_size'

# Increase buffer pool
# Edit config:
[storage]
buffer_pool_size = 25000  # ~100 MB

# Restart server
sudo systemctl restart rustydb
```

**Issue**: High memory usage
```bash
# Check current usage
ps aux | grep rusty-db-server | awk '{print $6 " KB"}'

# Check stats
curl http://localhost:8080/api/v1/stats/performance | jq '.memory_usage_bytes'

# Reduce buffer pool if needed
[storage]
buffer_pool_size = 1000  # ~4 MB
```

**Issue**: Too many connections
```bash
# Check connection count
curl http://localhost:8080/api/v1/pools/default/stats | jq '{active: .active_connections, total: .total_connections}'

# Increase max connections
[server]
max_connections = 1000

# Restart server
sudo systemctl restart rustydb
```

---

### Data Issues

**Issue**: Table not found
```bash
# List all tables
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ tables { name } }"}'

# Check specific table
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"sql": "SELECT * FROM information_schema.tables"}'
```

**Issue**: SQL parsing error
```bash
# Check for security blocking
# RustyDB blocks potentially dangerous SQL patterns

# Common blocks:
# - VARCHAR keyword → Use TEXT instead
# - Multi-row INSERT → Use separate INSERTs
# - IN clause → Use OR conditions
# - TRUNCATE → Use DELETE FROM

# Example fixes:
# ❌ CREATE TABLE users (name VARCHAR(255))
# ✅ CREATE TABLE users (name TEXT)

# ❌ INSERT INTO users VALUES (1, 'a'), (2, 'b')
# ✅ INSERT INTO users VALUES (1, 'a')
#    INSERT INTO users VALUES (2, 'b')
```

**Issue**: No data returned
```bash
# Check table has data
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ count(table: \"users\") }"}'

# Check WHERE clause
# Some operators may not be implemented

# Use GraphQL for complex queries
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ queryTable(table: \"users\", limit: 10) { __typename ... on QuerySuccess { rows { id fields } } } }"}'
```

---

### Service Issues

**Issue**: Systemd service won't start
```bash
# Check service status
sudo systemctl status rustydb

# Check logs
sudo journalctl -u rustydb -n 100 --no-pager

# Check service file
sudo systemctl cat rustydb

# Common issues:
# - Wrong binary path in ExecStart
# - Missing directories
# - Permission issues

# Reload systemd after changes
sudo systemctl daemon-reload
sudo systemctl restart rustydb
```

**Issue**: Service crashes on startup
```bash
# Check logs
sudo journalctl -u rustydb -n 100 --no-pager

# Check binary
file /usr/local/bin/rusty-db-server
ldd /usr/local/bin/rusty-db-server

# Run manually to see errors
sudo -u rustydb /usr/local/bin/rusty-db-server
```

**Issue**: Service not enabled on boot
```bash
# Enable service
sudo systemctl enable rustydb

# Verify
sudo systemctl is-enabled rustydb
```

---

### API Issues

**Issue**: GraphQL queries fail
```bash
# Check GraphQL endpoint
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ __schema { types { name } } }"}'

# Check for schema errors
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ __type(name: \"QueryRoot\") { fields { name } } }"}'

# Common issues:
# - Missing __typename in union types
# - Wrong field names
# - Missing fragments for union types
```

**Issue**: REST API returns empty response
```bash
# Check content type
curl -v http://localhost:8080/api/v1/health

# Should return JSON
# If empty, endpoint may not be implemented

# Check available endpoints
curl http://localhost:8080/api/v1/
```

**Issue**: Authentication required
```bash
# Some endpoints require admin auth
# Current version may not enforce auth

# If auth is required:
curl -H "Authorization: Bearer <token>" \
  http://localhost:8080/api/v1/admin/config
```

---

### Build Issues

**Issue**: Cargo build fails
```bash
# Update Rust
rustup update

# Clean build
cargo clean
cargo build --release

# Check Rust version
rustc --version  # Should be 1.70+

# Check dependencies
cargo tree
```

**Issue**: Linking errors
```bash
# Install required libraries (Debian/Ubuntu)
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev

# CentOS/RHEL
sudo yum install -y gcc openssl-devel
```

**Issue**: Out of disk space during build
```bash
# Check disk space
df -h

# Clean cargo cache
cargo clean
rm -rf ~/.cargo/registry/cache
```

---

### Testing Issues

**Issue**: Tests fail
```bash
# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name -- --nocapture

# Run single-threaded
cargo test -- --test-threads=1

# Check test logs
RUST_LOG=debug cargo test
```

**Issue**: API tests fail
```bash
# Ensure server is running
ps aux | grep rusty-db-server

# Check server is healthy
curl http://localhost:8080/api/v1/health

# Run tests with verbose output
./graphql_curl_commands.sh 2>&1 | tee test_output.log
```

---

### Log Analysis

**View Recent Errors**
```bash
# Last 100 lines
sudo journalctl -u rustydb -n 100

# Only errors
sudo journalctl -u rustydb -p err

# Follow logs
sudo journalctl -u rustydb -f

# Since timestamp
sudo journalctl -u rustydb --since "1 hour ago"
sudo journalctl -u rustydb --since "2025-12-28 10:00:00"
```

**Common Error Patterns**
```bash
# Connection errors
sudo journalctl -u rustydb | grep -i "connection"

# Memory errors
sudo journalctl -u rustydb | grep -i "memory\|oom"

# Parse errors
sudo journalctl -u rustydb | grep -i "parse\|syntax"

# Permission errors
sudo journalctl -u rustydb | grep -i "permission\|denied"
```

---

### Emergency Recovery

**Server Unresponsive**
```bash
# Force kill
sudo pkill -9 rusty-db-server

# Or with systemd
sudo systemctl kill -s KILL rustydb

# Start fresh
sudo systemctl start rustydb
```

**Database Corruption**
```bash
# Stop server
sudo systemctl stop rustydb

# Backup current data
sudo cp -r /var/lib/rusty-db/data /var/lib/rusty-db/data.backup

# Restore from backup
sudo cp -r /var/lib/rusty-db/backups/latest /var/lib/rusty-db/data

# Start server
sudo systemctl start rustydb
```

**Reset to Clean State**
```bash
# WARNING: Deletes all data!

# Stop server
sudo systemctl stop rustydb

# Remove data
sudo rm -rf /var/lib/rusty-db/data/*
sudo rm -rf /var/lib/rusty-db/wal/*

# Start server (will initialize fresh)
sudo systemctl start rustydb
```

---

## Diagnostic Commands

### System Information
```bash
# OS version
uname -a
cat /etc/os-release

# Memory
free -h

# Disk space
df -h

# CPU
lscpu
nproc
```

### Network Diagnostics
```bash
# Test local connectivity
nc -zv localhost 5432
nc -zv localhost 8080

# Test remote connectivity
nc -zv <server_ip> 5432
nc -zv <server_ip> 8080

# DNS resolution
nslookup <hostname>
dig <hostname>

# Firewall status
sudo ufw status
sudo iptables -L -n
```

### Performance Diagnostics
```bash
# CPU usage
top -p $(pgrep rusty-db-server)

# Memory usage
ps aux | grep rusty-db-server

# Disk I/O
sudo iotop -p $(pgrep rusty-db-server)

# Network usage
sudo nethogs
```

---

## Debug Mode

### Enable Verbose Logging
```bash
# Environment variable
export RUST_LOG=debug
./builds/linux/rusty-db-server

# Or in systemd
sudo nano /etc/systemd/system/rustydb.service
# Add to [Service]:
Environment="RUST_LOG=debug"

sudo systemctl daemon-reload
sudo systemctl restart rustydb
```

### Trace Logging
```bash
# Maximum verbosity
export RUST_LOG=trace
./builds/linux/rusty-db-server
```

---

## Getting Help

### Collect Diagnostic Information
```bash
# Create diagnostic report
cat > diagnostic_report.txt << 'EOF'
=== RustyDB Diagnostic Report ===
Date: $(date)

1. Version:
EOF

./builds/linux/rusty-db-server --version >> diagnostic_report.txt 2>&1 || echo "Cannot get version" >> diagnostic_report.txt

cat >> diagnostic_report.txt << 'EOF'

2. Process Status:
EOF
ps aux | grep rusty-db >> diagnostic_report.txt

cat >> diagnostic_report.txt << 'EOF'

3. Port Status:
EOF
sudo netstat -tulpn | grep -E '5432|8080' >> diagnostic_report.txt

cat >> diagnostic_report.txt << 'EOF'

4. Recent Logs:
EOF
sudo journalctl -u rustydb -n 50 >> diagnostic_report.txt 2>&1 || echo "No systemd logs" >> diagnostic_report.txt

cat >> diagnostic_report.txt << 'EOF'

5. Configuration:
EOF
curl -s http://localhost:8080/api/v1/admin/config >> diagnostic_report.txt 2>&1 || echo "Cannot get config" >> diagnostic_report.txt

cat >> diagnostic_report.txt << 'EOF'

6. System Info:
EOF
uname -a >> diagnostic_report.txt
free -h >> diagnostic_report.txt
df -h >> diagnostic_report.txt

echo "Diagnostic report saved to diagnostic_report.txt"
```

---

## Quick Reference Card

```
ISSUE                          SOLUTION
─────────────────────────────────────────────────────
Port in use                    sudo lsof -i :5432
                               sudo systemctl stop postgresql

Permission denied              chmod +x builds/linux/rusty-db-server
                               sudo chown -R rustydb:rustydb /var/lib/rusty-db

Connection refused             ps aux | grep rusty-db-server
                               sudo ufw allow 5432/tcp

Slow queries                   Increase buffer_pool_size
                               Create indexes

High memory                    Reduce buffer_pool_size
                               Check for leaks in logs

Service won't start            sudo journalctl -u rustydb -n 100
                               sudo systemctl daemon-reload

GraphQL errors                 Check for union type handling
                               Use __typename and fragments

SQL parsing errors             Avoid VARCHAR, use TEXT
                               Check security blocks

Server unresponsive            sudo systemctl restart rustydb
                               sudo pkill -9 rusty-db-server

Database corruption            Restore from backup
                               sudo cp -r /backups/latest /var/lib/rusty-db/data

No data returned               Check table exists: { tables { name } }
                               Check row count: { count(table: "...") }
```

---

## Support Checklist

Before reporting an issue:

- [ ] Check server is running: `ps aux | grep rusty-db-server`
- [ ] Check ports are open: `nc -zv localhost 5432`
- [ ] Check API health: `curl http://localhost:8080/api/v1/health`
- [ ] Review logs: `sudo journalctl -u rustydb -n 100`
- [ ] Check version: `./builds/linux/rusty-db-server --version`
- [ ] Try restart: `sudo systemctl restart rustydb`
- [ ] Collect diagnostics: Run diagnostic report script above

---

**Troubleshooting Guide** | RustyDB v0.6.0 | Enterprise Database Server
