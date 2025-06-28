use anyhow::Context;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{env, fs, path::PathBuf};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RepoConfig {
    pub external_registries_file: Option<String>,
    #[serde(default)]
    pub registries: Vec<RegistryConfig>,
    #[serde(default)]
    pub dependencies: Vec<DependencyConfig>,
    #[serde(default)]
    pub publishes: Vec<PublishConfig>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegistryConfig {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub auth: AuthConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum AuthConfig {
    None,
    Basic {
        username: String,
        password_env: String,
    },
    Token {
        token_env: String,
    },
    Bearer {
        token_env: String,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DependencyConfig {
    pub name: String,
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub registry: String,
    pub output_path: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PublishConfig {
    pub name: String,
    pub input_path: String,
    pub version: String,
    pub registry: String,

    // Optional fields with smart defaults
    #[serde(default)]
    pub group_id: Option<String>, // defaults from name if contains /
    #[serde(default)]
    pub artifact_id: Option<String>, // defaults from name
    #[serde(default)]
    pub r#type: Option<ArtifactType>, // auto-detect from extension if not specified
    #[serde(default)]
    pub if_exists: IfExistsAction,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub labels: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub references: Vec<ArtifactReference>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactType {
    Protobuf,
    Avro,
    JsonSchema,
    Openapi,
    AsyncApi,
    GraphQL,
    Xml,
    Wsdl,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IfExistsAction {
    Fail,
    CreateVersion,
    FindOrCreateVersion,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactReference {
    // Either use name (group/artifact format) or explicit groupId/artifactId
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub artifact_id: Option<String>,

    pub version: String, // EXACT version only (e.g., "1.2.3"), no ranges

    #[serde(default)]
    pub name_alias: Option<String>, // for proto imports like "text_message.proto"
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GlobalConfig {
    #[serde(default)]
    pub registries: Vec<RegistryConfig>,
}

impl RepoConfig {
    /// Merge global, external, and repo-local registries (later wins)
    pub fn merge_registries(&self, global: GlobalConfig) -> anyhow::Result<Vec<RegistryConfig>> {
        let mut map = std::collections::HashMap::new();
        // 1) global
        for reg in global.registries {
            map.insert(reg.name.clone(), reg);
        }
        // 2) external file
        if let Some(path) = &self.external_registries_file {
            let contents = fs::read_to_string(path)
                .with_context(|| format!("reading external registries from {}", path))?;
            let ext: GlobalConfig = serde_yaml::from_str(&contents)?;
            for reg in ext.registries {
                map.insert(reg.name.clone(), reg);
            }
        }
        // 3) repo-local
        for reg in &self.registries {
            map.insert(reg.name.clone(), reg.clone());
        }
        Ok(map.into_iter().map(|(_, v)| v).collect())
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig::None
    }
}

impl Default for IfExistsAction {
    fn default() -> Self {
        IfExistsAction::Fail
    }
}

impl PublishConfig {
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

pub fn load_repo_config(path: &PathBuf) -> anyhow::Result<RepoConfig> {
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

pub fn expand_env_placeholders(input: &str) -> String {
    let re = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)(?:(:?[-+])([^}]*))?\}").unwrap();
    re.replace_all(input, |caps: &regex::Captures| {
        let var_name = &caps[1];
        let op = caps.get(2).map_or("", |m| m.as_str());
        let val = caps.get(3).map_or("", |m| m.as_str());
        let var = env::var(var_name).ok();

        match (var.as_deref(), op) {
            (Some(v), _) if op == "" => v.to_string(), // ${VAR}
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
