use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Clone)]
pub struct LockedDependency {
    pub name: String,
    pub registry: String,
    pub resolved_version: String,
    pub download_url: String,
    pub sha256: String,
    pub output_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct LockFile {
    #[serde(rename = "lockedDependencies")]
    pub locked_dependencies: Vec<LockedDependency>,
}

impl LockFile {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let data = fs::read_to_string(path)?;
        let lf: LockFile = serde_yaml::from_str(&data)?;
        Ok(lf)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let data = serde_yaml::to_string(self)?;
        fs::write(path, data)?;
        Ok(())
    }
}
