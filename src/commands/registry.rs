use crate::config::{load_global_config, save_global_config, AuthConfig, RegistryConfig};
use anyhow::{anyhow, Result};
use clap::Subcommand;
use dialoguer::Select;
use std::io::{stdin, stdout, Write};

#[derive(Subcommand, Debug)]
pub enum RegistryCommands {
    /// List all global registries
    List,
    /// Add a new global registry
    Add,
    /// Remove a global registry by name
    Remove { name: String },
}

fn prompt(msg: &str) -> Result<String> {
    print!("{msg}: ");
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    let val = input.trim().to_string();
    if val.is_empty() {
        Err(anyhow!("{} cannot be empty", msg))
    } else {
        Ok(val)
    }
}

pub async fn run(cmd: RegistryCommands) -> Result<()> {
    let mut global = load_global_config()?;

    match cmd {
        RegistryCommands::List => {
            if global.registries.is_empty() {
                println!("(no global registries defined)");
            } else {
                for r in &global.registries {
                    println!(" - {} → {} (type={:?})", r.name, r.url, r.auth);
                }
            }
        }
        RegistryCommands::Add => {
            let name = prompt("Registry name")?;
            if global.registries.iter().any(|r| r.name == name) {
                return Err(anyhow!("registry '{}' already exists", name));
            }
            let url = prompt("Registry URL")?;

            // Use select menu for auth types
            let auth_options = vec!["none", "basic", "token", "bearer"];
            let selection = Select::new()
                .with_prompt("Auth type")
                .items(&auth_options)
                .default(0)
                .interact()?;

            let auth_type = auth_options[selection];
            let auth = match auth_type {
                "none" => AuthConfig::None,
                "basic" => {
                    let user = prompt("Username")?;
                    let pw_env = prompt("Password env var")?;
                    AuthConfig::Basic {
                        username: user,
                        password_env: pw_env,
                    }
                }
                "token" => {
                    let ev = prompt("Token env var")?;
                    AuthConfig::Token { token_env: ev }
                }
                "bearer" => {
                    let ev = prompt("Bearer-token env var")?;
                    AuthConfig::Bearer { token_env: ev }
                }
                other => return Err(anyhow!("unknown auth type '{}'", other)),
            };
            global.registries.push(RegistryConfig {
                name: name.clone(),
                url,
                auth,
            });
            save_global_config(&global)?;
            println!("✅ Added registry '{name}' successfully");
        }
        RegistryCommands::Remove { name } => {
            let before = global.registries.len();
            global.registries.retain(|r| r.name != name);
            if global.registries.len() == before {
                println!("no such registry '{name}'");
            } else {
                save_global_config(&global)?;
                println!("removed '{name}'");
            }
        }
    }

    Ok(())
}
