# Quick Start Guide

## Installation

### Prerequisites
- Rust 1.70 or higher
- Git

### Clone and Build

```bash
git clone https://github.com/harborgrid-justin/rusty-db.git
cd rusty-db
cargo build --release
```

## Running the Database

### Start the Server

Terminal 1:
```bash
cargo run --bin rusty-db-server
```

You should see:
```
╔══════════════════════════════════════════════════════════╗
║          RustyDB - Enterprise Database System           ║
║         Rust-based Oracle DB Competitor v0.1.0          ║
╚══════════════════════════════════════════════════════════╝

Starting RustyDB server...
Data directory: ./data
Port: 5432
Page size: 4096 bytes
Buffer pool size: 1000 pages

Server listening on 127.0.0.1:5432
Ready to accept connections!
```

### Connect with CLI

Terminal 2:
```bash
cargo run --bin rusty-db-cli
```

You should see:
```
╔══════════════════════════════════════════════════════════╗
║          RustyDB CLI - Interactive SQL Client           ║
╚══════════════════════════════════════════════════════════╝

Connecting to RustyDB server at 127.0.0.1:5432...
Connected successfully!
Type SQL commands or 'exit' to quit.

rustydb> 
```

## Basic Operations

### Create a Table

```sql
rustydb> CREATE TABLE employees (id INT, name VARCHAR(255), salary INT);
0 row(s) affected
```

### Insert Data

```sql
rustydb> INSERT INTO employees (id, name, salary) VALUES (1, 'Alice Johnson', 75000);
1 row(s) affected
```

### Query Data

```sql
rustydb> SELECT * FROM employees;
id                  name                salary              
------------------------------------------------------------
0 row(s) affected
```

Note: Full data storage and retrieval will be implemented in future versions.

### Update Data

```sql
rustydb> UPDATE employees SET salary = 80000 WHERE id = 1;
0 row(s) affected
```

### Delete Data

```sql
rustydb> DELETE FROM employees WHERE id = 1;
0 row(s) affected
```

### Drop Table

```sql
rustydb> DROP TABLE employees;
0 row(s) affected
```

## Transaction Example

```sql
rustydb> BEGIN;
Transaction started: 0

rustydb> INSERT INTO employees VALUES (1, 'Alice', 75000);
1 row(s) affected

rustydb> INSERT INTO employees VALUES (2, 'Bob', 80000);
1 row(s) affected

rustydb> COMMIT;
OK
```

## Exit

```sql
rustydb> exit
Goodbye!
```

Or use `Ctrl+C` to exit.

## Testing

Run the test suite:

```bash
cargo test
```

Expected output:
```
running 14 tests
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured
```

## Demo Script

Run the demo to see project statistics and features:

```bash
./demo.sh
```

## Troubleshooting

### Server won't start
- Check if port 5432 is already in use
- Ensure you have write permissions in the current directory

### CLI can't connect
- Verify the server is running
- Check firewall settings

### Build errors
- Update Rust: `rustup update`
- Clean build: `cargo clean && cargo build`

## Next Steps

1. Explore the [Architecture Documentation](ARCHITECTURE.md)
2. Read the full [README](README.md)
3. Check out the source code in `src/`
4. Contribute improvements via pull requests

## Support

For issues or questions:
- Open an issue on GitHub
- Check existing documentation
- Review the test cases for usage examples

---

**Happy querying with RustyDB!** 🦀
