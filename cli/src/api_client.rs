use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Production API URL (used by default in release builds)
#[cfg(not(feature = "local"))]
const DEFAULT_API_URL: &str = "https://api.usehyperstack.com";

/// Local development API URL (enabled with --features local)
#[cfg(feature = "local")]
const DEFAULT_API_URL: &str = "http://localhost:3000";

/// Default domain suffix for WebSocket URLs
pub const DEFAULT_DOMAIN_SUFFIX: &str = "stack.usehyperstack.com";

#[derive(Debug, Clone)]
pub struct ApiClient {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::blocking::Client,
}

// DTOs matching backend models
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub api_key: Option<String>,
    pub user: User,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub entity_name: String,
    pub crate_name: String,
    pub module_path: String,
    pub description: Option<String>,
    pub package_name: Option<String>,
    pub output_path: Option<String>,
    pub url_slug: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Spec {
    pub fn websocket_url(&self, domain_suffix: &str) -> String {
        format!(
            "wss://{}-{}.{}",
            self.name.to_lowercase(),
            self.url_slug,
            domain_suffix
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSpecRequest {
    pub name: String,
    pub entity_name: String,
    pub crate_name: String,
    pub module_path: String,
    pub description: Option<String>,
    pub package_name: Option<String>,
    pub output_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSpecRequest {
    pub name: Option<String>,
    pub entity_name: Option<String>,
    pub crate_name: Option<String>,
    pub module_path: Option<String>,
    pub description: Option<String>,
    pub package_name: Option<String>,
    pub output_path: Option<String>,
}

// ============================================================================
// Spec Version DTOs
// ============================================================================

/// Combined view of spec version with its AST content
#[derive(Debug, Serialize, Deserialize)]
pub struct SpecVersionWithContent {
    pub id: i32,
    pub spec_id: i32,
    pub version_number: i32,
    pub content_hash: String,
    pub version_created_at: String,
    // AST content info
    pub state_name: String,
    pub program_id: Option<String>,
    pub handler_count: i32,
    pub section_count: i32,
}

#[derive(Debug, Serialize)]
pub struct CreateSpecVersionRequest {
    pub ast_payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateSpecVersionResponse {
    pub version: SpecVersionWithContent,
    /// True if the AST content already existed globally
    pub content_is_new: bool,
    /// True if this spec version is new (same spec didn't have this content before)
    pub version_is_new: bool,
    #[allow(dead_code)]
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct SpecWithVersion {
    #[serde(flatten)]
    #[allow(dead_code)]
    pub spec: Spec,
    pub latest_version: Option<SpecVersionWithContent>,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
}

// ============================================================================
// Build DTOs
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildStatus {
    Pending,
    Uploading,
    Queued,
    Building,
    Pushing,
    Deploying,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for BuildStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildStatus::Pending => write!(f, "pending"),
            BuildStatus::Uploading => write!(f, "uploading"),
            BuildStatus::Queued => write!(f, "queued"),
            BuildStatus::Building => write!(f, "building"),
            BuildStatus::Pushing => write!(f, "pushing"),
            BuildStatus::Deploying => write!(f, "deploying"),
            BuildStatus::Completed => write!(f, "completed"),
            BuildStatus::Failed => write!(f, "failed"),
            BuildStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl BuildStatus {
    /// Returns true if this is a terminal state (no more transitions expected)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            BuildStatus::Completed | BuildStatus::Failed | BuildStatus::Cancelled
        )
    }
}

/// Sanitized Build response from API (excludes AWS internals)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Build {
    pub id: i32,
    pub spec_id: Option<i32>,
    pub spec_version_id: Option<i32>,
    pub status: BuildStatus,
    pub status_message: Option<String>,
    pub phase: Option<String>,
    pub progress: Option<i32>,
    pub websocket_url: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

/// Sanitized BuildEvent response from API (excludes raw_payload and event_source)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildEvent {
    pub id: i32,
    pub build_id: i32,
    pub event_type: String,
    pub phase: Option<String>,
    pub previous_status: Option<BuildStatus>,
    pub new_status: Option<BuildStatus>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreateBuildRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_version_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast_payload: Option<serde_json::Value>,
    /// Branch name for branch deployments (e.g., "preview-abc123")
    /// Branch deployments get URL: {spec-name}-{branch}.stack.usehyperstack.com
    /// Production deployments (no branch) get: {spec-name}.stack.usehyperstack.com
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBuildResponse {
    pub build_id: i32,
    pub status: BuildStatus,
    #[allow(dead_code)]
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildStatusResponse {
    pub build: Build,
    pub events: Vec<BuildEvent>,
}

// ============================================================================
// Deployment DTOs
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    Active,
    Updating,
    Stopped,
    Failed,
}

impl std::fmt::Display for DeploymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentStatus::Active => write!(f, "active"),
            DeploymentStatus::Updating => write!(f, "updating"),
            DeploymentStatus::Stopped => write!(f, "stopped"),
            DeploymentStatus::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentResponse {
    pub id: i32,
    pub spec_id: i32,
    pub spec_name: String,
    pub atom_name: String,
    pub branch: Option<String>,
    pub current_build_id: Option<i32>,
    pub current_version: Option<i32>,
    pub current_image_tag: Option<String>,
    pub websocket_url: String,
    pub status: DeploymentStatus,
    pub status_message: Option<String>,
    pub first_deployed_at: Option<String>,
    pub last_deployed_at: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct StopDeploymentResponse {
    pub message: String,
    pub deployment_id: i32,
    pub status: DeploymentStatus,
}

impl ApiClient {
    pub fn new() -> Result<Self> {
        let base_url =
            std::env::var("HYPERSTACK_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string());

        let api_key = Self::load_api_key().ok();

        Ok(ApiClient {
            base_url,
            api_key,
            client: reqwest::blocking::Client::new(),
        })
    }

    #[allow(dead_code)]
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    // Authentication endpoints

    pub fn register(&self, username: &str, password: &str) -> Result<LoginResponse> {
        let req = RegisterRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/api/auth/register", self.base_url))
            .json(&req)
            .send()
            .context("Failed to send register request")?;

        Self::handle_response(response)
    }

    pub fn login(&self, username: &str, password: &str) -> Result<LoginResponse> {
        let req = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = self
            .client
            .post(format!("{}/api/auth/login", self.base_url))
            .json(&req)
            .send()
            .context("Failed to send login request")?;

        Self::handle_response(response)
    }

    // Spec endpoints

    pub fn list_specs(&self) -> Result<Vec<Spec>> {
        let api_key = self.require_api_key()?;

        let response = self
            .client
            .get(format!("{}/api/specs", self.base_url))
            .bearer_auth(api_key)
            .send()
            .context("Failed to send list specs request")?;

        Self::handle_response(response)
    }

    #[allow(dead_code)]
    pub fn get_spec(&self, spec_id: i32) -> Result<Spec> {
        let api_key = self.require_api_key()?;

        let response = self
            .client
            .get(format!("{}/api/specs/{}", self.base_url, spec_id))
            .bearer_auth(api_key)
            .send()
            .context("Failed to send get spec request")?;

        Self::handle_response(response)
    }

    pub fn create_spec(&self, req: CreateSpecRequest) -> Result<Spec> {
        let api_key = self.require_api_key()?;

        let response = self
            .client
            .post(format!("{}/api/specs", self.base_url))
            .bearer_auth(api_key)
            .json(&req)
            .send()
            .context("Failed to send create spec request")?;

        Self::handle_response(response)
    }

    pub fn update_spec(&self, spec_id: i32, req: UpdateSpecRequest) -> Result<Spec> {
        let api_key = self.require_api_key()?;

        let response = self
            .client
            .put(format!("{}/api/specs/{}", self.base_url, spec_id))
            .bearer_auth(api_key)
            .json(&req)
            .send()
            .context("Failed to send update spec request")?;

        Self::handle_response(response)
    }

    pub fn delete_spec(&self, spec_id: i32) -> Result<()> {
        let api_key = self.require_api_key()?;

        let response = self
            .client
            .delete(format!("{}/api/specs/{}", self.base_url, spec_id))
            .bearer_auth(api_key)
            .send()
            .context("Failed to send delete spec request")?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error: ErrorResponse = response.json()?;
            anyhow::bail!("API error: {}", error.error);
        }
    }

    // Spec version endpoints

    /// Upload AST to create a new spec version
    pub fn create_spec_version(
        &self,
        spec_id: i32,
        ast_payload: serde_json::Value,
    ) -> Result<CreateSpecVersionResponse> {
        let api_key = self.require_api_key()?;

        let req = CreateSpecVersionRequest { ast_payload };

        let response = self
            .client
            .post(format!("{}/api/specs/{}/versions", self.base_url, spec_id))
            .bearer_auth(api_key)
            .json(&req)
            .send()
            .context("Failed to send create spec version request")?;

        Self::handle_response(response)
    }

    /// Get spec with its latest version info
    pub fn get_spec_with_latest_version(&self, spec_id: i32) -> Result<SpecWithVersion> {
        let api_key = self.require_api_key()?;

        let response = self
            .client
            .get(format!(
                "{}/api/specs/{}/versions/latest",
                self.base_url, spec_id
            ))
            .bearer_auth(api_key)
            .send()
            .context("Failed to send get spec with version request")?;

        Self::handle_response(response)
    }

    /// List all versions for a spec
    pub fn list_spec_versions(&self, spec_id: i32) -> Result<Vec<SpecVersionWithContent>> {
        let api_key = self.require_api_key()?;

        let response = self
            .client
            .get(format!("{}/api/specs/{}/versions", self.base_url, spec_id))
            .bearer_auth(api_key)
            .send()
            .context("Failed to send list spec versions request")?;

        Self::handle_response(response)
    }

    /// List all versions for a spec with pagination
    pub fn list_spec_versions_paginated(
        &self,
        spec_id: i32,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SpecVersionWithContent>> {
        let api_key = self.require_api_key()?;

        let mut url = format!("{}/api/specs/{}/versions", self.base_url, spec_id);
        let mut params = vec![];
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self
            .client
            .get(&url)
            .bearer_auth(api_key)
            .send()
            .context("Failed to send list spec versions request")?;

        Self::handle_response(response)
    }

    /// Helper to get spec by name
    pub fn get_spec_by_name(&self, name: &str) -> Result<Option<Spec>> {
        let specs = self.list_specs()?;
        Ok(specs.into_iter().find(|s| s.name == name))
    }

    // ========================================================================
    // Build endpoints
    // ========================================================================

    /// Create a new build
    pub fn create_build(&self, req: CreateBuildRequest) -> Result<CreateBuildResponse> {
        let api_key = self.require_api_key()?;

        let response = self
            .client
            .post(format!("{}/api/builds", self.base_url))
            .bearer_auth(api_key)
            .json(&req)
            .send()
            .context("Failed to send create build request")?;

        Self::handle_response(response)
    }

    /// List builds for the authenticated user
    pub fn list_builds(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<Build>> {
        self.list_builds_filtered(limit, offset, None)
    }

    /// List builds for the authenticated user, optionally filtered by spec_id
    pub fn list_builds_filtered(
        &self,
        limit: Option<i64>,
        offset: Option<i64>,
        spec_id: Option<i32>,
    ) -> Result<Vec<Build>> {
        let api_key = self.require_api_key()?;

        let mut url = format!("{}/api/builds", self.base_url);
        let mut params = vec![];
        if let Some(l) = limit {
            params.push(format!("limit={}", l));
        }
        if let Some(o) = offset {
            params.push(format!("offset={}", o));
        }
        if let Some(sid) = spec_id {
            params.push(format!("spec_id={}", sid));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self
            .client
            .get(&url)
            .bearer_auth(api_key)
            .send()
            .context("Failed to send list builds request")?;

        Self::handle_response(response)
    }

    /// Get build status and events by ID
    pub fn get_build(&self, build_id: i32) -> Result<BuildStatusResponse> {
        let api_key = self.require_api_key()?;

        let response = self
            .client
            .get(format!("{}/api/builds/{}", self.base_url, build_id))
            .bearer_auth(api_key)
            .send()
            .context("Failed to send get build request")?;

        Self::handle_response(response)
    }

    // ========================================================================
    // Deployment endpoints
    // ========================================================================

    /// List all deployments for the authenticated user
    pub fn list_deployments(&self, limit: i64) -> Result<Vec<DeploymentResponse>> {
        let api_key = self.require_api_key()?;

        let url = format!("{}/api/deployments?limit={}", self.base_url, limit);

        let response = self
            .client
            .get(&url)
            .bearer_auth(api_key)
            .send()
            .context("Failed to send list deployments request")?;

        Self::handle_response(response)
    }

    /// Get deployment by ID
    #[allow(dead_code)]
    pub fn get_deployment(&self, deployment_id: i32) -> Result<DeploymentResponse> {
        let api_key = self.require_api_key()?;

        let response = self
            .client
            .get(format!(
                "{}/api/deployments/{}",
                self.base_url, deployment_id
            ))
            .bearer_auth(api_key)
            .send()
            .context("Failed to send get deployment request")?;

        Self::handle_response(response)
    }

    /// Stop a deployment
    #[allow(dead_code)]
    pub fn stop_deployment(&self, deployment_id: i32) -> Result<StopDeploymentResponse> {
        let api_key = self.require_api_key()?;

        let response = self
            .client
            .delete(format!(
                "{}/api/deployments/{}",
                self.base_url, deployment_id
            ))
            .bearer_auth(api_key)
            .send()
            .context("Failed to send stop deployment request")?;

        Self::handle_response(response)
    }

    // Helper methods

    fn require_api_key(&self) -> Result<&str> {
        self.api_key
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated. Run 'hs auth login' first."))
    }

    fn handle_response<T: for<'de> Deserialize<'de>>(
        response: reqwest::blocking::Response,
    ) -> Result<T> {
        if response.status().is_success() {
            response.json().context("Failed to parse response JSON")
        } else {
            let status = response.status();
            let error: ErrorResponse = response.json().unwrap_or_else(|_| ErrorResponse {
                error: "Unknown error".to_string(),
            });
            anyhow::bail!("API error ({}): {}", status, error.error);
        }
    }

    // Credentials management

    fn credentials_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        Ok(home.join(".hyperstack").join("credentials.toml"))
    }

    pub fn save_api_key(api_key: &str) -> Result<()> {
        let path = Self::credentials_path()?;

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = format!("api_key = \"{}\"\n", api_key);
        fs::write(&path, content).context("Failed to save API key")?;

        Ok(())
    }

    pub fn load_api_key() -> Result<String> {
        let path = Self::credentials_path()?;
        let content = fs::read_to_string(&path)
            .context("Failed to read credentials file. Have you run 'hs auth login'?")?;

        #[derive(Deserialize)]
        struct Credentials {
            api_key: String,
        }

        let creds: Credentials =
            toml::from_str(&content).context("Failed to parse credentials file")?;

        Ok(creds.api_key)
    }

    pub fn delete_api_key() -> Result<()> {
        let path = Self::credentials_path()?;
        if path.exists() {
            fs::remove_file(&path).context("Failed to delete credentials file")?;
        }
        Ok(())
    }
}
