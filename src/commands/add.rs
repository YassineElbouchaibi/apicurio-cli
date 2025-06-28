use crate::{
    config::{DependencyConfig, load_global_config, load_repo_config},
    constants::APICURIO_CONFIG,
    identifier::Identifier,
    registry::RegistryClient,
};
use anyhow::{Result, anyhow};
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

    // Complete the identifier interactively (except version)
    identifier.complete_interactive(&registry_names, &repo.dependencies)?;

    // Now complete the version using registry access
    if !identifier.is_complete() {
        // Find the registry configuration for version completion
        let registry_name = identifier.registry.as_ref().unwrap();
        let registry_config = regs
            .iter()
            .find(|r| &r.name == registry_name)
            .ok_or_else(|| anyhow!("Registry '{}' not found", registry_name))?;

        let registry_client = RegistryClient::new(registry_config)?;
        identifier
            .complete_version_with_registry(&registry_client)
            .await?;
    }

    if !identifier.is_complete() {
        return Err(anyhow!("Failed to complete dependency identifier"));
    }

    let default_output_path = generate_default_output_path(&identifier);
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
        println!("🔄 Replaced existing dependency: {}", dep_name);
    } else {
        // Add new dependency
        repo.dependencies.push(new_dependency);
        println!("✅ Added dependency: {}", dep_name);
    }

    // Save the configuration
    let serialized = serde_yaml::to_string(&repo)?;
    fs::write(repo_path, serialized)?;

    // Pull the dependency immediately
    crate::commands::pull::run().await?;

    Ok(())
}

fn generate_default_output_path(identifier: &Identifier) -> String {
    let artifact_id = identifier.artifact_id.as_ref().unwrap();
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
