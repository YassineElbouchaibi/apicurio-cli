# Testing Strategy Guide

This document explains how tests are organized in the Apicurio CLI project and provides examples of how to add new tests.

## Test Organization

### 1. Unit Tests (Lib Tests)
Unit tests are embedded within the source files they test, using `#[cfg(test)]` modules.

**Location**: `src/` directory, in the same files as the code being tested

**Purpose**: Test individual functions, methods, and structs in isolation

**Example**:
```rust
// In src/lockfile.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_hash_computation() {
        // Test implementation
    }
}
```

**Run unit tests only**:
```bash
cargo test --lib
```

### 2. Integration Tests
Integration tests are in separate files in the `tests/` directory at the project root.

**Location**: `tests/` directory

**Purpose**: Test the interaction between multiple modules, public API usage, and end-to-end functionality

**Examples**:
- `tests/lockfile_integration_tests.rs` - Tests lockfile functionality with real file I/O
- `tests/cli_integration_tests.rs` - Tests CLI commands end-to-end

**Run integration tests only**:
```bash
cargo test --test lockfile_integration_tests
cargo test --test cli_integration_tests
```

## Key Differences

| Aspect | Unit Tests | Integration Tests |
|--------|------------|-------------------|
| **Location** | `src/` files | `tests/` directory |
| **Access** | Can test private functions | Only public API |
| **Scope** | Single module/function | Multiple modules/components |
| **Compilation** | Part of library crate | Separate test executables |
| **Imports** | `use super::*;` | `use apicurio_cli::module;` |

## Running Tests

### All Tests
```bash
cargo test                          # All tests (unit + integration)
```

### Unit Tests Only
```bash
cargo test --lib                    # Unit tests only
```

### Integration Tests Only
```bash
cargo test --tests                  # All integration tests

# Specific integration test files
cargo test --test lockfile_integration_tests
cargo test --test cli_integration_tests  
```

### Test Runners with Different Features

### Using Cargo Make
```bash
cargo make test                     # All tests
cargo make test-unit               # Unit tests only
cargo make test-integration        # All integration tests
cargo make test-integration-lockfile # Lockfile integration tests only
cargo make test-integration-cli    # CLI integration tests only
cargo make test-watch              # Watch mode
cargo make test-ci                 # CI-style testing
cargo make dev-check               # Quick development check
```

### Specific Test Function
```bash
cargo test test_config_hash_computation
cargo test test_lockfile_regeneration_scenarios
```

### Tests with Output
```bash
cargo test -- --nocapture
```

## Test Naming Conventions

### Unit Tests
- File: Same as module being tested
- Module: `#[cfg(test)] mod tests`
- Function: `test_function_name_scenario`

### Integration Tests
- File: `{feature}_integration_tests.rs`
- Function: `test_{feature}_{scenario}`

## Adding New Tests

### Adding a Unit Test
1. Find the source file you want to test
2. Add a test within the existing `#[cfg(test)] mod tests` block
3. Use `use super::*;` to import the module's items

```rust
#[test]
fn test_new_functionality() {
    // Test code here
}
```

### Adding an Integration Test
1. Create a new file in `tests/` directory (or add to existing)
2. Import the crate modules you need
3. Write tests that use the public API

```rust
use apicurio_cli::{config, lockfile};

#[test]
fn test_integration_scenario() {
    // Test code here
}
```

## Test Utilities

### Common Test Dependencies
The following dev-dependencies are available for testing:
- `tempfile` - For creating temporary directories and files
- `cargo-make` - For build automation

### Example Patterns

#### Testing with Temporary Files
```rust
use tempfile::TempDir;

#[test]
fn test_with_temp_dir() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.yaml");
    // Test with temporary files
}
```

#### Testing CLI Commands
```rust
use std::process::Command;

#[test]
fn test_cli_command() {
    let output = Command::new("cargo")
        .args(&["run", "--", "help"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
}
```

## Test Categories

### Fast Tests (Unit Tests)
- Pure function testing
- Data structure validation
- Algorithm verification
- Mock/stub interactions

### Slow Tests (Integration Tests)
- File I/O operations
- Network requests (when applicable)
- CLI command execution
- Full workflow testing

## Best Practices

1. **Unit tests** should be fast and test single responsibilities
2. **Integration tests** should test realistic user scenarios
3. Use descriptive test names that explain the scenario
4. Include both positive and negative test cases
5. Use `tempfile` for tests that need file system access
6. Mock external dependencies in unit tests
7. Test actual external interactions in integration tests

## Debugging Tests

### Running with Debug Output
```bash
cargo test -- --nocapture
```

### Running Specific Tests
```bash
cargo test test_name
```

### Running Tests in VS Code
- Use the "Run Test" code lens above each test function
- Use the Testing panel to run groups of tests
- Set breakpoints in tests for debugging
