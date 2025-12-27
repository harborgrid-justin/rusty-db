# RustyDB Enterprise Deployment - Quick Start Guide
## Agent 11 - $350M Deployment Coordination

**Status**: ✅ READY FOR DEPLOYMENT
**Date**: December 27, 2025

---

## 🚀 Immediate Deployment (Development Mode)

### Single Command Startup

```bash
# Navigate to project
cd /home/user/rusty-db

# Start server
./builds/linux/rusty-db-server
```

**That's it!** The server will:
- ✅ Listen on port 5432 (PostgreSQL protocol)
- ✅ Start REST API on port 8080
- ✅ Enable GraphQL at `/graphql`
- ✅ Enable WebSocket at `/ws`
- ✅ Create `data/` and `wal/` directories automatically

### Access Points

| Service | URL | Purpose |
|---------|-----|---------|
| PostgreSQL | `localhost:5432` | Database connections |
| REST API | `http://localhost:8080/api/v1` | HTTP API |
| GraphQL | `http://localhost:8080/graphql` | GraphQL queries |
| WebSocket | `ws://localhost:8080/ws` | Real-time updates |
| Swagger UI | `http://localhost:8080/swagger-ui` | API documentation |

---

## 🎯 Component Status

### Binary Verification
```
✅ rusty-db-server: 38 MB, executable, optimized
✅ rusty-db-cli: 922 KB, executable, ready
✅ Build: Release mode, LTO enabled, Level 3 optimization
✅ Platform: Linux x86-64, kernel 3.2.0+
```

### Configuration Status
```
✅ Default config: Functional (4 core settings)
✅ Node.js adapter: v0.2.640, TypeScript ready
✅ Frontend: v1.0.0, production build tested
⚠️ Extended config: Available but requires parsing implementation
```

---

## 📋 Complete Deployment Options

### Option 1: Development (Quick Start)
**Use Case**: Testing, development, demos
**Time**: < 1 minute

```bash
cd /home/user/rusty-db
./builds/linux/rusty-db-server
```

### Option 2: Production (Systemd)
**Use Case**: Enterprise deployment, auto-restart, logging
**Time**: ~10 minutes

```bash
# Install binaries
sudo cp builds/linux/rusty-db-server /usr/local/bin/
sudo cp builds/linux/rusty-db-cli /usr/local/bin/

# Create system user and directories
sudo useradd -r -s /bin/false rustydb
sudo mkdir -p /var/lib/rusty-db/{data,wal,backups}
sudo mkdir -p /var/log/rusty-db
sudo chown -R rustydb:rustydb /var/lib/rusty-db
sudo chown -R rustydb:rustydb /var/log/rusty-db

# Install and start service
sudo cp deploy/systemd/rustydb-single.service /etc/systemd/system/rustydb.service
sudo systemctl daemon-reload
sudo systemctl enable rustydb
sudo systemctl start rustydb

# Verify
sudo systemctl status rustydb
```

### Option 3: With Frontend
**Use Case**: Full management UI
**Time**: ~15 minutes

```bash
# Start server (Option 1 or 2)
./builds/linux/rusty-db-server

# In another terminal, start frontend
cd frontend
npm install
npm run dev

# Access frontend at http://localhost:3000
```

---

## ✅ Deployment Validation

### 1. Server Running
```bash
# Check process
ps aux | grep rusty-db-server

# Should show: rusty-db-server process running
```

### 2. Ports Listening
```bash
# Check PostgreSQL port
nc -zv localhost 5432
# Expected: Connection succeeded

# Check API port
nc -zv localhost 8080
# Expected: Connection succeeded
```

### 3. REST API Health
```bash
curl http://localhost:8080/api/v1/health

# Expected response:
# {"status":"healthy","version":"0.5.1","uptime_seconds":XX}
```

### 4. GraphQL Endpoint
```bash
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ health { status version } }"}'

# Expected: GraphQL response with health data
```

### 5. Database Operations
```bash
# Using CLI
./builds/linux/rusty-db-cli --host localhost --port 5432

# Test query
rusty-db-cli> SELECT 1;
# Expected: Result returned
```

---

## 🔧 Management Commands

### Server Control
```bash
# Development mode
./builds/linux/rusty-db-server                    # Start
Ctrl+C                                             # Stop

# Production mode (systemd)
sudo systemctl start rustydb                       # Start
sudo systemctl stop rustydb                        # Stop
sudo systemctl restart rustydb                     # Restart
sudo systemctl status rustydb                      # Check status
```

### View Logs
```bash
# Development mode
# Logs appear in terminal

# Production mode
sudo journalctl -u rustydb -f                      # Follow logs
sudo journalctl -u rustydb -n 100                  # Last 100 lines
sudo journalctl -u rustydb --since "1 hour ago"    # Recent logs
```

### Database Operations
```bash
# Connect to database
./builds/linux/rusty-db-cli

# Execute single command
./builds/linux/rusty-db-cli --command "SELECT version();"

# Execute SQL file
./builds/linux/rusty-db-cli --file script.sql
```

---

## 🌐 Integration Points

### Frontend → Server
```
Frontend (port 3000/80)
    ↓ HTTP/GraphQL/WebSocket
RustyDB Server (port 8080)
    ↓ Internal
Database Engine (port 5432)
```

**Configuration** (frontend/.env.production):
```env
VITE_API_URL=http://localhost:8080
VITE_GRAPHQL_URL=http://localhost:8080/graphql
VITE_WS_URL=ws://localhost:8080/ws
```

### Node.js Adapter → Server
```typescript
import { createRustyDbClient, createConfig } from '@rustydb/adapter';

const config = createConfig()
  .server({ host: 'localhost', port: 5432 })
  .api({ baseUrl: 'http://localhost:8080' })
  .build();

const client = await createRustyDbClient(config);
await client.initialize();

// Use client...
const health = await client.monitoring.healthCheck();
```

### CLI → Server
```bash
# Direct PostgreSQL protocol connection
./builds/linux/rusty-db-cli --host localhost --port 5432
```

---

## 🔐 Security Notes

### Development Mode
- ⚠️ Binds to localhost (127.0.0.1) for security
- ⚠️ No authentication by default
- ⚠️ Not suitable for production

### Production Mode
**Recommendations**:
1. Enable authentication
2. Configure firewall rules
3. Use SSL/TLS certificates
4. Restrict network access
5. Enable audit logging
6. Configure backups

See `/home/user/rusty-db/docs/SECURITY_ARCHITECTURE.md` for details.

---

## 📊 Performance Baseline

### Default Configuration
```
Buffer Pool: 1,000 pages (~4 MB)
Page Size: 4,096 bytes (4 KB)
PostgreSQL Port: 5432
REST API Port: 8080
Max Connections: 100 (configurable via adapter)
```

### Expected Performance
```
Startup Time: < 2 seconds
Memory Usage: ~50-100 MB (minimal dataset)
CPU Usage: < 5% idle, depends on workload
Disk I/O: Depends on query patterns
```

### Scaling
- **Vertical**: Increase buffer_pool_size for more caching
- **Horizontal**: Add read replicas (requires clustering setup)

---

## 🐛 Troubleshooting

### Issue: Port 5432 already in use
```bash
# Check what's using the port
sudo lsof -i :5432

# If PostgreSQL is running
sudo systemctl stop postgresql
```

### Issue: Port 8080 already in use
```bash
# Check what's using the port
sudo lsof -i :8080

# Kill the process or change server port
```

### Issue: Permission denied
```bash
# Make binary executable
chmod +x builds/linux/rusty-db-server

# For production, check ownership
sudo chown -R rustydb:rustydb /var/lib/rusty-db
```

### Issue: Cannot connect from frontend
```bash
# Check server is running
curl http://localhost:8080/api/v1/health

# Check firewall
sudo ufw status
sudo ufw allow 8080/tcp
```

---

## 📚 Documentation

### Core Documents
- **Full Deployment Report**: `/home/user/rusty-db/ENTERPRISE_DEPLOYMENT_COORDINATION_REPORT.md`
- **Project README**: `/home/user/rusty-db/CLAUDE.md`
- **Deployment Guide**: `/home/user/rusty-db/docs/DEPLOYMENT_GUIDE.md`
- **Architecture**: `/home/user/rusty-db/docs/ARCHITECTURE.md`
- **Security**: `/home/user/rusty-db/docs/SECURITY_ARCHITECTURE.md`

### Build Information
- **Build Info**: `/home/user/rusty-db/builds/BUILD_INFO.md`
- **Cargo Config**: `/home/user/rusty-db/Cargo.toml`

### Component READMEs
- **Node.js Adapter**: `/home/user/rusty-db/nodejs-adapter/README.md`
- **Frontend**: `/home/user/rusty-db/frontend/README.md`

---

## 🎯 Next Steps

### For Development
1. ✅ Start server: `./builds/linux/rusty-db-server`
2. ✅ Test connection: `curl http://localhost:8080/api/v1/health`
3. ✅ Start frontend (optional): `cd frontend && npm run dev`
4. ✅ Start coding!

### For Production
1. Read full deployment guide
2. Follow Option 2 (Systemd) above
3. Configure security (SSL, firewall, authentication)
4. Set up monitoring
5. Configure backups
6. Test disaster recovery

### For Integration
1. Review Node.js adapter documentation
2. Install adapter: `cd nodejs-adapter && npm install`
3. Build adapter: `npm run build`
4. Use in your application (see examples)

---

## 📞 Support

**Internal Documentation**:
- See `.scratchpad/` for agent coordination reports
- See `docs/` for comprehensive guides

**Issue Reporting**:
- Check logs: `sudo journalctl -u rustydb -n 100`
- Review error messages
- Consult troubleshooting section above

---

## ✅ Pre-Flight Checklist

Before starting deployment, verify:

- [ ] Linux kernel 3.2.0+ (check: `uname -r`)
- [ ] Ports 5432 and 8080 available
- [ ] Binaries are executable (`ls -la builds/linux/`)
- [ ] Sufficient disk space (20 GB minimum)
- [ ] Sufficient RAM (4 GB minimum)

---

**Deployment Coordinator**: Agent 11
**Status**: ✅ READY FOR IMMEDIATE DEPLOYMENT
**Date**: December 27, 2025

**Quick Start**: Run `./builds/linux/rusty-db-server` from `/home/user/rusty-db/`

---

*For detailed enterprise deployment instructions, see ENTERPRISE_DEPLOYMENT_COORDINATION_REPORT.md*
