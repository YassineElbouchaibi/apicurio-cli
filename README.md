# Apicurio CLI

A powerful Rust-based command-line tool for managing schema artifacts from Apicurio Registry. The `apicurio` CLI provides dependency management for Protobuf, Avro, JSON Schema, and other schema artifacts with lockfile-based reproducible builds.

## Installation

### From Source

```bash
git clone https://github.com/YassineElbouchaibi/apicurio-cli.git
cd apicurio-cli
cargo build --release
cp target/release/apicurio /usr/local/bin/
```

### From Cargo

```bash
cargo install apicurio-cli
```

## Features

- 🔒 **Lockfile-based dependency management** - Reproducible builds with exact version locking
- 📦 **Multiple artifact types** - Support for Protobuf, Avro, JSON Schema, OpenAPI, GraphQL, and more
- 🔐 **Flexible authentication** - Basic, token, and bearer authentication support
- 🌐 **Multi-registry support** - Work with multiple Apicurio Registry instances
- 📋 **Semver resolution** - Smart semantic version resolution with range support
- 🚀 **Publishing capabilities** - Publish artifacts back to registries
- 🔍 **Status monitoring** - Check for outdated dependencies
- ✅ **Integrity verification** - SHA256 checksums for artifact validation

## Installation

### From Source

```bash
git clone https://github.com/your-org/apicurio-cli.git
cd apicurio-cli
cargo build --release
cp target/release/apicurio /usr/local/bin/
```

### From Cargo

```bash
cargo install apicurio-cli
```

## Quick Start

1. **Initialize a new project:**
   ```bash
   apicurio init
   ```

2. **Add a dependency:**
   ```bash
   apicurio add my-registry/com.example/user-service@^1.0.0
   ```

3. **Pull dependencies:**
   ```bash
   apicurio pull
   ```

4. **Check status:**
   ```bash
   apicurio status
   ```

## Configuration

### Repository Configuration (`apicurioconfig.yaml`)

The main configuration file that defines registries, dependencies, and publishing configuration:

```yaml
# Optional: path to external registries file
externalRegistriesFile: ${APICURIO_REGISTRIES_PATH:-}

# Registry definitions
registries:
  - name: production
    url: https://registry.example.com
    auth:
      type: bearer
      tokenEnv: APICURIO_TOKEN
  - name: staging
    url: https://staging-registry.example.com
    auth:
      type: basic
      username: admin
      passwordEnv: STAGING_PASSWORD

# Dependencies to fetch
dependencies:
  - name: user-service-protos
    groupId: com.example.services
    artifactId: user-service
    version: ^1.2.0
    registry: production
    outputPath: protos/user-service.proto
  - name: payment-schemas
    groupId: com.example.schemas
    artifactId: payment-events
    version: ~2.1.0
    registry: production
    outputPath: schemas/payment.avsc

# Artifacts to publish
publishes:
  - name: com.example/my-service
    inputPath: protos/my-service.proto
    version: 1.0.0
    registry: production
    type: protobuf
    description: "My service API definition"
    labels:
      team: backend
      service: my-service
```

### Global Registries (`~/.config/apicurio/registries.yaml`)

Global registry definitions shared across projects:

```yaml
registries:
  - name: company-registry
    url: https://registry.company.com
    auth:
      type: bearer
      tokenEnv: COMPANY_REGISTRY_TOKEN
```

### Lock File (`apicuriolock.yaml`)

Auto-generated file containing exact resolved versions and checksums:

```yaml
lockedDependencies:
  - name: user-service-protos
    registry: production
    resolvedVersion: 1.2.3
    downloadUrl: https://registry.example.com/apis/registry/v3/groups/com.example.services/artifacts/user-service/versions/1.2.3/content
    sha256: a1b2c3d4e5f6...
    outputPath: protos/user-service.proto
    groupId: com.example.services
    artifactId: user-service
    versionSpec: ^1.2.0
lockfileVersion: 1
configHash: abc123...
generatedAt: "1735387200000000000"
```

## Commands

### Core Commands

| Command | Description |
|---------|-------------|
| `init` | Initialize a new project with config and lock files |
| `pull` | Fetch dependencies according to lock file (or resolve if no lock exists) |
| `update` | Re-resolve semver ranges and update lock file |
| `lock` | Update lock file based on current config without downloading |

### Dependency Management

| Command | Description |
|---------|-------------|
| `add <identifier>` | Add a new dependency (interactive if identifier incomplete) |
| `remove <identifier>` | Remove a dependency by identifier |
| `list` | List all configured dependencies and registries |
| `status` | Check for outdated dependencies |

### Registry Management

| Command | Description |
|---------|-------------|
| `registry add <name> <url>` | Add a registry to global config |
| `registry list` | List all configured registries |
| `registry remove <name>` | Remove a registry from global config |

### Publishing & Verification

| Command | Description |
|---------|-------------|
| `publish [name]` | Publish artifacts to registries |
| `verify` | Verify downloaded files against lock file checksums |
| `doctor` | Validate configuration and connectivity |

### Utilities

| Command | Description |
|---------|-------------|
| `completions <shell>` | Generate shell completion scripts |

## Examples

### Basic Workflow

```bash
# Initialize a new project
apicurio init

# Add a Protobuf dependency
apicurio add production/com.example/user-service@^1.0.0

# Pull dependencies
apicurio pull

# Check for updates
apicurio status

# Update to latest matching versions
apicurio update
```

### Working with Multiple Registries

```bash
# Add registries
apicurio registry add prod https://prod-registry.com
apicurio registry add dev https://dev-registry.com

# Add dependencies from different registries
apicurio add prod/com.example/users@^1.0.0
apicurio add dev/com.example/orders@^2.0.0
```

### Publishing Artifacts

```bash
# Configure a publish target in apicurioconfig.yaml
cat >> apicurioconfig.yaml << EOF
publishes:
  - name: com.example/my-api
    inputPath: protos/my-api.proto
    version: 1.0.0
    registry: production
    type: protobuf
    description: "My API definition"
EOF

# Publish the artifact
apicurio publish com.example/my-api
```

### Environment Variables

```bash
# Set authentication tokens
export APICURIO_TOKEN="your-bearer-token"
export STAGING_PASSWORD="your-password"

# Override registries file location
export APICURIO_REGISTRIES_PATH="/custom/path/registries.yaml"

# Pull dependencies
apicurio pull
```

## Authentication

### None (Anonymous)
```yaml
auth:
  type: none
```

### Basic Authentication
```yaml
auth:
  type: basic
  username: admin
  passwordEnv: REGISTRY_PASSWORD
```

### Token Authentication
```yaml
auth:
  type: token
  tokenEnv: REGISTRY_TOKEN
```

### Bearer Authentication
```yaml
auth:
  type: bearer
  tokenEnv: REGISTRY_BEARER_TOKEN
```

## Artifact Types

The CLI supports various artifact types with automatic content-type detection:

- **Protobuf** (`.proto`) - `application/x-protobuf`
- **Avro** (`.avsc`) - `application/json`
- **JSON Schema** (`.json`) - `application/json`
- **OpenAPI** (`.yaml`, `.json`) - `application/json`
- **GraphQL** (`.graphql`, `.gql`) - `application/graphql`
- **XML/WSDL** (`.xml`) - `application/xml`

## Semver Support

Version specifications support standard semantic versioning:

- `1.2.3` - Exact version
- `^1.2.0` - Compatible releases (>=1.2.0, <2.0.0)
- `~1.2.0` - Reasonably close (>=1.2.0, <1.3.0)
- `>=1.1.0, <1.4.0` - Range specification

## Development Setup

### Prerequisites

- Rust 1.70+ with Cargo
- Access to an Apicurio Registry instance (for testing)

### Building from Source

```bash
git clone https://github.com/YassineElbouchaibi/apicurio-cli.git
cd apicurio-cli
cargo build
```

### Running Tests

```bash
# Run unit tests
cargo test

# Run with logging
RUST_LOG=debug cargo test

# Run specific test
cargo test lockfile_integration

# Using cargo-make for comprehensive testing
cargo make test
cargo make test-integration
cargo make ci
```

### Development Environment

1. **Start a local Apicurio Registry:**
   ```bash
   docker-compose -f docker-compose.dev.yml up -d
   ```

2. **Build and test:**
   ```bash
   cargo build
   ./target/debug/apicurio --help
   ```

3. **Run integration tests:**
   ```bash
   # Ensure local registry is running on localhost:8080
   cargo test lockfile_integration
   ```

### Code Structure

```
src/
├── main.rs              # CLI entry point
├── commands/            # Command implementations
│   ├── mod.rs          # Command routing
│   ├── init.rs         # Project initialization
│   ├── pull.rs         # Dependency fetching
│   ├── add.rs          # Dependency addition
│   └── ...
├── config.rs           # Configuration management
├── lockfile.rs         # Lock file operations
├── registry.rs         # Registry client
├── dependency.rs       # Dependency resolution
└── identifier.rs       # Identifier parsing
```

## Configuration Reference

### Repository Config Schema

```yaml
# Optional external registries file
externalRegistriesFile: string

# Registry definitions
registries:
  - name: string                    # Required: unique registry name
    url: string                     # Required: registry base URL
    auth:                          # Optional: authentication config
      type: none|basic|token|bearer # Required if auth present
      username: string              # Required for basic auth
      passwordEnv: string           # Required for basic auth
      tokenEnv: string              # Required for token/bearer auth

# Dependencies to fetch
dependencies:
  - name: string           # Required: local alias
    groupId: string        # Required: artifact group
    artifactId: string     # Required: artifact ID
    version: string        # Required: semver specification
    registry: string       # Required: registry name reference
    outputPath: string     # Required: local file path

# Publishing configuration
publishes:
  - name: string                    # Required: publish identifier
    inputPath: string               # Required: source file path
    version: string                 # Required: exact version
    registry: string                # Required: target registry
    type: protobuf|avro|...        # Optional: auto-detected from extension
    groupId: string                 # Optional: defaults from name
    artifactId: string              # Optional: defaults from name
    ifExists: FAIL|CREATE_VERSION|FIND_OR_CREATE_VERSION
    description: string             # Optional: artifact description
    labels:                         # Optional: key-value labels
      key: value
    references:                     # Optional: artifact references
      - name: string                # Reference identifier
        version: string             # Exact version (no ranges)
        nameAlias: string           # Optional: import alias
```

## Troubleshooting

### Common Issues

**1. Authentication failures:**
```bash
# Check environment variables
echo $APICURIO_TOKEN

# Verify registry connectivity
apicurio doctor
```

**2. Lock file conflicts:**
```bash
# Regenerate lock file
rm apicuriolock.yaml
apicurio lock
```

**3. Version resolution failures:**
```bash
# Check available versions
apicurio status

# Update to latest compatible versions
apicurio update
```

**4. Network connectivity:**
```bash
# Test registry connectivity
curl -H "Authorization: Bearer $APICURIO_TOKEN" https://registry.example.com/apis/registry/v3/groups
```

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug apicurio pull

# Trace network requests
RUST_LOG=reqwest=trace apicurio pull
```

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make changes and add tests
4. Run tests: `cargo test`  
5. Format code: `cargo fmt`
6. Run linter: `cargo clippy`
7. Submit a pull request

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
