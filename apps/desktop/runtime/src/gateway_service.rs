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
