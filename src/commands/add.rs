use crate::{
    config::{load_global_config, load_repo_config, DependencyConfig},
    constants::APICURIO_CONFIG,
    identifier::Identifier,
    output_path,
    registry::RegistryClient,
};
use anyhow::{anyhow, Result};
use dialoguer::Input;
use std::path::PathBuf;

pub async fn run(identifier_str: Option<String>, latest: bool) -> Result<()> {
    // Parse the identifier string (if provided)
    let mut identifier = if let Some(id_str) = identifier_str {
        Identifier::parse(&id_str)
    } else {
        Identifier::parse("")
    };

    // Load configuration
    let repo_path = PathBuf::from(APICURIO_CONFIG);
    let mut repo = load_repo_config(&repo_path)?;
    let global = load_global_config()?;
    let regs = repo.merge_registries(global)?;

    if regs.is_empty() {
        return Err(anyhow!(
            "No registries configured. Please add a registry first using 'apicurio registry add'."
        ));
    }

    let registry_names: Vec<String> = regs.iter().map(|r| r.name.clone()).collect();

    // Get registry client for the selected/default registry
    let registry_client = if let Some(registry_name) = &identifier.registry {
        // Registry already specified, find it
        let registry_config = regs
            .iter()
            .find(|r| &r.name == registry_name)
            .ok_or_else(|| anyhow!("Registry '{}' not found", registry_name))?;
        Some(RegistryClient::new(registry_config)?)
    } else if regs.len() == 1 {
        // Only one registry, use it
        Some(RegistryClient::new(&regs[0])?)
    } else {
        // Multiple registries, will be selected during complete_interactive
        None
    };

    // Complete the identifier interactively (except version)
    identifier
        .complete_interactive(
            &registry_names,
            &repo.dependencies,
            registry_client.as_ref(),
        )
        .await?;

    // Create registry client if not already created
    let registry_client = if let Some(client) = registry_client {
        client
    } else {
        let registry_name = identifier.registry.as_ref().unwrap();
        let registry_config = regs
            .iter()
            .find(|r| &r.name == registry_name)
            .ok_or_else(|| anyhow!("Registry '{}' not found", registry_name))?;
        RegistryClient::new(registry_config)?
    };

    // Resolve version
    if latest {
        let group_id = identifier
            .group_id
            .as_ref()
            .ok_or_else(|| anyhow!("Group ID must be specified"))?;
        let artifact_id = identifier
            .artifact_id
            .as_ref()
            .ok_or_else(|| anyhow!("Artifact ID must be specified"))?;
        let mut versions = registry_client.list_versions(group_id, artifact_id).await?;
        versions.sort();
        if let Some(v) = versions.last() {
            identifier.version = Some(v.to_string());
        } else {
            identifier.version = Some("1.0.0".to_string());
        }
    } else if !identifier.is_complete() {
        identifier
            .complete_version_with_registry(&registry_client)
            .await?;
    }

    if !identifier.is_complete() {
        return Err(anyhow!("Failed to complete dependency identifier"));
    }

    // Validate that the artifact exists in the registry
    let artifact_exists = identifier
        .validate_artifact_exists(&registry_client)
        .await?;
    if !artifact_exists {
        return Err(anyhow!(
            "Artifact '{}/{}' does not exist in registry '{}'",
            identifier.group_id.as_ref().unwrap(),
            identifier.artifact_id.as_ref().unwrap(),
            identifier.registry.as_ref().unwrap()
        ));
    }

    // Get artifact metadata to determine type for output path generation
    let artifact_metadata = registry_client
        .get_artifact_metadata(
            identifier.group_id.as_ref().unwrap(),
            identifier.artifact_id.as_ref().unwrap(),
        )
        .await?;

    let pattern = repo
        .dependency_defaults
        .output_patterns
        .resolve(&artifact_metadata.artifact_type, None);
    let default_output_path = output_path::generate_output_path(
        &pattern,
        identifier.group_id.as_ref().unwrap(),
        identifier.artifact_id.as_ref().unwrap(),
        identifier.version.as_deref().unwrap_or("0.0.0"),
        &artifact_metadata.artifact_type,
    );
    let output_path = Input::new()
        .with_prompt("Output path")
        .default(default_output_path)
        .interact_text()?;

    // Generate a unique name for the dependency if needed
    let dep_name = format!(
        "{}/{}",
        identifier.group_id.as_ref().unwrap(),
        identifier.artifact_id.as_ref().unwrap()
    );

    // Check for existing dependency and replace if found
    let existing_index = repo.dependencies.iter().position(|d| d.name == dep_name);

    let new_dependency = DependencyConfig {
        name: dep_name.clone(),
        // Only set explicit group_id/artifact_id if they differ from what would be resolved from name
        group_id: {
            let resolved_from_name = if let Some((group, _)) = dep_name.split_once('/') {
                group.to_string()
            } else {
                "default".to_string()
            };
            if resolved_from_name == *identifier.group_id.as_ref().unwrap() {
                None // Can be resolved from name
            } else {
                Some(identifier.group_id.unwrap())
            }
        },
        artifact_id: {
            let resolved_from_name = if let Some((_, artifact)) = dep_name.split_once('/') {
                artifact.to_string()
            } else {
                dep_name.clone()
            };
            if resolved_from_name == *identifier.artifact_id.as_ref().unwrap() {
                None // Can be resolved from name
            } else {
                Some(identifier.artifact_id.unwrap())
            }
        },
        version: identifier.version.unwrap(),
        registry: Some(identifier.registry.unwrap()),
        output_path: Some(output_path),
        resolve_references: None, // Use global setting by default
    };

    if let Some(index) = existing_index {
        // Replace existing dependency
        repo.dependencies[index] = new_dependency;
        println!("🔄 Replaced existing dependency: {dep_name}");
    } else {
        // Add new dependency
        repo.dependencies.push(new_dependency);
        println!("✅ Added dependency: {dep_name}");
    }

    // Save the configuration preserving formatting
    crate::config::save_repo_config(&repo, &repo_path)?;

    // Pull the dependency immediately
    crate::commands::pull::run().await?;

    Ok(())
}
