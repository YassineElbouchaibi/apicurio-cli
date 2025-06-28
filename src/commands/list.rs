use crate::{
    config::{load_global_config, load_repo_config},
    constants::{APICURIO_CONFIG, APICURIO_LOCK},
    lockfile::LockFile,
};
use anyhow::Result;
use std::path::PathBuf;

pub async fn run() -> Result<()> {
    let repo_cfg = load_repo_config(&PathBuf::from(APICURIO_CONFIG))?;
    let global_cfg = load_global_config()?;
    let regs = repo_cfg.merge_registries(global_cfg)?;

    println!("Registries:");
    for r in regs {
        println!(" - {} → {}", r.name, r.url);
    }

    let lock = LockFile::load(&PathBuf::from(APICURIO_LOCK)).ok();
    println!("\nDependencies:");
    for dep in repo_cfg.dependencies {
        if let Some(lf) = &lock {
            if let Some(ld) = lf.locked_dependencies.iter().find(|d| d.name == dep.name) {
                println!(
                    " - {}: spec={} locked={}",
                    dep.name, dep.version, ld.resolved_version
                );
                continue;
            }
        }
        println!(" - {}: spec={}", dep.name, dep.version);
    }

    Ok(())
}
