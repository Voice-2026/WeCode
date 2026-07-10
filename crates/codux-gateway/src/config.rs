use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    8989
}
fn default_api_key() -> String {
    "my-super-secret-password-123".to_string()
}
fn default_region() -> String {
    "us-east-1".to_string()
}
fn default_token_refresh_threshold() -> u64 {
    600
}
fn default_max_retries() -> u32 {
    3
}
fn default_base_retry_delay() -> f64 {
    1.0
}
fn default_first_token_timeout() -> f64 {
    15.0
}
fn default_first_token_max_retries() -> u32 {
    3
}
fn default_streaming_read_timeout() -> f64 {
    300.0
}
fn default_tool_description_max_length() -> usize {
    10_000
}
fn default_max_payload_bytes() -> usize {
    600_000
}
fn default_fake_reasoning_max_tokens() -> u32 {
    4000
}
fn default_fake_reasoning_budget_cap() -> u32 {
    10_000
}
fn default_model_aliases() -> HashMap<String, String> {
    HashMap::from([
        ("auto-kiro".to_string(), "auto".to_string()),
        ("opus".to_string(), "claude-opus-4.8".to_string()),
        ("claude-opus-4".to_string(), "claude-opus-4.8".to_string()),
        ("claude-opus-4-8".to_string(), "claude-opus-4.8".to_string()),
    ])
}
fn default_hidden_from_list() -> Vec<String> {
    vec!["auto".to_string()]
}
fn default_max_input_tokens() -> u64 {
    200_000
}

/// Where the gateway loads Kiro credentials from (single-account, phases 1-4).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "kebab-case")]
pub enum CredentialSource {
    /// JSON credentials file (Kiro IDE style: refreshToken/accessToken/profileArn/...).
    File { path: PathBuf },
    /// kiro-cli SQLite database (auth_kv table).
    KiroCli {
        path: Option<PathBuf>,
        #[serde(default)]
        readonly: bool,
    },
    /// Raw refresh token (Kiro Desktop auth).
    RefreshToken {
        refresh_token: String,
        profile_arn: Option<String>,
        region: Option<String>,
    },
}

impl Default for CredentialSource {
    fn default() -> Self {
        CredentialSource::KiroCli {
            path: None,
            readonly: false,
        }
    }
}

/// One account in multi-account mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountEntry {
    #[serde(flatten)]
    pub credentials: CredentialSource,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub api_region: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Circuit-breaker / failover tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AccountSettings {
    /// Base recovery timeout for exponential backoff (seconds).
    pub recovery_timeout_secs: f64,
    /// Cap for the backoff multiplier (60s * cap = max cooldown).
    pub max_backoff_multiplier: f64,
    /// Probability (0..1) of retrying a cooled-down account anyway.
    pub probabilistic_retry_chance: f64,
    /// Interval for periodic state.json saves (seconds).
    pub state_save_interval_secs: u64,
}

impl Default for AccountSettings {
    fn default() -> Self {
        Self {
            recovery_timeout_secs: 60.0,
            max_backoff_multiplier: 1440.0,
            probabilistic_retry_chance: 0.1,
            state_save_interval_secs: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewayConfig {
    pub host: String,
    pub port: u16,
    /// API key clients must present (Authorization: Bearer or x-api-key).
    pub api_key: String,
    /// Single-account credentials (used when `accounts` is empty).
    pub credentials: CredentialSource,
    /// Multi-account credentials. When non-empty, enables the failover system.
    pub accounts: Vec<AccountEntry>,
    /// Path to the failover state file (state.json).
    pub state_file: Option<PathBuf>,
    pub account_settings: AccountSettings,
    /// SSO region for token refresh endpoints.
    pub region: String,
    /// Override for the Q/runtime API region (otherwise auto-detected).
    pub api_region: Option<String>,

    pub token_refresh_threshold_secs: u64,
    pub max_retries: u32,
    pub base_retry_delay_secs: f64,
    pub first_token_timeout_secs: f64,
    pub first_token_max_retries: u32,
    pub streaming_read_timeout_secs: f64,

    pub tool_description_max_length: usize,
    pub max_payload_bytes: usize,
    pub auto_trim_payload: bool,

    /// Inject <thinking_mode> tags ("fake reasoning"). Off until the streaming
    /// thinking parser lands (phase 6) so clients never see raw tags.
    pub fake_reasoning: bool,
    pub fake_reasoning_max_tokens: u32,
    pub fake_reasoning_budget_cap: u32,
    /// How to surface `<thinking>` blocks: "as_reasoning_content" | "strip_tags" |
    /// "remove" | "pass".
    pub fake_reasoning_handling: String,

    /// Inject synthetic notices when a prior response was truncated by Kiro.
    pub truncation_recovery: bool,

    /// Auto-inject a `web_search` tool and intercept its calls via Kiro's MCP API.
    pub web_search_enabled: bool,

    pub model_aliases: HashMap<String, String>,
    pub hidden_models: HashMap<String, String>,
    pub hidden_from_list: Vec<String>,
    pub default_max_input_tokens: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            api_key: default_api_key(),
            credentials: CredentialSource::default(),
            accounts: Vec::new(),
            state_file: None,
            account_settings: AccountSettings::default(),
            region: default_region(),
            api_region: None,
            token_refresh_threshold_secs: default_token_refresh_threshold(),
            max_retries: default_max_retries(),
            base_retry_delay_secs: default_base_retry_delay(),
            first_token_timeout_secs: default_first_token_timeout(),
            first_token_max_retries: default_first_token_max_retries(),
            streaming_read_timeout_secs: default_streaming_read_timeout(),
            tool_description_max_length: default_tool_description_max_length(),
            max_payload_bytes: default_max_payload_bytes(),
            auto_trim_payload: false,
            fake_reasoning: false,
            fake_reasoning_max_tokens: default_fake_reasoning_max_tokens(),
            fake_reasoning_budget_cap: default_fake_reasoning_budget_cap(),
            fake_reasoning_handling: "as_reasoning_content".to_string(),
            truncation_recovery: true,
            web_search_enabled: false,
            model_aliases: default_model_aliases(),
            hidden_models: HashMap::new(),
            hidden_from_list: default_hidden_from_list(),
            default_max_input_tokens: default_max_input_tokens(),
        }
    }
}

/// Models known to work on runtime.{region}.kiro.dev (used because the runtime
/// endpoint has no /ListAvailableModels).
pub const FALLBACK_MODELS: &[&str] = &[
    "auto",
    "claude-sonnet-4",
    "claude-sonnet-4.5",
    "claude-sonnet-4.6",
    "claude-haiku-4.5",
    "claude-opus-4.5",
    "claude-opus-4.6",
    "claude-opus-4.7",
    "deepseek-3.2",
    "glm-5",
    "minimax-m2.1",
    "minimax-m2.5",
    "qwen3-coder-next",
];

pub fn kiro_refresh_url(region: &str) -> String {
    format!("https://prod.{region}.auth.desktop.kiro.dev/refreshToken")
}

pub fn aws_sso_oidc_url(region: &str) -> String {
    format!("https://oidc.{region}.amazonaws.com/token")
}

pub fn kiro_api_host(region: &str) -> String {
    format!("https://runtime.{region}.kiro.dev")
}

impl GatewayConfig {
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, crate::error::GatewayError> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            crate::error::GatewayError::Internal(format!(
                "failed to read config {}: {e}",
                path.display()
            ))
        })?;
        serde_json::from_str(&text).map_err(|e| {
            crate::error::GatewayError::Internal(format!(
                "failed to parse config {}: {e}",
                path.display()
            ))
        })
    }
}
