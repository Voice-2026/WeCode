use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

pub const MODEL_CATALOG_SCHEMA_VERSION: u32 = 1;
pub const MODEL_CATALOG_STALE_AFTER_HOURS: i64 = 24;
pub const MODEL_DISCOVERY_MAX_OUTPUT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelClientCompatibility {
    #[serde(default)]
    pub claude_code: bool,
    #[serde(default)]
    pub codex_cli: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayModel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub context_window_tokens: u64,
    #[serde(default)]
    pub rate_multiplier: f64,
    #[serde(default = "default_rate_unit")]
    pub rate_unit: String,
    pub owned_by: String,
    #[serde(default)]
    pub compatibility: ModelClientCompatibility,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayModelCatalog {
    pub schema_version: u32,
    pub source: String,
    pub refreshed_at: DateTime<Utc>,
    #[serde(default)]
    pub default_model: Option<String>,
    pub models: Vec<GatewayModel>,
}

#[derive(Debug, Deserialize)]
struct KiroModelList {
    #[serde(default)]
    models: Vec<KiroModel>,
    #[serde(default)]
    default_model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KiroModel {
    model_name: String,
    model_id: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    context_window_tokens: u64,
    #[serde(default)]
    rate_multiplier: f64,
    #[serde(default = "default_rate_unit")]
    rate_unit: String,
}

fn default_rate_unit() -> String {
    "Credit".to_string()
}

impl GatewayModelCatalog {
    pub fn fallback() -> Self {
        let models = crate::config::FALLBACK_MODELS
            .iter()
            .map(|id| GatewayModel::from_id(id))
            .collect();
        Self {
            schema_version: MODEL_CATALOG_SCHEMA_VERSION,
            source: "fallback".to_string(),
            refreshed_at: DateTime::<Utc>::UNIX_EPOCH,
            default_model: Some("claude-sonnet-4.6".to_string()),
            models,
        }
    }

    pub fn is_stale(&self, now: DateTime<Utc>) -> bool {
        now.signed_duration_since(self.refreshed_at).num_hours() >= MODEL_CATALOG_STALE_AFTER_HOURS
    }

    pub fn is_stale_now(&self) -> bool {
        self.is_stale(Utc::now())
    }

    pub fn model(&self, id: &str) -> Option<&GatewayModel> {
        self.models.iter().find(|model| model.id == id)
    }

    pub fn claude_code_models(&self) -> impl Iterator<Item = &GatewayModel> {
        self.models
            .iter()
            .filter(|model| model.compatibility.claude_code)
    }

    pub fn codex_cli_models(&self) -> impl Iterator<Item = &GatewayModel> {
        self.models
            .iter()
            .filter(|model| model.compatibility.codex_cli)
    }
}

impl Default for GatewayModelCatalog {
    fn default() -> Self {
        Self::fallback()
    }
}

impl GatewayModel {
    fn from_id(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            context_window_tokens: 0,
            rate_multiplier: 0.0,
            rate_unit: default_rate_unit(),
            owned_by: owner_for_model(id).to_string(),
            compatibility: compatibility_for_model(id),
        }
    }
}

pub fn parse_kiro_model_catalog(
    output: &[u8],
    refreshed_at: DateTime<Utc>,
) -> Result<GatewayModelCatalog, String> {
    let response: KiroModelList = serde_json::from_slice(output)
        .map_err(|error| format!("Kiro CLI returned invalid model JSON: {error}"))?;
    if response.models.is_empty() {
        return Err("Kiro CLI returned an empty model list".to_string());
    }

    let mut models: Vec<_> = response
        .models
        .into_iter()
        .filter(|model| !model.model_id.trim().is_empty())
        .map(|model| GatewayModel {
            owned_by: owner_for_model(&model.model_id).to_string(),
            compatibility: compatibility_for_model(&model.model_id),
            id: model.model_id,
            name: model.model_name,
            description: model.description,
            context_window_tokens: model.context_window_tokens,
            rate_multiplier: model.rate_multiplier,
            rate_unit: model.rate_unit,
        })
        .collect();
    models.sort_by(|left, right| left.id.cmp(&right.id));
    models.dedup_by(|left, right| left.id == right.id);
    if models.is_empty() {
        return Err("Kiro CLI returned no valid model IDs".to_string());
    }

    Ok(GatewayModelCatalog {
        schema_version: MODEL_CATALOG_SCHEMA_VERSION,
        source: "kiro-cli".to_string(),
        refreshed_at,
        default_model: response.default_model,
        models,
    })
}

pub async fn discover_kiro_model_catalog(
    program: impl AsRef<Path>,
    timeout: Duration,
    max_output_bytes: usize,
) -> Result<GatewayModelCatalog, String> {
    let mut child = Command::new(program.as_ref())
        .args(["chat", "--list-models", "--format", "json"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("failed to start Kiro CLI model discovery: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Kiro CLI stdout unavailable".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Kiro CLI stderr unavailable".to_string())?;

    let result = tokio::time::timeout(timeout, async move {
        let stdout_task = async {
            let mut bytes = Vec::new();
            stdout
                .take((max_output_bytes + 1) as u64)
                .read_to_end(&mut bytes)
                .await
                .map(|_| bytes)
        };
        let stderr_task = async {
            let mut bytes = Vec::new();
            stderr
                .take(8193)
                .read_to_end(&mut bytes)
                .await
                .map(|_| bytes)
        };
        let (stdout, stderr, status) = tokio::try_join!(stdout_task, stderr_task, child.wait())?;
        Ok::<_, std::io::Error>((stdout, stderr, status))
    })
    .await
    .map_err(|_| "Kiro CLI model discovery timed out".to_string())?
    .map_err(|error| format!("failed to read Kiro CLI model discovery: {error}"))?;

    let (stdout, _stderr, status) = result;
    if stdout.len() > max_output_bytes {
        return Err("Kiro CLI model discovery output exceeded the size limit".to_string());
    }
    if !status.success() {
        return Err(format!(
            "Kiro CLI model discovery failed with status {}",
            status
                .code()
                .map_or_else(|| "unknown".to_string(), |code| code.to_string())
        ));
    }
    parse_kiro_model_catalog(&stdout, Utc::now())
}

pub fn load_cached_catalog(path: &Path) -> Result<GatewayModelCatalog, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read model catalog cache: {error}"))?;
    let catalog: GatewayModelCatalog = serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse model catalog cache: {error}"))?;
    if catalog.schema_version != MODEL_CATALOG_SCHEMA_VERSION || catalog.models.is_empty() {
        return Err("model catalog cache has an unsupported schema or no models".to_string());
    }
    Ok(catalog)
}

pub fn save_catalog_atomic(path: &Path, catalog: &GatewayModelCatalog) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "model catalog cache path has no parent".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create model catalog cache directory: {error}"))?;
    let temporary = temporary_catalog_path(path);
    let bytes = serde_json::to_vec_pretty(catalog)
        .map_err(|error| format!("failed to serialize model catalog cache: {error}"))?;
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    let mut file = options
        .open(&temporary)
        .map_err(|error| format!("failed to create model catalog cache: {error}"))?;
    use std::io::Write;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("failed to write model catalog cache: {error}"))?;
    std::fs::rename(&temporary, path)
        .map_err(|error| format!("failed to replace model catalog cache: {error}"))
}

fn temporary_catalog_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

fn owner_for_model(id: &str) -> &'static str {
    if id.starts_with("gpt-") {
        "openai"
    } else if id.starts_with("claude-") {
        "anthropic"
    } else {
        "kiro"
    }
}

fn compatibility_for_model(id: &str) -> ModelClientCompatibility {
    if id.starts_with("gpt-5.6-") {
        ModelClientCompatibility {
            claude_code: false,
            codex_cli: true,
        }
    } else if id.starts_with("claude-")
        || matches!(
            id,
            "deepseek-3.2" | "glm-5" | "minimax-m2.1" | "minimax-m2.5" | "qwen3-coder-next"
        )
    {
        ModelClientCompatibility {
            claude_code: true,
            codex_cli: false,
        }
    } else {
        ModelClientCompatibility::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn executable_script(contents: &str) -> (tempfile::TempDir, PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("fake-kiro-cli");
        std::fs::write(&path, contents).unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&path, permissions).unwrap();
        (temp, path)
    }

    #[test]
    fn parses_new_kiro_models_without_collapsing_ids() {
        let output = br#"{"models":[{"model_name":"claude-sonnet-5","model_id":"claude-sonnet-5","context_window_tokens":1000000,"rate_multiplier":1.3,"rate_unit":"Credit"},{"model_name":"gpt-5.6-sol","model_id":"gpt-5.6-sol","context_window_tokens":272000,"rate_multiplier":2.4,"rate_unit":"Credit"},{"model_name":"gpt-5.6-terra","model_id":"gpt-5.6-terra"},{"model_name":"gpt-5.6-luna","model_id":"gpt-5.6-luna"}],"default_model":"claude-sonnet-5","future":true}"#;
        let catalog = parse_kiro_model_catalog(output, Utc::now()).unwrap();
        assert_eq!(catalog.models.len(), 4);
        assert!(
            catalog
                .model("claude-sonnet-5")
                .unwrap()
                .compatibility
                .claude_code
        );
        assert!(
            catalog
                .model("gpt-5.6-sol")
                .unwrap()
                .compatibility
                .codex_cli
        );
        assert!(catalog.model("gpt-5.6-terra").is_some());
        assert!(catalog.model("gpt-5.6-luna").is_some());
    }

    #[test]
    fn rejects_empty_and_invalid_json() {
        assert!(parse_kiro_model_catalog(br#"{"models":[]}"#, Utc::now()).is_err());
        assert!(parse_kiro_model_catalog(b"not-json", Utc::now()).is_err());
    }

    #[test]
    fn cache_round_trip_and_corruption_fallback_are_explicit() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("gateway-models.json");
        let catalog = GatewayModelCatalog::fallback();
        save_catalog_atomic(&path, &catalog).unwrap();
        assert_eq!(load_cached_catalog(&path).unwrap(), catalog);
        std::fs::write(&path, b"broken").unwrap();
        assert!(load_cached_catalog(&path).is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn discovery_uses_program_arguments_and_parses_json() {
        let (_temp, path) = executable_script(
            "#!/bin/sh\nprintf '%s' '{\"models\":[{\"model_name\":\"claude-sonnet-5\",\"model_id\":\"claude-sonnet-5\"}]}'\n",
        );
        let catalog = discover_kiro_model_catalog(&path, Duration::from_secs(5), 4096)
            .await
            .unwrap();
        assert!(catalog.model("claude-sonnet-5").is_some());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn discovery_enforces_timeout_and_output_limit() {
        let (_temp, slow) = executable_script("#!/bin/sh\nsleep 2\n");
        let error = discover_kiro_model_catalog(&slow, Duration::from_millis(10), 4096)
            .await
            .unwrap_err();
        assert!(error.contains("timed out"));

        let (_temp, large) = executable_script("#!/bin/sh\nprintf '123456789'\n");
        let error = discover_kiro_model_catalog(&large, Duration::from_secs(5), 4)
            .await
            .unwrap_err();
        assert!(error.contains("size limit"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn discovery_error_does_not_echo_stderr() {
        let (_temp, path) = executable_script("#!/bin/sh\necho 'secret-token-value' >&2\nexit 7\n");
        let error = discover_kiro_model_catalog(&path, Duration::from_secs(5), 4096)
            .await
            .unwrap_err();
        assert!(error.contains("status 7"));
        assert!(!error.contains("secret-token-value"));
    }
}
