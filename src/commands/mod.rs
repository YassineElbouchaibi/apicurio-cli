//! Command implementations for the Apicurio CLI
//!
//! This module contains all the command implementations for the CLI tool.
//! Each command is implemented in its own module and handles a specific
//! aspect of dependency management or registry interaction.
//!
//! ## Command Categories
//!
//! ### Core Dependency Management
//! - `init` - Initialize a new project
//! - `pull` - Fetch dependencies
//! - `update` - Update dependencies to latest matching versions
//! - `lock` - Update lock file without downloading
//!
//! ### Dependency Lifecycle
//! - `add` - Add new dependencies
//! - `remove` - Remove existing dependencies
//! - `list` - List configured dependencies
//! - `status` - Check for outdated dependencies
//!
//! ### Registry Operations
//! - `registry` - Manage registry configurations
//! - `publish` - Publish artifacts to registries
//!
//! ### Validation & Utilities
//! - `verify` - Verify integrity of downloaded files
//! - `doctor` - Validate configuration and connectivity
//! - `completions` - Generate shell completion scripts

use anyhow::Result;
use clap::Subcommand;

pub mod add;
pub mod completions;
pub mod doctor;
pub mod init;
pub mod list;
pub mod lock;
pub mod publish;
pub mod pull;
pub mod registry;
pub mod remove;
pub mod status;
pub mod update;
pub mod verify;

/// All available CLI commands
///
/// Each variant corresponds to a subcommand that can be executed.
/// Commands are organized by functionality and include comprehensive
/// help text for user guidance.
#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = concat!(
        "Scaffold a blank config (and empty lock) in a new repo"
    ))]
    Init,
    #[command(
        about = "Fetch exactly what's in the lock; if no lock, resolve specs ⇒ download ⇒ lock"
    )]
    Pull,
    #[command(
        about = "Re-resolve semver ranges in config to latest matches; download ⇒ overwrite lock"
    )]
    Update,
    #[command(
        about = "Add a new dependency entry to the config using format registry/group_id/artifact_id@version"
    )]
    Add {
        #[arg(
            help = "Dependency identifier in format registry/group_id/artifact_id@version (all parts optional, will prompt for missing)"
        )]
        identifier: Option<String>,
        #[arg(long, help = "Automatically use the latest available version")]
        latest: bool,
    },
    #[command(about = "Remove an existing dependency by identifier")]
    Remove {
        #[arg(
            help = "Dependency identifier in format registry/group_id/artifact_id@version (partial matches supported)"
        )]
        identifier: String,
    },
    #[command(
        about = "Print all configured deps (spec'd & locked versions), and registries (no network)"
    )]
    List,
    #[command(about = "Compare lock vs. latest matching version in registry; flag outdated deps")]
    Status,
    #[command(about = "Re-hash downloaded files & confirm against lockfile hashes")]
    Verify,
    #[command(about = "Subcommand: manage global registries file (add/list/remove)")]
    Registry {
        #[command(subcommand)]
        cmd: registry::RegistryCommands,
    },
    #[command(
        about = "Validate config + lock semantics (semver syntax, missing fields, unreachable URLs)"
    )]
    Doctor,
    #[command(about = "Emit shell completion scripts (bash/zsh/fish)")]
    Completions { shell: String },
    #[command(about = "Publish to registries")]
    Publish {
        #[arg(
            help = "Specific publish name to publish (if not provided, publishes all configured artifacts)"
        )]
        name: Option<String>,
    },
    #[command(about = "Update the lockfile based on current dependencies")]
    Lock,
}

/// Command dispatcher that routes to the appropriate command implementation
///
/// Takes a parsed command and delegates to the corresponding module's run function.
/// All commands are async to support network operations and file I/O.
///
/// # Arguments
/// * `cmd` - The command to execute
///
/// # Returns
/// Result indicating success or failure of the command execution
pub async fn run(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Pull => pull::run().await,
        Commands::Update => update::run().await,
        Commands::Init => init::run().await,
        Commands::Add { identifier, latest } => add::run(identifier, latest).await,
        Commands::Remove { identifier } => remove::run(identifier).await,
        Commands::List => list::run().await,
        Commands::Status => status::run().await,
        Commands::Verify => verify::run().await,
        Commands::Registry { cmd } => registry::run(cmd).await,
        Commands::Doctor => doctor::run().await,
        Commands::Completions { shell } => completions::run(shell),
        Commands::Publish { name } => publish::run(name).await,
        Commands::Lock => lock::run().await,
    }
}
