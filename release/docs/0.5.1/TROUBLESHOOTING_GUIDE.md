# RustyDB v0.5.1 Troubleshooting Guide

**Version**: 0.5.1
**Release Date**: December 25, 2025
**Last Updated**: 2025-12-27
**Document Type**: Enterprise Operations Manual

---

## Table of Contents

1. [Troubleshooting Overview](#troubleshooting-overview)
2. [Startup Issues](#startup-issues)
3. [Connection Issues](#connection-issues)
4. [Performance Issues](#performance-issues)
5. [Transaction Issues](#transaction-issues)
6. [Replication Issues](#replication-issues)
7. [Storage Issues](#storage-issues)
8. [API Issues](#api-issues)
9. [Error Messages Reference](#error-messages-reference)
10. [Support and Escalation](#support-and-escalation)

---

## Troubleshooting Overview

### Diagnostic Methodology

RustyDB follows a structured approach to troubleshooting:

```
Problem Identification
    ↓
Gather Information (logs, metrics, system state)
    ↓
Isolate Root Cause (eliminate possibilities)
    ↓
Apply Solution (targeted fix)
    ↓
Verify Resolution (test and monitor)
    ↓
Document (update runbooks)
```

### Common Issue Categories

| Category | Frequency | Severity | Typical Resolution Time |
|----------|-----------|----------|-------------------------|
| Connection Issues | High | Low-Medium | 5-30 minutes |
| Performance Degradation | Medium | Medium-High | 30 minutes - 4 hours |
| Transaction Deadlocks | Medium | Medium | 15-60 minutes |
| Startup Failures | Low | High | 15 minutes - 2 hours |
| Data Corruption | Very Low | Critical | 2-8 hours |
| Security Incidents | Low | High-Critical | 15 minutes - 4 hours |

### Log File Locations

RustyDB generates logs in the following locations:

```
data/logs/
├── rustydb.log                  # Main server log
├── security_audit.log           # Security events and audit trail
├── transaction.log              # Transaction lifecycle events
├── performance.log              # Performance metrics and slow queries
├── replication.log              # Replication events
├── error.log                    # Error-only log
└── debug.log                    # Debug-level logging (when enabled)
```

**Log Rotation**:
- Logs rotate daily at midnight
- Compressed logs kept for 30 days
- Critical errors always logged regardless of level

**View Recent Logs**:
```bash
# Last 100 lines of main log
tail -n 100 data/logs/rustydb.log

# Follow log in real-time
tail -f data/logs/rustydb.log

# Search for errors in last hour
grep ERROR data/logs/rustydb.log | tail -n 50

# View with journalctl (systemd)
sudo journalctl -u rustydb -f
```

### Diagnostic Tools

#### Built-in Diagnostics

**Health Check**:
```bash
# HTTP health endpoint
curl http://localhost:8080/health | jq

# Expected output
{
  "status": "healthy",
  "version": "0.5.1",
  "uptime": 3600,
  "connections": {"active": 5, "max": 100},
  "buffer_pool": {"size": 1000, "used": 421, "hit_rate": 0.95},
  "transactions": {"active": 2, "committed": 15234, "aborted": 12}
}
```

**System Diagnostics**:
```bash
# Generate diagnostic bundle
rustydb-admin diagnostics --output /tmp/diagnostics-$(date +%s).tar.gz

# Bundle includes:
# - All log files
# - Configuration dump
# - System metrics
# - Active connections
# - Transaction state
# - Buffer pool statistics
```

**Real-time Monitoring**:
```graphql
# GraphQL monitoring queries
query SystemStatus {
  serverInfo {
    version
    uptime
    activeConnections
    totalTransactions
  }
  systemMetrics {
    cpuUsage
    memoryUsage
    diskIO
    networkIO
  }
}
```

#### External Tools

**System Monitoring**:
```bash
# CPU and memory usage
htop

# Disk I/O statistics
iostat -x 1 10

# Network connections
netstat -tlnp | grep rusty-db

# Open files
lsof -p $(pidof rusty-db-server)
```

**Performance Profiling**:
```bash
# CPU profiling with perf
perf record -p $(pidof rusty-db-server) -g -- sleep 30
perf report

# Memory profiling (if built with jemalloc)
export MALLOC_CONF=prof:true,prof_prefix:rustydb_heap
```

---

## Startup Issues

### Server Won't Start

#### Problem: Server Fails to Start with No Error Message

**Symptoms**:
- Server process exits immediately
- No log output generated
- Return code non-zero

**Diagnostic Steps**:
```bash
# 1. Check if server binary exists and is executable
ls -lh target/release/rusty-db-server
# Should show: -rwxr-xr-x

# 2. Try running with verbose logging
RUST_LOG=debug ./target/release/rusty-db-server

# 3. Check system resources
df -h              # Disk space
free -h            # Available memory
ulimit -n          # File descriptor limit
```

**Common Causes and Solutions**:

1. **Insufficient Permissions**:
   ```bash
   # Check binary permissions
   chmod +x target/release/rusty-db-server

   # Check data directory permissions
   ls -ld data/
   chmod 700 data/
   chown $USER:$USER data/
   ```

2. **Missing Dependencies**:
   ```bash
   # Check linked libraries
   ldd target/release/rusty-db-server

   # Install missing libraries (Ubuntu/Debian)
   sudo apt-get install libssl-dev libpq-dev
   ```

3. **Insufficient Resources**:
   ```bash
   # Increase file descriptor limit
   ulimit -n 65535

   # Add to /etc/security/limits.conf for permanent fix
   echo "* soft nofile 65535" | sudo tee -a /etc/security/limits.conf
   echo "* hard nofile 65535" | sudo tee -a /etc/security/limits.conf
   ```

---

#### Problem: Port Already in Use

**Symptoms**:
- Error: "Address already in use"
- Server fails to bind to port 5432 or 8080

**Diagnostic Steps**:
```bash
# Check what's using the port
sudo lsof -i :5432
sudo lsof -i :8080

# Alternative command
sudo netstat -tlnp | grep :5432
```

**Solutions**:

1. **Kill Conflicting Process**:
   ```bash
   # Find process ID
   PID=$(sudo lsof -t -i:5432)

   # Gracefully terminate
   kill -TERM $PID

   # Force kill if necessary
   kill -9 $PID
   ```

2. **Change Server Port**:
   ```bash
   # Start on different port
   ./target/release/rusty-db-server --port 5433 --graphql-port 8081

   # Or set environment variable
   export RUSTYDB_PORT=5433
   export RUSTYDB_GRAPHQL_PORT=8081
   ./target/release/rusty-db-server
   ```

---

#### Problem: Data Directory Corruption

**Symptoms**:
- Error: "Failed to initialize storage layer"
- Error: "Corrupt control file"
- WAL recovery failures

**Diagnostic Steps**:
```bash
# Check data directory structure
ls -lR data/

# Verify control file
file data/pg_control

# Check disk errors
dmesg | grep -i error
sudo smartctl -a /dev/sda  # Check disk health
```

**Solutions**:

1. **Restore from Backup** (Recommended):
   ```bash
   # Stop server
   sudo systemctl stop rustydb

   # Backup corrupted data
   mv data data.corrupted.$(date +%s)

   # Restore from last good backup
   tar -xzf /backups/rustydb-backup-20251226.tar.gz

   # Start server
   sudo systemctl start rustydb
   ```

2. **Attempt Recovery**:
   ```bash
   # Start in recovery mode
   ./target/release/rusty-db-server --recovery-mode

   # Check logs for recovery progress
   tail -f data/logs/rustydb.log
   ```

3. **Reinitialize (DESTRUCTIVE - All Data Lost)**:
   ```bash
   # BACKUP FIRST!
   cp -r data data.backup.$(date +%s)

   # Remove corrupted data
   rm -rf data/*

   # Server will reinitialize on next start
   ./target/release/rusty-db-server
   ```

---

#### Problem: Configuration Errors

**Symptoms**:
- Error: "Invalid configuration"
- Error: "Configuration validation failed"
- Server starts but behaves unexpectedly

**Diagnostic Steps**:
```bash
# Validate configuration
rustydb-admin validate-config

# Check environment variables
env | grep RUSTYDB

# Review effective configuration
rustydb-admin show-config
```

**Solutions**:

1. **Reset to Defaults**:
   ```bash
   # Clear all RustyDB environment variables
   unset $(env | grep RUSTYDB | cut -d= -f1)

   # Start with default configuration
   ./target/release/rusty-db-server
   ```

2. **Fix Invalid Values**:
   ```bash
   # Check common configuration issues

   # Buffer pool must be positive
   export RUSTYDB_BUFFER_POOL_SIZE=1000

   # Port must be 1024-65535
   export RUSTYDB_PORT=5432

   # Data directory must exist
   mkdir -p /var/lib/rustydb
   export RUSTYDB_DATA_DIR=/var/lib/rustydb
   ```

---

#### Problem: Recovery Mode Issues

**Symptoms**:
- WAL recovery hangs
- Error: "Inconsistent WAL segment"
- Recovery takes excessive time

**Diagnostic Steps**:
```bash
# Check WAL directory
ls -lh data/wal/

# Monitor recovery progress
tail -f data/logs/rustydb.log | grep -i "recovery\|wal"

# Check for corrupted WAL files
rustydb-admin verify-wal
```

**Solutions**:

1. **Skip Corrupted WAL Segment** (May Lose Data):
   ```bash
   # Identify last good WAL position
   rustydb-admin wal-status

   # Reset recovery to last checkpoint
   rustydb-admin reset-wal --to-checkpoint
   ```

2. **Restore WAL from Archive**:
   ```bash
   # If WAL archiving enabled
   cp /archive/wal/* data/wal/

   # Retry recovery
   ./target/release/rusty-db-server --recovery-mode
   ```

---

## Connection Issues

### Connection Refused

#### Problem: Cannot Connect to Database Server

**Symptoms**:
- Error: "Connection refused"
- Client timeout
- Cannot reach server

**Diagnostic Steps**:
```bash
# 1. Verify server is running
ps aux | grep rusty-db-server
sudo systemctl status rustydb

# 2. Test network connectivity
telnet localhost 5432
nc -zv localhost 5432

# 3. Check listening ports
sudo netstat -tlnp | grep rusty-db
```

**Solutions**:

1. **Server Not Running**:
   ```bash
   # Start the server
   sudo systemctl start rustydb

   # Or directly
   ./target/release/rusty-db-server
   ```

2. **Firewall Blocking**:
   ```bash
   # Check firewall status
   sudo ufw status

   # Allow database port
   sudo ufw allow 5432/tcp
   sudo ufw allow 8080/tcp

   # Or temporarily disable (testing only!)
   sudo ufw disable
   ```

3. **Server Binding to Wrong Interface**:
   ```bash
   # Check bind address
   sudo netstat -tlnp | grep :5432
   # Should show: 0.0.0.0:5432 (all interfaces)
   # Not: 127.0.0.1:5432 (localhost only)

   # Fix: Configure bind address
   # (Future feature in v0.6.0)
   ```

---

#### Problem: Connection Timeout

**Symptoms**:
- Connection hangs
- Error after 30-60 seconds
- Client reports timeout

**Diagnostic Steps**:
```bash
# 1. Check server load
uptime
htop

# 2. Check active connections
curl http://localhost:8080/health | jq '.connections'

# 3. Test network latency
ping <server_ip>
traceroute <server_ip>
```

**Solutions**:

1. **Server Overloaded**:
   ```graphql
   # Check connection pool status
   query {
     connectionPoolStats {
       active
       idle
       waiting
       max
     }
   }
   ```

   ```bash
   # Increase max connections
   export RUSTYDB_MAX_CONNECTIONS=200
   ./target/release/rusty-db-server
   ```

2. **Network Issues**:
   ```bash
   # Check for packet loss
   ping -c 100 <server_ip> | grep loss

   # Check network interface errors
   ifconfig | grep errors
   netstat -i
   ```

3. **Connection Queue Full**:
   ```bash
   # Increase system TCP backlog
   sudo sysctl -w net.core.somaxconn=1024
   sudo sysctl -w net.ipv4.tcp_max_syn_backlog=2048
   ```

---

#### Problem: Authentication Failures

**Symptoms**:
- Error: "Authentication failed"
- Error: "Invalid credentials"
- Error: "User not found"

**Diagnostic Steps**:
```bash
# Check authentication logs
grep "AUTH" data/logs/security_audit.log

# View failed login attempts
grep "AUTHENTICATION_FAILURE" data/logs/security_audit.log
```

**Solutions**:

1. **Invalid Credentials**:
   ```bash
   # Reset user password
   rustydb-admin reset-password --user <username>

   # Verify user exists
   rustydb-admin list-users
   ```

2. **Account Locked** (Security Feature):
   ```graphql
   # Check if user is blocked
   query {
     userStatus(username: "alice") {
       blocked
       blockReason
       failedLoginAttempts
     }
   }
   ```

   ```bash
   # Unblock user
   rustydb-admin unblock-user --user <username>
   ```

3. **IP Blocked** (DDoS Protection):
   ```bash
   # Check blocked IPs
   rustydb-admin list-blocked-ips

   # Unblock IP
   rustydb-admin unblock-ip --ip 192.168.1.100
   ```

---

#### Problem: TLS/SSL Errors

**Symptoms**:
- Error: "SSL handshake failed"
- Error: "Certificate verification failed"
- Encrypted connections fail

**Diagnostic Steps**:
```bash
# Test SSL connection
openssl s_client -connect localhost:5432 -starttls postgres

# Check certificate validity
openssl x509 -in /path/to/server.crt -noout -dates

# Verify certificate chain
openssl verify -CAfile /path/to/ca.crt /path/to/server.crt
```

**Solutions**:

1. **Certificate Expired**:
   ```bash
   # Generate new self-signed certificate
   openssl req -x509 -newkey rsa:4096 -keyout server.key -out server.crt -days 365 -nodes

   # Update configuration (v0.6.0 feature)
   # export RUSTYDB_TLS_CERT=/path/to/server.crt
   # export RUSTYDB_TLS_KEY=/path/to/server.key
   ```

2. **Certificate Validation Issues**:
   ```bash
   # Client: Disable certificate verification (testing only!)
   # Use proper CA certificate in production

   # Add CA to system trust store
   sudo cp ca.crt /usr/local/share/ca-certificates/
   sudo update-ca-certificates
   ```

---

#### Problem: Connection Pool Exhaustion

**Symptoms**:
- Error: "No available connections"
- Error: "Connection pool full"
- New connections rejected

**Diagnostic Steps**:
```graphql
# Check pool statistics
query {
  connectionPoolStats {
    active
    idle
    waiting
    max
    avgWaitTime
    maxWaitTime
  }
}
```

**Solutions**:

1. **Increase Pool Size**:
   ```bash
   export RUSTYDB_MAX_CONNECTIONS=200
   ./target/release/rusty-db-server
   ```

2. **Find Connection Leaks**:
   ```graphql
   # List long-running connections
   query {
     activeConnections {
       connectionId
       user
       connectedSince
       lastActivity
       state
     }
   }
   ```

   ```bash
   # Terminate idle connections
   rustydb-admin terminate-idle --idle-timeout 300
   ```

3. **Application Connection Management**:
   - Ensure clients close connections properly
   - Use connection pooling in application
   - Set appropriate connection timeouts

---

## Performance Issues

### Slow Queries

#### Problem: Queries Taking Too Long

**Symptoms**:
- Query response time > expected
- Timeouts on complex queries
- User complaints about slowness

**Diagnostic Steps**:
```bash
# 1. Check slow query log
tail -f data/logs/performance.log | grep "SLOW_QUERY"

# 2. Monitor active queries
curl http://localhost:8080/metrics | grep query_duration
```

```graphql
# 3. Analyze query execution
query {
  slowQueries(limit: 10, minDuration: 1000) {
    query
    duration
    timestamp
    user
    executionPlan
  }
}
```

**Solutions**:

1. **Missing Indexes**:
   ```sql
   -- Identify full table scans
   EXPLAIN ANALYZE SELECT * FROM users WHERE email = 'alice@example.com';

   -- Create appropriate index
   CREATE INDEX idx_users_email ON users(email);
   ```

2. **Inefficient Query Plan**:
   ```sql
   -- Use query hints (v0.5.1 feature)
   SELECT /*+ INDEX(users idx_users_email) */ *
   FROM users
   WHERE email = 'alice@example.com';

   -- Force specific join order
   SELECT /*+ LEADING(a b c) */ *
   FROM a, b, c
   WHERE a.id = b.a_id AND b.id = c.b_id;
   ```

3. **Large Result Sets**:
   ```sql
   -- Use pagination
   SELECT * FROM users
   ORDER BY created_at DESC
   LIMIT 100 OFFSET 0;

   -- Use cursor-based pagination (better performance)
   SELECT * FROM users
   WHERE id > 12345
   ORDER BY id
   LIMIT 100;
   ```

4. **Buffer Pool Misses**:
   ```bash
   # Check buffer pool hit rate
   curl http://localhost:8080/health | jq '.buffer_pool.hit_rate'
   # Should be > 0.90 (90%)

   # Increase buffer pool size
   export RUSTYDB_BUFFER_POOL_SIZE=10000  # ~80 MB
   ./target/release/rusty-db-server
   ```

---

### High CPU Usage

#### Problem: Server Consuming Excessive CPU

**Symptoms**:
- CPU usage > 80%
- System unresponsive
- Query latency increases

**Diagnostic Steps**:
```bash
# 1. Check CPU usage
top -p $(pidof rusty-db-server)

# 2. Identify hot functions
perf top -p $(pidof rusty-db-server)

# 3. Check active queries
curl http://localhost:8080/metrics | grep active_queries
```

**Solutions**:

1. **Expensive Queries**:
   ```graphql
   # Find CPU-intensive queries
   query {
     activeTransactions {
       transactionId
       query
       cpuTime
       startTime
     }
   }

   # Cancel expensive query
   mutation {
     cancelQuery(queryId: "query-123") {
       success
     }
   }
   ```

2. **Parallel Query Overhead**:
   ```bash
   # Reduce parallel workers
   export RUSTYDB_PARALLEL_WORKERS=4
   ./target/release/rusty-db-server
   ```

3. **Index Maintenance**:
   ```sql
   -- Rebuild fragmented indexes
   REINDEX INDEX idx_users_email;

   -- Update statistics
   ANALYZE users;
   ```

4. **Enable SIMD Optimizations**:
   ```bash
   # Rebuild with SIMD support (AVX2)
   cargo build --release --features simd
   ```

---

### Memory Exhaustion

#### Problem: Server Running Out of Memory

**Symptoms**:
- Error: "Out of memory"
- OOM killer terminates process
- System swapping heavily

**Diagnostic Steps**:
```bash
# 1. Check memory usage
free -h
ps aux | grep rusty-db-server

# 2. Monitor memory growth
watch -n 1 'ps aux | grep rusty-db-server'

# 3. Check for memory leaks
rustydb-admin memory-stats
```

**Solutions**:

1. **Buffer Pool Too Large**:
   ```bash
   # Reduce buffer pool size
   export RUSTYDB_BUFFER_POOL_SIZE=5000  # ~40 MB
   ./target/release/rusty-db-server
   ```

2. **Too Many Connections**:
   ```bash
   # Reduce max connections
   export RUSTYDB_MAX_CONNECTIONS=50
   ./target/release/rusty-db-server
   ```

3. **Large Transactions**:
   ```graphql
   # Find large transactions
   query {
     activeTransactions {
       transactionId
       memoryUsage
       operationCount
     }
   }
   ```

   - Break large transactions into smaller chunks
   - Commit more frequently
   - Use batch processing

4. **Memory Leak**:
   ```bash
   # Monitor for continuous growth
   rustydb-admin memory-profiling --enable

   # Collect heap profile (requires jemalloc)
   kill -SIGUSR1 $(pidof rusty-db-server)

   # Analyze profile
   jeprof --pdf rusty-db-server rustydb_heap.*.heap > leak.pdf
   ```

---

### I/O Bottlenecks

#### Problem: Slow Disk I/O

**Symptoms**:
- High disk wait time (iowait)
- Slow checkpoint operations
- WAL write delays

**Diagnostic Steps**:
```bash
# 1. Check disk I/O
iostat -x 1 10

# 2. Identify I/O heavy processes
iotop

# 3. Check disk health
sudo smartctl -a /dev/sda
```

**Solutions**:

1. **Optimize WAL Configuration**:
   ```bash
   # Use faster disk for WAL
   # Separate WAL from data directory
   mkdir -p /fast-disk/wal
   ln -s /fast-disk/wal data/wal
   ```

2. **Adjust Checkpoint Frequency**:
   ```bash
   # Reduce checkpoint frequency (reduce I/O spikes)
   # (Configuration feature in v0.6.0)
   # export RUSTYDB_CHECKPOINT_TIMEOUT=600  # 10 minutes
   ```

3. **Use SSD/NVMe**:
   - Move data directory to SSD
   - Enable io_uring for Linux (faster async I/O)
   ```bash
   cargo build --release --features io_uring
   ```

4. **Tune OS I/O Scheduler**:
   ```bash
   # Use deadline scheduler for database workloads
   echo deadline | sudo tee /sys/block/sda/queue/scheduler

   # Or use noop for SSD
   echo noop | sudo tee /sys/block/sda/queue/scheduler
   ```

---

### Lock Contention

#### Problem: High Lock Wait Times

**Symptoms**:
- Transactions waiting for locks
- Decreased concurrency
- Query latency spikes

**Diagnostic Steps**:
```graphql
# Check lock waits
query {
  lockWaits {
    waitingTransactionId
    blockingTransactionId
    resourceId
    lockType
    waitDuration
  }
}

# Check lock statistics
query {
  lockStats {
    totalLocks
    waitCount
    avgWaitTime
    maxWaitTime
  }
}
```

**Solutions**:

1. **Reduce Transaction Length**:
   - Keep transactions short
   - Commit more frequently
   - Avoid user interaction during transactions

2. **Change Lock Order**:
   - Always acquire locks in same order
   - Prevents circular wait (deadlock)

3. **Use Lower Isolation Level**:
   ```graphql
   # Use READ_COMMITTED instead of SERIALIZABLE
   mutation {
     beginTransaction(isolationLevel: READ_COMMITTED) {
       transactionId
     }
   }
   ```

4. **Optimize Locking Strategy**:
   - Use row-level locks instead of table locks
   - Use SELECT FOR UPDATE only when necessary
   - Consider optimistic locking for read-heavy workloads

---

## Transaction Issues

### Deadlocks

#### Problem: Transactions Deadlocking

**Symptoms**:
- Error: "Deadlock detected"
- Transaction automatically rolled back
- Multiple transactions waiting on each other

**Diagnostic Steps**:
```graphql
# View recent deadlocks
query {
  transactionDeadlocks(limit: 10) {
    timestamp
    transactionIds
    conflictingResources
    victimTransactionId
    detectionTime
  }
}
```

**Solutions**:

1. **Consistent Lock Ordering**:
   ```sql
   -- Bad: Different order
   -- Transaction 1: Lock A, then B
   -- Transaction 2: Lock B, then A

   -- Good: Same order
   -- All transactions: Lock A, then B
   ```

2. **Retry Logic**:
   ```python
   # Application code should retry deadlocked transactions
   max_retries = 3
   for attempt in range(max_retries):
       try:
           # Execute transaction
           execute_transaction()
           break
       except DeadlockDetected:
           if attempt < max_retries - 1:
               time.sleep(0.1 * (2 ** attempt))  # Exponential backoff
               continue
           else:
               raise
   ```

3. **Reduce Transaction Scope**:
   - Only lock what's necessary
   - Release locks as soon as possible
   - Break large transactions into smaller ones

4. **Use Savepoints**:
   ```graphql
   # Create savepoint before risky operation
   mutation {
     createSavepoint(
       transactionId: "txn-123"
       savepointName: "before_update"
     ) {
       success
     }
   }

   # Rollback to savepoint on deadlock
   mutation {
     rollbackToSavepoint(
       transactionId: "txn-123"
       savepointName: "before_update"
     ) {
       success
     }
   }
   ```

---

### Long-Running Transactions

#### Problem: Transactions Running Too Long

**Symptoms**:
- Transactions open for hours
- Blocking other transactions
- Memory consumption increasing

**Diagnostic Steps**:
```graphql
# Find long-running transactions
query {
  activeTransactions {
    transactionId
    startTime
    duration
    isolationLevel
    status
    operationCount
    locksHeld
  }
}
```

**Solutions**:

1. **Identify and Terminate**:
   ```graphql
   # Terminate stuck transaction
   mutation {
     abortTransaction(transactionId: "txn-123") {
       success
       reason
     }
   }
   ```

2. **Set Transaction Timeouts** (Application Level):
   ```python
   # Application should implement timeouts
   import signal

   def timeout_handler(signum, frame):
       raise TimeoutError("Transaction timeout")

   signal.signal(signal.SIGALRM, timeout_handler)
   signal.alarm(300)  # 5 minute timeout

   try:
       execute_transaction()
   finally:
       signal.alarm(0)  # Cancel timeout
   ```

3. **Prevent Long Transactions**:
   - Avoid user interaction during transactions
   - Use batch processing for large operations
   - Commit frequently in long-running processes

---

### Transaction Rollbacks

#### Problem: Unexpected Transaction Rollbacks

**Symptoms**:
- Transactions rolling back
- Error: "Transaction aborted"
- Data not persisted

**Diagnostic Steps**:
```bash
# Check transaction log
grep "ROLLBACK\|ABORT" data/logs/transaction.log

# Check error log
grep -A 5 "Transaction.*abort" data/logs/error.log
```

**Common Causes and Solutions**:

1. **Serialization Failure**:
   ```
   Error: "Could not serialize access due to concurrent update"
   ```

   **Solution**: Retry transaction or use lower isolation level
   ```graphql
   mutation {
     beginTransaction(isolationLevel: READ_COMMITTED) {
       transactionId
     }
   }
   ```

2. **Constraint Violation**:
   ```
   Error: "Unique constraint violation"
   Error: "Foreign key constraint violation"
   ```

   **Solution**: Fix application logic to respect constraints

3. **Deadlock Victim**:
   ```
   Error: "Transaction was chosen as deadlock victim"
   ```

   **Solution**: See [Deadlocks](#deadlocks) section

4. **System Resource Exhaustion**:
   ```
   Error: "Out of memory"
   Error: "Too many open files"
   ```

   **Solution**: See [Memory Exhaustion](#memory-exhaustion) section

---

### Lock Waits

#### Problem: Transactions Waiting for Locks

**Symptoms**:
- Transactions blocked
- Query latency increases
- Timeout errors

**Diagnostic Steps**:
```graphql
query {
  lockWaits {
    waitingTransactionId
    waitingQuery
    blockingTransactionId
    blockingQuery
    resourceId
    lockType
    waitDuration
  }
}
```

**Solutions**:

1. **Identify Blocking Transaction**:
   ```graphql
   # Get details of blocking transaction
   query {
     transactionInfo(transactionId: "blocking-txn-id") {
       startTime
       query
       user
       status
     }
   }
   ```

2. **Terminate Blocking Transaction** (If Appropriate):
   ```graphql
   mutation {
     abortTransaction(transactionId: "blocking-txn-id") {
       success
     }
   }
   ```

3. **Set Lock Timeout** (Application Level):
   ```sql
   -- Set statement timeout (PostgreSQL compatible)
   SET statement_timeout = 30000;  -- 30 seconds

   -- Execute query
   SELECT * FROM users WHERE ...;
   ```

---

## Replication Issues

### Replication Lag

#### Problem: Replicas Falling Behind

**Symptoms**:
- Increasing replication lag
- Stale data on replicas
- Read-after-write inconsistency

**Diagnostic Steps**:
```graphql
# Check replication status
query {
  replicationStatus {
    replicas {
      replicaId
      lag
      lastWalPosition
      connectionStatus
      syncState
    }
  }
}
```

```bash
# Monitor lag over time
watch -n 5 'rustydb-admin replication-status'
```

**Solutions**:

1. **Network Issues**:
   ```bash
   # Test network bandwidth
   iperf3 -c <replica_host>

   # Check packet loss
   ping -c 100 <replica_host> | grep loss
   ```

2. **Replica Overloaded**:
   ```bash
   # Check replica resource usage
   ssh <replica_host> 'top -b -n 1 | head -20'

   # Reduce read load on replica
   # - Add more replicas
   # - Use connection pooling
   # - Cache frequently accessed data
   ```

3. **Large Transactions**:
   - Break large transactions into smaller chunks
   - Avoid bulk operations during peak hours
   - Use parallel apply (if supported)

4. **WAL Archiving Bottleneck**:
   ```bash
   # Check WAL generation rate
   ls -lth data/wal/ | head -10

   # Increase WAL bandwidth
   # (Configuration in v0.6.0)
   ```

---

### Replication Failures

#### Problem: Replication Stops Working

**Symptoms**:
- Error: "Replication connection lost"
- Replica not receiving updates
- WAL sender errors

**Diagnostic Steps**:
```bash
# Check replication logs
tail -f data/logs/replication.log

# Check replica connectivity
telnet <replica_host> 5432

# Verify replication slots
rustydb-admin list-replication-slots
```

**Solutions**:

1. **Connection Issues**:
   ```bash
   # Test network connectivity
   nc -zv <replica_host> 5432

   # Check firewall
   sudo iptables -L -n | grep 5432
   ```

2. **Authentication Failure**:
   ```bash
   # Verify replication user credentials
   rustydb-admin verify-replication-user

   # Reset replication password
   rustydb-admin reset-replication-password
   ```

3. **WAL Segments Missing**:
   ```bash
   # Check available WAL files
   ls -lh data/wal/

   # If WAL archiving enabled, restore from archive
   cp /archive/wal/* data/wal/

   # Otherwise, rebuild replica from scratch
   rustydb-admin rebuild-replica --replica-id replica-1
   ```

4. **Replication Slot Dropped**:
   ```bash
   # Recreate replication slot
   rustydb-admin create-replication-slot --name replica_1_slot

   # Reconfigure replica to use new slot
   ```

---

### Split-Brain Scenarios

#### Problem: Multiple Masters After Failover

**Symptoms**:
- Two nodes accepting writes
- Data divergence
- Conflict errors on queries

**Diagnostic Steps**:
```bash
# Check which nodes think they're primary
rustydb-admin cluster-status

# Compare WAL positions
rustydb-admin compare-wal-positions
```

**Solutions**:

1. **Immediate Containment**:
   ```bash
   # STOP WRITES TO BOTH NODES IMMEDIATELY
   rustydb-admin set-read-only --node all

   # Assess data divergence
   rustydb-admin check-divergence --node1 primary --node2 replica-1
   ```

2. **Designate True Primary**:
   ```bash
   # Choose node with most recent WAL position
   rustydb-admin cluster-status

   # Demote false primary to replica
   rustydb-admin demote-to-replica --node replica-1

   # Promote true primary
   rustydb-admin promote-to-primary --node primary
   ```

3. **Rebuild Divergent Node**:
   ```bash
   # Stop divergent node
   ssh <divergent_node> 'systemctl stop rustydb'

   # Backup divergent data (for conflict resolution)
   ssh <divergent_node> 'tar -czf /backup/divergent-data.tar.gz data/'

   # Rebuild from primary
   rustydb-admin rebuild-replica \
     --replica-host <divergent_node> \
     --primary-host <true_primary>
   ```

4. **Prevent Future Split-Brain**:
   ```bash
   # Enable fencing (v0.6.0 feature)
   # export RUSTYDB_ENABLE_FENCING=true

   # Use quorum-based failover
   # export RUSTYDB_FAILOVER_QUORUM=majority
   ```

---

### Failover Issues

#### Problem: Automatic Failover Not Working

**Symptoms**:
- Primary down but no failover
- Manual intervention required
- Service outage

**Diagnostic Steps**:
```bash
# Check failover status
rustydb-admin failover-status

# Check cluster health
rustydb-admin cluster-health

# Review failover logs
grep "FAILOVER" data/logs/replication.log
```

**Solutions**:

1. **Failover Not Configured**:
   ```bash
   # Enable automatic failover (v0.6.0)
   # export RUSTYDB_AUTO_FAILOVER=true
   # export RUSTYDB_FAILOVER_TIMEOUT=30
   ```

2. **No Suitable Replica**:
   ```bash
   # Check replica readiness
   rustydb-admin list-replicas

   # Verify at least one synchronous replica exists
   rustydb-admin verify-sync-replicas
   ```

3. **Manual Failover**:
   ```bash
   # Promote replica to primary
   rustydb-admin promote-to-primary \
     --replica-id replica-1 \
     --force

   # Update clients to point to new primary
   # (Use DNS or load balancer for automatic redirect)
   ```

---

## Storage Issues

### Disk Space Issues

#### Problem: Running Out of Disk Space

**Symptoms**:
- Error: "No space left on device"
- Write operations failing
- WAL archiving errors

**Diagnostic Steps**:
```bash
# Check disk usage
df -h

# Find largest directories
du -sh data/* | sort -rh | head -10

# Check WAL usage
du -sh data/wal/

# Check inode usage
df -i
```

**Solutions**:

1. **Clean Up Old WAL Files**:
   ```bash
   # Remove archived WAL (if safe to do so)
   find data/wal/ -name "*.backup" -mtime +7 -delete

   # Compress old WAL files
   find data/wal/ -name "0*" -mtime +1 -exec gzip {} \;
   ```

2. **Clean Up Old Log Files**:
   ```bash
   # Remove old logs
   find data/logs/ -name "*.log.*" -mtime +30 -delete

   # Compress recent logs
   find data/logs/ -name "*.log.*" -mtime +7 -exec gzip {} \;
   ```

3. **Vacuum/Compact Tables**:
   ```sql
   -- Reclaim space from deleted rows (future feature)
   VACUUM FULL users;

   -- Compact table data
   CLUSTER users USING idx_users_pkey;
   ```

4. **Increase Disk Space**:
   ```bash
   # Add new disk
   # Mount to /mnt/newdisk

   # Move data directory
   sudo systemctl stop rustydb
   sudo mv /var/lib/rustydb /mnt/newdisk/
   sudo ln -s /mnt/newdisk/rustydb /var/lib/rustydb
   sudo systemctl start rustydb
   ```

---

### Corruption Detection

#### Problem: Data Corruption Detected

**Symptoms**:
- Error: "Page checksum mismatch"
- Error: "Corrupt block detected"
- Query returns inconsistent results

**Diagnostic Steps**:
```bash
# Run integrity check
rustydb-admin verify-integrity --full

# Check specific table
rustydb-admin verify-table --table users

# Check filesystem errors
dmesg | grep -i error
sudo smartctl -a /dev/sda
```

**Solutions**:

1. **Restore from Backup**:
   ```bash
   # Stop server
   sudo systemctl stop rustydb

   # Backup corrupted data
   mv data data.corrupted.$(date +%s)

   # Restore from last good backup
   tar -xzf /backups/rustydb-backup-20251226.tar.gz

   # Start server
   sudo systemctl start rustydb
   ```

2. **Attempt Auto-Recovery**:
   ```bash
   # Enable auto-recovery
   rustydb-admin auto-recovery --enable

   # Attempt repair
   rustydb-admin repair-corruption --table users --auto-recover
   ```

3. **Repair from Replica** (If Available):
   ```bash
   # Copy clean data from replica
   rsync -avz replica:/var/lib/rustydb/data/base/table_1.dat \
         /var/lib/rustydb/data/base/

   # Restart server
   sudo systemctl restart rustydb
   ```

4. **Mark Corrupt Blocks as Lost** (DESTRUCTIVE):
   ```bash
   # Last resort: Accept data loss
   rustydb-admin mark-corrupt-blocks-lost --table users

   # This will delete corrupted rows
   ```

---

### WAL Issues

#### Problem: WAL-Related Errors

**Symptoms**:
- Error: "Could not write to WAL"
- Error: "WAL file not found"
- Transaction commit failures

**Diagnostic Steps**:
```bash
# Check WAL directory
ls -lh data/wal/

# Check WAL file permissions
ls -l data/wal/

# Check disk space for WAL
df -h data/wal/

# Verify WAL integrity
rustydb-admin verify-wal
```

**Solutions**:

1. **WAL Disk Full**:
   ```bash
   # Clean up old WAL files
   rustydb-admin cleanup-wal --keep-days 7

   # Or move WAL to larger disk
   sudo systemctl stop rustydb
   mv data/wal /large-disk/wal
   ln -s /large-disk/wal data/wal
   sudo systemctl start rustydb
   ```

2. **WAL Permission Issues**:
   ```bash
   # Fix permissions
   chown -R rustydb:rustydb data/wal/
   chmod 700 data/wal/
   chmod 600 data/wal/*
   ```

3. **WAL Corruption**:
   ```bash
   # Attempt recovery
   rustydb-admin wal-recovery --repair

   # If unsuccessful, restore from backup
   ```

---

### Checkpoint Failures

#### Problem: Checkpoint Operations Failing

**Symptoms**:
- Error: "Checkpoint failed"
- WAL growing unbounded
- Performance degradation

**Diagnostic Steps**:
```bash
# Check checkpoint status
rustydb-admin checkpoint-status

# Monitor checkpoint operations
tail -f data/logs/rustydb.log | grep CHECKPOINT

# Check disk I/O during checkpoint
iostat -x 1
```

**Solutions**:

1. **I/O Bottleneck**:
   ```bash
   # Spread checkpoint I/O over longer period
   # (v0.6.0 configuration)
   # export RUSTYDB_CHECKPOINT_COMPLETION_TARGET=0.9
   ```

2. **Disk Full**:
   - See [Disk Space Issues](#disk-space-issues)

3. **Manual Checkpoint**:
   ```bash
   # Force checkpoint
   rustydb-admin checkpoint --force

   # Monitor progress
   tail -f data/logs/rustydb.log | grep CHECKPOINT
   ```

---

## API Issues

### REST API Errors

#### Problem: REST API Returning Errors

**Symptoms**:
- HTTP 500 errors
- HTTP 404 errors
- Timeout on API calls

**Diagnostic Steps**:
```bash
# Test API health
curl http://localhost:8080/health

# Test with verbose output
curl -v http://localhost:8080/api/v1/transactions

# Check API logs
tail -f data/logs/rustydb.log | grep "API"
```

**Solutions**:

1. **API Server Not Running**:
   ```bash
   # Verify both database and API server running
   ps aux | grep rusty-db-server

   # Check listening ports
   sudo netstat -tlnp | grep 8080
   ```

2. **Endpoint Not Found (404)**:
   ```bash
   # Check available endpoints
   curl http://localhost:8080/api/v1/openapi.json | jq '.paths | keys'

   # Common issue: Some endpoints not yet implemented in v0.5.1
   # See KNOWN_ISSUES.md for list of unavailable endpoints
   ```

3. **Authentication Required**:
   ```bash
   # Include authentication token
   curl -H "Authorization: Bearer <token>" \
        http://localhost:8080/api/v1/transactions
   ```

4. **Rate Limiting**:
   ```bash
   # Check rate limit headers
   curl -I http://localhost:8080/health

   # Headers:
   # X-RateLimit-Limit: 1000
   # X-RateLimit-Remaining: 999
   # X-RateLimit-Reset: 1640000000
   ```

---

### GraphQL Errors

#### Problem: GraphQL Queries Failing

**Symptoms**:
- GraphQL errors in response
- Null fields in response
- Timeout errors

**Diagnostic Steps**:
```bash
# Test GraphQL endpoint
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ serverInfo { version } }"}'

# Access GraphQL playground
open http://localhost:8080/graphql
```

**Solutions**:

1. **Query Syntax Error**:
   ```graphql
   # Bad query (missing closing brace)
   query {
     serverInfo {
       version
   }

   # Good query
   query {
     serverInfo {
       version
     }
   }
   ```

2. **Field Not Available**:
   ```graphql
   # Check schema for available fields
   query {
     __type(name: "ServerInfo") {
       fields {
         name
         type {
           name
         }
       }
     }
   }
   ```

3. **Resolver Error**:
   - Check server logs for error details
   - May indicate backend issue (database error, etc.)

4. **Timeout on Complex Query**:
   ```graphql
   # Use fragments to reduce query complexity
   fragment UserFields on User {
     id
     username
     email
   }

   query {
     users {
       ...UserFields
     }
   }
   ```

---

### Rate Limiting Issues

#### Problem: Rate Limit Exceeded

**Symptoms**:
- HTTP 429 "Too Many Requests"
- API calls throttled
- Service degradation

**Diagnostic Steps**:
```bash
# Check rate limit status
curl -I http://localhost:8080/health | grep RateLimit

# Monitor rate limit metrics
curl http://localhost:8080/metrics | grep rate_limit
```

**Solutions**:

1. **Implement Client-Side Rate Limiting**:
   ```python
   # Example: Token bucket rate limiter
   from time import sleep, time

   class RateLimiter:
       def __init__(self, rate=100, per=60):
           self.rate = rate
           self.per = per
           self.allowance = rate
           self.last_check = time()

       def allow_request(self):
           current = time()
           time_passed = current - self.last_check
           self.last_check = current
           self.allowance += time_passed * (self.rate / self.per)

           if self.allowance > self.rate:
               self.allowance = self.rate

           if self.allowance < 1:
               return False

           self.allowance -= 1
           return True
   ```

2. **Request Rate Limit Increase**:
   ```bash
   # Contact administrator to increase limits
   # (Enterprise customers may have higher limits)
   ```

3. **Use Batch Endpoints**:
   ```graphql
   # Instead of multiple queries
   query {
     user1: user(id: 1) { username }
     user2: user(id: 2) { username }
     user3: user(id: 3) { username }
   }
   ```

4. **Implement Caching**:
   - Cache frequently accessed data client-side
   - Use ETags for conditional requests
   - Implement query result caching

---

### Authentication/Authorization Issues

#### Problem: API Authentication Failing

**Symptoms**:
- HTTP 401 "Unauthorized"
- HTTP 403 "Forbidden"
- Invalid token errors

**Diagnostic Steps**:
```bash
# Test without authentication
curl http://localhost:8080/health

# Test with authentication
curl -H "Authorization: Bearer <token>" \
     http://localhost:8080/api/v1/transactions

# Verify token
rustydb-admin verify-token --token <token>
```

**Solutions**:

1. **Missing Token**:
   ```bash
   # Generate API token
   rustydb-admin create-api-token --user admin --expiry 30d

   # Use token in requests
   curl -H "Authorization: Bearer eyJ0eXAi..." \
        http://localhost:8080/api/v1/transactions
   ```

2. **Expired Token**:
   ```bash
   # Check token expiry
   rustydb-admin inspect-token --token <token>

   # Generate new token
   rustydb-admin create-api-token --user admin
   ```

3. **Insufficient Permissions**:
   ```bash
   # Check user permissions
   rustydb-admin show-user-permissions --user alice

   # Grant necessary permissions
   rustydb-admin grant-permission \
     --user alice \
     --permission api:transactions:read
   ```

---

## Error Messages Reference

### Common Error Codes

#### Database Errors

**ERR-DB-001**: Database initialization failed
- **Cause**: Data directory missing or corrupt
- **Solution**: Create data directory or restore from backup

**ERR-DB-002**: Storage layer initialization failed
- **Cause**: Disk I/O error, permission issue
- **Solution**: Check disk health, fix permissions

**ERR-DB-003**: WAL recovery failed
- **Cause**: Corrupted WAL file
- **Solution**: Restore from backup or skip corrupted segment

**ERR-DB-004**: Buffer pool initialization failed
- **Cause**: Insufficient memory
- **Solution**: Reduce buffer pool size or increase system memory

**ERR-DB-005**: Transaction manager initialization failed
- **Cause**: System resource exhaustion
- **Solution**: Check memory, file descriptors

---

#### Connection Errors

**ERR-CONN-001**: Connection refused
- **Cause**: Server not running or port blocked
- **Solution**: Start server, check firewall

**ERR-CONN-002**: Connection timeout
- **Cause**: Network issue or server overload
- **Solution**: Check network, increase timeout, scale server

**ERR-CONN-003**: Authentication failed
- **Cause**: Invalid credentials
- **Solution**: Verify username/password

**ERR-CONN-004**: Connection pool exhausted
- **Cause**: Too many active connections
- **Solution**: Increase max connections or close idle connections

**ERR-CONN-005**: SSL handshake failed
- **Cause**: Certificate issue or protocol mismatch
- **Solution**: Check certificate validity, verify SSL configuration

---

#### Transaction Errors

**ERR-TXN-001**: Deadlock detected
- **Cause**: Circular lock wait
- **Solution**: Retry transaction, fix lock ordering

**ERR-TXN-002**: Transaction aborted
- **Cause**: Conflict, constraint violation, or system error
- **Solution**: Review error details, fix data/logic issue

**ERR-TXN-003**: Isolation violation
- **Cause**: Concurrent modification under SERIALIZABLE
- **Solution**: Retry transaction or use lower isolation level

**ERR-TXN-004**: Transaction timeout
- **Cause**: Transaction exceeded time limit
- **Solution**: Optimize query, break into smaller transactions

**ERR-TXN-005**: Lock wait timeout
- **Cause**: Waited too long for lock
- **Solution**: Identify blocking transaction, reduce contention

---

#### Storage Errors

**ERR-STG-001**: Page checksum mismatch
- **Cause**: Data corruption
- **Solution**: Restore from backup or use auto-recovery

**ERR-STG-002**: Disk write failed
- **Cause**: Disk full or hardware error
- **Solution**: Free space or replace disk

**ERR-STG-003**: WAL write failed
- **Cause**: WAL disk full or I/O error
- **Solution**: Clean up WAL directory, check disk health

**ERR-STG-004**: Checkpoint failed
- **Cause**: I/O error during checkpoint
- **Solution**: Check disk health, reduce I/O load

**ERR-STG-005**: Index corruption detected
- **Cause**: Index data corrupted
- **Solution**: Rebuild index

---

#### API Errors

**ERR-API-001**: Endpoint not found (404)
- **Cause**: Invalid URL or endpoint not implemented
- **Solution**: Check API documentation, verify endpoint exists

**ERR-API-002**: Method not allowed (405)
- **Cause**: Wrong HTTP method
- **Solution**: Use correct method (GET, POST, etc.)

**ERR-API-003**: Invalid request format (400)
- **Cause**: Malformed JSON or missing required fields
- **Solution**: Validate request format

**ERR-API-004**: Rate limit exceeded (429)
- **Cause**: Too many requests
- **Solution**: Implement rate limiting, reduce request frequency

**ERR-API-005**: Internal server error (500)
- **Cause**: Backend error
- **Solution**: Check server logs for details

---

### Error Message Patterns

#### Pattern: "PANIC: ..."
- **Severity**: CRITICAL
- **Action**: Server will terminate, check logs immediately
- **Example**: "PANIC: Could not write WAL segment"

#### Pattern: "FATAL: ..."
- **Severity**: HIGH
- **Action**: Operation cannot continue, immediate attention required
- **Example**: "FATAL: Database corruption detected"

#### Pattern: "ERROR: ..."
- **Severity**: MEDIUM
- **Action**: Operation failed, review and retry
- **Example**: "ERROR: Deadlock detected"

#### Pattern: "WARNING: ..."
- **Severity**: LOW
- **Action**: Potential issue, monitor and investigate
- **Example**: "WARNING: Buffer pool hit rate below 80%"

#### Pattern: "INFO: ..."
- **Severity**: INFORMATIONAL
- **Action**: Normal operation, no action needed
- **Example**: "INFO: Checkpoint completed successfully"

---

### Security Error Messages

**SEC-001**: Authentication failure
- **Message**: "Authentication failed for user 'alice'"
- **Cause**: Invalid credentials
- **Action**: Verify credentials, check for account lockout

**SEC-002**: Authorization failure
- **Message**: "Permission denied for operation 'DROP TABLE'"
- **Cause**: Insufficient privileges
- **Action**: Grant necessary permissions

**SEC-003**: SQL injection detected
- **Message**: "Potential SQL injection blocked"
- **Cause**: Malicious query pattern detected
- **Action**: Review query, use parameterized queries

**SEC-004**: Rate limit exceeded
- **Message**: "Rate limit exceeded for IP 192.168.1.100"
- **Cause**: Too many requests from IP
- **Action**: Implement client-side rate limiting

**SEC-005**: Insider threat detected
- **Message**: "High-risk query blocked for user 'alice'"
- **Cause**: Behavioral anomaly detected
- **Action**: Review user activity, verify legitimate use

**SEC-006**: Encryption error
- **Message**: "Failed to decrypt data block"
- **Cause**: Wrong key or corrupted data
- **Action**: Verify encryption keys, check data integrity

---

## Support and Escalation

### Gathering Diagnostics

Before contacting support, gather the following information:

#### 1. System Information
```bash
# System details
uname -a
cat /etc/os-release

# RustyDB version
./target/release/rusty-db-server --version

# Hardware information
lscpu
free -h
df -h
```

#### 2. Configuration
```bash
# Current configuration
rustydb-admin show-config

# Environment variables
env | grep RUSTYDB

# Runtime parameters
curl http://localhost:8080/health | jq
```

#### 3. Recent Logs
```bash
# Last 500 lines of main log
tail -n 500 data/logs/rustydb.log > /tmp/rustydb-main.log

# All errors from last hour
grep ERROR data/logs/error.log > /tmp/rustydb-errors.log

# Security events
tail -n 200 data/logs/security_audit.log > /tmp/rustydb-security.log
```

#### 4. System State
```graphql
# Query system status
query {
  serverInfo { version uptime activeConnections }
  systemMetrics { cpuUsage memoryUsage diskIO }
  activeTransactions { transactionId status duration }
  connectionPoolStats { active idle waiting }
}
```

#### 5. Error Details
- Exact error message
- Timestamp when error occurred
- Steps to reproduce
- Frequency of occurrence

---

### Log Collection

#### Create Support Bundle

```bash
# Automated diagnostic collection
rustydb-admin create-support-bundle \
  --output /tmp/rustydb-support-$(date +%s).tar.gz \
  --include-logs \
  --include-config \
  --include-metrics \
  --include-core-dump \
  --anonymize-sensitive-data

# Bundle contents:
# - Configuration files
# - Last 7 days of logs
# - System metrics
# - Active queries
# - Transaction state
# - Replication status
# - Core dumps (if any)
# - Performance profiles
```

#### Manual Log Collection

```bash
# Create directory
mkdir -p /tmp/rustydb-diagnostics

# Copy logs
cp -r data/logs/* /tmp/rustydb-diagnostics/

# Export configuration
rustydb-admin show-config > /tmp/rustydb-diagnostics/config.txt

# Export metrics
curl http://localhost:8080/metrics > /tmp/rustydb-diagnostics/metrics.txt
curl http://localhost:8080/health > /tmp/rustydb-diagnostics/health.json

# System information
uname -a > /tmp/rustydb-diagnostics/system-info.txt
lscpu >> /tmp/rustydb-diagnostics/system-info.txt
free -h >> /tmp/rustydb-diagnostics/system-info.txt
df -h >> /tmp/rustydb-diagnostics/system-info.txt

# Create archive
tar -czf rustydb-diagnostics-$(date +%s).tar.gz -C /tmp rustydb-diagnostics/
```

---

### Support Channels

#### Community Support

**GitHub Issues**:
- URL: https://github.com/harborgrid-justin/rusty-db/issues
- For: Bug reports, feature requests
- Response Time: Best effort (24-72 hours)

**GitHub Discussions**:
- URL: https://github.com/harborgrid-justin/rusty-db/discussions
- For: Questions, ideas, general help
- Response Time: Community-driven

**Documentation**:
- Location: `/home/user/rusty-db/docs/`
- Online: (Coming soon)
- Includes: Architecture, API reference, tutorials

---

#### Enterprise Support (Coming Soon)

**Tier 1 - Standard Support**:
- Email: support@rustydb.io
- Hours: Business hours (9 AM - 5 PM local time)
- Response Time:
  - P0 (Critical): 4 hours
  - P1 (High): 8 hours
  - P2 (Medium): 2 business days
  - P3 (Low): 5 business days

**Tier 2 - Premium Support**:
- Email: premium-support@rustydb.io
- Phone: +1-XXX-XXX-XXXX
- Hours: 24/7/365
- Response Time:
  - P0 (Critical): 1 hour
  - P1 (High): 4 hours
  - P2 (Medium): 1 business day
  - P3 (Low): 3 business days
- Dedicated support engineer
- Quarterly business reviews

**Tier 3 - Mission-Critical Support**:
- Email: mission-critical@rustydb.io
- Phone: +1-XXX-XXX-XXXX (priority line)
- Hours: 24/7/365
- Response Time:
  - P0 (Critical): 15 minutes
  - P1 (High): 1 hour
  - P2 (Medium): 4 hours
  - P3 (Low): 1 business day
- Named support team
- Direct escalation to engineering
- On-site support available
- Monthly performance reviews

---

### Escalation Procedures

#### Severity Levels

**P0 - Critical**:
- **Definition**: Production system down, data loss, security breach
- **Examples**:
  - Database server crashed
  - Data corruption detected
  - Active security incident
- **Response**: Immediate escalation
- **Who**: Security team, CTO, CEO (for security incidents)

**P1 - High**:
- **Definition**: Major functionality broken, severe performance degradation
- **Examples**:
  - Replication failure
  - Transaction deadlocks preventing operations
  - API endpoints returning errors
- **Response**: Escalate within 1 hour if unresolved
- **Who**: Engineering lead, Database team

**P2 - Medium**:
- **Definition**: Functionality impaired but workaround exists
- **Examples**:
  - Slow query performance
  - Non-critical features not working
  - Configuration issues
- **Response**: Monitor, escalate if worsens
- **Who**: Support team

**P3 - Low**:
- **Definition**: Minor issue, cosmetic problem, feature request
- **Examples**:
  - Documentation error
  - Enhancement request
  - Informational questions
- **Response**: Normal queue
- **Who**: Support team

---

#### Escalation Matrix

```
Level 1: Community Support / Standard Support
  ↓ (P0 immediate, P1 after 1 hour, P2 after 4 hours)

Level 2: Senior Support Engineer / Engineering Team
  ↓ (P0 immediate, P1 after 4 hours)

Level 3: Engineering Manager / Database Architect
  ↓ (P0 only, if requires architectural decision)

Level 4: CTO / VP Engineering
  ↓ (P0 only, business-critical or security incident)

Level 5: CEO / Board (regulatory, legal, major breach)
```

---

#### Security Incident Escalation

For security incidents, follow **Incident Response Plan**:

**Immediate Actions** (< 5 minutes):
1. Activate incident response team
2. Contain threat (automated)
3. Preserve evidence
4. Notify security team

**Escalation Contacts**:
- Security Team: security@rustydb.io
- Incident Commander: [Contact in INCIDENT_RESPONSE.md]
- CTO: [Contact in INCIDENT_RESPONSE.md]
- Legal: [Contact in INCIDENT_RESPONSE.md]

**Reference**: See `/home/user/rusty-db/docs/INCIDENT_RESPONSE.md` for complete incident response procedures.

---

### When to Escalate

Escalate immediately if:

1. **Data Loss or Corruption**:
   - Data cannot be recovered from backup
   - Corruption spreading to replicas
   - Critical business data affected

2. **Security Breach**:
   - Unauthorized access detected
   - Data exfiltration suspected
   - Malware/ransomware attack
   - Compliance violation

3. **Extended Outage**:
   - Production system down > 1 hour
   - Cannot restore service with standard procedures
   - SLA breach imminent

4. **Unknown/Novel Issue**:
   - Error never seen before
   - Documentation doesn't cover scenario
   - Standard procedures don't work

5. **Multiple Concurrent Issues**:
   - Cascading failures
   - System instability
   - Unknown root cause

---

### Best Practices for Support Requests

#### DO:
- Provide complete diagnostic information
- Include exact error messages
- Describe steps to reproduce
- List what you've already tried
- Include relevant log excerpts
- State business impact
- Be specific about expected vs actual behavior

#### DON'T:
- Say "it doesn't work" without details
- Omit error messages
- Skip diagnostic steps
- Withhold relevant information
- Demand immediate fixes without justification
- Escalate prematurely

---

## Appendices

### Appendix A: Quick Reference Commands

```bash
# Server Management
./target/release/rusty-db-server                    # Start server
./target/release/rusty-db-server --port 5433        # Custom port
sudo systemctl status rustydb                        # Check status
sudo systemctl restart rustydb                       # Restart server

# Diagnostics
curl http://localhost:8080/health                    # Health check
rustydb-admin diagnostics                            # Run diagnostics
rustydb-admin verify-integrity                       # Data integrity check
tail -f data/logs/rustydb.log                        # View logs

# Performance
curl http://localhost:8080/metrics                   # Prometheus metrics
rustydb-admin checkpoint --force                     # Force checkpoint
rustydb-admin memory-stats                           # Memory statistics

# Connection Management
rustydb-admin list-connections                       # List connections
rustydb-admin terminate-connection --id <id>         # Kill connection
rustydb-admin block-ip --ip 192.168.1.100           # Block IP

# Transaction Management
rustydb-admin list-transactions                      # Active transactions
rustydb-admin abort-transaction --id <txn-id>       # Abort transaction

# Replication
rustydb-admin replication-status                     # Replication status
rustydb-admin promote-to-primary --replica-id <id>  # Promote replica

# Backup/Restore
rustydb-admin backup --full --output backup.tar.gz  # Full backup
rustydb-admin restore --input backup.tar.gz          # Restore

# Security
rustydb-admin list-blocked-users                     # Blocked users
rustydb-admin unblock-user --user <username>        # Unblock user
grep "AUTH" data/logs/security_audit.log            # Auth events
```

---

### Appendix B: Performance Tuning Checklist

- [ ] Buffer pool size appropriate for workload (target > 90% hit rate)
- [ ] SIMD optimizations enabled (`--features simd`)
- [ ] Appropriate isolation level (not always SERIALIZABLE)
- [ ] Indexes exist for frequent queries
- [ ] Statistics up-to-date (`ANALYZE` tables)
- [ ] WAL on fast disk (SSD/NVMe)
- [ ] Connection pooling configured
- [ ] Query timeout set to prevent runaway queries
- [ ] Parallel workers configured appropriately
- [ ] io_uring enabled on Linux (`--features io_uring`)
- [ ] Regular checkpoints (not too frequent)
- [ ] Monitoring and alerting configured
- [ ] Log rotation enabled
- [ ] Disk I/O scheduler optimized (deadline/noop)
- [ ] OS file descriptor limit increased (`ulimit -n 65535`)

---

### Appendix C: Troubleshooting Decision Tree

```
Problem Occurred
    ↓
Server Running? ──NO──→ See: Startup Issues
    ↓ YES
    ↓
Can Connect? ──NO──→ See: Connection Issues
    ↓ YES
    ↓
Transaction Successful? ──NO──→ See: Transaction Issues
    ↓ YES
    ↓
Performance OK? ──NO──→ See: Performance Issues
    ↓ YES
    ↓
Replication Working? ──NO──→ See: Replication Issues
    ↓ YES
    ↓
Data Integrity OK? ──NO──→ See: Storage Issues
    ↓ YES
    ↓
API Working? ──NO──→ See: API Issues
    ↓ YES
    ↓
System Healthy ✓
```

---

### Appendix D: Known Issues (v0.5.1)

**Note**: See `/home/user/rusty-db/release/docs/0.5.1/KNOWN_ISSUES.md` for complete list.

**Critical Known Issues**:
1. Some REST API endpoints not yet implemented (see KNOWN_ISSUES.md)
2. SQL DDL execution in development
3. Configuration file support coming in v0.6.0

**Workarounds**:
- Use GraphQL API for transaction management
- Use environment variables for configuration
- Check API documentation for available endpoints

---

### Appendix E: Monitoring Best Practices

**Key Metrics to Monitor**:

1. **System Health**:
   - Server uptime
   - Active connections
   - Buffer pool hit rate (> 90%)
   - Error rate

2. **Performance**:
   - Query latency (p50, p95, p99)
   - Transaction throughput
   - Checkpoint duration
   - WAL write latency

3. **Resources**:
   - CPU usage (< 80%)
   - Memory usage (< 90%)
   - Disk I/O utilization
   - Disk space (> 20% free)

4. **Transactions**:
   - Active transaction count
   - Deadlock rate
   - Rollback rate
   - Lock wait time

5. **Replication** (if enabled):
   - Replication lag (< 1 second)
   - WAL sender status
   - Replica connection status

6. **Security**:
   - Failed authentication attempts
   - Blocked IPs/users
   - High-risk queries blocked
   - Audit log growth

**Alerting Thresholds**:
- Buffer pool hit rate < 80%: WARNING
- Disk space < 20%: WARNING
- Disk space < 10%: CRITICAL
- Replication lag > 10 seconds: WARNING
- Replication lag > 60 seconds: CRITICAL
- Active connections > 80% max: WARNING
- Active connections > 95% max: CRITICAL
- Failed auth > 10/minute: WARNING
- Failed auth > 50/minute: CRITICAL

---

## Conclusion

This troubleshooting guide covers the most common issues encountered with RustyDB v0.5.1. For issues not covered here:

1. Check the [Documentation Index](./INDEX.md) for related guides
2. Review [Known Issues](./KNOWN_ISSUES.md) for current limitations
3. Consult [Incident Response Plan](../../docs/INCIDENT_RESPONSE.md) for security incidents
4. Contact support through appropriate channels

**Keep This Guide Updated**:
- Document new issues and solutions
- Share knowledge with team
- Update runbooks based on incidents
- Review quarterly for accuracy

---

**RustyDB v0.5.1 - Enterprise Edition**

**Document Version**: 1.0
**Last Updated**: 2025-12-27
**Maintained By**: Enterprise Documentation Team
**Next Review**: 2026-03-27

For questions or corrections: docs@rustydb.io
