use crate::{config::load_repo_config, constants::APICURIO_CONFIG, identifier::Identifier};
use anyhow::{anyhow, Result};
use dialoguer::Select;
use std::{fs, path::PathBuf};

pub async fn run(identifier_str: String) -> Result<()> {
    let repo_path = PathBuf::from(APICURIO_CONFIG);
    let mut repo = load_repo_config(&repo_path)?;

    if repo.dependencies.is_empty() {
        println!("No dependencies to remove.");
        return Ok(());
    }

    // Parse the identifier
    let identifier = Identifier::parse(&identifier_str);

    // Find matching dependencies
    let matches = identifier.find_matches(&repo.dependencies);

    if matches.is_empty() {
        println!("No dependencies found matching identifier: '{identifier_str}'");
        println!("Available dependencies:");
        for dep in &repo.dependencies {
            println!(
                "  - {} ({}@{})",
                dep.name,
                dep.resolved_artifact_id(),
                dep.version
            );
        }
        return Ok(());
    }

    let dependency_name = if matches.len() == 1 {
        // Exact match or single fuzzy match
        matches[0].name.clone()
    } else {
        // Multiple matches, let user choose
        println!("Multiple dependencies match the identifier:");
        let items: Vec<String> = matches
            .iter()
            .map(|dep| {
                format!(
                    "{} ({}@{} from {})",
                    dep.name,
                    dep.resolved_artifact_id(),
                    dep.version,
                    dep.registry
                )
            })
            .collect();

        let selection = Select::new()
            .with_prompt("Select dependency to remove")
            .items(&items)
            .default(0)
            .interact()?;

        matches[selection].name.clone()
    };

    // Remove the dependency
    let before_count = repo.dependencies.len();
    repo.dependencies.retain(|d| d.name != dependency_name);

    if repo.dependencies.len() < before_count {
        let serialized = serde_yaml::to_string(&repo)?;
        fs::write(repo_path, serialized)?;
        println!("✅ Removed dependency: {dependency_name}");

        // Pull the dependency immediately
        crate::commands::pull::run().await?;
    } else {
        return Err(anyhow!("Failed to remove dependency: {}", dependency_name));
    }

    Ok(())
}
