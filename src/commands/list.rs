use anyhow::Result;
use std::path::PathBuf;
use crate::{
    config::{load_repo_config, load_global_config},
    lockfile::LockFile,
};

pub async fn run() -> Result<()> {
    let repo_cfg = load_repo_config(&PathBuf::from("apicurioconfig.yaml"))?;
    let global_cfg = load_global_config()?;
    let regs = repo_cfg.merge_registries(global_cfg)?;

    println!("Registries:");
    for r in regs {
        println!(" - {} → {}", r.name, r.url);
    }

    let lock = LockFile::load(&PathBuf::from("apicuriolock.yaml")).ok();
    println!("\nDependencies:");
    for dep in repo_cfg.dependencies {
        if let Some(lf) = &lock {
            if let Some(ld) = lf
                .locked_dependencies
                .iter()
                .find(|d| d.name == dep.name)
            {
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
