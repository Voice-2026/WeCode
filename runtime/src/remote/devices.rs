use super::http::{
    remote_error_message, remote_http_client, remote_parse_response, remote_post,
    remote_server_url, remote_url,
};
use super::types::{RemoteDeviceSettings, RemoteSummary};
use super::{RemoteService, remote_settings_from_raw, remote_settings_mut};
use serde::Deserialize;
use serde_json::{Value, json};

impl RemoteService {
    pub fn revoke_device(&self, device_id: &str) -> Result<RemoteSummary, String> {
        let device_id = device_id.trim();
        if device_id.is_empty() {
            return Err("Missing device id.".to_string());
        }
        let mut raw = self.raw_settings();
        let settings = remote_settings_from_raw(&raw);
        if settings.host_id.trim().is_empty() || settings.host_token.trim().is_empty() {
            return Err("Remote Host is not registered.".to_string());
        }
        remote_post::<Value>(
            &remote_server_url(&settings),
            "/api/devices/revoke",
            json!({
                "hostId": settings.host_id,
                "token": settings.host_token,
                "deviceId": device_id,
            }),
        )?;

        let remote = remote_settings_mut(&mut raw)?;
        let devices = remote
            .get_mut("cachedDevices")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| "Remote cached devices are not configured.".to_string())?;
        let before_len = devices.len();
        devices.retain(|device| {
            device
                .get("id")
                .and_then(Value::as_str)
                .map(|id| id != device_id)
                .unwrap_or(true)
        });
        if devices.len() == before_len {
            return Err("Remote device not found.".to_string());
        }
        self.save_raw_settings(&raw)?;
        let mut summary = self.summary();
        summary.status = "connected".to_string();
        summary.message = "Device removed.".to_string();
        if let Ok(mut refreshed) = self.refresh_devices() {
            refreshed.status = summary.status;
            refreshed.message = summary.message;
            summary = refreshed;
        }
        Ok(summary)
    }

    pub fn refresh_devices(&self) -> Result<RemoteSummary, String> {
        let mut raw = self.raw_settings();
        let mut settings = remote_settings_from_raw(&raw);
        if settings.host_id.trim().is_empty() {
            settings = self.register_host_in_raw(&mut raw)?;
            self.save_raw_settings(&raw)?;
        }
        let relay = remote_server_url(&settings);
        if relay.trim().is_empty()
            || settings.host_id.trim().is_empty()
            || settings.host_token.trim().is_empty()
        {
            return Ok(super::remote_summary_from_settings(settings));
        }

        #[derive(Deserialize)]
        struct DeviceList {
            devices: Vec<RemoteDeviceSettings>,
        }

        let path = format!("/api/hosts/{}/devices", settings.host_id);
        let url = remote_url(
            &relay,
            &path,
            &[("token", settings.host_token.as_str())],
            false,
        )?;
        let response = remote_http_client()?
            .get(url)
            .send()
            .map_err(remote_error_message)?;
        let mut list = remote_parse_response::<DeviceList>(response)?;
        list.devices.retain(|device| device.revoked_at.is_none());
        let devices = list
            .devices
            .into_iter()
            .map(|mut device| {
                device.online = Some(false);
                device
            })
            .collect::<Vec<_>>();
        let remote = remote_settings_mut(&mut raw)?;
        remote.insert(
            "cachedDevices".to_string(),
            serde_json::to_value(&devices).map_err(|error| error.to_string())?,
        );
        self.save_raw_settings(&raw)?;
        Ok(self.summary())
    }
}
