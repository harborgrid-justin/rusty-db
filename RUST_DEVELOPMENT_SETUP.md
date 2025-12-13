# Rust Development Setup

## Environment Information
- **Rust Version:** 1.92.0 (stable)
- **Cargo Version:** 1.92.0
- **Platform:** x86_64-unknown-linux-gnu
- **OS:** Ubuntu 24.04.3 LTS (Dev Container)

## Installed Components
- ✅ rustc (Rust compiler)
- ✅ cargo (Rust package manager)
- ✅ clippy (Rust linter)
- ✅ rustfmt (Rust formatter)
- ✅ rust-docs (Offline documentation)

## Additional Tools Installed
- **cargo-edit** - Add, remove, and upgrade dependencies
  - `cargo add <crate>` - Add a dependency
  - `cargo rm <crate>` - Remove a dependency
  - `cargo upgrade` - Upgrade dependencies

- **cargo-watch** - Watch for file changes and run commands
  - `cargo watch -x check` - Check on file changes
  - `cargo watch -x test` - Run tests on file changes
  - `cargo watch -x run` - Run project on file changes

- **cargo-audit** - Security vulnerability scanner
  - `cargo audit` - Check for security vulnerabilities

## Quick Start Commands

### Building
```bash
cargo build                 # Debug build
cargo build --release       # Release build
cargo check                 # Fast syntax check without producing binary
```

### Running
```bash
cargo run                   # Run debug build
cargo run --release         # Run release build
```

### Testing
```bash
cargo test                  # Run all tests
cargo test --nocapture      # Run tests with output
cargo test <test_name>      # Run specific test
```

### Linting & Formatting
```bash
cargo clippy                # Run linter
cargo fmt                   # Format code
cargo fmt --check           # Check formatting without modifying
```

### Documentation
```bash
cargo doc --open            # Generate and open documentation
```

### Cleaning
```bash
cargo clean                 # Remove build artifacts
```

## VS Code Tasks Available
This workspace has pre-configured tasks for common operations:
- cargo build
- cargo build (release)
- cargo check
- cargo clippy
- cargo test
- cargo test (with output)
- cargo run
- cargo run (release)
- cargo doc (open)
- cargo clean
- cargo fmt
- cargo fmt (check)
- Full CI Check
- cargo bench

Run tasks with: Ctrl+Shift+P → "Tasks: Run Task"

## Environment Setup

The Rust environment is installed at `~/.cargo`. To ensure it's available in new shell sessions, run:

```bash
source "$HOME/.cargo/env"
```

Or add this line to your shell profile (~/.bashrc, ~/.zshrc, etc.):
```bash
. "$HOME/.cargo/env"
```

## Useful Resources
- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/)
- [crates.io](https://crates.io/) - Package registry

## Project Structure
This is a Rust database project (`rusty-db`) with comprehensive testing and CI/CD configurations. Check the other documentation files in the workspace for module-specific information.
