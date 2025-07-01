use crate::{
    config::{load_global_config, load_repo_config},
    constants::{APICURIO_CONFIG, APICURIO_LOCK},
    dependency::Dependency,
    lockfile::LockFile,
    registry::RegistryClient,
};
use anyhow::Result;
use semver::Version;
use std::{collections::HashMap, path::PathBuf};

pub async fn run() -> Result<()> {
    let repo_cfg = load_repo_config(&PathBuf::from(APICURIO_CONFIG))?;
    let global_cfg = load_global_config()?;
    let regs = repo_cfg.merge_registries(global_cfg)?;
    let mut clients = HashMap::new();
    for r in &regs {
        clients.insert(r.name.clone(), RegistryClient::new(r)?);
    }

    let lock = LockFile::load(&PathBuf::from(APICURIO_LOCK)).ok();
    let mut any_outdated = false;

    for dep_cfg in &repo_cfg.dependencies {
        let dep = Dependency::from_config_with_defaults(dep_cfg, &repo_cfg.dependency_defaults)?;
        let client = &clients[&dep.registry];
        let versions = client
            .list_versions(&dep.group_id, &dep.artifact_id)
            .await?;
        let latest = versions
            .into_iter()
            .filter(|v| dep.req.matches(v))
            .max()
            .ok_or_else(|| anyhow::anyhow!("no matching version for {}", dep.name))?;

        if let Some(lf) = &lock {
            if let Some(ld) = lf.locked_dependencies.iter().find(|d| d.name == dep.name) {
                let locked_ver = Version::parse(&ld.resolved_version)?;
                if locked_ver < latest {
                    println!("🔴 {}: locked={} latest={}", dep.name, locked_ver, latest);
                    any_outdated = true;
                } else {
                    println!("✔️  {} up-to-date ({})", dep.name, locked_ver);
                }
                continue;
            }
        }

        println!("⚪ {} not pulled yet (latest={})", dep.name, latest);
        any_outdated = true;
    }

    if any_outdated {
        std::process::exit(1);
    }
    Ok(())
}
