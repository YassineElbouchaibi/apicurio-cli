use crate::{
    config::{DependencyConfig, load_global_config, load_repo_config},
    constants::APICURIO_CONFIG,
    identifier::Identifier,
};
use anyhow::{Result, anyhow};
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

    // Complete the identifier interactively
    identifier.complete_interactive(&registry_names, &repo.dependencies)?;

    if !identifier.is_complete() {
        return Err(anyhow!("Failed to complete dependency identifier"));
    }

    // Get output path
    let default_output_path = format!(
        "protos/{}/{}.proto",
        identifier.artifact_id.as_ref().unwrap(),
        identifier.artifact_id.as_ref().unwrap()
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

    // Check for duplicate dependencies
    if repo.dependencies.iter().any(|d| d.name == dep_name) {
        return Err(anyhow!("Dependency '{}' already exists", dep_name));
    }

    // Add the dependency
    repo.dependencies.push(DependencyConfig {
        name: dep_name.clone(),
        group_id: identifier.group_id.unwrap(),
        artifact_id: identifier.artifact_id.unwrap(),
        version: identifier.version.unwrap(),
        registry: identifier.registry.unwrap(),
        output_path,
    });

    // Save the configuration
    let serialized = serde_yaml::to_string(&repo)?;
    fs::write(repo_path, serialized)?;

    println!("✅ Added dependency: {}", dep_name);
    Ok(())
}
