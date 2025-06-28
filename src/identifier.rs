use anyhow::{Result, anyhow};
use dialoguer::{Input, Select};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

/// Represents a parsed identifier in the format registry/group_id/artifact_id@version
#[derive(Debug, Clone)]
pub struct Identifier {
    pub registry: Option<String>,
    pub group_id: Option<String>,
    pub artifact_id: Option<String>,
    pub version: Option<String>,
}

impl Identifier {
    /// Parse an identifier string in the format registry/group_id/artifact_id@version
    /// All parts are optional
    pub fn parse(input: &str) -> Self {
        let mut identifier = Identifier {
            registry: None,
            group_id: None,
            artifact_id: None,
            version: None,
        };

        // Split by @ to separate version
        let main_part = if let Some(at_pos) = input.rfind('@') {
            let version = &input[at_pos + 1..];
            if !version.is_empty() {
                identifier.version = Some(version.to_string());
            }
            &input[..at_pos]
        } else {
            input
        };

        // Split by / to get registry/group_id/artifact_id
        let parts: Vec<&str> = main_part.split('/').collect();

        match parts.len() {
            1 if !parts[0].is_empty() => {
                // Could be any of the three parts, we'll prompt for clarification
                identifier.artifact_id = Some(parts[0].to_string());
            }
            2 => {
                if !parts[0].is_empty() {
                    identifier.group_id = Some(parts[0].to_string());
                }
                if !parts[1].is_empty() {
                    identifier.artifact_id = Some(parts[1].to_string());
                }
            }
            3 => {
                if !parts[0].is_empty() {
                    identifier.registry = Some(parts[0].to_string());
                }
                if !parts[1].is_empty() {
                    identifier.group_id = Some(parts[1].to_string());
                }
                if !parts[2].is_empty() {
                    identifier.artifact_id = Some(parts[2].to_string());
                }
            }
            _ => {
                // Empty or too many parts, will be handled during completion
            }
        }

        identifier
    }

    /// Complete missing fields by prompting the user with available options
    pub async fn complete_interactive(
        &mut self,
        available_registries: &[String],
        existing_dependencies: &[crate::config::DependencyConfig],
        registry_client: Option<&crate::registry::RegistryClient>,
    ) -> Result<()> {
        // Complete registry
        if self.registry.is_none() {
            if available_registries.is_empty() {
                return Err(anyhow!(
                    "No registries available. Please configure a registry first."
                ));
            }

            if available_registries.len() == 1 {
                self.registry = Some(available_registries[0].clone());
                println!("Using registry: {}", available_registries[0]);
            } else {
                let selection = Select::new()
                    .with_prompt("Select registry")
                    .items(available_registries)
                    .default(0)
                    .interact()?;
                self.registry = Some(available_registries[selection].clone());
            }
        } else {
            // Validate the provided registry
            if !available_registries.contains(self.registry.as_ref().unwrap()) {
                return Err(anyhow!(
                    "Registry '{}' not found. Available registries: {}",
                    self.registry.as_ref().unwrap(),
                    available_registries.join(", ")
                ));
            }
        }

        // Complete group_id
        if self.group_id.is_none() {
            let available_group_ids: Vec<String>;

            // Try to fetch groups from the registry if available
            if let Some(client) = registry_client {
                match client.list_groups().await {
                    Ok(registry_groups) => {
                        available_group_ids = registry_groups;
                    }
                    Err(_) => {
                        // Fall back to existing dependencies if registry query fails
                        available_group_ids = existing_dependencies
                            .iter()
                            .map(|d| d.group_id.clone())
                            .collect::<std::collections::HashSet<_>>()
                            .into_iter()
                            .collect();
                    }
                }
            } else {
                // No registry client, use existing dependencies
                available_group_ids = existing_dependencies
                    .iter()
                    .map(|d| d.group_id.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
            }

            if !available_group_ids.is_empty() {
                // Show available group IDs and allow selection or custom input
                let mut options = available_group_ids.clone();
                options.push("📝 Enter custom group ID".to_string());

                let selection = Select::new()
                    .with_prompt("Group ID")
                    .items(&options)
                    .default(options.len() - 1) // Default to custom input
                    .interact()?;

                if selection == options.len() - 1 {
                    // User chose to enter custom group ID
                    self.group_id = Some(
                        Input::new()
                            .with_prompt("Enter custom group ID")
                            .interact_text()?,
                    );
                } else {
                    // User selected an available group ID
                    self.group_id = Some(available_group_ids[selection].clone());
                }
            } else {
                // No available group IDs, default to "default"
                self.group_id = Some("default".to_string());
                println!("ℹ️ No groups found, using default group: 'default'");
            }
        }

        // Complete artifact_id
        if self.artifact_id.is_none() {
            let available_artifacts: Vec<String>;

            // Try to fetch artifacts from the registry if available
            if let Some(client) = registry_client {
                if let Some(group_id) = &self.group_id {
                    match client.list_artifacts(group_id).await {
                        Ok(registry_artifacts) => {
                            available_artifacts = registry_artifacts;
                        }
                        Err(_) => {
                            // Fall back to existing dependencies if registry query fails
                            available_artifacts = existing_dependencies
                                .iter()
                                .filter(|d| d.group_id == *group_id)
                                .map(|d| d.artifact_id.clone())
                                .collect::<std::collections::HashSet<_>>()
                                .into_iter()
                                .collect();
                        }
                    }
                } else {
                    available_artifacts = Vec::new();
                }
            } else {
                // No registry client, use existing dependencies
                available_artifacts = existing_dependencies
                    .iter()
                    .filter(|d| d.group_id == *self.group_id.as_ref().unwrap_or(&String::new()))
                    .map(|d| d.artifact_id.clone())
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();
            }

            if !available_artifacts.is_empty() {
                // Show available artifacts and allow selection or custom input
                let mut options = available_artifacts.clone();
                options.push("📝 Enter custom artifact ID".to_string());

                let selection = Select::new()
                    .with_prompt("Artifact ID")
                    .items(&options)
                    .default(options.len() - 1) // Default to custom input
                    .interact()?;

                if selection == options.len() - 1 {
                    // User chose to enter custom artifact ID
                    self.artifact_id = Some(
                        Input::new()
                            .with_prompt("Enter custom artifact ID")
                            .interact_text()?,
                    );
                } else {
                    // User selected an available artifact ID
                    self.artifact_id = Some(available_artifacts[selection].clone());
                }
            } else {
                // No available artifacts, just prompt for input
                self.artifact_id = Some(Input::new().with_prompt("Artifact ID").interact_text()?);
            }
        }

        // Complete version - we'll handle this separately after we have registry access
        // This will be completed later in the add command

        Ok(())
    }

    /// Complete the version field by fetching available versions from the registry
    pub async fn complete_version_with_registry(
        &mut self,
        registry_client: &crate::registry::RegistryClient,
    ) -> Result<()> {
        if self.version.is_some() {
            return Ok(()); // Version already specified
        }

        let group_id = self
            .group_id
            .as_ref()
            .ok_or_else(|| anyhow!("Group ID must be specified before completing version"))?;

        let artifact_id = self
            .artifact_id
            .as_ref()
            .ok_or_else(|| anyhow!("Artifact ID must be specified before completing version"))?;

        // Fetch available versions from the registry
        match registry_client.list_versions(group_id, artifact_id).await {
            Ok(mut versions) => {
                if versions.is_empty() {
                    // No existing versions, prompt user with default
                    self.version = Some(
                        Input::new()
                            .with_prompt("Version (semver)")
                            .default("1.0.0".to_string())
                            .interact_text()?,
                    );
                } else {
                    // Sort versions in descending order to get the latest first
                    versions.sort_by(|a, b| b.cmp(a));
                    let version_strings: Vec<String> =
                        versions.iter().map(|v| v.to_string()).collect();

                    // Create options for the select menu
                    let mut options = version_strings.clone();
                    options.push("📝 Enter custom version".to_string());

                    let selection = Select::new()
                        .with_prompt(&format!("Select version for {}/{}", group_id, artifact_id))
                        .items(&options)
                        .default(0) // Default to latest version
                        .interact()?;

                    if selection == options.len() - 1 {
                        // User chose to enter custom version
                        self.version = Some(
                            Input::new()
                                .with_prompt("Enter custom version (semver)")
                                .interact_text()?,
                        );
                    } else {
                        // User selected an existing version
                        self.version = Some(version_strings[selection].clone());
                    }
                }
            }
            Err(_) => {
                // Registry query failed (artifact might not exist yet), use default
                println!("ℹ️ Could not fetch existing versions (artifact may not exist yet)");
                self.version = Some(
                    Input::new()
                        .with_prompt("Version (semver)")
                        .default("1.0.0".to_string())
                        .interact_text()?,
                );
            }
        }

        Ok(())
    }

    /// Validate that the artifact exists in the registry
    pub async fn validate_artifact_exists(
        &self,
        registry_client: &crate::registry::RegistryClient,
    ) -> Result<bool> {
        if let (Some(group_id), Some(artifact_id)) = (&self.group_id, &self.artifact_id) {
            registry_client.artifact_exists(group_id, artifact_id).await
        } else {
            Ok(false)
        }
    }

    /// Find dependencies that match this identifier (for removal)
    pub fn find_matches<'a>(
        &self,
        dependencies: &'a [crate::config::DependencyConfig],
    ) -> Vec<&'a crate::config::DependencyConfig> {
        let matcher = SkimMatcherV2::default();
        let mut matches = Vec::new();

        for dep in dependencies {
            let mut score = 0i64;
            let mut is_match = true;

            // Check registry match
            if let Some(registry) = &self.registry {
                if dep.registry == *registry {
                    score += 100;
                } else {
                    is_match = false;
                }
            }

            // Check group_id match
            if let Some(group_id) = &self.group_id {
                if dep.group_id == *group_id {
                    score += 100;
                } else if let Some(fuzzy_score) = matcher.fuzzy_match(&dep.group_id, group_id) {
                    score += fuzzy_score;
                } else {
                    is_match = false;
                }
            }

            // Check artifact_id match
            if let Some(artifact_id) = &self.artifact_id {
                if dep.artifact_id == *artifact_id {
                    score += 100;
                } else if let Some(fuzzy_score) = matcher.fuzzy_match(&dep.artifact_id, artifact_id)
                {
                    score += fuzzy_score;
                } else {
                    is_match = false;
                }
            }

            // Check version match (exact or fuzzy)
            if let Some(version) = &self.version {
                if dep.version == *version {
                    score += 50;
                } else if let Some(fuzzy_score) = matcher.fuzzy_match(&dep.version, version) {
                    score += fuzzy_score / 2; // Lower weight for version fuzzy match
                }
            }

            if is_match && score > 0 {
                matches.push(dep);
            }
        }

        // Sort by best match first
        matches.sort_by_key(|dep| {
            let mut score = 0i64;

            if let Some(registry) = &self.registry {
                if dep.registry == *registry {
                    score += 100;
                }
            }
            if let Some(group_id) = &self.group_id {
                if dep.group_id == *group_id {
                    score += 100;
                }
            }
            if let Some(artifact_id) = &self.artifact_id {
                if dep.artifact_id == *artifact_id {
                    score += 100;
                }
            }
            if let Some(version) = &self.version {
                if dep.version == *version {
                    score += 50;
                }
            }

            -score // Negative for descending sort
        });

        matches
    }

    /// Convert to a display string
    #[allow(dead_code)]
    pub fn to_display_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(registry) = &self.registry {
            parts.push(registry.clone());
        }
        if let Some(group_id) = &self.group_id {
            if parts.is_empty() {
                parts.push(String::new()); // placeholder for missing registry
            }
            parts.push(group_id.clone());
        }
        if let Some(artifact_id) = &self.artifact_id {
            while parts.len() < 2 {
                parts.push(String::new()); // placeholders
            }
            parts.push(artifact_id.clone());
        }

        let main_part = parts.join("/");

        if let Some(version) = &self.version {
            format!("{}@{}", main_part, version)
        } else {
            main_part
        }
    }

    /// Check if all required fields are present
    pub fn is_complete(&self) -> bool {
        self.registry.is_some()
            && self.group_id.is_some()
            && self.artifact_id.is_some()
            && self.version.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_identifier() {
        let id = Identifier::parse("myregistry/com.example/myartifact@1.0.0");
        assert_eq!(id.registry, Some("myregistry".to_string()));
        assert_eq!(id.group_id, Some("com.example".to_string()));
        assert_eq!(id.artifact_id, Some("myartifact".to_string()));
        assert_eq!(id.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_parse_partial_identifier() {
        let id = Identifier::parse("com.example/myartifact");
        assert_eq!(id.registry, None);
        assert_eq!(id.group_id, Some("com.example".to_string()));
        assert_eq!(id.artifact_id, Some("myartifact".to_string()));
        assert_eq!(id.version, None);
    }

    #[test]
    fn test_parse_artifact_only() {
        let id = Identifier::parse("myartifact@2.0.0");
        assert_eq!(id.registry, None);
        assert_eq!(id.group_id, None);
        assert_eq!(id.artifact_id, Some("myartifact".to_string()));
        assert_eq!(id.version, Some("2.0.0".to_string()));
    }

    #[test]
    fn test_display_string() {
        let mut id = Identifier::parse("myregistry/com.example/myartifact@1.0.0");
        assert_eq!(
            id.to_display_string(),
            "myregistry/com.example/myartifact@1.0.0"
        );

        id.version = None;
        assert_eq!(id.to_display_string(), "myregistry/com.example/myartifact");
    }
}
