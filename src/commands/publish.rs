use anyhow::{Context, Result};
use std::fs;

use crate::config::{load_global_config, load_repo_config, PublishConfig};
use crate::constants::APICURIO_CONFIG;
use crate::registry::RegistryClient;

pub async fn run(name: Option<String>) -> Result<()> {
    let config_path = std::env::current_dir()?.join(APICURIO_CONFIG);

    if !config_path.exists() {
        anyhow::bail!(
            "No {} found in current directory. Run 'apicurio init' first.",
            APICURIO_CONFIG
        );
    }

    let repo_config = load_repo_config(&config_path)?;
    let global_config = load_global_config()?;
    let registries = repo_config.merge_registries(global_config)?;

    if repo_config.publishes.is_empty() {
        println!("No publishes configured in {}", APICURIO_CONFIG);
        return Ok(());
    }

    // Filter publishes based on the name parameter
    let publishes_to_process: Vec<&PublishConfig> = if let Some(ref filter_name) = name {
        repo_config
            .publishes
            .iter()
            .filter(|p| p.name == *filter_name)
            .collect()
    } else {
        repo_config.publishes.iter().collect()
    };

    if publishes_to_process.is_empty() {
        if let Some(filter_name) = name {
            anyhow::bail!("No publish configuration found with name '{}'", filter_name);
        } else {
            println!("No publishes configured in {}", APICURIO_CONFIG);
            return Ok(());
        }
    }

    println!("Publishing {} artifacts...", publishes_to_process.len());

    for publish in publishes_to_process {
        publish_artifact(publish, &registries).await?;
    }

    println!("✅ All artifacts published successfully!");
    Ok(())
}

async fn publish_artifact(
    publish: &PublishConfig,
    registries: &[crate::config::RegistryConfig],
) -> Result<()> {
    // Validate references have exact versions
    for reference in &publish.references {
        reference
            .validate_exact_version()
            .with_context(|| format!("Invalid reference in publish '{}'", publish.name))?;
    }

    // Find the registry
    let registry = registries
        .iter()
        .find(|r| r.name == publish.registry)
        .ok_or_else(|| anyhow::anyhow!("Registry '{}' not found", publish.registry))?;

    // Read the artifact content
    let content = fs::read_to_string(&publish.input_path)
        .with_context(|| format!("Failed to read file: {}", publish.input_path))?;

    println!(
        "Publishing {}@{} to registry '{}'...",
        publish.name, publish.version, publish.registry
    );

    // Create registry client and publish
    let client = RegistryClient::new(registry)?;
    client.publish_artifact(publish, &content).await?;

    Ok(())
}
