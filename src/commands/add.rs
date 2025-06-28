use crate::{
    config::{load_global_config, load_repo_config, DependencyConfig},
    constants::APICURIO_CONFIG,
    identifier::Identifier,
    registry::RegistryClient,
};
use anyhow::{anyhow, Result};
use convert_case::{Case, Casing};
use dialoguer::Input;
use std::{fs, path::PathBuf};

pub async fn run(identifier_str: Option<String>) -> Result<()> {
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

    // Now complete the version using registry access
    if !identifier.is_complete() {
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

    let default_output_path =
        generate_default_output_path(&identifier, &artifact_metadata.artifact_type);
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
        group_id: identifier.group_id.unwrap(),
        artifact_id: identifier.artifact_id.unwrap(),
        version: identifier.version.unwrap(),
        registry: identifier.registry.unwrap(),
        output_path,
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

    // Save the configuration
    let serialized = serde_yaml::to_string(&repo)?;
    fs::write(repo_path, serialized)?;

    // Pull the dependency immediately
    crate::commands::pull::run().await?;

    Ok(())
}

fn generate_default_output_path(identifier: &Identifier, artifact_type: &str) -> String {
    let artifact_id = identifier.artifact_id.as_ref().unwrap();

    match artifact_type.to_uppercase().as_str() {
        "PROTOBUF" => {
            // For protobuf files, use the original logic
            let mut parts = artifact_id
                .split('.')
                .map(str::to_string)
                .collect::<Vec<_>>();
            let last = parts.pop().unwrap().to_case(Case::Snake);

            let mut path = PathBuf::from("protos");
            for seg in parts {
                path.push(seg.to_lowercase());
            }
            path.push(last);
            path.set_extension("proto");

            path.to_string_lossy().into_owned()
        }
        "AVRO" => {
            // For Avro schemas
            let mut parts = artifact_id
                .split('.')
                .map(str::to_string)
                .collect::<Vec<_>>();
            let last = parts.pop().unwrap().to_case(Case::Snake);

            let mut path = PathBuf::from("schemas");
            for seg in parts {
                path.push(seg.to_lowercase());
            }
            path.push(last);
            path.set_extension("avsc");

            path.to_string_lossy().into_owned()
        }
        "JSON" => {
            // For JSON schemas
            let mut parts = artifact_id
                .split('.')
                .map(str::to_string)
                .collect::<Vec<_>>();
            let last = parts.pop().unwrap().to_case(Case::Snake);

            let mut path = PathBuf::from("schemas");
            for seg in parts {
                path.push(seg.to_lowercase());
            }
            path.push(last);
            path.set_extension("json");

            path.to_string_lossy().into_owned()
        }
        "OPENAPI" => {
            // For OpenAPI specs
            let mut parts = artifact_id
                .split('.')
                .map(str::to_string)
                .collect::<Vec<_>>();
            let last = parts.pop().unwrap().to_case(Case::Snake);

            let mut path = PathBuf::from("openapi");
            for seg in parts {
                path.push(seg.to_lowercase());
            }
            path.push(last);
            path.set_extension("yaml");

            path.to_string_lossy().into_owned()
        }
        _ => {
            // Default fallback for unknown types
            let mut parts = artifact_id
                .split('.')
                .map(str::to_string)
                .collect::<Vec<_>>();
            let last = parts.pop().unwrap().to_case(Case::Snake);

            let mut path = PathBuf::from("schemas");
            for seg in parts {
                path.push(seg.to_lowercase());
            }
            path.push(last);
            // Use a generic extension for unknown types
            path.set_extension("schema");

            path.to_string_lossy().into_owned()
        }
    }
}
