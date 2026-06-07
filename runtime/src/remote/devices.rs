use super::relay::{remote_get, remote_server_url};
use super::types::{RemoteDeviceSettings, RemoteSummary};
use super::{RemoteService, remote_settings_from_raw};
use serde::Deserialize;

impl RemoteService {
    pub fn revoke_device(&self, device_id: &str) -> Result<RemoteSummary, String> {
        let device_id = device_id.trim();
        if device_id.is_empty() {
            return Err("Missing device id.".to_string());
        }
        let mut raw = self.raw_settings();
        let mut settings = remote_settings_from_raw(&raw);
        let before_len = settings.cached_devices.len();
        settings
            .cached_devices
            .retain(|device| device.id != device_id);
        if settings.cached_devices.len() == before_len {
            return Err("Remote device not found.".to_string());
        }
        raw.insert(
            "remote".to_string(),
            serde_json::to_value(&settings).map_err(|error| error.to_string())?,
        );
        self.save_raw_settings(&raw)?;
        let mut summary = self.summary();
        summary.message = "Device removed.".to_string();
        Ok(summary)
    }

    pub fn refresh_devices(&self) -> Result<RemoteSummary, String> {
        crate::async_runtime::block_on(self.refresh_devices_async())
    }

    pub async fn refresh_devices_async(&self) -> Result<RemoteSummary, String> {
        let mut raw = self.raw_settings();
        let mut settings = remote_settings_from_raw(&raw);
        if !settings.is_enabled
            || settings.host_id.trim().is_empty()
            || settings.host_token.trim().is_empty()
        {
            return Ok(self.summary());
        }
        #[derive(Deserialize)]
        struct DeviceList {
            devices: Vec<RemoteDeviceSettings>,
        }
        let relay = remote_server_url(&settings.server_url);
        let path = format!("/api/hosts/{}/devices", settings.host_id);
        let mut list = remote_get::<DeviceList>(
            &relay,
            &path,
            &[("token", settings.host_token.as_str())],
        )
        .await?;
        list.devices.retain(|device| device.revoked_at.is_none());
        settings.cached_devices = list
            .devices
            .into_iter()
            .map(|mut device| {
                device.online = Some(false);
                device
            })
            .collect();
        raw.insert(
            "remote".to_string(),
            serde_json::to_value(&settings).map_err(|error| error.to_string())?,
        );
        self.save_raw_settings(&raw)?;
        Ok(self.summary())
    }
}
