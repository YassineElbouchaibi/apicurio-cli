use crate::config::{AuthConfig, RegistryConfig};
use anyhow::Result;
use reqwest::{
    Client,
    header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue},
};
use semver::Version;
use serde::Deserialize;
use std::env;

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
                let token = base64::encode_config(format!("{}:{}", username, pw), base64::STANDARD);
                let hv = HeaderValue::from_str(&format!("Basic {}", token))?;
                headers.insert(AUTHORIZATION, hv);
            }
            AuthConfig::Token { token_env } => {
                let tok = env::var(token_env)?;
                let hv = HeaderValue::from_str(&tok)?;
                headers.insert(AUTHORIZATION, hv);
            }
            AuthConfig::Bearer { token_env } => {
                let tok = env::var(token_env)?;
                let hv = HeaderValue::from_str(&format!("Bearer {}", tok))?;
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

    /// Create a new artifact or add a new version to an existing artifact.
    /// If `version` is `Some(v)`, sends it as X-Registry-Version; otherwise the registry auto-assigns.
    pub async fn create_or_update(
        &self,
        group_id: &str,
        artifact_id: &str,
        version: Option<&str>,
        data: &[u8],
    ) -> anyhow::Result<()> {
        // POST to /apis/registry/v2/groups/{group}/artifacts
        let url = format!(
            "{}/apis/registry/v2/groups/{}/artifacts",
            self.base_url, group_id
        );
        let mut req = self.client.post(&url);

        // tell Apicurio which artifact ID to use
        req = req.header("X-Registry-ArtifactId", artifact_id);

        // optionally pin the version number
        if let Some(ver) = version {
            req = req.header("X-Registry-Version", ver);
        }

        // content is Protobuf; tell Apicurio to treat it as PROTOBUF
        req = req.header(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-protobuf; artifactType=PROTOBUF"),
        );

        // attach the raw bytes
        let _resp = req.body(data.to_vec()).send().await?.error_for_status()?;
        Ok(())
    }
}
