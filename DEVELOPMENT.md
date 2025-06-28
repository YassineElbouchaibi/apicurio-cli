# Development Guide

This guide covers how to set up a development environment, run tests, and contribute to the Apicurio CLI.

## Prerequisites

- **Rust 1.70+** with Cargo
- **Docker & Docker Compose** (for local registry)
- **Git** for version control

## Environment Setup

### 1. Clone and Build

```bash
git clone https://github.com/YassineElbouchaibi/apicurio-cli.git
cd apicurio-cli
cargo build
```

### 2. Start Local Registry

For development and testing, start a local Apicurio Registry:

```bash
# Start registry with in-memory storage
docker-compose -f examples/docker-compose.yaml up -d

# Verify registry is running
curl http://localhost:8080/apis/registry/v2/system/info
```

### 3. Run the CLI

```bash
# Build and run
cargo run -- --help

# Or build once and use the binary
cargo build
./target/debug/apicurio --help
```

## Testing

### Unit Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test config

# Run with debug logging
RUST_LOG=debug cargo test
```

### Integration Tests

The integration tests require a running Apicurio Registry on `localhost:8080`:

```bash
# Start local registry
docker-compose -f examples/docker-compose.yaml up -d

# Run integration tests
cargo test lockfile_integration

# Run all tests including integration
cargo test
```

### Manual Testing

```bash
# Initialize a test project
mkdir test-project
cd test-project
cargo run -- init

# Add a test dependency (requires running registry)
cargo run -- add local/com.example/test-schema@1.0.0

# Pull dependencies
cargo run -- pull

# Check status
cargo run -- status
```

## Code Organization

```
src/
├── main.rs              # CLI entry point and argument parsing
├── commands/            # Command implementations
│   ├── mod.rs          # Command routing and definitions
│   ├── init.rs         # Project initialization
│   ├── pull.rs         # Dependency fetching
│   ├── add.rs          # Interactive dependency addition
│   ├── remove.rs       # Dependency removal
│   ├── list.rs         # List dependencies and registries
│   ├── status.rs       # Check for outdated dependencies
│   ├── update.rs       # Update dependencies
│   ├── lock.rs         # Lock file generation
│   ├── publish.rs      # Artifact publishing
│   ├── registry.rs     # Registry management
│   ├── verify.rs       # Integrity verification
│   ├── doctor.rs       # Configuration validation
│   └── completions.rs  # Shell completion generation
├── config.rs           # Configuration loading and merging
├── lockfile.rs         # Lock file operations
├── registry.rs         # Registry client implementation
├── dependency.rs       # Dependency resolution logic
├── identifier.rs       # Identifier parsing utilities
└── constants.rs        # Shared constants
```

## Architecture Overview

### Configuration Management

The tool uses a hierarchical configuration system:

1. **Global registries** (`~/.config/apicurio/registries.yaml`)
2. **External registries** (specified by `externalRegistriesFile`)
3. **Repository config** (`apicurioconfig.yaml`)

Configurations are merged with later definitions overriding earlier ones.

### Dependency Resolution

1. **Parse version ranges** using semver crate
2. **Query registry** for available versions
3. **Select best match** based on semver constraints
4. **Generate lock file** with exact versions and checksums

### Authentication

Supports multiple auth types:
- **None**: Anonymous access
- **Basic**: Username + password from env var
- **Token**: Custom token header from env var  
- **Bearer**: OAuth/JWT bearer token from env var

## Contributing

### Code Style

- Follow Rust conventions and `cargo fmt`
- Add documentation for public APIs
- Include examples in doc comments where helpful
- Use `clippy` to catch common issues

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check documentation
cargo doc --open
```

### Adding New Commands

1. Create new module in `src/commands/`
2. Add module declaration in `src/commands/mod.rs`
3. Add command variant to `Commands` enum
4. Add command handler to `run()` function
5. Add tests for the new functionality

Example command structure:

```rust
// src/commands/mycommand.rs
use anyhow::Result;

/// Implementation of the my-command functionality
pub async fn run(arg: String) -> Result<()> {
    // Command implementation
    println!("Running my command with arg: {}", arg);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_command() {
        let result = run("test".to_string()).await;
        assert!(result.is_ok());
    }
}
```

### Testing Guidelines

- Add unit tests for new functionality
- Include integration tests for registry operations
- Test error conditions and edge cases
- Use `tempfile` crate for temporary test files
- Mock network calls when appropriate

### Documentation

- Add rustdoc comments to public APIs
- Include examples in documentation
- Update README.md for user-facing changes
- Add usage examples for new commands

## Debugging

### Enable Debug Logging

```bash
# Debug level logging
RUST_LOG=debug cargo run -- pull

# Trace level for network requests
RUST_LOG=reqwest=trace cargo run -- pull

# Module-specific logging
RUST_LOG=apicurio_cli::registry=debug cargo run -- pull
```

### Common Development Issues

**1. Registry Connection Failures**
```bash
# Check if registry is running
curl http://localhost:8080/apis/registry/v2/system/info

# Restart registry
docker-compose -f examples/docker-compose.yaml restart
```

**2. Lock File Issues**
```bash
# Delete and regenerate lock file
rm apicuriolock.yaml
cargo run -- lock
```

**3. Authentication Problems**
```bash
# Check environment variables
echo $APICURIO_TOKEN

# Test registry access manually
curl -H "Authorization: Bearer $APICURIO_TOKEN" \
  http://localhost:8080/apis/registry/v2/groups
```

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Create git tag
5. Build release binary
6. Publish to cargo (if applicable)

```bash
# Update version and test
cargo test
cargo build --release

# Tag release
git tag v0.2.0
git push origin v0.2.0

# Build release artifacts
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-apple-darwin
cargo build --release --target x86_64-pc-windows-gnu
```

## Performance Considerations

- Use async/await for network operations
- Implement connection pooling for multiple requests
- Cache registry metadata when possible
- Use streaming for large file downloads
- Implement retry logic with backoff

## Security Considerations

- Never log authentication tokens
- Store credentials only in environment variables
- Validate downloaded file checksums
- Use HTTPS for all registry communications
- Implement proper error handling to avoid information leaks
