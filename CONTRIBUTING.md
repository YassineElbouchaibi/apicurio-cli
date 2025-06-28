# Contributing to Apicurio CLI

Thank you for your interest in contributing to the Apicurio CLI! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Process](#development-process)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Issue Reporting](#issue-reporting)

## Code of Conduct

This project adheres to a code of conduct that we expect all contributors to follow. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Git
- Docker and Docker Compose (for testing)

### Setting Up Development Environment

1. **Fork the repository** on GitHub

2. **Clone your fork:**
   ```bash
   git clone https://github.com/your-username/apicurio-cli.git
   cd apicurio-cli
   ```

3. **Set up upstream remote:**
   ```bash
   git remote add upstream https://github.com/YassineElbouchaibi/apicurio-cli.git
   ```

4. **Build the project:**
   ```bash
   cargo build
   ```

5. **Run tests:**
   ```bash
   # Start local registry for integration tests
   docker-compose -f examples/docker-compose.yaml up -d
   
   # Run all tests
   cargo test
   ```

## Development Process

### Branching Strategy

- `main` - Stable branch with latest release
- `develop` - Integration branch for features (if using GitFlow)
- `feature/your-feature-name` - Feature branches
- `fix/issue-description` - Bug fix branches

### Making Changes

1. **Create a feature branch:**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following the coding standards

3. **Add tests** for new functionality

4. **Update documentation** if needed

5. **Commit your changes:**
   ```bash
   git commit -m "feat: add new feature description"
   ```

   Use [Conventional Commits](https://www.conventionalcommits.org/) format:
   - `feat:` - New features
   - `fix:` - Bug fixes
   - `docs:` - Documentation changes
   - `test:` - Test changes
   - `refactor:` - Code refactoring
   - `chore:` - Maintenance tasks

## Pull Request Process

1. **Update your branch** with latest upstream changes:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Push your branch:**
   ```bash
   git push origin feature/your-feature-name
   ```

3. **Create a Pull Request** on GitHub with:
   - Clear title and description
   - Reference to related issues
   - List of changes made
   - Testing information

4. **Address review feedback** promptly

5. **Ensure CI passes** before merging

### Pull Request Checklist

- [ ] Code follows project coding standards
- [ ] Tests added for new functionality
- [ ] All tests pass locally
- [ ] Documentation updated (if applicable)
- [ ] Changelog updated (for significant changes)
- [ ] Commit messages follow conventional format

## Coding Standards

### Rust Code Style

- Follow standard Rust conventions
- Use `cargo fmt` for consistent formatting
- Use `cargo clippy` to catch common issues
- Add documentation for public APIs

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check documentation
cargo doc
```

### Code Organization

- Keep modules focused and cohesive
- Use descriptive names for functions and variables
- Add comprehensive error handling
- Include examples in documentation

### Example Code Structure

```rust
//! Module documentation
//!
//! Detailed description of what this module does.

use std::collections::HashMap;
use anyhow::Result;

/// Struct documentation with examples
///
/// # Examples
///
/// ```
/// use apicurio_cli::MyStruct;
/// let instance = MyStruct::new("example");
/// ```
pub struct MyStruct {
    /// Field documentation
    pub field: String,
}

impl MyStruct {
    /// Function documentation
    ///
    /// # Arguments
    /// * `param` - Description of parameter
    ///
    /// # Returns
    /// Description of return value
    ///
    /// # Errors
    /// Description of possible errors
    pub fn new(param: &str) -> Result<Self> {
        // Implementation
        Ok(Self {
            field: param.to_string(),
        })
    }
}
```

## Testing Guidelines

### Test Types

1. **Unit Tests** - Test individual functions and modules
2. **Integration Tests** - Test command-line interface
3. **Registry Tests** - Test against local Apicurio Registry

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_unit_functionality() {
        // Unit test example
        let result = my_function("input");
        assert_eq!(result.unwrap(), "expected");
    }

    #[tokio::test]
    async fn test_async_functionality() {
        // Async test example
        let result = async_function().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_with_registry() {
        // Integration test with registry
        // Note: Requires running registry on localhost:8080
        let config = test_config();
        let result = registry_operation(config).await;
        assert!(result.is_ok());
    }
}
```

### Test Environment

- Use `tempfile` crate for temporary files/directories
- Mock external dependencies when possible
- Ensure tests are deterministic and don't depend on external state
- Clean up resources after tests

### Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# With logging
RUST_LOG=debug cargo test
```

## Documentation

### Types of Documentation

1. **API Documentation** - Rustdoc comments in code
2. **User Guide** - README and tutorial files
3. **Developer Guide** - DEVELOPMENT.md
4. **Examples** - Working configuration examples

### Documentation Standards

- Use complete sentences in documentation
- Include examples where helpful
- Document error conditions
- Keep documentation up-to-date with code changes

### Building Documentation

```bash
# Generate and open docs
cargo doc --open

# Generate without dependencies
cargo doc --no-deps

# Check documentation coverage
cargo doc --document-private-items
```

## Issue Reporting

### Before Reporting

1. Search existing issues for duplicates
2. Check if issue exists in latest version
3. Try to reproduce with minimal example

### Issue Template

When reporting bugs, include:

- **Version** of apicurio-cli
- **Operating system** and version
- **Steps to reproduce** the issue
- **Expected behavior**
- **Actual behavior**
- **Error messages** or logs
- **Configuration files** (sanitized)

### Feature Requests

For feature requests, include:

- **Use case** description
- **Proposed solution** or API
- **Alternatives considered**
- **Additional context**

## Release Process

### Version Numbering

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR** - Incompatible API changes
- **MINOR** - Backward-compatible functionality additions
- **PATCH** - Backward-compatible bug fixes

### Release Checklist

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Create git tag
5. Build release artifacts
6. Update documentation
7. Announce release

## Getting Help

- **Documentation** - Check README, DEVELOPMENT.md, and rustdocs
- **Issues** - Search existing issues or create new one
- **Discussions** - Use GitHub Discussions for questions
- **Code Review** - Ask for feedback in pull requests

## Recognition

Contributors will be recognized in:

- Git commit history
- Release notes
- Contributors section (if added)

Thank you for contributing to make Apicurio CLI better!
