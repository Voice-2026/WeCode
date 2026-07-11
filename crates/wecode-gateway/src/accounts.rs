//! Multi-account manager with sticky selection + circuit-breaker failover.
//! Faithful port of account_manager.py (simplified: the runtime endpoint uses
//! the static FALLBACK_MODELS list, so no per-account model fetching is needed).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use rand::Rng;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::auth::KiroAuth;
use crate::config::{AccountSettings, GatewayConfig};
use crate::error::GatewayError;
use crate::upstream::request_kiro;

/// FATAL → return to client; RECOVERABLE → try next account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    Fatal,
    Recoverable,
}

/// Classify a Kiro API error for failover, matching account_errors.py.
pub fn classify_error(status: u16, reason: Option<&str>) -> ErrorClass {
    match status {
        402 | 403 | 429 => ErrorClass::Recoverable,
        400 => match reason {
            Some("INVALID_MODEL_ID") => ErrorClass::Recoverable,
            _ => ErrorClass::Fatal,
        },
        422 => ErrorClass::Fatal,
        s if (500..600).contains(&s) => ErrorClass::Fatal,
        _ => ErrorClass::Fatal,
    }
}

/// Extract the Kiro error `reason` from a JSON error body, if present.
pub fn extract_reason(body: &str) -> Option<String> {
    let v: Value = serde_json::from_str(body).ok()?;
    v.get("reason").and_then(Value::as_str).map(str::to_string)
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
struct Stats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
}

#[derive(Default)]
struct Runtime {
    failures: u32,
    last_failure_time: f64,
    stats: Stats,
}

struct Slot {
    id: String,
    auth: Arc<KiroAuth>,
}

struct ManagerState {
    current_index: usize,
    per: Vec<Runtime>,
}

pub struct AccountManager {
    slots: Vec<Slot>,
    state_file: Option<PathBuf>,
    settings: AccountSettings,
    state: Mutex<ManagerState>,
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

impl AccountManager {
    pub fn from_config(
        config: &GatewayConfig,
        http: reqwest::Client,
    ) -> Result<Arc<Self>, GatewayError> {
        let mut slots = Vec::new();

        if config.accounts.is_empty() {
            // Single-account mode.
            let auth = KiroAuth::from_config(config, http)?;
            slots.push(Slot {
                id: "default".to_string(),
                auth,
            });
        } else {
            for (i, entry) in config.accounts.iter().enumerate() {
                if !entry.enabled {
                    continue;
                }
                let region = entry
                    .region
                    .clone()
                    .unwrap_or_else(|| config.region.clone());
                let auth = KiroAuth::new(
                    entry.credentials.clone(),
                    region,
                    entry.api_region.clone(),
                    config.token_refresh_threshold_secs,
                    http.clone(),
                )?;
                slots.push(Slot {
                    id: format!("account-{i}"),
                    auth,
                });
            }
            if slots.is_empty() {
                return Err(GatewayError::Internal(
                    "no enabled accounts in configuration".into(),
                ));
            }
        }

        let per = (0..slots.len()).map(|_| Runtime::default()).collect();
        let mut state = ManagerState {
            current_index: 0,
            per,
        };

        let state_file = config.state_file.clone();
        if let Some(path) = &state_file {
            load_state(path, &mut state, &slots);
        }

        Ok(Arc::new(Self {
            slots,
            state_file,
            settings: config.account_settings.clone(),
            state: Mutex::new(state),
        }))
    }

    pub fn is_single(&self) -> bool {
        self.slots.len() == 1
    }

    /// Profile ARN of the currently-sticky account (for /v1/models etc).
    pub async fn any_profile_arn(&self) -> Option<String> {
        let idx = self
            .state
            .lock()
            .await
            .current_index
            .min(self.slots.len() - 1);
        self.slots[idx].auth.profile_arn().await
    }

    /// Authentication manager for native server-tool requests that bypass the
    /// normal model failover path.
    pub async fn current_auth(&self) -> Arc<KiroAuth> {
        let idx = self
            .state
            .lock()
            .await
            .current_index
            .min(self.slots.len() - 1);
        self.slots[idx].auth.clone()
    }

    /// Select the next account index to try, honoring the circuit breaker.
    async fn select(&self, exclude: &HashSet<usize>) -> Option<usize> {
        let st = self.state.lock().await;

        // Single account: bypass the breaker entirely.
        if self.slots.len() == 1 {
            if exclude.contains(&0) {
                return None;
            }
            return Some(0);
        }

        let n = self.slots.len();
        let start = st.current_index;
        for i in 0..n {
            let idx = (start + i) % n;
            if exclude.contains(&idx) {
                continue;
            }
            let rt = &st.per[idx];
            if rt.failures > 0 {
                let elapsed = now() - rt.last_failure_time;
                let mult =
                    (2f64.powi((rt.failures - 1) as i32)).min(self.settings.max_backoff_multiplier);
                let effective = self.settings.recovery_timeout_secs * mult;
                if elapsed < effective {
                    // Still cooling down: probabilistic retry.
                    let roll: f64 = rand::thread_rng().gen();
                    if roll > self.settings.probabilistic_retry_chance {
                        continue;
                    }
                }
            }
            return Some(idx);
        }
        None
    }

    async fn report_success(&self, idx: usize) {
        let mut st = self.state.lock().await;
        st.per[idx].failures = 0;
        st.per[idx].stats.total_requests += 1;
        st.per[idx].stats.successful_requests += 1;
        st.current_index = idx;
        drop(st);
        self.persist().await;
    }

    async fn report_failure(&self, idx: usize, class: ErrorClass, reason: Option<&str>) {
        let mut st = self.state.lock().await;
        // INVALID_MODEL_ID is model discovery, not an account fault.
        if reason == Some("INVALID_MODEL_ID") {
            st.per[idx].stats.total_requests += 1;
            drop(st);
            self.persist().await;
            return;
        }
        if class == ErrorClass::Recoverable {
            st.per[idx].failures += 1;
            st.per[idx].last_failure_time = now();
        }
        st.per[idx].stats.total_requests += 1;
        st.per[idx].stats.failed_requests += 1;
        drop(st);
        self.persist().await;
    }

    async fn persist(&self) {
        if self.slots.len() <= 1 {
            return;
        }
        let Some(path) = &self.state_file else { return };
        let st = self.state.lock().await;
        save_state(path, &st, &self.slots);
    }

    /// Run a request through the failover loop. `payload_builder` receives the
    /// selected account's profile ARN and returns the Kiro payload.
    pub async fn request_with_failover<F>(
        self: &Arc<Self>,
        payload_builder: F,
        config: &GatewayConfig,
    ) -> Result<(Arc<KiroAuth>, reqwest::Response), GatewayError>
    where
        F: Fn(Option<String>) -> Result<Value, GatewayError>,
    {
        // Single-account: no breaker, surface real errors.
        if self.is_single() {
            let auth = self.slots[0].auth.clone();
            let profile_arn = auth.profile_arn().await;
            let payload = payload_builder(profile_arn)?;
            let resp = request_kiro(&auth, &payload, config).await?;
            return Ok((auth, resp));
        }

        let max_attempts = self.slots.len() * 2;
        let mut tried: HashSet<usize> = HashSet::new();
        let mut last_err: Option<GatewayError> = None;

        for _ in 0..max_attempts {
            let Some(idx) = self.select(&tried).await else {
                break;
            };
            tried.insert(idx);
            let auth = self.slots[idx].auth.clone();
            let profile_arn = auth.profile_arn().await;
            let payload = match payload_builder(profile_arn) {
                Ok(p) => p,
                Err(e) => return Err(e),
            };

            match request_kiro(&auth, &payload, config).await {
                Ok(resp) => {
                    self.report_success(idx).await;
                    return Ok((auth, resp));
                }
                Err(GatewayError::Upstream { status, body }) => {
                    let reason = extract_reason(&body);
                    let class = classify_error(status, reason.as_deref());
                    self.report_failure(idx, class, reason.as_deref()).await;
                    if class == ErrorClass::Fatal {
                        return Err(GatewayError::Upstream { status, body });
                    }
                    last_err = Some(GatewayError::Upstream { status, body });
                }
                Err(e) => {
                    // Network errors are recoverable → try next account.
                    self.report_failure(idx, ErrorClass::Recoverable, None)
                        .await;
                    last_err = Some(e);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| GatewayError::Upstream {
            status: 502,
            body: "all accounts unavailable".into(),
        }))
    }
}

// ---------- state.json persistence ----------

fn load_state(path: &std::path::Path, state: &mut ManagerState, slots: &[Slot]) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let Ok(data): Result<Value, _> = serde_json::from_str(&text) else {
        return;
    };
    if let Some(i) = data.get("current_account_index").and_then(Value::as_u64) {
        state.current_index = (i as usize).min(slots.len().saturating_sub(1));
    }
    if let Some(accounts) = data.get("accounts").and_then(Value::as_object) {
        for (i, slot) in slots.iter().enumerate() {
            if let Some(a) = accounts.get(&slot.id) {
                state.per[i].failures =
                    a.get("failures").and_then(Value::as_u64).unwrap_or(0) as u32;
                state.per[i].last_failure_time = a
                    .get("last_failure_time")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                if let Some(s) = a.get("stats") {
                    state.per[i].stats = serde_json::from_value(s.clone()).unwrap_or_default();
                }
            }
        }
    }
}

fn save_state(path: &std::path::Path, state: &ManagerState, slots: &[Slot]) {
    let mut accounts = serde_json::Map::new();
    for (i, slot) in slots.iter().enumerate() {
        let rt = &state.per[i];
        accounts.insert(
            slot.id.clone(),
            json!({
                "failures": rt.failures,
                "last_failure_time": rt.last_failure_time,
                "stats": {
                    "total_requests": rt.stats.total_requests,
                    "successful_requests": rt.stats.successful_requests,
                    "failed_requests": rt.stats.failed_requests,
                }
            }),
        );
    }
    let data = json!({
        "current_account_index": state.current_index,
        "accounts": Value::Object(accounts),
    });
    let tmp = path.with_extension("json.tmp");
    if serde_json::to_string_pretty(&data)
        .ok()
        .and_then(|s| std::fs::write(&tmp, s).ok())
        .is_some()
    {
        let _ = std::fs::rename(&tmp, path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_errors() {
        assert_eq!(classify_error(402, None), ErrorClass::Recoverable);
        assert_eq!(classify_error(403, None), ErrorClass::Recoverable);
        assert_eq!(classify_error(429, None), ErrorClass::Recoverable);
        assert_eq!(
            classify_error(400, Some("INVALID_MODEL_ID")),
            ErrorClass::Recoverable
        );
        assert_eq!(
            classify_error(400, Some("CONTENT_LENGTH_EXCEEDS_THRESHOLD")),
            ErrorClass::Fatal
        );
        assert_eq!(classify_error(400, None), ErrorClass::Fatal);
        assert_eq!(classify_error(422, None), ErrorClass::Fatal);
        assert_eq!(classify_error(500, None), ErrorClass::Fatal);
    }
}
