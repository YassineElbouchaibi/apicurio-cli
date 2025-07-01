use crate::config::{DependencyConfig, DependencyDefaultsConfig};
use anyhow::Result;
use semver::VersionReq;

pub struct Dependency {
    pub name: String,
    pub group_id: String,
    pub artifact_id: String,
    pub req: VersionReq,
    pub registry: String,
    pub output_path: Option<String>,
}

impl Dependency {
    pub fn from_config_with_defaults(
        cfg: &DependencyConfig,
        defaults: &DependencyDefaultsConfig,
    ) -> Result<Self> {
        let registry = cfg
            .registry
            .clone()
            .or_else(|| defaults.registry.clone())
            .ok_or_else(|| {
                anyhow::anyhow!("No registry specified for dependency '{}'", cfg.name)
            })?;

        Ok(Dependency {
            name: cfg.name.clone(),
            group_id: cfg.resolved_group_id(),
            artifact_id: cfg.resolved_artifact_id(),
            req: VersionReq::parse(&cfg.version)?,
            registry,
            output_path: cfg.output_path.clone(),
        })
    }
}
