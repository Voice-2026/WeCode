use super::*;
use codux_runtime::{
    app_info::{AppAboutMetadata, DiagnosticsExportRequest},
    dialog::{DialogFilter, LocalizedSaveDialogRequest},
};

const CODUX_WEBSITE_URL: &str = "https://codux.dux.cn";
const CODUX_GITHUB_URL: &str = "https://github.com/duxweb/codux";
const CODUX_IDENTIFIER: &str = "com.duxweb.codux";

impl CoduxApp {
    pub(in crate::app) fn about_workspace(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let locale = locale_from_language_setting(&self.state.settings.language);
        let about = self
            .runtime_service
            .about_metadata(env!("CARGO_PKG_VERSION"), CODUX_IDENTIFIER);
        let update = self.runtime_service.update_status(
            std::env::current_dir().unwrap_or_default(),
            env!("CARGO_PKG_VERSION"),
        );

        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .bg(color(theme::BG))
            .text_color(color(theme::TEXT))
            .child(div().h(px(28.0)).flex_shrink_0())
            .child(about_icon_mark())
            .child(
                div()
                    .mt(px(14.0))
                    .text_size(px(20.0))
                    .line_height(px(24.0))
                    .font_weight(FontWeight::BOLD)
                    .child(about.name.clone()),
            )
            .child(
                div()
                    .mt(px(6.0))
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .text_color(color(theme::TEXT_MUTED))
                    .child(format!(
                        "{} · {}/{} · {}",
                        about.version, about.target_os, about.target_arch, about.build_profile
                    )),
            )
            .child(
                div()
                    .mt(px(22.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .text_color(color(theme::TEXT_MUTED))
                            .child(translate(
                                &locale,
                                "about.tagline",
                                "AI-Powered Terminal Workspace",
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .line_height(px(15.0))
                            .text_color(color(theme::TEXT_DIM))
                            .child(translate(
                                &locale,
                                "about.copyright",
                                "Copyright (c) 2025 Codux contributors",
                            )),
                    ),
            )
            .child(about_status_card(&about, &update, &locale, cx))
            .child(about_action_row(&locale, cx))
            .child(
                div()
                    .mt(px(18.0))
                    .max_w(px(300.0))
                    .truncate()
                    .text_size(px(11.0))
                    .line_height(px(15.0))
                    .text_color(color(theme::TEXT_DIM))
                    .child(about.identifier),
            )
    }

    pub(in crate::app) fn open_about_window(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if Self::activate_child_window(&mut self.about_window, cx) {
            self.status_message = "about window already opened".to_string();
            self.invalidate_status_bar(cx);
            return;
        }

        let bounds = Bounds::centered(None, size(px(420.0), px(520.0)), cx);
        let result = cx.open_window(
            WindowOptions {
                titlebar: Some(theme::codux_titlebar("About Codux")),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(380.0), px(480.0))),
                ..Default::default()
            },
            |window, cx| {
                let mut app = CoduxApp::new_settings_window_from_state(
                    self.state.clone(),
                    self.runtime.clone(),
                    self.runtime_service.clone(),
                );
                app.window_mode = AppWindowMode::About;
                theme::apply_component_theme(
                    &app.state.settings.theme,
                    &app.state.settings.theme_color,
                    Some(window),
                    cx,
                );
                let view = cx.new(|_| app);
                cx.new(|cx| Root::new(view, window, cx))
            },
        );

        self.status_message = match result {
            Ok(handle) => {
                self.about_window = Some(handle.into());
                "about window opened".to_string()
            }
            Err(error) => format!("failed to open about window: {error}"),
        };
        self.invalidate_status_bar(cx);
    }

    pub(in crate::app) fn open_memory_manager_window(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if Self::activate_child_window(&mut self.memory_manager_window, cx) {
            self.status_message = "memory manager window already opened".to_string();
            self.invalidate_status_bar(cx);
            return;
        }

        let bounds = Bounds::centered(None, size(px(900.0), px(720.0)), cx);
        let state = self.state.clone();
        let runtime = self.runtime.clone();
        let runtime_service = self.runtime_service.clone();
        let result = cx.open_window(
            WindowOptions {
                titlebar: Some(theme::codux_titlebar("Memory Manager")),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(720.0), px(560.0))),
                ..Default::default()
            },
            |window, cx| {
                let app = CoduxApp::new_memory_manager_window(state, runtime, runtime_service);
                theme::apply_component_theme(
                    &app.state.settings.theme,
                    &app.state.settings.theme_color,
                    Some(window),
                    cx,
                );
                let view = cx.new(|_| app);
                view.update(cx, |app, cx| app.reload_memory_manager_snapshot_async(cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        );

        self.status_message = match result {
            Ok(handle) => {
                self.memory_manager_window = Some(handle.into());
                "memory manager window opened".to_string()
            }
            Err(error) => format!("failed to open memory manager window: {error}"),
        };
        self.invalidate_status_bar(cx);
    }

    pub(in crate::app) fn open_codux_website(&mut self, cx: &mut Context<Self>) {
        match self.runtime_service.open_url(CODUX_WEBSITE_URL) {
            Ok(()) => self.status_message = "Codux website opened".to_string(),
            Err(error) => self.status_message = format!("failed to open Codux website: {error}"),
        }
        self.invalidate_status_bar(cx);
    }

    pub(in crate::app) fn open_codux_github(&mut self, cx: &mut Context<Self>) {
        match self.runtime_service.open_url(CODUX_GITHUB_URL) {
            Ok(()) => self.status_message = "Codux GitHub opened".to_string(),
            Err(error) => self.status_message = format!("failed to open Codux GitHub: {error}"),
        }
        self.invalidate_status_bar(cx);
    }

    pub(in crate::app) fn open_runtime_log(&mut self, cx: &mut Context<Self>) {
        self.runtime_trace("help", "open_runtime_log");
        match self.runtime_service.open_runtime_log() {
            Ok(()) => self.status_message = "runtime log opened".to_string(),
            Err(error) => self.status_message = format!("failed to open runtime log: {error}"),
        }
        self.invalidate_status_bar(cx);
    }

    pub(in crate::app) fn open_live_log(&mut self, cx: &mut Context<Self>) {
        self.runtime_trace("help", "open_live_log");
        match self.runtime_service.open_live_log() {
            Ok(()) => self.status_message = "live log opened".to_string(),
            Err(error) => self.status_message = format!("failed to open live log: {error}"),
        }
        self.invalidate_status_bar(cx);
    }

    pub(in crate::app) fn request_restart(&mut self, cx: &mut Context<Self>) {
        match self.runtime_service.request_restart() {
            Ok(()) => self.status_message = "restart requested".to_string(),
            Err(error) => self.status_message = format!("failed to request restart: {error}"),
        }
        self.invalidate_status_bar(cx);
    }

    pub(in crate::app) fn export_diagnostics(&mut self, cx: &mut Context<Self>) {
        self.runtime_trace("help", "export_diagnostics choose_destination");
        let destination =
            match self
                .runtime_service
                .localized_save_dialog(LocalizedSaveDialogRequest {
                    title: self.text("about.diagnostics.export", "Export Diagnostics"),
                    message: self.text(
                        "about.diagnostics.export.message",
                        "Choose where to save the diagnostics report.",
                    ),
                    prompt: self.text("common.save", "Save"),
                    default_path: Some(format!("codux-diagnostics-{}.json", timestamp_slug())),
                    filters: vec![DialogFilter {
                        _name: "JSON".to_string(),
                        extensions: vec!["json".to_string()],
                    }],
                    can_create_directories: Some(true),
                }) {
                Ok(Some(path)) => path,
                Ok(None) => {
                    self.status_message = "diagnostics export canceled".to_string();
                    self.invalidate_status_bar(cx);
                    return;
                }
                Err(error) => {
                    self.status_message = format!("failed to choose diagnostics path: {error}");
                    self.invalidate_status_bar(cx);
                    return;
                }
            };

        let about = self
            .runtime_service
            .about_metadata(env!("CARGO_PKG_VERSION"), CODUX_IDENTIFIER);
        let update = self.runtime_service.update_status(
            std::env::current_dir().unwrap_or_default(),
            env!("CARGO_PKG_VERSION"),
        );
        match self.runtime_service.export_diagnostics(
            DiagnosticsExportRequest {
                destination_path: destination,
            },
            about,
            update,
        ) {
            Ok(result) => {
                self.runtime_trace(
                    "help",
                    &format!(
                        "export_diagnostics success path={} bytes={}",
                        result.path, result.bytes
                    ),
                );
                self.status_message = format!(
                    "diagnostics exported: {} ({} bytes)",
                    result.path, result.bytes
                );
            }
            Err(error) => {
                self.runtime_trace("help", &format!("export_diagnostics failed error={error}"));
                self.status_message = format!("failed to export diagnostics: {error}");
            }
        }
        self.invalidate_status_bar(cx);
    }
}

fn about_icon_mark() -> impl IntoElement {
    div()
        .size(px(96.0))
        .rounded(px(22.0))
        .flex()
        .items_center()
        .justify_center()
        .bg(linear_gradient(
            145.0,
            linear_color_stop(color(theme::ACCENT), 0.0),
            linear_color_stop(color(0x7C4DFF), 1.0),
        ))
        .child(
            div()
                .text_size(px(36.0))
                .line_height(px(40.0))
                .font_weight(FontWeight::BOLD)
                .text_color(color(0xFFFFFF))
                .child("C"),
        )
}

fn about_status_card(
    about: &AppAboutMetadata,
    update: &codux_runtime::update::UpdateStatus,
    locale: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let tr = |key: &str, fallback: &str| translate(locale, key, fallback);
    let update_label = if !update.configured {
        tr("settings.update.mode.not_configured", "Not configured")
    } else if update.available {
        update
            .latest_version
            .as_ref()
            .map(|version| {
                tr("about.update.available_format", "New version %@ available")
                    .replace("%@", version)
            })
            .unwrap_or_else(|| tr("about.update.available", "New version available"))
    } else {
        tr("about.update.latest", "You are up to date")
    };

    div()
        .mt(px(22.0))
        .w(px(312.0))
        .rounded(px(8.0))
        .bg(cx.theme().group_box)
        .border_1()
        .border_color(color(theme::BORDER_SOFT))
        .p(px(12.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(about_info_row(
            tr("about.description", "Description"),
            about.description.clone(),
        ))
        .child(about_info_row(tr("about.updates", "Updates"), update_label))
        .child(about_info_row(
            tr("about.mode", "Mode"),
            update.installation_mode.clone(),
        ))
}

fn about_info_row(label: String, value: String) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(
            div()
                .flex_shrink_0()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(color(theme::TEXT_DIM))
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(color(theme::TEXT_MUTED))
                .child(value),
        )
}

fn about_action_row(locale: &str, cx: &mut Context<CoduxApp>) -> impl IntoElement {
    let tr = |key: &str, fallback: &str| translate(locale, key, fallback);
    div()
        .mt(px(20.0))
        .flex()
        .flex_wrap()
        .justify_center()
        .gap(px(8.0))
        .child(about_button(
            "about-website",
            tr("about.website", "Website"),
            HeroIconName::ArrowTopRightOnSquare,
            cx,
            |app, _event, _window, cx| app.open_codux_website(cx),
        ))
        .child(about_button(
            "about-check-updates",
            tr("about.updates", "Check for Updates"),
            HeroIconName::ArrowPath,
            cx,
            |app, _event, window, cx| app.reload_update(window, cx),
        ))
        .child(about_button(
            "about-install-update",
            tr("about.install_update", "Install Update"),
            HeroIconName::ArrowTopRightOnSquare,
            cx,
            |app, _event, window, cx| app.install_update(window, cx),
        ))
        .child(about_button(
            "about-diagnostics",
            tr("about.diagnostics.export", "Export Diagnostics"),
            HeroIconName::Document,
            cx,
            |app, _event, _window, cx| app.export_diagnostics(cx),
        ))
        .child(about_button(
            "about-runtime-log",
            tr("menu.help.open_runtime_log", "Runtime Log"),
            HeroIconName::Document,
            cx,
            |app, _event, _window, cx| app.open_runtime_log(cx),
        ))
        .child(about_button(
            "about-live-log",
            tr("menu.help.open_live_log", "Live Log"),
            HeroIconName::Document,
            cx,
            |app, _event, _window, cx| app.open_live_log(cx),
        ))
        .child(about_button(
            "about-restart",
            tr("common.restart_now", "Restart Now"),
            HeroIconName::ArrowUturnRight,
            cx,
            |app, _event, _window, cx| app.request_restart(cx),
        ))
}

fn about_button(
    id: &'static str,
    label: String,
    icon: HeroIconName,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> impl IntoElement {
    Button::new(id)
        .secondary()
        .compact()
        .text_color(cx.theme().secondary_foreground)
        .on_click(cx.listener(on_click))
        .child(
            div()
                .h(px(22.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(cx.theme().secondary_foreground)
                .child(Icon::new(icon).size_3())
                .child(label),
        )
}

fn timestamp_slug() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    seconds.to_string()
}
