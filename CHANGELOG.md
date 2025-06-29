# Changelog

All notable changes to the Apicurio CLI project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.5] - 2025-06-29

### Added
- Smart dependency resolution for `groupId` and `artifactId` fields
  - Dependencies can now use `name` in "group/artifact" format for automatic resolution
  - `groupId` and `artifactId` are now optional in dependency configuration
  - Explicit fields override smart resolution when needed
  - Consistent behavior with publishing configuration
- Advanced reference resolution system for automatic transitive dependency management
  - New `referenceResolution` configuration section with global and per-dependency controls
  - Flexible output path patterns with advanced variable substitution (e.g., `{artifactId.path}`, `{artifactId.lastLowercase}`)
  - Output overrides for complex artifact name mappings
  - Registry-specific override support with fallback to group-level mappings
  - Per-dependency `resolveReferences` control to enable/disable reference resolution
  - Maximum depth protection to prevent infinite recursion
  - Support for excluding specific artifacts from resolution (set to `null`)
- New documentation file `REFERENCE_RESOLUTION.md` with comprehensive examples
- New example configurations:
  - `examples/nprod-example.yaml` - Demonstrates solving complex artifact name mappings
  - `examples/reference-resolution-example.yaml` - Shows advanced reference resolution features

### Changed
- Dependency configuration now supports smart resolution from `name` field
- Updated documentation with comprehensive smart resolution examples
- Enhanced unit tests for edge cases in dependency resolution
- Lock file generation now includes transitive dependencies from reference resolution
- Configuration schema expanded to support reference resolution settings

### Fixed
- Identifier matching now uses resolved values for accurate dependency lookup
- Remove command displays resolved artifact IDs for better user experience

## [0.1.3] - 2025-06-28

### Added
- Core CLI tool for managing Apicurio Registry artifacts
- Lockfile-based dependency management for reproducible builds
- Support for multiple artifact types (Protobuf, Avro, JSON Schema, OpenAPI, etc.)
- Multi-registry support with flexible authentication
- Semantic version resolution with range support
- Publishing capabilities for uploading artifacts
- Configuration validation and connectivity checking
- Integrity verification with SHA256 checksums
- Comprehensive documentation including README, development guide, and tutorial
- Example configuration files for common use cases
- Docker Compose setup for local development
- Enhanced rustdoc comments throughout the codebase
- Shell completion support for bash, zsh, and fish
- CI/CD integration examples for GitHub Actions and Jenkins
- cargo-make integration with comprehensive task definitions

### Commands
- `init` - Initialize new project with config and lock files
- `pull` - Fetch dependencies according to lock file
- `update` - Re-resolve dependencies to latest matching versions
- `add` - Add new dependencies interactively
- `remove` - Remove existing dependencies
- `list` - List configured dependencies and registries
- `status` - Check for outdated dependencies
- `verify` - Verify downloaded file integrity
- `lock` - Update lock file without downloading
- `publish` - Publish artifacts to registries
- `registry` - Manage global registry configurations
- `doctor` - Validate configuration and connectivity
- `completions` - Generate shell completion scripts

### Authentication Support
- None (anonymous access)
- Basic authentication with username/password
- Token-based authentication
- Bearer token authentication (OAuth/JWT)

### Configuration Features
- Hierarchical configuration merging (global, external, repository)
- Environment variable expansion in configuration files
- Smart defaults for publishing configuration
- Artifact type auto-detection from file extensions
- Reference management for artifact dependencies
