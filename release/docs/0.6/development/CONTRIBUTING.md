# RustyDB v0.6.0 - Contributing Guide

**Version**: 0.6.0
**Release**: $856M Enterprise Server
**Last Updated**: 2025-12-28

---

## Table of Contents

1. [Welcome](#welcome)
2. [Code of Conduct](#code-of-conduct)
3. [How to Contribute](#how-to-contribute)
4. [Development Workflow](#development-workflow)
5. [Pull Request Process](#pull-request-process)
6. [Coding Standards](#coding-standards)
7. [Testing Requirements](#testing-requirements)
8. [Documentation Guidelines](#documentation-guidelines)
9. [Security Policy](#security-policy)
10. [Getting Help](#getting-help)

---

## Welcome

Thank you for your interest in contributing to RustyDB! We welcome contributions from developers of all skill levels. Whether you're fixing bugs, adding features, improving documentation, or helping with testing, your contributions are valuable.

### Types of Contributions

- **Bug Reports**: Help us identify and fix issues
- **Feature Requests**: Suggest new capabilities
- **Code Contributions**: Fix bugs or implement features
- **Documentation**: Improve guides, examples, and API docs
- **Testing**: Write tests, improve coverage
- **Performance**: Optimize critical paths
- **Security**: Report vulnerabilities, improve security

---

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive environment for all contributors. We expect everyone to:

- Be respectful and considerate
- Use welcoming and inclusive language
- Accept constructive criticism gracefully
- Focus on what's best for the community
- Show empathy towards others

### Unacceptable Behavior

- Harassment, discrimination, or intimidation
- Trolling, insulting comments, or personal attacks
- Public or private harassment
- Publishing others' private information
- Any conduct inappropriate in a professional setting

### Reporting

If you experience or witness unacceptable behavior, please report it to the project maintainers.

---

## How to Contribute

### Finding Work

1. **Check Issues**: Browse [GitHub Issues](https://github.com/harborgrid-justin/rusty-db/issues)
2. **Good First Issues**: Look for `good-first-issue` label
3. **Help Wanted**: Check `help-wanted` label
4. **Feature Requests**: See `enhancement` label

### Before You Start

1. **Search existing issues** to avoid duplicates
2. **Create an issue** to discuss major changes
3. **Ask questions** in Discussions
4. **Read documentation** to understand the codebase

### Claiming Work

Comment on an issue to let others know you're working on it.

---

## Development Workflow

### 1. Fork and Clone

```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/rusty-db.git
cd rusty-db

# Add upstream remote
git remote add upstream https://github.com/harborgrid-justin/rusty-db.git
```

### 2. Create Feature Branch

```bash
# Update main branch
git checkout main
git pull upstream main

# Create feature branch
git checkout -b feature/your-feature-name
```

**Branch Naming**:
- `feature/feature-name` - New features
- `fix/bug-description` - Bug fixes
- `docs/documentation-update` - Documentation
- `refactor/module-name` - Refactoring
- `test/test-description` - Test additions

### 3. Make Changes

```bash
# Write code
# Add tests
# Update documentation

# Run checks
cargo fmt
cargo clippy
cargo test
```

### 4. Commit Changes

```bash
# Stage changes
git add .

# Commit with descriptive message
git commit -m "[Module] Brief description

Detailed explanation of changes...

Fixes #123"
```

**Commit Message Format**:
```
[Module] Brief description (50 chars max)

Detailed explanation of what and why (not how).
Wrap at 72 characters.

- List specific changes
- Reference issues

Fixes #123
Closes #456
```

**Examples**:
- `[Storage] Add LSM tree compaction support`
- `[Security] Fix SQL injection in query parser`
- `[Docs] Update installation instructions`

### 5. Push and Create PR

```bash
# Push to your fork
git push origin feature/your-feature-name

# Create Pull Request on GitHub
```

---

## Pull Request Process

### Before Submitting

Ensure your PR meets these requirements:

**Code Quality**:
- [ ] Code follows Rust API guidelines
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] No compiler warnings

**Documentation**:
- [ ] Public APIs have doc comments
- [ ] README updated if needed
- [ ] CHANGELOG updated for notable changes
- [ ] Examples added for new features

**Testing**:
- [ ] Unit tests added for new code
- [ ] Integration tests added where appropriate
- [ ] Edge cases covered
- [ ] No regression in existing tests

### PR Title Format

```
[Module] Brief description
```

**Examples**:
- `[Storage] Add LSM tree implementation`
- `[Security] Fix SQL injection vulnerability`
- `[Docs] Update architecture documentation`

### PR Description Template

```markdown
## Description
Brief description of changes.

## Motivation
Why is this change needed? What problem does it solve?

## Changes
- Added X
- Modified Y
- Fixed Z

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manual testing completed
- [ ] Performance benchmarks run (if applicable)

## Breaking Changes
List any breaking changes and migration path.

## Related Issues
Fixes #123
Closes #456

## Checklist
- [ ] Code follows style guidelines
- [ ] Documentation updated
- [ ] Tests pass
- [ ] No new warnings
- [ ] Reviewed my own code
```

### Review Process

1. **Automated Checks**: CI/CD runs automatically
2. **Code Review**: Maintainer reviews code
3. **Feedback**: Address review comments
4. **Approval**: Maintainer approves PR
5. **Merge**: Maintainer merges PR

### Responding to Feedback

- **Be Open**: Accept constructive criticism
- **Ask Questions**: Clarify unclear feedback
- **Make Changes**: Address review comments
- **Push Updates**: Update PR branch
- **Re-request Review**: When ready

---

## Coding Standards

### Follow Rust Guidelines

All code must follow:
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- RustyDB [CODE_STANDARDS.md](./CODE_STANDARDS.md)

### Key Standards

**Formatting**:
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check
```

**Linting**:
```bash
# Run clippy
cargo clippy

# Fail on warnings
cargo clippy -- -D warnings

# Auto-fix
cargo clippy --fix
```

**Naming**:
- Functions: `snake_case`
- Types: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`

**Error Handling**:
```rust
use crate::error::{DbError, Result};

pub fn my_function() -> Result<()> {
    some_operation()?;
    Ok(())
}
```

**Documentation**:
```rust
/// Brief description.
///
/// Detailed explanation...
///
/// # Arguments
///
/// * `arg1` - Description
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// When this function returns an error...
///
/// # Examples
///
/// ```
/// let result = my_function(42)?;
/// ```
pub fn my_function(arg1: i32) -> Result<String> {
    // Implementation
}
```

---

## Testing Requirements

### Test Coverage

All contributions must include tests:

**Unit Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        let result = my_function(42);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_error_case() {
        let result = my_function(-1);
        assert!(result.is_err());
    }
}
```

**Integration Tests**:
```rust
// tests/integration_test.rs
use rusty_db::*;

#[test]
fn test_end_to_end_scenario() {
    // Test complete workflow
}
```

### Running Tests

```bash
# All tests
cargo test

# Specific module
cargo test storage::

# With output
cargo test -- --nocapture

# Single test
cargo test test_buffer_pool
```

### Test Quality

- **Coverage**: Test happy path and edge cases
- **Clarity**: Use descriptive test names
- **Independence**: Tests should be isolated
- **Speed**: Keep tests fast
- **Reliability**: No flaky tests

---

## Documentation Guidelines

### Code Documentation

**Module-Level**:
```rust
//! Module for doing X.
//!
//! This module provides...
//!
//! # Examples
//!
//! ```
//! use module::Thing;
//! let thing = Thing::new();
//! ```
```

**Function-Level**:
```rust
/// Brief description.
///
/// Longer explanation...
///
/// # Arguments
/// # Returns
/// # Errors
/// # Examples
/// # Panics
pub fn function() -> Result<()> { }
```

### User Documentation

Update relevant documentation files:
- `README.md` - Overview and quick start
- `docs/ARCHITECTURE.md` - Architecture details
- `docs/API.md` - API documentation
- `CHANGELOG.md` - Notable changes

### Examples

Provide working examples:
```rust
// examples/basic_usage.rs
use rusty_db::*;

fn main() -> Result<()> {
    // Example code
    Ok(())
}
```

---

## Security Policy

### Reporting Vulnerabilities

**Do NOT** create public issues for security vulnerabilities.

**Instead**:
1. Email security@rustydb.io (if available)
2. Provide detailed description
3. Include reproduction steps
4. Wait for acknowledgment

### Security Best Practices

When contributing:
- **Validate all inputs**
- **Use safe abstractions**
- **Avoid SQL injection**
- **Sanitize user data**
- **Follow security guidelines**
- **Run security audit**: `cargo audit`

### Secure Coding

```rust
// Good - use parameterized queries
let stmt = "SELECT * FROM users WHERE id = ?";
executor.execute(stmt, &[&user_id])?;

// Bad - string concatenation
// let stmt = format!("SELECT * FROM users WHERE id = {}", user_id);
```

---

## Getting Help

### Resources

**Documentation**:
- [DEVELOPMENT_OVERVIEW.md](./DEVELOPMENT_OVERVIEW.md) - Setup guide
- [BUILD_INSTRUCTIONS.md](./BUILD_INSTRUCTIONS.md) - Build guide
- [CODE_STANDARDS.md](./CODE_STANDARDS.md) - Coding standards
- [ARCHITECTURE.md](../architecture/OVERVIEW.md) - System architecture

**Community**:
- GitHub Issues - Bug reports, feature requests
- GitHub Discussions - Questions, ideas
- Discord - Real-time chat (if available)

### Common Questions

**Q: How do I set up my development environment?**
A: See [DEVELOPMENT_OVERVIEW.md](./DEVELOPMENT_OVERVIEW.md)

**Q: How do I run tests?**
A: `cargo test` - See [BUILD_INSTRUCTIONS.md](./BUILD_INSTRUCTIONS.md)

**Q: What are the coding standards?**
A: See [CODE_STANDARDS.md](./CODE_STANDARDS.md)

**Q: How do I submit a pull request?**
A: See [Pull Request Process](#pull-request-process) above

**Q: Can I work on multiple issues?**
A: Yes, but create separate branches and PRs for each

**Q: How long does review take?**
A: Usually 1-7 days, depending on PR size and complexity

---

## Recognition

### Contributors

All contributors are recognized:
- **CONTRIBUTORS.md** - List of contributors
- **GitHub insights** - Contribution statistics
- **Release notes** - Credit for contributions

### Types of Recognition

- First-time contributors highlighted
- Significant contributions featured
- Regular contributors may become maintainers

---

## Advanced Topics

### Large Contributions

For major features or refactoring:

1. **Create RFC (Request for Comments)**
2. **Discuss approach** with maintainers
3. **Break into smaller PRs** if possible
4. **Coordinate with team** to avoid conflicts

### Becoming a Maintainer

Active contributors may be invited to become maintainers. Requirements:
- Consistent high-quality contributions
- Good understanding of codebase
- Helpful in reviews and discussions
- Alignment with project goals

---

## Quick Reference

### Common Commands

```bash
# Setup
git clone https://github.com/YOUR_USERNAME/rusty-db.git
git remote add upstream https://github.com/harborgrid-justin/rusty-db.git

# Development
git checkout -b feature/my-feature
cargo fmt
cargo clippy
cargo test

# Submit
git add .
git commit -m "[Module] Description"
git push origin feature/my-feature
# Create PR on GitHub

# Update
git fetch upstream
git rebase upstream/main
git push --force-with-lease origin feature/my-feature
```

### Checklist Before Submitting PR

- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] All tests pass (`cargo test`)
- [ ] Documentation updated
- [ ] Commit messages follow format
- [ ] PR description complete
- [ ] No merge conflicts
- [ ] Branch up to date with main

---

## License

By contributing to RustyDB, you agree that your contributions will be licensed under the same license as the project.

---

## Thank You

We appreciate your contributions to RustyDB! Every contribution, no matter how small, helps make the project better.

**Questions?** Open an issue or discussion on GitHub.

**Ready to contribute?** Pick an issue and get started!

---

**Welcome to the RustyDB community!**
