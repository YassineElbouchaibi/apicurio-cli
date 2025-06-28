//! # Apicurio CLI Library
//!
//! Core library functionality for the Apicurio CLI tool.

use clap::Parser;

pub mod commands;
pub mod config;
pub mod constants;
pub mod dependency;
pub mod identifier;
pub mod lockfile;
pub mod registry;

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
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Option<commands::Commands>,
}
