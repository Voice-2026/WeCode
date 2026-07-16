//! Lifecycle wrapper that runs the embedded `wecode-gateway` (OpenAI/Anthropic
//! API backed by Kiro) on the shared WeCode async runtime.
//!
//! The gateway configuration lives in `settings.json` under the `"gateway"` key:
//!
//! ```json
//! {
//!   "gateway": {
//!     "enabled": true,
//!     "config": {
//!       "host": "127.0.0.1",
//!       "port": 8989,
//!       "api_key": "…",
//!       "credentials": { "source": "kiro-cli" }
//!     }
//!   }
//! }
//! ```
//!
//! A settings pane (`apps/desktop/src/app/settings/panes/gateway/`) can read and
//! write this section via [`ConfigStore`] and call [`GatewayService::restart`]
//! when the user toggles or edits the configuration.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::OnceLock;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
pub use wecode_gateway::GatewayConfig;
pub use wecode_gateway::auth::kiro_app_credentials_path;
pub use wecode_gateway::config::CredentialSource;
pub use wecode_gateway::model_catalog::{GatewayModel, GatewayModelCatalog};

const MODEL_CATALOG_FILE: &str = "gateway-models.json";
const MODEL_DISCOVERY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

static MODEL_CATALOG_REFRESH: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
static CURRENT_MODEL_CATALOG: OnceLock<Mutex<GatewayModelCatalog>> = OnceLock::new();

use crate::async_runtime;
use crate::config::ConfigStore;

static GATEWAY_RUNTIME_STATUS: OnceLock<Mutex<GatewayRuntimeStatus>> = OnceLock::new();

/// The `"gateway"` section of `settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub config: GatewayConfig,
    #[serde(default = "default_gateway_claude_model")]
    pub default_claude_model: String,
    #[serde(default = "default_gateway_codex_model")]
    pub default_codex_model: String,
}

fn default_gateway_claude_model() -> String {
    "claude-sonnet-5".to_string()
}

fn default_gateway_codex_model() -> String {
    "gpt-5.6-terra".to_string()
}

impl Default for GatewaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            config: GatewayConfig::default(),
            default_claude_model: default_gateway_claude_model(),
            default_codex_model: default_gateway_codex_model(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GatewayRuntimeStatus {
    pub enabled: bool,
    pub addr: Option<SocketAddr>,
    pub error: Option<String>,
}

fn gateway_status_cell() -> &'static Mutex<GatewayRuntimeStatus> {
    GATEWAY_RUNTIME_STATUS.get_or_init(|| Mutex::new(GatewayRuntimeStatus::default()))
}

fn set_gateway_status(status: GatewayRuntimeStatus) {
    *gateway_status_cell().lock() = status;
}

impl GatewaySettings {
    pub fn load(support_dir: impl Into<PathBuf>) -> Self {
        ConfigStore::for_settings_dir(support_dir)
            .get_as::<GatewaySettings>("gateway")
            .unwrap_or_default()
    }

    pub fn save(&self, support_dir: impl Into<PathBuf>) -> Result<(), String> {
        ConfigStore::for_settings_dir(support_dir).set_as("gateway", self)
    }
}

pub fn gateway_model_catalog_path(support_dir: impl Into<PathBuf>) -> PathBuf {
    support_dir.into().join(MODEL_CATALOG_FILE)
}

pub fn load_gateway_model_catalog(support_dir: impl Into<PathBuf>) -> GatewayModelCatalog {
    let path = gateway_model_catalog_path(support_dir);
    let catalog = wecode_gateway::model_catalog::load_cached_catalog(&path).unwrap_or_default();
    *CURRENT_MODEL_CATALOG
        .get_or_init(|| Mutex::new(catalog.clone()))
        .lock() = catalog.clone();
    catalog
}

pub fn current_gateway_model_catalog() -> GatewayModelCatalog {
    CURRENT_MODEL_CATALOG
        .get_or_init(|| Mutex::new(GatewayModelCatalog::fallback()))
        .lock()
        .clone()
}

pub async fn refresh_gateway_model_catalog(
    support_dir: impl Into<PathBuf>,
) -> Result<GatewayModelCatalog, String> {
    let _guard = MODEL_CATALOG_REFRESH
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await;
    let catalog = wecode_gateway::model_catalog::discover_kiro_model_catalog(
        "kiro-cli",
        MODEL_DISCOVERY_TIMEOUT,
        wecode_gateway::model_catalog::MODEL_DISCOVERY_MAX_OUTPUT_BYTES,
    )
    .await?;
    let path = gateway_model_catalog_path(support_dir);
    wecode_gateway::model_catalog::save_catalog_atomic(&path, &catalog)?;
    *CURRENT_MODEL_CATALOG
        .get_or_init(|| Mutex::new(catalog.clone()))
        .lock() = catalog.clone();
    Ok(catalog)
}

pub fn gateway_claude_environment(
    base_url: &str,
    api_key: &str,
    model: &str,
) -> HashMap<String, String> {
    HashMap::from([
        ("WECODE_KIRO_GATEWAY".to_string(), "1".to_string()),
        ("WECODE_KIRO_GATEWAY_MODEL".to_string(), model.to_string()),
        ("WECODE_AI_AGENT_ID".to_string(), "claude".to_string()),
        ("WECODE_AI_PROVIDER_ID".to_string(), "kiro".to_string()),
        ("WECODE_AI_PROVIDER_NAME".to_string(), "Kiro".to_string()),
        ("WECODE_AI_MODEL_ID".to_string(), model.to_string()),
        ("ANTHROPIC_API_KEY".to_string(), api_key.to_string()),
        ("ANTHROPIC_BASE_URL".to_string(), base_url.to_string()),
    ])
}

pub fn gateway_claude_command(model: &str, resume_id: Option<&str>) -> String {
    let cli_model = model.replace('.', "-");
    let mut command = format!(
        "claude --permission-mode bypassPermissions --model {}",
        shell_quote(&cli_model)
    );
    if let Some(resume_id) = resume_id.filter(|value| !value.trim().is_empty()) {
        command.push_str(" --resume ");
        command.push_str(&shell_quote(resume_id));
    }
    command
}

pub fn gateway_codex_environment(api_key: &str, model: &str) -> HashMap<String, String> {
    HashMap::from([
        ("WECODE_KIRO_GATEWAY".to_string(), "1".to_string()),
        ("WECODE_KIRO_GATEWAY_MODEL".to_string(), model.to_string()),
        ("WECODE_AI_AGENT_ID".to_string(), "codex".to_string()),
        ("WECODE_AI_PROVIDER_ID".to_string(), "kiro".to_string()),
        ("WECODE_AI_PROVIDER_NAME".to_string(), "Kiro".to_string()),
        ("WECODE_AI_MODEL_ID".to_string(), model.to_string()),
        (
            "WECODE_KIRO_GATEWAY_API_KEY".to_string(),
            api_key.to_string(),
        ),
    ])
}

pub fn gateway_codex_command(model: &str, base_url: &str, context_window_tokens: u64) -> String {
    let mut command = format!(
        "codex --model {} -c {} -c {} -c {} -c {} -c {} -c {} -c {}",
        shell_quote(model),
        shell_quote("model_provider=\"wecode-kiro\""),
        shell_quote("model_providers.wecode-kiro.name=\"Kiro\""),
        shell_quote(&format!(
            "model_providers.wecode-kiro.base_url=\"{base_url}\""
        )),
        shell_quote("model_providers.wecode-kiro.env_key=\"WECODE_KIRO_GATEWAY_API_KEY\""),
        shell_quote("model_providers.wecode-kiro.wire_api=\"responses\""),
        shell_quote("model_providers.wecode-kiro.requires_openai_auth=false"),
        shell_quote("check_for_update_on_startup=false"),
    );
    if context_window_tokens > 0 {
        command.push_str(" -c ");
        command.push_str(&shell_quote(&format!(
            "model_context_window={context_window_tokens}"
        )));
    }
    command.push_str(" -c ");
    command.push_str(&shell_quote("service_tier=\"default\""));
    command
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Owns a running gateway instance. Dropping it (or calling [`stop`]) shuts the
/// server down gracefully.
pub struct GatewayService {
    stop_tx: Mutex<Option<oneshot::Sender<()>>>,
    addr: Arc<Mutex<Option<SocketAddr>>>,
}

impl GatewayService {
    pub fn inactive() -> Arc<Self> {
        Arc::new(Self {
            stop_tx: Mutex::new(None),
            addr: Arc::new(Mutex::new(None)),
        })
    }

    /// Start the gateway from the `settings.json` in `support_dir`. Returns a
    /// handle even when disabled (in which case no server is bound).
    pub fn start_from_support_dir(support_dir: impl Into<PathBuf>) -> Arc<Self> {
        let support_dir = support_dir.into();
        let settings = GatewaySettings::load(support_dir.clone());
        let catalog = load_gateway_model_catalog(support_dir);
        Self::start_with_catalog(settings, catalog)
    }

    /// Start the gateway from an explicit settings value.
    pub fn start(settings: GatewaySettings) -> Arc<Self> {
        Self::start_with_catalog(settings, GatewayModelCatalog::fallback())
    }

    pub fn start_with_catalog(
        mut settings: GatewaySettings,
        catalog: GatewayModelCatalog,
    ) -> Arc<Self> {
        let (stop_tx, stop_rx) = oneshot::channel();
        let addr = Arc::new(Mutex::new(None));
        let service = Arc::new(Self {
            stop_tx: Mutex::new(Some(stop_tx)),
            addr: addr.clone(),
        });
        apply_runtime_model_aliases(&mut settings.config);

        set_gateway_status(GatewayRuntimeStatus {
            enabled: settings.enabled,
            addr: None,
            error: None,
        });

        if settings.enabled {
            async_runtime::spawn(async move {
                match wecode_gateway::start_with_model_catalog(settings.config, catalog).await {
                    Ok(handle) => {
                        let local_addr = handle.local_addr();
                        *addr.lock() = Some(local_addr);
                        set_gateway_status(GatewayRuntimeStatus {
                            enabled: true,
                            addr: Some(local_addr),
                            error: None,
                        });
                        // Run until asked to stop.
                        let _ = stop_rx.await;
                        handle.shutdown().await;
                    }
                    Err(e) => {
                        set_gateway_status(GatewayRuntimeStatus {
                            enabled: true,
                            addr: None,
                            error: Some(e.to_string()),
                        });
                        eprintln!("[wecode-gateway] failed to start: {e}");
                    }
                }
            });
        }

        service
    }

    /// The bound socket address once the server is listening, if enabled.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        *self.addr.lock()
    }

    pub fn global_status() -> GatewayRuntimeStatus {
        gateway_status_cell().lock().clone()
    }

    /// Signal the running server to shut down.
    pub fn stop(&self) {
        if let Some(tx) = self.stop_tx.lock().take() {
            let _ = tx.send(());
        }
        set_gateway_status(GatewayRuntimeStatus::default());
    }

    /// Stop this instance and start a fresh one from `settings.json`.
    pub fn restart_from_support_dir(&self, support_dir: impl Into<PathBuf>) -> Arc<Self> {
        self.stop();
        Self::start_from_support_dir(support_dir)
    }
}

fn apply_runtime_model_aliases(config: &mut wecode_gateway::GatewayConfig) {
    for (alias, target) in [
        ("opus", "claude-opus-4.8"),
        ("claude-opus-4", "claude-opus-4.8"),
        ("claude-opus-4-8", "claude-opus-4.8"),
    ] {
        config
            .model_aliases
            .entry(alias.to_string())
            .or_insert_with(|| target.to_string());
    }
}

impl Drop for GatewayService {
    fn drop(&mut self) {
        if let Some(tx) = self.stop_tx.lock().take() {
            let _ = tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{gateway_codex_command, gateway_codex_environment};

    #[test]
    fn codex_gateway_command_disables_interactive_update_prompt() {
        let command = gateway_codex_command("gpt-5.6-terra", "http://127.0.0.1:18989/v1", 272_000);
        assert!(command.contains("check_for_update_on_startup=false"));
        assert!(command.contains("model_provider=\"wecode-kiro\""));
        assert!(command.contains("model_providers.wecode-kiro.name=\"Kiro\""));
        assert!(command.starts_with("codex --model gpt-5.6-terra"));
        assert!(command.contains("service_tier=\"default\""));

        let env = gateway_codex_environment("secret", "gpt-5.6-terra");
        assert_eq!(env["WECODE_AI_AGENT_ID"], "codex");
        assert_eq!(env["WECODE_AI_PROVIDER_ID"], "kiro");
        assert_eq!(env["WECODE_AI_PROVIDER_NAME"], "Kiro");
        assert_eq!(env["WECODE_AI_MODEL_ID"], "gpt-5.6-terra");
    }
}
