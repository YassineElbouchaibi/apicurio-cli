# Changelog

All notable changes to the Apicurio CLI project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
