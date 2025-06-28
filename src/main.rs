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

mod commands;
mod config;
mod constants;
mod dependency;
mod identifier;
mod lockfile;
mod registry;

#[cfg(test)]
mod lockfile_integration_tests;

use anyhow::Result;
use clap::Parser;

/// CLI tool for managing schema artifacts from Apicurio Registry
///
/// The Apicurio CLI provides lockfile-based dependency management for schema artifacts
/// including Protobuf, Avro, JSON Schema, OpenAPI, and more. It supports multiple
/// registries, flexible authentication, and semantic version resolution.
#[derive(Parser)]
#[command(
    name = "apicurio",
    version,
    about = "CLI tool for managing schema artifacts from Apicurio Registry",
    long_about = "A powerful Rust-based command-line tool for managing schema artifacts from Apicurio Registry.\n\nFeatures lockfile-based dependency management, multi-registry support, flexible authentication,\nand semantic version resolution for Protobuf, Avro, JSON Schema, OpenAPI, and other schema types."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<commands::Commands>,
}

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
