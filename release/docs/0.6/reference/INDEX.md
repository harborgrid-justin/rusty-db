# RustyDB v0.6.0 - Quick Reference Index

**Version**: 0.6.0 | **Release Date**: December 28, 2025

Welcome to the RustyDB Quick Reference documentation. These guides provide concise, actionable information for rapid enterprise deployment and daily operations.

---

## Available Quick References

### 1. [QUICK_START.md](./QUICK_START.md) - 5-Minute Quick Start
**Status**: ✅ Complete | **Lines**: 194 | **Size**: 3.3 KB

Get RustyDB running in under 5 minutes. Covers:
- One-command deployment
- First database operations
- Health verification
- Port reference
- System requirements

**Use When**: First-time setup, demos, quick testing

---

### 2. [SQL_REFERENCE.md](./SQL_REFERENCE.md) - SQL Syntax Quick Reference
**Status**: ✅ Complete | **Lines**: 563 | **Size**: 9.9 KB

Complete SQL command reference. Covers:
- DDL statements (CREATE, DROP, ALTER)
- DML statements (INSERT, UPDATE, DELETE)
- SELECT queries with filters
- Aggregate functions
- String and numeric functions
- Transaction control
- Data types and operators

**Use When**: Writing SQL queries, learning syntax, debugging queries

---

### 3. [GRAPHQL_REFERENCE.md](./GRAPHQL_REFERENCE.md) - GraphQL API Quick Reference
**Status**: ✅ Complete | **Lines**: 698 | **Size**: 13 KB

GraphQL API complete reference. Covers:
- Query operations (14 operations)
- Mutation operations (30 operations)
- Subscription operations (8 operations)
- Filter operators
- Aggregate functions
- Error handling
- Pagination strategies
- Complete curl examples

**Use When**: Building applications, API integration, real-time features

---

### 4. [CLI_REFERENCE.md](./CLI_REFERENCE.md) - CLI Command Reference
**Status**: ✅ Complete | **Lines**: 623 | **Size**: 11 KB

Complete command-line reference. Covers:
- Server commands (start, stop, restart)
- CLI client commands
- Build commands (cargo)
- Test commands
- Code quality tools
- Database maintenance
- Log management
- Health checks
- Performance monitoring

**Use When**: Server administration, DevOps, debugging, monitoring

---

### 5. [CONFIG_REFERENCE.md](./CONFIG_REFERENCE.md) - Configuration Quick Reference
**Status**: ✅ Complete | **Lines**: 601 | **Size**: 13 KB

Configuration options and tuning. Covers:
- Server configuration
- Storage configuration (buffer pool, page size)
- Memory management
- Transaction settings
- Logging configuration
- Security settings
- Performance tuning
- Replication and backup
- Environment variables

**Use When**: Performance tuning, production setup, troubleshooting

---

### 6. [INDEX_REFERENCE.md](./INDEX_REFERENCE.md) - Index Types Quick Reference
**Status**: ✅ Complete | **Lines**: 582 | **Size**: 13 KB

Index types and usage guide. Covers:
- 11 index types (B+ Tree, LSM Tree, Hash, R-Tree, etc.)
- When to use each type
- Creation commands (SQL and REST)
- Performance characteristics
- Configuration options
- Index operations
- Selection guide
- Monitoring

**Use When**: Query optimization, performance tuning, schema design

---

### 7. [PROCEDURES_REFERENCE.md](./PROCEDURES_REFERENCE.md) - Stored Procedures Reference
**Status**: ✅ Complete | **Lines**: 783 | **Size**: 15 KB

PL/SQL stored procedures reference. Covers:
- Procedure syntax
- Control structures (IF, LOOP, CASE)
- Variables and types
- Exception handling
- Cursors (explicit, implicit)
- Built-in packages (DBMS_OUTPUT, DBMS_SQL, UTL_FILE)
- Built-in functions
- DML operations
- User-defined functions
- Best practices

**Use When**: Writing stored procedures, business logic, batch processing

---

### 8. [TROUBLESHOOTING_QUICK.md](./TROUBLESHOOTING_QUICK.md) - Quick Troubleshooting
**Status**: ✅ Complete | **Lines**: 665 | **Size**: 14 KB

Rapid problem resolution guide. Covers:
- Quick diagnostics (1-minute health check)
- Common issues and solutions
- Server startup problems
- Connection issues
- Performance problems
- Data issues
- Service issues
- API issues
- Log analysis
- Emergency recovery

**Use When**: Problems occur, debugging, incident response

---

## Quick Navigation

### By Use Case

**First-Time User**:
1. Start here: [QUICK_START.md](./QUICK_START.md)
2. Then read: [SQL_REFERENCE.md](./SQL_REFERENCE.md)
3. Learn APIs: [GRAPHQL_REFERENCE.md](./GRAPHQL_REFERENCE.md)

**Developer**:
- API Integration: [GRAPHQL_REFERENCE.md](./GRAPHQL_REFERENCE.md)
- SQL Queries: [SQL_REFERENCE.md](./SQL_REFERENCE.md)
- Stored Procedures: [PROCEDURES_REFERENCE.md](./PROCEDURES_REFERENCE.md)

**Database Administrator**:
- Server Management: [CLI_REFERENCE.md](./CLI_REFERENCE.md)
- Configuration: [CONFIG_REFERENCE.md](./CONFIG_REFERENCE.md)
- Troubleshooting: [TROUBLESHOOTING_QUICK.md](./TROUBLESHOOTING_QUICK.md)

**Performance Engineer**:
- Indexes: [INDEX_REFERENCE.md](./INDEX_REFERENCE.md)
- Configuration: [CONFIG_REFERENCE.md](./CONFIG_REFERENCE.md)
- Monitoring: [CLI_REFERENCE.md](./CLI_REFERENCE.md)

**Operations/DevOps**:
- Quick Start: [QUICK_START.md](./QUICK_START.md)
- CLI Tools: [CLI_REFERENCE.md](./CLI_REFERENCE.md)
- Troubleshooting: [TROUBLESHOOTING_QUICK.md](./TROUBLESHOOTING_QUICK.md)

---

## Documentation Statistics

| Guide | Lines | Size | Sections | Examples |
|-------|-------|------|----------|----------|
| QUICK_START | 194 | 3.3 KB | 12 | 15+ |
| SQL_REFERENCE | 563 | 9.9 KB | 18 | 100+ |
| GRAPHQL_REFERENCE | 698 | 13 KB | 22 | 50+ |
| CLI_REFERENCE | 623 | 11 KB | 20 | 80+ |
| CONFIG_REFERENCE | 601 | 13 KB | 16 | 30+ |
| INDEX_REFERENCE | 582 | 13 KB | 15 | 40+ |
| PROCEDURES_REFERENCE | 783 | 15 KB | 19 | 60+ |
| TROUBLESHOOTING_QUICK | 665 | 14 KB | 14 | 70+ |
| **TOTAL** | **4,709** | **92 KB** | **136** | **445+** |

---

## Document Features

All quick reference guides include:

✅ **Copy-Paste Ready** - All examples are production-ready
✅ **Concise Format** - Cheat sheet style for quick lookup
✅ **Common Use Cases** - Real-world scenarios covered
✅ **Quick Navigation** - Clear section headers and tables
✅ **Version Specific** - Updated for v0.6.0
✅ **Cross-Referenced** - Links to related guides
✅ **Complete Examples** - Working code snippets

---

## Additional Documentation

For more detailed information, see:

- **Architecture**: `/release/docs/0.6/architecture/`
- **API Documentation**: `/release/docs/0.6/api/`
- **Deployment Guides**: `/release/docs/0.6/deployment/`
- **Security**: `/release/docs/0.6/security/`
- **Enterprise Features**: `/release/docs/0.6/enterprise/`
- **Testing**: `/release/docs/0.6/testing/`

---

## Quick Links

### Essential Commands

```bash
# Start server
./builds/linux/rusty-db-server

# Check health
curl http://localhost:8080/api/v1/health

# Connect with CLI
./builds/linux/rusty-db-cli

# View logs
sudo journalctl -u rustydb -f

# Check configuration
curl http://localhost:8080/api/v1/admin/config | jq
```

### Essential Queries

```bash
# List tables (GraphQL)
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ tables { name rowCount } }"}'

# Execute SQL
curl -X POST http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"sql": "SELECT * FROM users LIMIT 10"}'

# Count rows
curl -X POST http://localhost:8080/graphql \
  -H "Content-Type: application/json" \
  -d '{"query":"{ count(table: \"users\") }"}'
```

---

## Update History

| Date | Version | Changes |
|------|---------|---------|
| 2025-12-28 | 0.6.0 | Initial consolidated quick reference guides |

---

## Contributing

To update these quick references:

1. Update source quick reference files in `/docs/`
2. Regenerate consolidated guides
3. Update version numbers and dates
4. Test all examples
5. Update this index

---

**Quick Reference Documentation** | RustyDB v0.6.0 | Enterprise Database Server

For support, see [TROUBLESHOOTING_QUICK.md](./TROUBLESHOOTING_QUICK.md)
