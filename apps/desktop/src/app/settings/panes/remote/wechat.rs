use wecode_runtime::wechat_bridge_service::{self, WeChatBridgeSnapshot, WeChatBridgeState};

use super::overlays::remote_pairing_qr;
use super::*;

/// The "WeChat" card in the Remote settings pane: bridge status, QR login,
/// and connect/stop/logout controls. Reads the bridge snapshot on render;
/// [`WeCodeApp::wechat_bridge_watch`] keeps re-rendering while a login or
/// connection attempt is in flight.
pub(in crate::app::settings) fn settings_remote_wechat_card(
    language: &str,
    window: &mut Window,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    let snapshot = wechat_bridge_service::wechat_bridge_snapshot();

    let mut children: Vec<AnyElement> = vec![wechat_status_row(&snapshot, language, cx)];
    for binding in &snapshot.bindings {
        children.extend(wechat_binding_rows(binding, language, window, cx));
    }

    if snapshot.state == WeChatBridgeState::WaitingScan
        || snapshot.state == WeChatBridgeState::Scanned
    {
        if let Some(scan_url) = snapshot.scan_url.as_deref() {
            children.push(
                div()
                    .py(px(12.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(10.0))
                    .child(remote_pairing_qr(scan_url))
                    .child(
                        div()
                            .text_size(rems(0.75))
                            .line_height(rems(1.0625))
                            .text_color(color(theme::TEXT_DIM))
                            .child(settings_text(
                                language,
                                "settings.remote.wechat.scan_hint",
                                "Scan with WeChat to sign in",
                            )),
                    )
                    .into_any_element(),
            );
        }
    }

    if let Some((chat_id, code)) = snapshot.pending_pairing.as_ref() {
        children.push(wechat_pairing_row(chat_id, code, language, cx));
    }

    settings_card(
        Some(settings_text(
            language,
            "settings.remote.wechat.title",
            "WeChat",
        )),
        Some(settings_text(
            language,
            "settings.remote.wechat.description",
            "Sign in with WeChat to drive a terminal session from chat. Messages from paired WeChat users are typed into the bound terminal.",
        )),
        children,
        cx,
    )
    .into_any_element()
}

fn wechat_binding_rows(
    binding: &wechat_bridge_service::WeChatBindingSnapshot,
    language: &str,
    window: &mut Window,
    cx: &mut Context<WeCodeApp>,
) -> Vec<AnyElement> {
    let label = settings_text(
        language,
        "settings.remote.wechat.binding.peer",
        "WeChat {peer}",
    )
    .replace("{peer}", &binding.peer_label);
    let description = binding
        .session_label
        .as_ref()
        .map(|session_label| {
            settings_text(
                language,
                "settings.remote.wechat.binding.session",
                "Bound to {session}",
            )
            .replace("{session}", session_label)
        })
        .unwrap_or_else(|| {
            settings_text(
                language,
                "settings.remote.wechat.binding.session_unavailable",
                "Previous terminal is no longer available",
            )
        });
    let active_action = if binding.active {
        settings_status_tag(
            settings_text(language, "settings.remote.wechat.binding.active", "Active"),
            theme::GREEN,
        )
    } else {
        let chat_id = binding.chat_id.clone();
        settings_small_button_state(
            format!("settings-remote-wechat-activate-{chat_id}"),
            settings_text(language, "settings.remote.wechat.binding.switch", "Switch"),
            false,
            false,
            cx,
            move |app, _event, _window, cx| {
                if wechat_bridge_service::wechat_bridge_set_active_binding(&chat_id) {
                    app.invalidate_ui_region(cx, UiRegion::Root);
                }
            },
        )
    };

    let chat_id = binding.chat_id.clone();
    let note_input = settings_text_input(
        format!("wechat-binding-note-{chat_id}"),
        binding.note.clone().unwrap_or_default(),
        settings_text(
            language,
            "settings.remote.wechat.binding.note_placeholder",
            "Add a note",
        ),
        false,
        window,
        cx,
        move |_app, value, _window, _cx| {
            wechat_bridge_service::wechat_bridge_set_binding_note(&chat_id, &value);
        },
    );

    let actions = div()
        .w(px(232.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(div().w(px(160.0)).child(note_input))
        .child(div().w(px(64.0)).flex().justify_end().child(active_action))
        .into_any_element();

    vec![settings_row(label, Some(description), actions).into_any_element()]
}

fn wechat_status_row(
    snapshot: &WeChatBridgeSnapshot,
    language: &str,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    let (dot, label) = match snapshot.state {
        WeChatBridgeState::Connected => {
            let label = if snapshot.binding_count == 0 {
                settings_text(
                    language,
                    "settings.remote.wechat.connected_unbound",
                    "Connected · no terminal bound",
                )
            } else {
                settings_text(
                    language,
                    "settings.remote.wechat.connected_bound",
                    "Connected · {count} bound",
                )
                .replace("{count}", &snapshot.binding_count.to_string())
            };
            (theme::GREEN, label)
        }
        WeChatBridgeState::WaitingScan => (
            theme::ACCENT,
            settings_text(
                language,
                "settings.remote.wechat.waiting_scan",
                "Waiting for scan…",
            ),
        ),
        WeChatBridgeState::Scanned => (
            theme::ACCENT,
            settings_text(
                language,
                "settings.remote.wechat.scanned",
                "Scanned, confirm on phone…",
            ),
        ),
        WeChatBridgeState::Connecting => (
            theme::ACCENT,
            settings_text(language, "settings.remote.wechat.connecting", "Connecting…"),
        ),
        WeChatBridgeState::Error => (
            theme::RED,
            snapshot.error.clone().unwrap_or_else(|| {
                settings_text(language, "settings.remote.wechat.error", "Error")
            }),
        ),
        WeChatBridgeState::Disconnected => (
            theme::TEXT_DIM,
            settings_text(
                language,
                "settings.remote.wechat.disconnected",
                "Disconnected",
            ),
        ),
    };

    let busy = matches!(
        snapshot.state,
        WeChatBridgeState::WaitingScan | WeChatBridgeState::Scanned | WeChatBridgeState::Connecting
    );

    let mut actions: Vec<AnyElement> = Vec::new();
    match snapshot.state {
        WeChatBridgeState::Connected => {
            actions.push(settings_small_button_state(
                "settings-remote-wechat-stop",
                settings_text(language, "settings.remote.wechat.stop", "Disconnect"),
                false,
                false,
                cx,
                |app, _event, _window, cx| {
                    wecode_runtime::wechat_bridge_service::wechat_bridge_stop();
                    app.wechat_bridge_watch(cx);
                },
            ));
        }
        _ => {
            if snapshot.has_credentials && !busy {
                actions.push(settings_small_button_state(
                    "settings-remote-wechat-resume",
                    settings_text(language, "settings.remote.wechat.resume", "Connect"),
                    false,
                    false,
                    cx,
                    |app, _event, _window, cx| {
                        wecode_runtime::wechat_bridge_service::wechat_bridge_start_saved();
                        app.wechat_bridge_watch(cx);
                    },
                ));
            }
            actions.push(settings_small_button_state(
                "settings-remote-wechat-login",
                if busy {
                    settings_text(language, "settings.remote.wechat.refresh_qr", "Refresh QR")
                } else {
                    settings_text(language, "settings.remote.wechat.login", "Scan to Sign In")
                },
                false,
                false,
                cx,
                |app, _event, _window, cx| {
                    wecode_runtime::wechat_bridge_service::wechat_bridge_begin_login();
                    app.wechat_bridge_watch(cx);
                },
            ));
        }
    }
    if snapshot.has_credentials {
        actions.push(settings_small_button_state(
            "settings-remote-wechat-logout",
            settings_text(language, "settings.remote.wechat.logout", "Sign Out"),
            false,
            false,
            cx,
            |app, _event, _window, cx| {
                wecode_runtime::wechat_bridge_service::wechat_bridge_logout();
                app.wechat_bridge_watch(cx);
            },
        ));
    }

    div()
        .py(px(10.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(div().size(px(8.0)).rounded_full().bg(color(dot)))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_size(rems(0.75))
                .line_height(rems(1.0))
                .text_color(color(theme::TEXT_DIM))
                .truncate()
                .child(label),
        )
        .child(
            div()
                .flex_shrink_0()
                .flex()
                .items_center()
                .gap(px(8.0))
                .children(actions),
        )
        .into_any_element()
}

/// A pending pairing request: peer + code with confirm (bind to the active
/// terminal) and ignore actions. Confirming writes the binding; from then on
/// that peer's messages are typed into the bound session.
fn wechat_pairing_row(
    chat_id: &str,
    code: &str,
    language: &str,
    cx: &mut Context<WeCodeApp>,
) -> AnyElement {
    let peer: String = if chat_id.chars().count() > 12 {
        let head: String = chat_id.chars().take(12).collect();
        format!("{head}…")
    } else {
        chat_id.to_string()
    };
    let prompt = settings_text(
        language,
        "settings.remote.wechat.pairing_request",
        "WeChat user {peer} requests pairing (code {code})",
    )
    .replace("{peer}", &peer)
    .replace("{code}", code);

    div()
        .py(px(10.0))
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_size(rems(0.8125))
                .line_height(rems(1.125))
                .text_color(color(theme::TEXT))
                .child(prompt),
        )
        .child(settings_small_button_state(
            "settings-remote-wechat-pair-confirm",
            settings_text(
                language,
                "settings.remote.wechat.pairing_confirm",
                "Bind to Active Terminal",
            ),
            false,
            false,
            cx,
            |app, _event, _window, cx| app.wechat_confirm_pairing(cx),
        ))
        .child(settings_small_button_state(
            "settings-remote-wechat-pair-dismiss",
            settings_text(language, "settings.remote.wechat.pairing_dismiss", "Ignore"),
            false,
            false,
            cx,
            |app, _event, _window, cx| app.wechat_dismiss_pairing(cx),
        ))
        .into_any_element()
}

impl WeCodeApp {
    /// Mirror async bridge-state transitions (scan, confirm, connect, pairing
    /// requests, errors) into renders. Polls the snapshot once a second and
    /// re-renders only on change, so a long-lived connected bridge costs one
    /// cheap comparison per second. Exits once the bridge settles into
    /// disconnected/error, so nothing runs when the feature is unused.
    pub(in crate::app) fn wechat_bridge_watch(&mut self, cx: &mut Context<Self>) {
        if self.wechat_bridge_watching {
            return;
        }
        self.wechat_bridge_watching = true;
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx| {
            let mut last = wechat_bridge_service::wechat_bridge_snapshot();
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
                let snapshot = wechat_bridge_service::wechat_bridge_snapshot();
                let changed = snapshot.state != last.state
                    || snapshot.scan_url != last.scan_url
                    || snapshot.error != last.error
                    || snapshot.has_credentials != last.has_credentials
                    || snapshot.binding_count != last.binding_count
                    || snapshot.bindings != last.bindings
                    || snapshot.allowlist_count != last.allowlist_count
                    || snapshot.pending_pairing != last.pending_pairing;
                let done = matches!(
                    snapshot.state,
                    WeChatBridgeState::Disconnected | WeChatBridgeState::Error
                );
                last = snapshot;
                let updated = this.update(cx, |app, cx| {
                    if changed {
                        app.invalidate_ui_region(cx, UiRegion::Root);
                    }
                    if done {
                        app.wechat_bridge_watching = false;
                    }
                });
                if updated.is_err() || done {
                    return;
                }
            }
        })
        .detach();
    }
}
