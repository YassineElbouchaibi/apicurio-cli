//! # Apicurio CLI
//!
//! A powerful Rust-based command-line tool for managing schema artifacts from Apicurio Registry.
//!
//! The `apicurio` CLI provides dependency management for Protobuf, Avro, JSON Schema, and other
//! schema artifacts with lockfile-based reproducible builds.
//!
//! ## Features
//!
//! - 🔒 **Lockfile-based dependency management** - Reproducible builds with exact version locking
//! - 📦 **Multiple artifact types** - Support for Protobuf, Avro, JSON Schema, OpenAPI, GraphQL, and more
//! - 🔐 **Flexible authentication** - Basic, token, and bearer authentication support
//! - 🌐 **Multi-registry support** - Work with multiple Apicurio Registry instances
//! - 📋 **Semver resolution** - Smart semantic version resolution with range support
//!
//! ## Quick Start
//!
//! ```bash
//! # Initialize a new project
//! apicurio init
//!
//! # Add a dependency
//! apicurio add my-registry/com.example/user-service@^1.0.0
//!
//! # Pull dependencies
//! apicurio pull
//!
//! # Check status
//! apicurio status
//! ```
//!
//! ## Configuration
//!
//! The tool uses two main configuration files:
//! - `apicurioconfig.yaml` - Project-specific configuration
//! - `apicuriolock.yaml` - Lock file with exact resolved versions
//!
//! Global registries can be configured in `~/.config/apicurio/registries.yaml`.

use anyhow::Result;
use apicurio_cli::{commands, Cli};
use clap::Parser;

/// Main entry point for the Apicurio CLI
///
/// Parses command-line arguments and delegates to the appropriate command handler.
/// If no command is provided, displays an error message and exits.
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cmd = cli.cmd.unwrap_or_else(|| {
        eprintln!("No command provided. Use --help to see available commands.");
        std::process::exit(1);
    });
    commands::run(cmd).await
}
