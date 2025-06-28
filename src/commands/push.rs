use anyhow::{Context, Result};
use semver::Version;
use std::{collections::HashMap, fs, path::PathBuf};

use crate::{
    config::{load_global_config, load_repo_config},
    constants::APICURIO_CONFIG,
    dependency::Dependency,
    registry::RegistryClient,
};

pub async fn run() -> Result<()> {
    // 1) load repo + global + merged registries
    let repo_cfg = load_repo_config(&PathBuf::from(APICURIO_CONFIG))?;
    let global_cfg = load_global_config()?;
    let regs = repo_cfg.merge_registries(global_cfg)?;

    // 2) build clients
    let mut clients = HashMap::new();
    for r in &regs {
        clients.insert(r.name.clone(), RegistryClient::new(r)?);
    }

    // 3) for each dependency, read its .proto and push
    for dep_cfg in &repo_cfg.dependencies {
        let dep = Dependency::from_config(dep_cfg)?;

        // locate the file we expect was generated/pulled earlier
        let file_path = PathBuf::from(&dep.output_path);
        let data =
            fs::read(&file_path).with_context(|| format!("reading {}", file_path.display()))?;

        // only set a custom version header if the config.version is an exact semver
        let version_header = Version::parse(&dep_cfg.version).ok().map(|v| v.to_string());

        // push it
        let client = &clients[&dep.registry];
        client
            .create_or_update(
                &dep.group_id,
                &dep.artifact_id,
                version_header.as_deref(),
                &data,
            )
            .await
            .with_context(|| {
                format!(
                    "pushing {} (artifact={}, group={})",
                    dep.name, dep_cfg.artifact_id, dep_cfg.group_id
                )
            })?;

        println!(
            "✅ pushed `{}` → registry `{}` as version `{}`",
            dep.name,
            dep.registry,
            version_header.unwrap_or_else(|| "auto".into())
        );
    }

    Ok(())
}
