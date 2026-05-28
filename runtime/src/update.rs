use crate::settings::{AppSettings, UpdateSettings as AppUpdateSettings};
use semver::Version;
use serde::Serialize;
use serde_json::Value;
use std::{fs, path::PathBuf, time::Duration};

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSummary {
    pub enabled: bool,
    pub channel: String,
    pub endpoint: String,
    pub latest_version: Option<String>,
    pub platform_count: usize,
    pub notes_preview: String,
    pub error: Option<String>,
}

pub struct UpdateService {
    settings_path: PathBuf,
    repo_root: PathBuf,
}

impl UpdateService {
    pub fn new(support_dir: PathBuf, repo_root: PathBuf) -> Self {
        Self {
            settings_path: support_dir.join("settings.json"),
            repo_root,
        }
    }

    pub fn summary(&self) -> UpdateSummary {
        let settings = self.settings();
        let mut summary = UpdateSummary {
            enabled: settings.enabled,
            channel: if settings.channel.is_empty() {
                "stable".to_string()
            } else {
                settings.channel
            },
            endpoint: settings.endpoint,
            ..Default::default()
        };
        match self.load_latest_manifest(&summary) {
            Ok(value) => {
                summary.latest_version = value
                    .get("version")
                    .or_else(|| value.get("latestVersion"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                summary.platform_count = value
                    .get("platforms")
                    .and_then(Value::as_object)
                    .map(|platforms| platforms.len())
                    .unwrap_or(0);
                summary.notes_preview = value
                    .get("notes")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .lines()
                    .take(4)
                    .collect::<Vec<_>>()
                    .join(" ");
            }
            Err(error) => summary.error = Some(error),
        }
        summary
    }

    pub fn status(&self, current_version: &str) -> UpdateStatus {
        let settings = self.settings();
        self.status_for_update_settings(&settings, current_version)
    }

    pub fn status_from_settings(
        settings: &AppSettings,
        repo_root: PathBuf,
        current_version: &str,
    ) -> UpdateStatus {
        Self::new(PathBuf::new(), repo_root)
            .status_for_update_settings(&settings.update, current_version)
    }

    pub fn status_for_update_settings(
        &self,
        settings: &AppUpdateSettings,
        current_version: &str,
    ) -> UpdateStatus {
        let endpoint_configured = settings.enabled && !settings.endpoint.trim().is_empty();
        if !endpoint_configured {
            return UpdateStatus {
                configured: false,
                checking: false,
                available: false,
                automatic_install_supported: false,
                signed_updater_configured: false,
                manifest_endpoint_configured: false,
                current_version: current_version.to_string(),
                latest_version: None,
                download_url: None,
                notes: None,
                channel: Some(settings.channel.clone()).filter(|value| !value.trim().is_empty()),
                installation_mode: if settings.enabled {
                    "notConfigured".to_string()
                } else {
                    "disabled".to_string()
                },
                message: if settings.enabled {
                    "Unable to check the GitHub update channel for this build.".to_string()
                } else {
                    "Update checks are turned off.".to_string()
                },
            };
        }
        match self.load_latest_manifest(&UpdateSummary {
            enabled: settings.enabled,
            channel: settings.channel.clone(),
            endpoint: settings.endpoint.clone(),
            ..Default::default()
        }) {
            Ok(value) => {
                update_status_from_manifest(current_version, settings.channel.clone(), value)
            }
            Err(error) => UpdateStatus {
                configured: true,
                checking: false,
                available: false,
                automatic_install_supported: false,
                signed_updater_configured: false,
                manifest_endpoint_configured: true,
                current_version: current_version.to_string(),
                latest_version: None,
                download_url: None,
                notes: None,
                channel: Some(settings.channel.clone()).filter(|value| !value.trim().is_empty()),
                installation_mode: "manifest".to_string(),
                message: format!("Unable to check updates: {error}"),
            },
        }
    }

    fn settings(&self) -> AppUpdateSettings {
        fs::read_to_string(&self.settings_path)
            .ok()
            .and_then(|content| serde_json::from_str::<Value>(&content).ok())
            .and_then(|value| value.get("update").cloned())
            .and_then(|value| serde_json::from_value::<AppUpdateSettings>(value).ok())
            .unwrap_or_default()
    }

    fn load_latest_manifest(&self, settings: &UpdateSummary) -> Result<Value, String> {
        if settings
            .endpoint
            .contains("raw.githubusercontent.com/duxweb/codux/main/updates/")
        {
            let local = self
                .repo_root
                .join("updates")
                .join(&settings.channel)
                .join("latest.json");
            return read_json_file(local);
        }
        if let Some(path) = settings.endpoint.strip_prefix("file://") {
            return read_json_file(PathBuf::from(path));
        }
        if settings.endpoint.starts_with("http://") || settings.endpoint.starts_with("https://") {
            return fetch_json(&settings.endpoint);
        }
        if settings.endpoint.trim().is_empty() {
            return Err("Update endpoint is empty.".to_string());
        }
        read_json_file(PathBuf::from(&settings.endpoint))
    }
}

fn read_json_file(path: PathBuf) -> Result<Value, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn fetch_json(endpoint: &str) -> Result<Value, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| error.to_string())?
        .get(endpoint)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?
        .json::<Value>()
        .map_err(|error| error.to_string())
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub configured: bool,
    pub checking: bool,
    pub available: bool,
    pub automatic_install_supported: bool,
    pub signed_updater_configured: bool,
    pub manifest_endpoint_configured: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub download_url: Option<String>,
    pub notes: Option<String>,
    pub channel: Option<String>,
    pub installation_mode: String,
    pub message: String,
}

fn update_status_from_manifest(
    current_version: &str,
    channel: String,
    manifest: Value,
) -> UpdateStatus {
    let latest = manifest
        .get("version")
        .or_else(|| manifest.get("latestVersion"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let available = latest
        .as_deref()
        .is_some_and(|version| version_is_newer(version, current_version));
    let latest_text = latest
        .clone()
        .unwrap_or_else(|| current_version.to_string());
    let message = if available {
        format!(
            "A new version {latest_text} is available. Automatic installation requires signed updater packaging; open the download URL to update manually."
        )
    } else {
        format!("Current version {current_version} is up to date.")
    };
    UpdateStatus {
        configured: true,
        checking: false,
        available,
        automatic_install_supported: false,
        signed_updater_configured: false,
        manifest_endpoint_configured: true,
        current_version: current_version.to_string(),
        latest_version: latest,
        download_url: manifest
            .get("downloadUrl")
            .or_else(|| manifest.get("url"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        notes: manifest
            .get("notes")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        channel: Some(channel).filter(|value| !value.trim().is_empty()),
        installation_mode: "manifest".to_string(),
        message,
    }
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let latest = latest.trim().trim_start_matches('v');
    let current = current.trim().trim_start_matches('v');
    match (Version::parse(latest), Version::parse(current)) {
        (Ok(latest), Ok(current)) => latest > current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_from_settings_reports_disabled_update_checks() {
        let mut settings = AppSettings::default();
        settings.update.enabled = false;

        let status = UpdateService::status_from_settings(&settings, PathBuf::new(), "1.0.0");

        assert!(!status.configured);
        assert!(!status.available);
        assert_eq!(status.installation_mode, "disabled");
        assert_eq!(status.message, "Update checks are turned off.");
    }

    #[test]
    fn status_from_settings_reads_local_manifest_endpoint() {
        let dir = std::env::temp_dir().join(format!(
            "codux-runtime-update-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).unwrap();
        let manifest_path = dir.join("latest.json");
        fs::write(
            &manifest_path,
            r#"{"version":"1.2.0","downloadUrl":"https://example.com/codux","notes":"new build"}"#,
        )
        .unwrap();

        let mut settings = AppSettings::default();
        settings.update.enabled = true;
        settings.update.channel = "stable".to_string();
        settings.update.endpoint = manifest_path.display().to_string();

        let status = UpdateService::status_from_settings(&settings, PathBuf::new(), "1.0.0");

        assert!(status.configured);
        assert!(status.available);
        assert_eq!(status.latest_version.as_deref(), Some("1.2.0"));
        assert_eq!(
            status.download_url.as_deref(),
            Some("https://example.com/codux")
        );
        assert_eq!(status.installation_mode, "manifest");

        let _ = fs::remove_dir_all(dir);
    }
}
