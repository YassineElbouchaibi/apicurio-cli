use crate::{
    config::{GlobalConfig, load_global_config, load_repo_config},
    dependency::Dependency,
    lockfile::LockFile,
    registry::RegistryClient,
};
use anyhow::{Context, Result};
use semver::Version;
use std::{collections::HashSet, fs, path::PathBuf};

pub async fn run() -> Result<()> {
    // 1) load repo + external + global, check duplicate names
    let repo_cfg = load_repo_config(&PathBuf::from("apicurioconfig.yaml"))?;
    let global_cfg = load_global_config()?;
    let mut seen = HashSet::new();

    for r in &repo_cfg.registries {
        if !seen.insert(r.name.clone()) {
            return Err(anyhow::anyhow!("duplicate registry '{}'", r.name));
        }
    }
    if let Some(path) = &repo_cfg.external_registries_file {
        let ext_content = fs::read_to_string(path)?;
        let ext: GlobalConfig = serde_yaml::from_str(&ext_content)?;
        for r in ext.registries.into_iter() {
            if !seen.insert(r.name.clone()) {
                return Err(anyhow::anyhow!("duplicate registry '{}'", r.name));
            }
        }
    }
    for r in &global_cfg.registries {
        if !seen.insert(r.name.clone()) {
            return Err(anyhow::anyhow!("duplicate registry '{}'", r.name));
        }
    }

    // 2) merge and try to ping each registry
    let merged = repo_cfg.merge_registries(global_cfg.clone())?;
    for r in &merged {
        let client = RegistryClient::new(r)?;
        let health = format!("{}/apis/registry/v2/health", r.url);
        client
            .client
            .get(&health)
            .send()
            .await
            .with_context(|| format!("cannot reach {}", health))?;
    }

    // 3) check each dependency’s semver & registry existence
    for dep in &repo_cfg.dependencies {
        let _ =
            Dependency::from_config(dep).with_context(|| format!("invalid dep '{}'", dep.name))?;
        if !seen.contains(&dep.registry) {
            return Err(anyhow::anyhow!(
                "dependency '{}' references unknown registry '{}'",
                dep.name,
                dep.registry
            ));
        }
    }

    // 4) check lockfile semantic
    let lf = LockFile::load(&PathBuf::from("apicuriolock.yaml")).context("loading lockfile")?;
    for ld in &lf.locked_dependencies {
        if !seen.contains(&ld.registry) {
            return Err(anyhow::anyhow!(
                "lockfile references unknown registry '{}'",
                ld.registry
            ));
        }
        let _ = Version::parse(&ld.resolved_version)
            .with_context(|| format!("invalid version in lock for '{}'", ld.name))?;
    }

    println!("✅ doctor checks passed");
    Ok(())
}
