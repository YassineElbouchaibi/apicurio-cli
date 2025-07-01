//! Configuration management for Apicurio CLI
//!
//! This module handles loading, parsing, and merging of configuration files including:
//! - Repository configuration (`apicurioconfig.yaml`)
//! - Global registries configuration
//! - Environment variable expansion
//! - Configuration validation
//!
//! ## Configuration Files
//!
//! ### Repository Configuration
//! The main project configuration file that defines dependencies, registries, and publishing settings.
//!
//! ### Global Registries
//! Shared registry definitions stored in `~/.config/apicurio/registries.yaml` or
//! the path specified by `APICURIO_REGISTRIES_PATH`.
//!
//! ## Environment Variable Expansion
//!
//! Configuration files support environment variable expansion with the following syntax:
//! - `${VAR}` - Simple substitution
//! - `${VAR:-default}` - Use default if VAR is unset or empty
//! - `${VAR-default}` - Use default if VAR is unset
//! - `${VAR:+alt}` - Use alt if VAR is set and non-empty
//! - `${VAR+alt}` - Use alt if VAR is set

use anyhow::Context;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{env, fs, path::PathBuf};

/// Configuration for automatic reference resolution
///
/// Controls how transitive dependencies (references) are automatically resolved
/// and where they are stored when not explicitly declared in dependencies.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase", default)]
pub struct ReferenceResolutionConfig {
    /// Whether to automatically resolve references
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Output path pattern for resolved references
    /// Variables: {groupId}, {artifactId}, {version}, {ext}
    /// Advanced variables: {artifactParts[0]}, {artifactParts[1]}, etc.
    #[serde(default = "default_reference_pattern")]
    pub output_pattern: String,
    /// Maximum depth for reference resolution (prevents infinite loops)
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    /// Explicit output path overrides for specific artifacts
    /// Key format: "groupId/artifactId" or "registry/groupId/artifactId"
    /// Value: exact output path to use, or null to skip resolution entirely
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub output_overrides: std::collections::HashMap<String, Option<String>>,
}


fn default_true() -> bool {
    true
}

fn default_reference_pattern() -> String {
    "references/{groupId}/{artifactId}/{version}.{ext}".to_string()
}

fn default_max_depth() -> u32 {
    5
}

fn is_default_reference_resolution(r: &ReferenceResolutionConfig) -> bool {
    r == &ReferenceResolutionConfig::default()
}

/// Repository-specific configuration loaded from `apicurioconfig.yaml`
///
/// This is the main configuration file for a project, containing:
/// - Registry definitions (can be merged with global registries)
/// - Dependencies to fetch from registries
/// - Publishing configuration for uploading artifacts
///
/// # Example
///
/// ```yaml
/// externalRegistriesFile: ${APICURIO_REGISTRIES_PATH:-}
/// registries:
///   - name: production
///     url: https://registry.example.com
///     auth:
///       type: bearer
///       tokenEnv: APICURIO_TOKEN
/// dependencies:
///   # Smart resolution from name (groupId: com.example, artifactId: user-service)
///   - name: com.example/user-service
///     version: ^1.0.0
///     registry: production
///     outputPath: protos/user-service.proto
/// ```
#[derive(Deserialize, Serialize, Debug, Default)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase", default)]
pub struct RepoConfig {
    /// Optional path to external registries file for additional registry definitions
    pub external_registries_file: Option<String>,
    /// Registry definitions specific to this repository
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<RegistryConfig>,
    /// Dependencies to fetch from registries
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<DependencyConfig>,
    /// Configuration for automatic reference resolution
    #[serde(default, skip_serializing_if = "is_default_reference_resolution")]
    pub reference_resolution: ReferenceResolutionConfig,
    /// Artifacts to publish to registries
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub publishes: Vec<PublishConfig>,
}

/// Registry configuration defining connection details and authentication
///
/// Registries can be defined globally or per-repository. Repository-specific
/// registries override global registries with the same name.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegistryConfig {
    /// Unique name for this registry (used as reference in dependencies)
    pub name: String,
    /// Base URL of the Apicurio Registry API
    pub url: String,
    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthConfig,
}

/// Authentication configuration for registry access
///
/// Supports multiple authentication methods commonly used with Apicurio Registry.
/// Credentials are always sourced from environment variables for security.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
#[derive(Default)]
pub enum AuthConfig {
    /// No authentication (anonymous access)
    #[default]
    None,
    /// HTTP Basic authentication
    Basic {
        /// Username for basic auth
        username: String,
        /// Environment variable containing the password
        password_env: String,
    },
    /// Token-based authentication (custom header)
    Token {
        /// Environment variable containing the token
        token_env: String,
    },
    /// Bearer token authentication (Authorization header)
    Bearer {
        /// Environment variable containing the bearer token
        token_env: String,
    },
}

/// Dependency configuration for artifacts to fetch from registries
///
/// Dependencies support semantic version ranges and are resolved to exact
/// versions in the lock file for reproducible builds.
///
/// Smart resolution is supported for `group_id` and `artifact_id`:
/// - If `name` is in "group/artifact" format, group_id defaults to "group" and artifact_id to "artifact"
/// - If `name` is simple, group_id defaults to "default" and artifact_id to the name
/// - Explicit `group_id` and `artifact_id` override the smart defaults
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase", default)]
pub struct DependencyConfig {
    /// Local name/alias for this dependency (supports group/artifact format for smart resolution)
    pub name: String,
    /// Group ID of the artifact in the registry (optional - resolved from name if not provided)
    pub group_id: Option<String>,
    /// Artifact ID in the registry (optional - resolved from name if not provided)
    pub artifact_id: Option<String>,
    /// Version specification (supports semver ranges like ^1.0.0, ~2.1.0)
    pub version: String,
    /// Name of the registry to fetch from (must match a registry name)
    pub registry: String,
    /// Local path where the artifact should be saved
    pub output_path: String,
    /// Override reference resolution for this specific dependency
    pub resolve_references: Option<bool>,
}

/// Publishing configuration for uploading artifacts to registries
///
/// Defines how local artifacts should be published to registries, including
/// metadata, references, and conflict resolution behavior.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase", default)]
pub struct PublishConfig {
    /// Name/identifier for this publish configuration
    pub name: String,
    /// Local path to the file to publish
    pub input_path: String,
    /// Exact version to publish (no semver ranges allowed)
    pub version: String,
    /// Target registry name
    pub registry: String,

    // Optional fields with smart defaults
    /// Group ID (defaults from name if contains /)
    pub group_id: Option<String>,
    /// Artifact ID (defaults from name)
    pub artifact_id: Option<String>,
    /// Artifact type (auto-detected from file extension if not specified)
    pub r#type: Option<ArtifactType>,
    /// Behavior when artifact already exists
    #[serde(default)]
    pub if_exists: IfExistsAction,
    /// Human-readable description
    pub description: Option<String>,
    /// Key-value labels for metadata
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub labels: std::collections::HashMap<String, String>,
    /// References to other artifacts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<ArtifactReference>,
}

/// Supported artifact types for publishing
///
/// The CLI can auto-detect most types from file extensions, but explicit
/// specification is supported for edge cases.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactType {
    /// Protocol Buffers (.proto files)
    Protobuf,
    /// Apache Avro schemas
    Avro,
    /// JSON Schema definitions
    JsonSchema,
    /// OpenAPI specifications
    Openapi,
    /// AsyncAPI specifications
    AsyncApi,
    /// GraphQL schemas
    GraphQL,
    /// XML schemas
    Xml,
    /// WSDL definitions
    Wsdl,
}

/// Behavior when publishing an artifact that already exists
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Default)]
pub enum IfExistsAction {
    /// Fail if artifact already exists
    #[default]
    Fail,
    /// Create a new version if artifact exists
    CreateVersion,
    /// Find existing version or create new one
    FindOrCreateVersion,
}

/// Reference to another artifact (used in publishing)
///
/// Artifacts can reference other artifacts to establish dependencies.
/// References must use exact versions (no semver ranges) to ensure
/// deterministic builds.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[derive(Default)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase", default)]
pub struct ArtifactReference {
    // Either use name (group/artifact format) or explicit groupId/artifactId
    /// Name in group/artifact format (alternative to explicit groupId/artifactId)
    pub name: Option<String>,
    /// Explicit group ID (alternative to name)
    pub group_id: Option<String>,
    /// Explicit artifact ID (alternative to name)
    pub artifact_id: Option<String>,

    /// EXACT version only (e.g., "1.2.3"), no ranges
    pub version: String,

    /// Optional alias for imports (e.g., "text_message.proto")
    pub name_alias: Option<String>,
}

/// Global configuration for shared registry definitions
///
/// This configuration is loaded from `~/.config/apicurio/registries.yaml`
/// or the path specified by `APICURIO_REGISTRIES_PATH` environment variable.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde_with::skip_serializing_none]
#[serde(rename_all = "camelCase", default)]
pub struct GlobalConfig {
    /// Shared registry definitions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<RegistryConfig>,
}

impl RepoConfig {
    /// Merge global, external, and repo-local registries
    ///
    /// Registry definitions are merged in the following order (later wins):
    /// 1. Global registries from `~/.config/apicurio/registries.yaml`
    /// 2. External registries from file specified in `externalRegistriesFile`
    /// 3. Repository-local registries from `apicurioconfig.yaml`
    ///
    /// # Arguments
    /// * `global` - Global configuration containing shared registries
    ///
    /// # Returns
    /// Vector of merged registry configurations with duplicates resolved
    ///
    /// # Errors
    /// Returns error if external registries file cannot be read or parsed
    pub fn merge_registries(&self, global: GlobalConfig) -> anyhow::Result<Vec<RegistryConfig>> {
        let mut map = std::collections::HashMap::new();
        // 1) global
        for reg in global.registries {
            map.insert(reg.name.clone(), reg);
        }
        // 2) external file
        if let Some(path) = &self.external_registries_file {
            let contents = fs::read_to_string(path)
                .with_context(|| format!("reading external registries from {path}"))?;
            let ext: GlobalConfig = serde_yaml::from_str(&contents)?;
            for reg in ext.registries {
                map.insert(reg.name.clone(), reg);
            }
        }
        // 3) repo-local
        for reg in &self.registries {
            map.insert(reg.name.clone(), reg.clone());
        }
        Ok(map.into_values().collect())
    }
}

impl PublishConfig {
    /// Get the resolved group ID for this publish configuration
    ///
    /// If `group_id` is explicitly set, uses that value. Otherwise:
    /// - If `name` contains '/', uses the part before '/' as group ID
    /// - Otherwise defaults to "default"
    ///
    /// # Examples
    /// - name: "com.example/my-service" → group_id: "com.example"
    /// - name: "my-service" → group_id: "default"
    pub fn resolved_group_id(&self) -> String {
        self.group_id.clone().unwrap_or_else(|| {
            if let Some((group, _)) = self.name.split_once('/') {
                group.to_string()
            } else {
                "default".to_string()
            }
        })
    }

    pub fn resolved_artifact_id(&self) -> String {
        self.artifact_id.clone().unwrap_or_else(|| {
            if let Some((_, artifact)) = self.name.split_once('/') {
                artifact.to_string()
            } else {
                self.name.clone()
            }
        })
    }

    pub fn resolved_content_type(&self) -> String {
        if let Some(ref artifact_type) = self.r#type {
            match artifact_type {
                ArtifactType::Protobuf => "application/x-protobuf".to_string(),
                ArtifactType::Avro => "application/json".to_string(),
                ArtifactType::JsonSchema => "application/json".to_string(),
                ArtifactType::Openapi => "application/json".to_string(),
                ArtifactType::AsyncApi => "application/json".to_string(),
                ArtifactType::GraphQL => "application/graphql".to_string(),
                ArtifactType::Xml => "application/xml".to_string(),
                ArtifactType::Wsdl => "application/xml".to_string(),
            }
        } else {
            // Auto-detect from file extension
            let path = std::path::Path::new(&self.input_path);
            match path.extension().and_then(|e| e.to_str()) {
                Some("proto") => "application/x-protobuf".to_string(),
                Some("avsc") => "application/json".to_string(),
                Some("json") => "application/json".to_string(),
                Some("yaml") | Some("yml") => "application/yaml".to_string(),
                Some("xml") => "application/xml".to_string(),
                Some("graphql") | Some("gql") => "application/graphql".to_string(),
                _ => "application/octet-stream".to_string(),
            }
        }
    }

    pub fn resolved_artifact_type(&self) -> String {
        if let Some(ref artifact_type) = self.r#type {
            match artifact_type {
                ArtifactType::Protobuf => "PROTOBUF".to_string(),
                ArtifactType::Avro => "AVRO".to_string(),
                ArtifactType::JsonSchema => "JSON".to_string(),
                ArtifactType::Openapi => "OPENAPI".to_string(),
                ArtifactType::AsyncApi => "ASYNCAPI".to_string(),
                ArtifactType::GraphQL => "GRAPHQL".to_string(),
                ArtifactType::Xml => "XML".to_string(),
                ArtifactType::Wsdl => "WSDL".to_string(),
            }
        } else {
            // Auto-detect from file extension
            let path = std::path::Path::new(&self.input_path);
            match path.extension().and_then(|e| e.to_str()) {
                Some("proto") => "PROTOBUF".to_string(),
                Some("avsc") => "AVRO".to_string(),
                Some("json") => "JSON".to_string(),
                Some("yaml") | Some("yml") => "JSON".to_string(),
                Some("xml") => "XML".to_string(),
                Some("graphql") | Some("gql") => "GRAPHQL".to_string(),
                _ => "JSON".to_string(),
            }
        }
    }
}

impl DependencyConfig {
    /// Get the resolved group ID for this dependency configuration
    ///
    /// If `group_id` is explicitly set, uses that value. Otherwise:
    /// - If `name` contains '/', uses the part before '/' as group ID
    /// - Otherwise defaults to "default"
    ///
    /// # Examples
    /// - name: "com.example/my-service" → group_id: "com.example"
    /// - name: "my-service" → group_id: "default"
    pub fn resolved_group_id(&self) -> String {
        self.group_id.clone().unwrap_or_else(|| {
            if let Some((group, _)) = self.name.split_once('/') {
                group.to_string()
            } else {
                "default".to_string()
            }
        })
    }

    /// Get the resolved artifact ID for this dependency configuration
    ///
    /// If `artifact_id` is explicitly set, uses that value. Otherwise:
    /// - If `name` contains '/', uses the part after '/' as artifact ID
    /// - Otherwise uses the entire `name` as artifact ID
    ///
    /// # Examples
    /// - name: "com.example/my-service" → artifact_id: "my-service"
    /// - name: "my-service" → artifact_id: "my-service"
    pub fn resolved_artifact_id(&self) -> String {
        self.artifact_id.clone().unwrap_or_else(|| {
            if let Some((_, artifact)) = self.name.split_once('/') {
                artifact.to_string()
            } else {
                self.name.clone()
            }
        })
    }
}

impl ArtifactReference {
    /// Validate that the version is exact (no semver ranges)
    pub fn validate_exact_version(&self) -> anyhow::Result<()> {
        if self.version.contains('^')
            || self.version.contains('~')
            || self.version.contains('*')
            || self.version.contains('>')
            || self.version.contains('<')
        {
            anyhow::bail!(
                "Reference version must be exact, got '{}'. Use exact version like '1.2.3'",
                self.version
            );
        }
        Ok(())
    }

    pub fn resolved_group_id(&self) -> String {
        self.group_id.clone().unwrap_or_else(|| {
            if let Some(name) = &self.name {
                if let Some((group, _)) = name.split_once('/') {
                    group.to_string()
                } else {
                    "default".to_string()
                }
            } else {
                "default".to_string()
            }
        })
    }

    pub fn resolved_artifact_id(&self) -> String {
        self.artifact_id.clone().unwrap_or_else(|| {
            if let Some(name) = &self.name {
                if let Some((_, artifact)) = name.split_once('/') {
                    artifact.to_string()
                } else {
                    name.clone()
                }
            } else {
                panic!("Either name or artifactId must be specified for reference")
            }
        })
    }
}

pub fn load_repo_config(path: &Path) -> anyhow::Result<RepoConfig> {
    let preprocessed_data = preprocess_config(path)?; // Preprocess the YAML file to expand environment variables
    let cfg: RepoConfig = serde_yaml::from_str(&preprocessed_data)?;
    Ok(cfg)
}

pub fn load_global_config() -> anyhow::Result<GlobalConfig> {
    let path = env::var("APICURIO_REGISTRIES_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
            p.push("apicurio/registries.yaml");
            p
        });
    if !path.exists() {
        return Ok(GlobalConfig { registries: vec![] });
    }
    let data = fs::read_to_string(&path)
        .with_context(|| format!("reading global registries {}", path.display()))?;
    let cfg: GlobalConfig = serde_yaml::from_str(&data)?;
    Ok(cfg)
}

pub fn save_global_config(cfg: &GlobalConfig) -> anyhow::Result<()> {
    // same path logic as load_global_config
    let path = env::var("APICURIO_REGISTRIES_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
            p.push("apicurio/registries.yaml");
            p
        });
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_yaml::to_string(cfg)?;
    fs::write(&path, data)?;
    println!("Saved global registries to {}", path.display());
    Ok(())
}

pub fn save_repo_config(cfg: &RepoConfig, path: &Path) -> anyhow::Result<()> {
    use nondestructive::yaml;

    let original = fs::read_to_string(path).unwrap_or_default();
    let mut doc = match yaml::from_slice(original.as_bytes()) {
        Ok(d) => d,
        Err(_) => {
            let data = serde_yaml::to_string(cfg)?;
            fs::write(path, data)?;
            return Ok(());
        }
    };

    let mut root = doc
        .as_mut()
        .into_mapping_mut()
        .context("missing root mapping")?;

    // update dependencies sequence
    let mut seq = match root.get_mut("dependencies") {
        Some(v) => v
            .into_sequence_mut()
            .context("dependencies not a sequence")?,
        None => root.insert("dependencies", yaml::Separator::Auto).make_sequence(),
    };

    seq.clear();
    for dep in &cfg.dependencies {
        let mut item = seq.push(yaml::Separator::Auto).make_mapping();
        item.insert_str("name", &dep.name);
        if let Some(g) = &dep.group_id {
            item.insert_str("groupId", g);
        }
        if let Some(a) = &dep.artifact_id {
            item.insert_str("artifactId", a);
        }
        item.insert_str("version", &dep.version);
        item.insert_str("registry", &dep.registry);
        item.insert_str("outputPath", &dep.output_path);
        if let Some(r) = dep.resolve_references {
            item.insert_bool("resolveReferences", r);
        }
    }

    fs::write(path, doc.to_string())?;
    Ok(())
}

pub fn expand_env_placeholders(input: &str) -> String {
    let re = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)(?:(:?[-+])([^}]*))?\}").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let var_name = &caps[1];
        let op = caps.get(2).map_or("", |m| m.as_str());
        let val = caps.get(3).map_or("", |m| m.as_str());
        let var = env::var(var_name).ok();

        match (var.as_deref(), op) {
            (Some(v), _) if op.is_empty() => v.to_string(), // ${VAR}
            (Some(v), ":-") if !v.is_empty() => v.to_string(), // ${VAR:-default}
            (None, ":-") => val.to_string(),
            (Some(v), "-") => {
                if v.is_empty() {
                    val.to_string()
                } else {
                    v.to_string()
                }
            } // ${VAR-default}
            (None, "-") => val.to_string(),
            (Some(v), ":+") if !v.is_empty() => val.to_string(), // ${VAR:+alt}
            (Some(_), "+") => val.to_string(),                   // ${VAR+alt}
            _ => "".to_string(),
        }
    })
    .to_string()
}

pub fn preprocess_config(path: &Path) -> anyhow::Result<String> {
    let raw_data = fs::read_to_string(path)?;
    Ok(expand_env_placeholders(&raw_data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_smart_resolution() {
        // Test group/artifact format
        let dep_with_slash = DependencyConfig {
            name: "com.example/user-service".to_string(),
            group_id: None,
            artifact_id: None,
            version: "1.0.0".to_string(),
            registry: "test".to_string(),
            output_path: "out.proto".to_string(),
            resolve_references: None,
        };

        assert_eq!(dep_with_slash.resolved_group_id(), "com.example");
        assert_eq!(dep_with_slash.resolved_artifact_id(), "user-service");

        // Test simple name format
        let dep_simple = DependencyConfig {
            name: "user-service".to_string(),
            group_id: None,
            artifact_id: None,
            version: "1.0.0".to_string(),
            registry: "test".to_string(),
            output_path: "out.proto".to_string(),
            resolve_references: None,
        };

        assert_eq!(dep_simple.resolved_group_id(), "default");
        assert_eq!(dep_simple.resolved_artifact_id(), "user-service");

        // Test explicit values override smart resolution
        let dep_explicit = DependencyConfig {
            name: "com.example/user-service".to_string(),
            group_id: Some("custom.group".to_string()),
            artifact_id: Some("custom-artifact".to_string()),
            version: "1.0.0".to_string(),
            registry: "test".to_string(),
            output_path: "out.proto".to_string(),
            resolve_references: None,
        };

        assert_eq!(dep_explicit.resolved_group_id(), "custom.group");
        assert_eq!(dep_explicit.resolved_artifact_id(), "custom-artifact");

        // Test the example from the attachment
        let dep_nprod = DependencyConfig {
            name: "nprod/sp.frame.Frame".to_string(),
            group_id: None,
            artifact_id: None,
            version: "4.3.1".to_string(),
            registry: "nprod-apicurio".to_string(),
            output_path: "protos/sp/frame/frame.proto".to_string(),
            resolve_references: None,
        };

        assert_eq!(dep_nprod.resolved_group_id(), "nprod");
        assert_eq!(dep_nprod.resolved_artifact_id(), "sp.frame.Frame");
    }

    #[test]
    fn test_dependency_smart_resolution_edge_cases() {
        // Test with multiple slashes (only first split should be used)
        let dep_multi_slash = DependencyConfig {
            name: "com.example/nested/artifact".to_string(),
            group_id: None,
            artifact_id: None,
            version: "1.0.0".to_string(),
            registry: "test".to_string(),
            output_path: "out.proto".to_string(),
            resolve_references: None,
        };

        assert_eq!(dep_multi_slash.resolved_group_id(), "com.example");
        assert_eq!(dep_multi_slash.resolved_artifact_id(), "nested/artifact");

        // Test with empty group part
        let dep_empty_group = DependencyConfig {
            name: "/artifact-only".to_string(),
            group_id: None,
            artifact_id: None,
            version: "1.0.0".to_string(),
            registry: "test".to_string(),
            output_path: "out.proto".to_string(),
            resolve_references: None,
        };

        assert_eq!(dep_empty_group.resolved_group_id(), "");
        assert_eq!(dep_empty_group.resolved_artifact_id(), "artifact-only");

        // Test with empty artifact part
        let dep_empty_artifact = DependencyConfig {
            name: "group.only/".to_string(),
            group_id: None,
            artifact_id: None,
            version: "1.0.0".to_string(),
            registry: "test".to_string(),
            output_path: "out.proto".to_string(),
            resolve_references: None,
        };

        assert_eq!(dep_empty_artifact.resolved_group_id(), "group.only");
        assert_eq!(dep_empty_artifact.resolved_artifact_id(), "");

        // Test partial override (only group_id specified)
        let dep_partial_override = DependencyConfig {
            name: "com.example/user-service".to_string(),
            group_id: Some("override.group".to_string()),
            artifact_id: None, // Should use smart resolution from name
            version: "1.0.0".to_string(),
            registry: "test".to_string(),
            output_path: "out.proto".to_string(),
            resolve_references: None,
        };

        assert_eq!(dep_partial_override.resolved_group_id(), "override.group");
        assert_eq!(dep_partial_override.resolved_artifact_id(), "user-service");

        // Test partial override (only artifact_id specified)
        let dep_partial_override2 = DependencyConfig {
            name: "com.example/user-service".to_string(),
            group_id: None, // Should use smart resolution from name
            artifact_id: Some("override-artifact".to_string()),
            version: "1.0.0".to_string(),
            registry: "test".to_string(),
            output_path: "out.proto".to_string(),
            resolve_references: None,
        };

        assert_eq!(dep_partial_override2.resolved_group_id(), "com.example");
        assert_eq!(
            dep_partial_override2.resolved_artifact_id(),
            "override-artifact"
        );
    }

    #[test]
    fn test_dependency_resolution_consistency_with_publish() {
        // Ensure DependencyConfig and PublishConfig resolve identically
        let name = "com.example/user-service";

        let dep = DependencyConfig {
            name: name.to_string(),
            group_id: None,
            artifact_id: None,
            version: "1.0.0".to_string(),
            registry: "test".to_string(),
            output_path: "out.proto".to_string(),
            resolve_references: None,
        };

        let publish = PublishConfig {
            name: name.to_string(),
            input_path: "input.proto".to_string(),
            version: "1.0.0".to_string(),
            registry: "test".to_string(),
            group_id: None,
            artifact_id: None,
            r#type: None,
            if_exists: IfExistsAction::Fail,
            description: None,
            labels: std::collections::HashMap::new(),
            references: Vec::new(),
        };

        assert_eq!(dep.resolved_group_id(), publish.resolved_group_id());
        assert_eq!(dep.resolved_artifact_id(), publish.resolved_artifact_id());
    }
    #[test]
    fn test_empty_output_overrides_not_serialized() {
        let cfg = RepoConfig::default();
        let yaml = serde_yaml::to_string(&cfg).unwrap();
        assert!(!yaml.contains("outputOverrides"));
    }
}
