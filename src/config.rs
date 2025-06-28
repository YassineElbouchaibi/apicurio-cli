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

#[derive(Deserialize, Serialize, Debug)]
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

pub fn load_repo_config(path: &PathBuf) -> anyhow::Result<RepoConfig> {
    let preprocessed_data = preprocess_config(path)?; // Preprocess the YAML file to expand environment variables
    println!("Preprocessed config data:\n{}", preprocessed_data);
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
