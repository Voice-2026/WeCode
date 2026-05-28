use crate::{
    app_info::{
        AppAboutMetadata, AppDiagnosticsSnapshot, DiagnosticsExportRequest, DiagnosticsExportResult,
        UpdateInstallResult, UpdateStatus,
    },
    notification::{NotificationDispatchRequest, NotificationDispatchResult},
    power::PowerManager,
    settings::AppSettings,
};
use std::path::PathBuf;

pub fn app_about_metadata(
    version: impl Into<String>,
    identifier: impl Into<String>,
) -> AppAboutMetadata {
    crate::app_info::about_metadata(version, identifier)
}

pub fn app_update_status(
    settings: &AppSettings,
    repo_root: PathBuf,
    current_version: &str,
) -> Result<UpdateStatus, String> {
    Ok(crate::app_info::update_status(
        settings,
        repo_root,
        current_version,
    ))
}

pub fn app_update_install(
    settings: &AppSettings,
    repo_root: PathBuf,
    current_version: &str,
) -> Result<UpdateInstallResult, String> {
    crate::app_info::install_update(settings, repo_root, current_version)
}

pub fn diagnostics_export(
    request: DiagnosticsExportRequest,
    about: AppAboutMetadata,
    update: UpdateStatus,
    snapshot: AppDiagnosticsSnapshot,
) -> Result<DiagnosticsExportResult, String> {
    crate::app_info::export_diagnostics(request, about, update, snapshot)
}

pub fn app_open_runtime_log() -> Result<(), String> {
    crate::app_info::open_runtime_log()
}

pub fn app_open_live_log() -> Result<(), String> {
    crate::app_info::open_live_log()
}

pub fn app_open_url(url: String) -> Result<(), String> {
    crate::app_info::open_url(&url)
}

pub fn app_request_restart() -> Result<(), String> {
    crate::app_info::request_restart()
}

pub fn app_toggle_devtools() -> bool {
    cfg!(debug_assertions)
}

pub fn app_window_close() -> bool {
    true
}

pub fn power_set_sleep_prevention(
    manager: &PowerManager,
    mode: String,
) -> Result<bool, String> {
    manager.set_sleep_prevention(mode)
}

pub fn notification_dispatch_channels(
    request: NotificationDispatchRequest,
) -> NotificationDispatchResult {
    crate::notification::dispatch_notification_channels(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn app_command_names_delegate_to_runtime_system_logic() {
        let mut settings = AppSettings::default();
        settings.update.enabled = false;
        let status = app_update_status(&settings, PathBuf::new(), "1.2.3")
            .expect("update status");
        assert_eq!(status.current_version, "1.2.3");
        assert_eq!(status.installation_mode, "disabled");

        let about = app_about_metadata("1.2.3", "com.duxweb.codux.test");
        assert_eq!(about.version, "1.2.3");
        assert_eq!(about.identifier, "com.duxweb.codux.test");

        let manager = PowerManager::default();
        assert!(!power_set_sleep_prevention(&manager, "off".to_string()).expect("power off"));
        assert!(app_window_close());
    }

    #[test]
    fn diagnostics_export_command_writes_redacted_runtime_report() {
        let destination = std::env::temp_dir().join(format!(
            "codux-app-command-diagnostics-{}.json",
            Uuid::new_v4()
        ));
        let request = DiagnosticsExportRequest {
            destination_path: destination.display().to_string(),
        };
        let about = app_about_metadata("1.0.0", "com.duxweb.codux.test");
        let update = UpdateStatus {
            current_version: "1.0.0".to_string(),
            installation_mode: "disabled".to_string(),
            message: "disabled".to_string(),
            ..Default::default()
        };
        let result = diagnostics_export(
            request,
            about,
            update,
            AppDiagnosticsSnapshot {
                settings: json!({
                    "ai": {
                        "providers": [
                            {
                                "apiKey": "secret",
                                "api_key": "secret",
                                "token": "secret"
                            }
                        ]
                    }
                }),
                projects: json!([]),
                ai_runtime: json!({}),
                ai_state: json!({}),
                performance: json!({}),
                ssh: json!({
                    "profiles": [
                        {
                            "password": "secret",
                            "privateKeyPath": "/tmp/key"
                        }
                    ]
                }),
            },
        )
        .expect("export diagnostics");

        assert!(result.bytes > 0);
        let content = std::fs::read_to_string(&destination).expect("diagnostics file");
        assert!(content.contains("\"apiKey\": \"******\""));
        assert!(content.contains("\"api_key\": \"******\""));
        assert!(content.contains("\"password\": \"******\""));
        assert!(content.contains("\"privateKeyPath\": \"******\""));
        let _ = std::fs::remove_file(destination);
    }
}
