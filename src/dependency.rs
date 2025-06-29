use crate::config::DependencyConfig;
use anyhow::Result;
use semver::VersionReq;

pub struct Dependency {
    pub name: String,
    pub group_id: String,
    pub artifact_id: String,
    pub req: VersionReq,
    pub registry: String,
    pub output_path: String,
}

impl Dependency {
    pub fn from_config(cfg: &DependencyConfig) -> Result<Self> {
        Ok(Dependency {
            name: cfg.name.clone(),
            group_id: cfg.resolved_group_id(),
            artifact_id: cfg.resolved_artifact_id(),
            req: VersionReq::parse(&cfg.version)?,
            registry: cfg.registry.clone(),
            output_path: cfg.output_path.clone(),
        })
    }
}
