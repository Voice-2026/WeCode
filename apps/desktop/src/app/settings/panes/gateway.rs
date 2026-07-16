use super::widgets::*;
use super::*;
use wecode_runtime::gateway_service::{
    CredentialSource, GatewayModelCatalog, GatewayRuntimeStatus, GatewayService, GatewaySettings,
    kiro_app_credentials_path,
};

pub(super) fn settings_gateway_pane(
    settings: &GatewaySettings,
    _gateway_service: &GatewayService,
    catalog: &GatewayModelCatalog,
    refreshing_models: bool,
    model_error: Option<&str>,
    language: &str,
    window: &mut Window,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    let config = &settings.config;
    let status = gateway_status(settings.enabled, GatewayService::global_status(), language);
    let mut cards = vec![
        settings_card(
            Some(settings_text(
                language,
                "settings.gateway.section.service",
                "WeCode Gateway",
            )),
            Some(settings_text(
                language,
                "settings.gateway.section.service.description",
                "Use Kiro only as a model provider for Claude Code.",
            )),
            vec![
                settings_row(
                    settings_text(language, "settings.gateway.enabled", "Enable Gateway"),
                    None,
                    settings_toggle("settings-gateway-enabled", settings.enabled, cx, {
                        move |app, _window, cx| {
                            let next = !app.gateway_settings.enabled;
                            app.set_gateway_enabled(next, cx);
                        }
                    }),
                )
                .into_any_element(),
                settings_row(
                    settings_text(language, "settings.gateway.status", "Status"),
                    None,
                    status,
                )
                .into_any_element(),
                settings_row(
                    settings_text(language, "settings.gateway.mode", "Mode"),
                    Some(settings_text(
                        language,
                        "settings.gateway.mode.provider_only.description",
                        "Claude Code owns sessions, tools, Skills, Hooks, MCP and approvals. Built-in Web Search is proxied as a server tool.",
                    )),
                    settings_status_tag(
                        settings_text(
                            language,
                            "settings.gateway.mode.provider_only",
                            "Model Provider Only",
                        ),
                        theme::GREEN,
                    ),
                )
                .into_any_element(),
                settings_row(
                    settings_text(language, "settings.gateway.search_transport", "Web Search"),
                    Some(settings_text(
                        language,
                        "settings.gateway.search_transport.description",
                        "Proxy Claude Code's built-in server search without enabling the Kiro agent.",
                    )),
                    settings_status_tag(
                        settings_text(
                            language,
                            "settings.gateway.search_transport.enabled",
                            "Claude Code Search Proxy",
                        ),
                        theme::GREEN,
                    ),
                )
                .into_any_element(),
            ],
            cx,
        )
        .into_any_element(),
        settings_card(
            Some(settings_text(
                language,
                "settings.gateway.section.endpoint",
                "Endpoint",
            )),
            None,
            vec![
                settings_row(
                    settings_text(language, "settings.gateway.host", "Host"),
                    None,
                    settings_text_input(
                        "gateway-host",
                        &config.host,
                        "127.0.0.1",
                        false,
                        window,
                        cx,
                        |app, value, _window, cx| app.set_gateway_host(value, cx),
                    ),
                )
                .into_any_element(),
                settings_row(
                    settings_text(language, "settings.gateway.port", "Port"),
                    None,
                    settings_text_input(
                        "gateway-port",
                        config.port.to_string(),
                        "8989",
                        false,
                        window,
                        cx,
                        |app, value, _window, cx| app.set_gateway_port(value, cx),
                    ),
                )
                .into_any_element(),
                settings_row(
                    settings_text(language, "settings.gateway.api_key", "API Key"),
                    Some(settings_text(
                        language,
                        "settings.gateway.api_key.description",
                        "Clients must send this value as a bearer token or x-api-key.",
                    )),
                    settings_text_input(
                        "gateway-api-key",
                        &config.api_key,
                        "my-super-secret-password-123",
                        true,
                        window,
                        cx,
                        |app, value, _window, cx| app.set_gateway_api_key(value, cx),
                    ),
                )
                .into_any_element(),
                settings_row(
                    settings_text(language, "settings.gateway.region", "Region"),
                    None,
                    settings_text_input(
                        "gateway-region",
                        &config.region,
                        "us-east-1",
                        false,
                        window,
                        cx,
                        |app, value, _window, cx| app.set_gateway_region(value, cx),
                    ),
                )
                .into_any_element(),
            ],
            cx,
        )
        .into_any_element(),
    ];

    cards.push(
        settings_card(
            Some(settings_text(
                language,
                "settings.gateway.section.models",
                "Models",
            )),
            Some(settings_text(
                language,
                "settings.gateway.section.models.description",
                "Refresh the catalog from Kiro CLI and choose defaults for each client.",
            )),
            model_rows(
                settings,
                catalog,
                refreshing_models,
                model_error,
                language,
                window,
                cx,
            ),
            cx,
        )
        .into_any_element(),
    );

    cards.push(
        settings_card(
            Some(settings_text(
                language,
                "settings.gateway.section.credentials",
                "Credentials",
            )),
            None,
            credential_rows(settings, language, window, cx),
            cx,
        )
        .into_any_element(),
    );

    settings_form(cards).into_any_element()
}

fn model_rows(
    settings: &GatewaySettings,
    catalog: &GatewayModelCatalog,
    refreshing: bool,
    error: Option<&str>,
    language: &str,
    window: &mut Window,
    cx: &mut Context<WeCodeApp>,
) -> Vec<AnyElement> {
    let stale = catalog.is_stale_now();
    let status = if let Some(error) = error {
        settings_status_tag(format!("Failed · {error}"), theme::RED)
    } else if stale {
        settings_status_tag(
            format!("{} models · Stale", catalog.models.len()),
            theme::ORANGE,
        )
    } else {
        settings_status_tag(
            format!("{} models · {}", catalog.models.len(), catalog.source),
            theme::GREEN,
        )
    };
    let claude_options = catalog
        .claude_code_models()
        .map(|model| (model.id.clone(), SharedString::from(model.name.clone())))
        .collect();
    let codex_options = catalog
        .codex_cli_models()
        .map(|model| (model.id.clone(), SharedString::from(model.name.clone())))
        .collect();

    let mut rows = vec![
        settings_row(
            settings_text(language, "settings.gateway.models.status", "Catalog"),
            Some(format!(
                "Last refreshed: {}",
                catalog.refreshed_at.to_rfc3339()
            )),
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.0))
                .child(status)
                .child(settings_small_button_state(
                    "settings-gateway-models-refresh",
                    if refreshing {
                        "Refreshing..."
                    } else {
                        "Refresh"
                    },
                    refreshing,
                    refreshing,
                    cx,
                    |app, _event, _window, cx| app.refresh_gateway_models(cx),
                ))
                .into_any_element(),
        )
        .into_any_element(),
        settings_row(
            settings_text(
                language,
                "settings.gateway.models.default_claude",
                "Claude Code Default",
            ),
            None,
            settings_select_impl(
                "settings-gateway-default-claude-model",
                &settings.default_claude_model,
                claude_options,
                window,
                cx,
                language,
                |app, value, _window, cx| app.set_gateway_default_claude_model(value, cx),
            ),
        )
        .into_any_element(),
        settings_row(
            settings_text(
                language,
                "settings.gateway.models.default_codex",
                "Codex CLI Default",
            ),
            None,
            settings_select_impl(
                "settings-gateway-default-codex-model",
                &settings.default_codex_model,
                codex_options,
                window,
                cx,
                language,
                |app, value, _window, cx| app.set_gateway_default_codex_model(value, cx),
            ),
        )
        .into_any_element(),
    ];
    for model in &catalog.models {
        let clients = match (
            model.compatibility.claude_code,
            model.compatibility.codex_cli,
        ) {
            (true, true) => "Claude Code · Codex CLI",
            (true, false) => "Claude Code",
            (false, true) => "Codex CLI",
            (false, false) => "Not enabled",
        };
        let detail = if model.description.is_empty() {
            format!("{} context tokens", model.context_window_tokens)
        } else {
            format!(
                "{} · {} context tokens",
                model.description, model.context_window_tokens
            )
        };
        rows.push(
            settings_row(
                model.name.clone(),
                Some(detail),
                settings_status_tag(
                    format!("{} · {}×", clients, model.rate_multiplier),
                    if model.compatibility.claude_code || model.compatibility.codex_cli {
                        theme::GREEN
                    } else {
                        theme::TEXT_DIM
                    },
                ),
            )
            .into_any_element(),
        );
    }
    rows
}

fn gateway_status(enabled: bool, status: GatewayRuntimeStatus, language: &str) -> AnyElement {
    if !enabled {
        return settings_status_tag(
            settings_text(language, "settings.gateway.status.disabled", "Disabled"),
            theme::TEXT_DIM,
        );
    }
    if let Some(error) = status.error {
        return settings_status_tag(
            format!(
                "{} · {error}",
                settings_text(language, "settings.gateway.status.failed", "Failed")
            ),
            theme::RED,
        );
    }
    match status.addr {
        Some(addr) => settings_status_tag(
            format!(
                "{} · {addr}",
                settings_text(language, "settings.gateway.status.listening", "Listening")
            ),
            theme::GREEN,
        ),
        None => settings_status_tag(
            settings_text(language, "settings.gateway.status.enabled", "Enabled"),
            theme::ORANGE,
        ),
    }
}

fn credential_rows(
    settings: &GatewaySettings,
    language: &str,
    window: &mut Window,
    cx: &mut Context<WeCodeApp>,
) -> Vec<AnyElement> {
    let source_value = credential_source_value(&settings.config.credentials);
    let mut rows = vec![
        settings_row(
            settings_text(language, "settings.gateway.credential_source", "Source"),
            None,
            settings_select_impl(
                "settings-gateway-credential-source",
                source_value,
                credential_source_options(language),
                window,
                cx,
                language,
                |app, value, _window, cx| app.set_gateway_credential_source(value, cx),
            ),
        )
        .into_any_element(),
    ];

    match &settings.config.credentials {
        CredentialSource::KiroApp { path } => {
            let credentials_path = kiro_app_credentials_path(path.clone());
            let (status, color) = if credentials_path.is_file() {
                (
                    settings_text(language, "settings.gateway.kiro_app.detected", "Detected"),
                    theme::GREEN,
                )
            } else {
                (
                    settings_text(language, "settings.gateway.kiro_app.not_found", "Not Found"),
                    theme::RED,
                )
            };
            rows.push(
                settings_row(
                    settings_text(
                        language,
                        "settings.gateway.kiro_app.credentials",
                        "Kiro App Credentials",
                    ),
                    Some(settings_text(
                        language,
                        "settings.gateway.kiro_app.credentials.description",
                        "Automatically uses the account signed in to Kiro App. WeCode never modifies Kiro's credential file; sign in to Kiro App first if it is not detected.",
                    )),
                    settings_status_tag(status, color),
                )
                .into_any_element(),
            );
        }
        CredentialSource::KiroCli { path, readonly } => {
            rows.push(
                settings_row(
                    settings_text(
                        language,
                        "settings.gateway.kiro_cli_path",
                        "Kiro CLI DB Path",
                    ),
                    Some(settings_text(
                        language,
                        "settings.gateway.kiro_cli_path.description",
                        "Leave empty to use the default Kiro CLI credential database.",
                    )),
                    settings_text_input(
                        "gateway-kiro-cli-path",
                        path.as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_default(),
                        "~/.kiro/.../auth.db",
                        false,
                        window,
                        cx,
                        |app, value, _window, cx| app.set_gateway_kiro_cli_path(value, cx),
                    ),
                )
                .into_any_element(),
            );
            rows.push(
                settings_row(
                    settings_text(language, "settings.gateway.kiro_cli_readonly", "Read Only"),
                    None,
                    settings_toggle("settings-gateway-kiro-readonly", *readonly, cx, {
                        move |app, _window, cx| {
                            let next = match &app.gateway_settings.config.credentials {
                                CredentialSource::KiroCli { readonly, .. } => !readonly,
                                _ => true,
                            };
                            app.set_gateway_kiro_cli_readonly(next, cx);
                        }
                    }),
                )
                .into_any_element(),
            );
        }
        CredentialSource::File { path } => {
            rows.push(
                settings_row(
                    settings_text(
                        language,
                        "settings.gateway.credentials_file",
                        "Credentials File",
                    ),
                    None,
                    settings_text_input(
                        "gateway-credentials-file",
                        path.display().to_string(),
                        "/path/to/kiro-credentials.json",
                        false,
                        window,
                        cx,
                        |app, value, _window, cx| app.set_gateway_credential_file_path(value, cx),
                    ),
                )
                .into_any_element(),
            );
        }
        CredentialSource::RefreshToken {
            refresh_token,
            profile_arn,
            region,
        } => {
            rows.push(
                settings_row(
                    settings_text(language, "settings.gateway.refresh_token", "Refresh Token"),
                    None,
                    settings_text_input(
                        "gateway-refresh-token",
                        refresh_token,
                        "refresh token",
                        true,
                        window,
                        cx,
                        |app, value, _window, cx| app.set_gateway_refresh_token(value, cx),
                    ),
                )
                .into_any_element(),
            );
            rows.push(
                settings_row(
                    settings_text(language, "settings.gateway.profile_arn", "Profile ARN"),
                    None,
                    settings_text_input(
                        "gateway-profile-arn",
                        profile_arn.clone().unwrap_or_default(),
                        "arn:aws:...",
                        false,
                        window,
                        cx,
                        |app, value, _window, cx| app.set_gateway_refresh_profile_arn(value, cx),
                    ),
                )
                .into_any_element(),
            );
            rows.push(
                settings_row(
                    settings_text(language, "settings.gateway.refresh_region", "Token Region"),
                    None,
                    settings_text_input(
                        "gateway-refresh-region",
                        region.clone().unwrap_or_default(),
                        "us-east-1",
                        false,
                        window,
                        cx,
                        |app, value, _window, cx| app.set_gateway_refresh_region(value, cx),
                    ),
                )
                .into_any_element(),
            );
        }
    }

    rows
}

fn credential_source_value(source: &CredentialSource) -> &'static str {
    match source {
        CredentialSource::KiroApp { .. } => "kiro-app",
        CredentialSource::KiroCli { .. } => "kiro-cli",
        CredentialSource::File { .. } => "file",
        CredentialSource::RefreshToken { .. } => "refresh-token",
    }
}

fn credential_source_options(language: &str) -> Vec<(String, SharedString)> {
    vec![
        (
            "kiro-app".to_string(),
            SharedString::from(settings_text(
                language,
                "settings.gateway.credential_source.kiro_app",
                "Kiro App",
            )),
        ),
        (
            "kiro-cli".to_string(),
            SharedString::from(settings_text(
                language,
                "settings.gateway.credential_source.kiro_cli",
                "Kiro CLI",
            )),
        ),
        (
            "file".to_string(),
            SharedString::from(settings_text(
                language,
                "settings.gateway.credential_source.file",
                "Credentials File",
            )),
        ),
        (
            "refresh-token".to_string(),
            SharedString::from(settings_text(
                language,
                "settings.gateway.credential_source.refresh_token",
                "Refresh Token",
            )),
        ),
    ]
}
