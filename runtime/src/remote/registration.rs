use super::crypto::{ensure_remote_host_identity, remote_random_token};
use super::http::{default_remote_server_url, remote_post_blocking, remote_server_url};
use super::summary::remote_summary_from_settings;
use super::types::{RemoteSettings, RemoteSummary};
use super::{RemoteService, remote_settings_from_raw, remote_settings_mut};
use serde::Deserialize;
use serde_json::{Map, Value, json};

impl RemoteService {
    pub fn set_enabled(&self, enabled: bool) -> Result<RemoteSummary, String> {
        let mut raw = self.raw_settings();
        let remote = remote_settings_mut(&mut raw)?;
        remote.insert("isEnabled".to_string(), Value::Bool(enabled));
        if !remote.contains_key("serverURL") {
            remote.insert(
                "serverURL".to_string(),
                Value::String(default_remote_server_url()),
            );
        }
        self.save_raw_settings(&raw)?;
        Ok(self.summary())
    }

    pub fn set_server_url(&self, server_url: &str) -> Result<RemoteSummary, String> {
        let mut raw = self.raw_settings();
        let remote = remote_settings_mut(&mut raw)?;
        remote.insert(
            "serverURL".to_string(),
            Value::String(server_url.trim().chars().take(512).collect()),
        );
        self.save_raw_settings(&raw)?;
        Ok(self.summary())
    }

    pub fn register_host(&self) -> Result<RemoteSummary, String> {
        let mut raw = self.raw_settings();
        let settings = self.register_host_in_raw(&mut raw)?;
        self.save_raw_settings(&raw)?;
        Ok(remote_summary_from_settings(settings))
    }

    pub fn reconnect(&self) -> Result<RemoteSummary, String> {
        self.register_host()?;
        self.refresh_devices()
    }

    pub(super) fn register_host_in_raw(
        &self,
        raw: &mut Map<String, Value>,
    ) -> Result<RemoteSettings, String> {
        let mut settings = remote_settings_from_raw(raw);
        if !settings.is_enabled {
            return Ok(settings);
        }
        if settings.host_id.trim().is_empty() {
            settings.host_id = uuid::Uuid::new_v4().to_string();
        }
        if settings.host_token.trim().is_empty() {
            settings.host_token = remote_random_token();
        }
        ensure_remote_host_identity(&mut settings);

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RegisterResponse {
            host_id: String,
            token: String,
        }

        let response = remote_post_blocking::<RegisterResponse>(
            &remote_server_url(&settings),
            "/api/hosts/register",
            json!({
                "hostId": settings.host_id,
                "name": super::crypto::remote_host_name(),
                "token": settings.host_token,
                "publicKey": settings.host_public_key,
            }),
        )?;
        settings.host_id = response.host_id;
        settings.host_token = response.token;
        raw.insert(
            "remote".to_string(),
            serde_json::to_value(&settings).map_err(|error| error.to_string())?,
        );
        Ok(settings)
    }
}
