use crate::config::{AuthConfig, IfExistsAction, PublishConfig, RegistryConfig};
use anyhow::Result;
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client,
};
use semver::Version;
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;

/// Suggest a version bump for a given version string
fn suggest_version_bump(version: &str) -> String {
    if let Ok(parsed_version) = Version::parse(version) {
        // Bump patch version
        let mut new_version = parsed_version.clone();
        new_version.patch += 1;
        new_version.to_string()
    } else {
        // If not a valid semver, try simple string manipulation
        if let Some(last_dot) = version.rfind('.') {
            let (prefix, suffix) = version.split_at(last_dot + 1);
            if let Ok(patch) = suffix.parse::<u64>() {
                format!("{}{}", prefix, patch + 1)
            } else {
                format!("{version}.1")
            }
        } else {
            format!("{version}.1")
        }
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactMetadata {
    pub artifact_id: String,
    pub artifact_type: String,
    #[serde(default, alias = "groupId", alias = "group")]
    pub group_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub built_on: String,
}

pub struct RegistryClient {
    #[allow(dead_code)]
    pub name: String,
    pub base_url: String,
    pub client: Client,
}

impl RegistryClient {
    pub fn new(cfg: &RegistryConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        match &cfg.auth {
            AuthConfig::None => {}
            AuthConfig::Basic {
                username,
                password_env,
            } => {
                let pw = env::var(password_env)?;
                let token = base64::encode_config(format!("{username}:{pw}"), base64::STANDARD);
                let hv = HeaderValue::from_str(&format!("Basic {token}"))?;
                headers.insert(AUTHORIZATION, hv);
            }
            AuthConfig::Token { token_env } => {
                let tok = env::var(token_env)?;
                let hv = HeaderValue::from_str(&tok)?;
                headers.insert(AUTHORIZATION, hv);
            }
            AuthConfig::Bearer { token_env } => {
                let tok = env::var(token_env)?;
                let hv = HeaderValue::from_str(&format!("Bearer {tok}"))?;
                headers.insert(AUTHORIZATION, hv);
            }
        }

        let client = Client::builder().default_headers(headers).build()?;
        Ok(RegistryClient {
            name: cfg.name.clone(),
            base_url: cfg.url.clone(),
            client,
        })
    }

    /// List all published versions for a given artifact
    pub async fn list_versions(&self, group_id: &str, artifact_id: &str) -> Result<Vec<Version>> {
        let url = format!(
            "{}/apis/registry/v3/groups/{}/artifacts/{}/versions",
            self.base_url, group_id, artifact_id
        );
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        #[derive(Deserialize)]
        struct ApiResponse {
            #[allow(dead_code)]
            count: usize,
            versions: Vec<ApiVersion>,
        }

        #[derive(Deserialize)]
        struct ApiVersion {
            version: String,
        }

        let api_response: ApiResponse = resp.json().await?;
        let mut semver_versions = Vec::new();
        for v in api_response.versions {
            if let Ok(parsed) = Version::parse(&v.version) {
                semver_versions.push(parsed);
            }
        }
        Ok(semver_versions)
    }

    pub fn get_download_url(&self, group_id: &str, artifact_id: &str, version: &Version) -> String {
        format!(
            "{}/apis/registry/v3/groups/{}/artifacts/{}/versions/{}/content",
            self.base_url, group_id, artifact_id, version
        )
    }

    /// Download the raw content for a specific version
    pub async fn download(
        &self,
        group_id: &str,
        artifact_id: &str,
        version: &Version,
    ) -> Result<bytes::Bytes> {
        let url = self.get_download_url(group_id, artifact_id, version);
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        Ok(resp.bytes().await?)
    }

    /// List all groups in the registry
    pub async fn list_groups(&self) -> Result<Vec<String>> {
        let url = format!("{}/apis/registry/v3/groups", self.base_url);
        let resp = self.client.get(&url).send().await?.error_for_status()?;

        #[derive(Deserialize)]
        struct ApiResponse {
            #[allow(dead_code)]
            count: usize,
            groups: Vec<ApiGroup>,
        }

        #[derive(Deserialize)]
        struct ApiGroup {
            #[serde(rename = "groupId")]
            group_id: String,
        }

        let api_response: ApiResponse = resp.json().await?;
        Ok(api_response
            .groups
            .into_iter()
            .map(|g| g.group_id)
            .collect())
    }

    /// List all artifacts in a specific group
    pub async fn list_artifacts(&self, group_id: &str) -> Result<Vec<String>> {
        let url = format!(
            "{}/apis/registry/v3/groups/{}/artifacts",
            self.base_url, group_id
        );
        let resp = self.client.get(&url).send().await?.error_for_status()?;

        #[derive(Deserialize)]
        struct ApiResponse {
            #[allow(dead_code)]
            count: usize,
            artifacts: Vec<ApiArtifact>,
        }

        #[derive(Deserialize)]
        struct ApiArtifact {
            #[serde(rename = "artifactId")]
            artifact_id: String,
        }

        let api_response: ApiResponse = resp.json().await?;
        Ok(api_response
            .artifacts
            .into_iter()
            .map(|a| a.artifact_id)
            .collect())
    }

    /// Check if an artifact exists in the registry
    pub async fn artifact_exists(&self, group_id: &str, artifact_id: &str) -> Result<bool> {
        let url = format!(
            "{}/apis/registry/v3/groups/{}/artifacts/{}",
            self.base_url, group_id, artifact_id
        );

        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Get artifact metadata including type
    pub async fn get_artifact_metadata(
        &self,
        group_id: &str,
        artifact_id: &str,
    ) -> Result<ArtifactMetadata> {
        let url = format!(
            "{}/apis/registry/v3/groups/{}/artifacts/{}",
            self.base_url, group_id, artifact_id
        );
        let resp = self.client.get(&url).send().await?.error_for_status()?;

        let mut metadata: ArtifactMetadata = resp.json().await?;
        // Ensure group_id is set even if not provided by the API response
        if metadata.group_id.is_none() {
            metadata.group_id = Some(group_id.to_string());
        }
        Ok(metadata)
    }

    /// Publish an artifact to the registry
    pub async fn publish_artifact(&self, publish: &PublishConfig, content: &str) -> Result<()> {
        let group_id = publish.resolved_group_id();
        let artifact_id = publish.resolved_artifact_id();
        let content_type = publish.resolved_content_type();
        let artifact_type = publish.resolved_artifact_type();

        // Check if the version already exists
        if self
            .version_exists(&group_id, &artifact_id, &publish.version)
            .await?
        {
            // Version exists, compare content
            match self
                .get_version_content(&group_id, &artifact_id, &publish.version)
                .await
            {
                Ok(existing_content) => {
                    if existing_content.trim() == content.trim() {
                        println!(
                            "  ℹ️  Version {}@{} already published with identical content",
                            artifact_id, publish.version
                        );
                        return Ok(());
                    } else {
                        // Content is different, suggest version bump
                        println!(
                            "  ⚠️  Version {}@{} already exists with different content",
                            artifact_id, publish.version
                        );
                        println!(
                            "     Consider bumping the version (e.g., {}) to publish the updated content",
                            suggest_version_bump(&publish.version)
                        );
                        anyhow::bail!("Cannot publish different content with same version");
                    }
                }
                Err(_) => {
                    // Could not retrieve existing content, proceed with normal flow
                    println!(
                        "  ⚠️  Version {}@{} exists but content comparison failed, proceeding with publish",
                        artifact_id, publish.version
                    );
                }
            }
        }

        // Build references array for the API
        let references: Vec<Value> = publish
            .references
            .iter()
            .map(|r| {
                json!({
                    "groupId": r.resolved_group_id(),
                    "artifactId": r.resolved_artifact_id(),
                    "version": r.version,
                    "name": r.name_alias.as_deref().unwrap_or(&r.resolved_artifact_id())
                })
            })
            .collect();

        // Check if artifact exists to determine which endpoint to use
        let artifact_exists = self.artifact_exists(&group_id, &artifact_id).await?;

        if artifact_exists {
            // Artifact exists, create a new version using the versions endpoint
            let version_payload = json!({
                "version": publish.version,
                "content": {
                    "content": content,
                    "contentType": content_type,
                    "references": references
                },
                "name": &publish.name,
                "description": publish.description.as_deref().unwrap_or(""),
                "labels": {}
            });

            let url = format!(
                "{}/apis/registry/v3/groups/{}/artifacts/{}/versions",
                self.base_url.trim_end_matches('/'),
                group_id,
                artifact_id
            );

            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&version_payload)
                .send()
                .await?;

            if response.status().is_success() {
                println!("  ✅ Published {}@{}", artifact_id, publish.version);
                Ok(())
            } else {
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                anyhow::bail!(
                    "Failed to publish {}@{}: HTTP {} - {}",
                    artifact_id,
                    publish.version,
                    status,
                    body
                );
            }
        } else {
            // Artifact doesn't exist, create new artifact with first version
            let payload = json!({
                "artifactId": artifact_id,
                "artifactType": artifact_type,
                "name": &publish.name,
                "description": publish.description.as_deref().unwrap_or(""),
                "labels": publish.labels,
                "firstVersion": {
                    "version": publish.version,
                    "content": {
                        "content": content,
                        "contentType": content_type,
                        "references": references
                    },
                    "name": &publish.name,
                    "description": publish.description.as_deref().unwrap_or(""),
                    "labels": {}
                }
            });

            // Determine the ifExists parameter
            let if_exists_param = match publish.if_exists {
                IfExistsAction::Fail => "FAIL",
                IfExistsAction::CreateVersion => "CREATE_VERSION",
                IfExistsAction::FindOrCreateVersion => "FIND_OR_CREATE_VERSION",
            };

            // Make the HTTP request
            let url = format!(
                "{}/apis/registry/v3/groups/{}/artifacts?ifExists={}",
                self.base_url.trim_end_matches('/'),
                group_id,
                if_exists_param
            );

            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await?;

            if response.status().is_success() {
                println!("  ✅ Published {}@{}", artifact_id, publish.version);
                Ok(())
            } else {
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                anyhow::bail!(
                    "Failed to publish {}@{}: HTTP {} - {}",
                    artifact_id,
                    publish.version,
                    status,
                    body
                );
            }
        }
    }

    /// Check if a specific artifact version exists
    pub async fn version_exists(
        &self,
        group_id: &str,
        artifact_id: &str,
        version: &str,
    ) -> Result<bool> {
        let url = format!(
            "{}/apis/registry/v3/groups/{}/artifacts/{}/versions/{}",
            self.base_url, group_id, artifact_id, version
        );

        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Get the content of a specific artifact version as a string
    pub async fn get_version_content(
        &self,
        group_id: &str,
        artifact_id: &str,
        version: &str,
    ) -> Result<String> {
        let url = format!(
            "{}/apis/registry/v3/groups/{}/artifacts/{}/versions/{}/content",
            self.base_url, group_id, artifact_id, version
        );
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        Ok(resp.text().await?)
    }

    /// Get system information from the registry
    pub async fn get_system_info(&self) -> Result<SystemInfo> {
        let url = format!("{}/apis/registry/v3/system/info", self.base_url);
        let resp = self.client.get(&url).send().await?.error_for_status()?;
        let system_info: SystemInfo = resp.json().await?;
        Ok(system_info)
    }
}
