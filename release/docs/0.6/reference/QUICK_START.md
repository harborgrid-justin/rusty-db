# RustyDB v0.6.0 - 5-Minute Quick Start

**Version**: 0.6.0 | **Status**: Production Ready | **Updated**: December 28, 2025

---

## Instant Deployment (1 Command)

```bash
cd /home/user/rusty-db
./builds/linux/rusty-db-server
```

**Done!** Server running on:
- PostgreSQL: `localhost:5432`
- REST API: `http://localhost:8080/api/v1`
- GraphQL: `http://localhost:8080/graphql`
- WebSocket: `ws://localhost:8080/ws`

---

## Verify Installation

```bash
# Check server health
curl http://localhost:8080/api/v1/health

# Expected: {"status":"healthy","version":"0.6.0"}
```

---

## First Commands (30 Seconds)

### Using CLI
```bash
# Connect to database
./builds/linux/rusty-db-cli

# Run query
SELECT version();
```

### Using REST API
```bash
# Execute query
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"sql": "SELECT 1 as test"}'
```

### Using GraphQL
```bash
# List tables
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ tables { name rowCount } }"}'
```

---

## Create Your First Table

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"sql": "CREATE TABLE users (id INTEGER, name TEXT, email TEXT)"}'
```

---

## Insert Data

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"sql": "INSERT INTO users (id, name, email) VALUES (1, '\''Alice'\'', '\''alice@example.com'\'')"}'
```

---

## Query Data

```bash
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"sql": "SELECT * FROM users"}'
```

---

## Port Reference

| Service | Port | Protocol | Purpose |
|---------|------|----------|---------|
| PostgreSQL | 5432 | TCP | Database connections |
| REST API | 8080 | HTTP | REST endpoints |
| GraphQL | 8080 | HTTP | GraphQL endpoint |
| WebSocket | 8080 | WS | Real-time updates |

---

## System Requirements

**Minimum**:
- OS: Linux kernel 3.2.0+
- RAM: 4 GB
- Disk: 20 GB
- CPU: 2 cores

**Recommended**:
- RAM: 16 GB+
- Disk: 100 GB SSD
- CPU: 8 cores

---

## Default Configuration

```
Data Directory: ./data
WAL Directory: ./wal
Buffer Pool: 1,000 pages (~4 MB)
Page Size: 4,096 bytes (4 KB)
Max Connections: 100
```

---

## Common Operations

### Stop Server
```bash
# Development mode
Ctrl+C

# Production mode (systemd)
sudo systemctl stop rustydb
```

### View Logs
```bash
# Development mode
# Logs appear in terminal

# Production mode
sudo journalctl -u rustydb -f
```

### Check Status
```bash
# Server process
ps aux | grep rusty-db-server

# Port listening
nc -zv localhost 5432
nc -zv localhost 8080
```

---

## Next Steps

1. **Production Deployment**: See `/release/docs/0.6/deployment/PRODUCTION_DEPLOYMENT.md`
2. **GraphQL API**: See `GRAPHQL_REFERENCE.md`
3. **SQL Reference**: See `SQL_REFERENCE.md`
4. **Configuration**: See `CONFIG_REFERENCE.md`
5. **Troubleshooting**: See `TROUBLESHOOTING_QUICK.md`

---

## Quick Troubleshooting

**Port in use?**
```bash
sudo lsof -i :5432
sudo lsof -i :8080
```

**Permission denied?**
```bash
chmod +x builds/linux/rusty-db-server
```

**Cannot connect?**
```bash
# Check firewall
sudo ufw allow 5432/tcp
sudo ufw allow 8080/tcp
```

---

**Quick Start Guide** | RustyDB v0.6.0 | Enterprise Database Server
