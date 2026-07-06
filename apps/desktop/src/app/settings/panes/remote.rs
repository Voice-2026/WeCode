use super::options::*;
use super::widgets::*;
use super::*;

pub(super) fn settings_remote_relay_url_editor(
    value: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
    _language: &str,
) -> AnyElement {
    let value = value.to_string();
    let state_key = SharedString::from(format!("settings-remote-relay-url-draft-{value}"));
    let state = window.use_keyed_state(state_key, cx, |window, cx| {
        InputState::new(window, cx).default_value(value.clone())
    });
    cx.subscribe_in(&state, window, |app, _state, event, _window, cx| {
        if matches!(event, InputEvent::Change) {
            app.invalidate_remote_panel(cx);
        }
    })
    .detach();
    let has_changes = state.read(cx).value().as_ref() != value.as_str();
    let input_state = state.clone();
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            Input::new(&state)
                .with_size(gpui_component::Size::Medium)
                .w_full(),
        )
        .when(has_changes, |this| {
            this.child(settings_icon_button_state(
                "settings-remote-relay-url-apply",
                HeroIconName::Check,
                false,
                cx,
                move |app, _event, window, cx| {
                    app.set_remote_relay_url(input_state.read(cx).value().to_string(), window, cx)
                },
            ))
        })
        .into_any_element()
}

pub(super) fn settings_remote_relay_authentication_editor(
    value: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let value = value.to_string();
    let state_key = SharedString::from(format!(
        "settings-remote-relay-authentication-draft-{}",
        value.len()
    ));
    let state = window.use_keyed_state(state_key, cx, |window, cx| {
        InputState::new(window, cx)
            .default_value(value.clone())
            .masked(true)
    });
    cx.subscribe_in(&state, window, |app, _state, event, _window, cx| {
        if matches!(event, InputEvent::Change) {
            app.invalidate_remote_panel(cx);
        }
    })
    .detach();
    let has_changes = state.read(cx).value().as_ref() != value.as_str();
    let input_state = state.clone();
    div()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            Input::new(&state)
                .with_size(gpui_component::Size::Medium)
                .w_full(),
        )
        .when(has_changes, |this| {
            this.child(settings_icon_button_state(
                "settings-remote-relay-authentication-apply",
                HeroIconName::Check,
                false,
                cx,
                move |app, _event, window, cx| {
                    app.set_remote_relay_authentication(
                        input_state.read(cx).value().to_string(),
                        window,
                        cx,
                    )
                },
            ))
        })
        .into_any_element()
}
pub(super) fn remote_pairing_overlay(
    pairing: Option<RemotePairingInfo>,
    loading: bool,
    error: Option<&str>,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let title = settings_text(language, "settings.remote.pairing", "Pairing");
    div()
        .absolute()
        .top(px(0.0))
        .right(px(0.0))
        .bottom(px(0.0))
        .left(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .p(px(16.0))
        .bg(cx.theme().overlay)
        // Block clicks (e.g. on the confirm button) from passing through to the
        // settings content behind the modal backdrop.
        .occlude()
        .child(
            div()
                .w(px(420.0))
                .max_w(relative(1.0))
                .rounded(px(16.0))
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().background)
                .shadow_lg()
                .p(px(20.0))
                .child(
                    div()
                        .text_size(rems(1.125))
                        .line_height(rems(1.5))
                        .text_color(cx.theme().foreground)
                        .child(title),
                )
                .child(
                    div()
                        .mt(px(24.0))
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(if let Some(pairing) = pairing.as_ref() {
                            remote_pairing_qr(&pairing.qr_payload)
                        } else {
                            remote_pairing_placeholder(cx)
                        })
                        .child(remote_pairing_detail(
                            pairing.as_ref(),
                            loading,
                            error,
                            language,
                            cx,
                        )),
                )
                .child(
                    div()
                        .mt(px(24.0))
                        .flex()
                        .gap(px(8.0))
                        .justify_center()
                        .when_some(
                            pairing.as_ref().map(|pairing| pairing.qr_payload.clone()),
                            |row, payload| {
                                row.child(settings_small_button(
                                    "settings-remote-pairing-copy",
                                    settings_text(language, "remote.copyLink", "Copy link"),
                                    cx,
                                    move |app, _event, _window, cx| {
                                        app.copy_remote_pairing_link(payload.clone(), cx)
                                    },
                                ))
                            },
                        )
                        .child(remote_pairing_cancel_button(pairing, language, cx)),
                ),
        )
        .into_any_element()
}

/// The Devices "+" dropdown, using the shared popup-menu component (auto-anchored
/// to the button): Share this device (advertise via QR/link) or Connect to a
/// device (paste another host's ticket).
pub(super) fn remote_add_dropdown(
    language: &str,
    disabled: bool,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let app_entity = cx.entity();
    let share = settings_text(language, "remote.add.share", "Share this device");
    let connect = settings_text(language, "remote.add.connect", "Connect to a device");
    Button::new("settings-remote-add")
        .compact()
        .ghost()
        .disabled(disabled)
        .text_color(cx.theme().secondary_foreground)
        .bg(cx.theme().transparent)
        .icon(
            Icon::new(HeroIconName::Plus)
                .size_3p5()
                .text_color(cx.theme().secondary_foreground),
        )
        .dropdown_menu(move |menu, _window, _cx| {
            let share_entity = app_entity.clone();
            let connect_entity = app_entity.clone();
            menu.item(
                PopupMenuItem::new(share.clone())
                    .icon(HeroIconName::QrCode)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&share_entity, |app, cx| {
                            app.create_remote_pairing(window, cx)
                        });
                    }),
            )
            .item(
                PopupMenuItem::new(connect.clone())
                    .icon(HeroIconName::Link)
                    .on_click(move |_, _window, cx| {
                        cx.update_entity(&connect_entity, |app, cx| app.open_remote_connect(cx));
                    }),
            )
        })
        .into_any_element()
}

/// "Connect to a device" overlay: paste another host's `codux://pair` ticket to
/// pair this desktop to it (controller direction). Mirrors the project-editor
/// pairing panel but lives in Settings → Remote.
pub(super) fn remote_connect_overlay(
    ticket: &str,
    name: &str,
    error: Option<&str>,
    busy: bool,
    language: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let mut card = div()
        .w(px(420.0))
        .rounded(px(16.0))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .shadow_lg()
        .p(px(20.0))
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(
            div()
                .text_size(rems(1.125))
                .line_height(rems(1.5))
                .text_color(cx.theme().foreground)
                .child(settings_text(
                    language,
                    "remote.connect.title",
                    "Connect to a device",
                )),
        )
        .child(
            div()
                .text_size(rems(0.8125))
                .text_color(cx.theme().muted_foreground)
                .child(settings_text(
                    language,
                    "remote.connect.hint",
                    "Paste the codux://pair link from the host. The name below is how this desktop will appear on that host.",
                )),
        )
        .child(settings_textarea(
            "settings-remote-connect-ticket",
            ticket,
            3,
            settings_text(
                language,
                "remote.connect.ticket_placeholder",
                "codux://pair?payload=…",
            ),
            window,
            cx,
            |app, value, window, cx| app.set_remote_connect_ticket(value, window, cx),
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(rems(0.75))
                        .line_height(rems(1.0))
                        .text_color(cx.theme().muted_foreground)
                        .child(settings_text(
                            language,
                            "remote.connect.name_label",
                            "This desktop name",
                        )),
                )
                .child(settings_text_input(
                    "settings-remote-connect-name",
                    name,
                    settings_text(
                        language,
                        "remote.connect.name_placeholder",
                        "This desktop name",
                    ),
                    false,
                    window,
                    cx,
                    |app, value, window, cx| app.set_remote_connect_name(value, window, cx),
                )),
        );
    if let Some(error) = error {
        card = card.child(
            div()
                .text_size(rems(0.8125))
                .text_color(cx.theme().danger)
                .child(error.to_string()),
        );
    }
    let card = card.child(
        div()
            .mt(px(4.0))
            .flex()
            .gap(px(8.0))
            .justify_end()
            .child(settings_small_button(
                "settings-remote-connect-cancel",
                settings_text(language, "common.cancel", "Cancel"),
                cx,
                |app, _event, _window, cx| app.close_remote_connect(cx),
            ))
            .child(
                dialog_primary_button(
                    "settings-remote-connect-submit",
                    settings_text(language, "remote.connect.submit", "Connect"),
                    cx,
                    |app, _event, window, cx| app.submit_remote_connect(window, cx),
                )
                .disabled(busy)
                .loading(busy),
            ),
    );

    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(cx.theme().overlay)
        .occlude()
        .child(card)
        .into_any_element()
}

pub(super) fn remote_pairing_placeholder(cx: &mut Context<CoduxApp>) -> AnyElement {
    div()
        .size(px(242.0))
        .rounded(px(12.0))
        .border_1()
        .border_color(cx.theme().border)
        .bg(color(0xFFFFFF))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .size(px(64.0))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(color(0xF3F4F6))
                .child(
                    Spinner::new()
                        .with_size(px(34.0))
                        .color(color(theme::TEXT_DIM)),
                ),
        )
        .into_any_element()
}

pub(super) fn remote_pairing_detail(
    pairing: Option<&RemotePairingInfo>,
    loading: bool,
    error: Option<&str>,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    if let Some(error) = error.filter(|value| !value.trim().is_empty()) {
        return div()
            .mt(px(18.0))
            .max_w(px(320.0))
            .text_align(gpui::TextAlign::Center)
            .text_size(rems(0.8125))
            .line_height(rems(1.125))
            .text_color(color(theme::RED))
            .child(error.to_string())
            .into_any_element();
    }

    if let Some(pairing) = pairing {
        return div()
            .mt(px(16.0))
            .text_align(gpui::TextAlign::Center)
            .child(
                div()
                    .text_size(rems(0.75))
                    .line_height(rems(1.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(settings_text(
                        language,
                        "settings.remote.waiting_scan",
                        "Waiting for mobile scan...",
                    )),
            )
            .child(
                div()
                    .mt(px(4.0))
                    .text_size(rems(0.75))
                    .line_height(rems(1.0))
                    .text_color(cx.theme().muted_foreground)
                    .child(settings_text(
                        language,
                        "settings.remote.scan_code",
                        "Scan code",
                    )),
            )
            .child(
                div()
                    .mt(px(6.0))
                    .text_size(rems(1.25))
                    .line_height(rems(1.625))
                    .text_color(cx.theme().foreground)
                    .child(pairing.code.clone()),
            )
            .into_any_element();
    }

    div()
        .h(px(54.0))
        .flex()
        .items_center()
        .justify_center()
        .text_size(rems(0.875))
        .line_height(rems(1.25))
        .text_color(cx.theme().muted_foreground)
        .child(if loading {
            settings_text(
                language,
                "settings.remote.creating_pairing",
                "Creating pairing QR...",
            )
        } else {
            settings_text(
                language,
                "settings.remote.configure_hint",
                "Enable Remote Host before pairing mobile devices.",
            )
        })
        .into_any_element()
}

pub(super) fn remote_pairing_cancel_button(
    pairing: Option<RemotePairingInfo>,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    if let Some(pairing) = pairing {
        let pairing_id = pairing.pairing_id;
        return settings_small_button(
            "settings-remote-pairing-cancel",
            settings_text(language, "common.cancel", "Cancel"),
            cx,
            move |app, _event, window, cx| {
                app.cancel_remote_pairing(pairing_id.clone(), window, cx)
            },
        );
    }

    settings_small_button(
        "settings-remote-pairing-close",
        settings_text(language, "common.cancel", "Cancel"),
        cx,
        |app, _event, _window, cx| app.close_remote_pairing_sheet(cx),
    )
}

pub(super) fn remote_pending_pairing_overlay(
    pairing: RemotePendingPairing,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let confirm_id = pairing.id.clone();
    let reject_id = pairing.id.clone();
    div()
        .absolute()
        .top(px(0.0))
        .right(px(0.0))
        .bottom(px(0.0))
        .left(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .p(px(24.0))
        .bg(cx.theme().overlay)
        // Block clicks (e.g. on the confirm button) from passing through to the
        // settings content behind the modal backdrop.
        .occlude()
        .child(
            div()
                .w(px(400.0))
                .max_w(relative(1.0))
                .rounded(px(12.0))
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().background)
                .shadow_lg()
                .p(px(20.0))
                .flex()
                .flex_col()
                .gap(px(18.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(
                            div()
                                .size(px(40.0))
                                .flex_shrink_0()
                                .rounded(px(10.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(cx.theme().primary.opacity(0.14))
                                .child(
                                    Icon::new(HeroIconName::DevicePhoneMobile)
                                        .size_5()
                                        .text_color(cx.theme().primary),
                                ),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(3.0))
                                .child(
                                    div()
                                        .text_size(rems(0.9375))
                                        .line_height(rems(1.25))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(cx.theme().foreground)
                                        .child(settings_text(
                                            language,
                                            "settings.remote.confirm_pairing_title",
                                            "Confirm Device Pairing",
                                        )),
                                )
                                .child(
                                    div()
                                        .text_size(rems(0.75))
                                        .line_height(rems(1.0))
                                        .text_color(cx.theme().muted_foreground)
                                        .child(settings_text(
                                            language,
                                            "settings.remote.confirm_pairing_hint",
                                            "Verify the device and pairing code before confirming.",
                                        )),
                                ),
                        ),
                )
                .child(remote_pending_pairing_details(&pairing, language, cx))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(8.0))
                        .child(
                            Button::new("settings-remote-pending-reject")
                                .ghost()
                                .text_color(cx.theme().danger)
                                .on_click(cx.listener(move |app, _event, window, cx| {
                                    app.reject_remote_pairing(reject_id.clone(), window, cx)
                                }))
                                .child(
                                    div()
                                        .text_size(rems(0.8125))
                                        .line_height(rems(1.125))
                                        .child(settings_text(
                                            language,
                                            "settings.remote.reject_pairing",
                                            "Reject",
                                        )),
                                ),
                        )
                        .child(
                            Button::new("settings-remote-pending-confirm")
                                .primary()
                                .text_color(cx.theme().primary_foreground)
                                .on_click(cx.listener(move |app, _event, window, cx| {
                                    app.confirm_remote_pairing(confirm_id.clone(), window, cx)
                                }))
                                .child(
                                    div()
                                        .text_size(rems(0.8125))
                                        .line_height(rems(1.125))
                                        .child(settings_text(
                                            language,
                                            "common.confirm",
                                            "Confirm",
                                        )),
                                ),
                        ),
                ),
        )
        .into_any_element()
}

pub(super) fn remote_pending_pairing_details(
    pairing: &RemotePendingPairing,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    div()
        .w_full()
        .rounded(px(10.0))
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary)
        .child(remote_pending_pairing_row(
            HeroIconName::DevicePhoneMobile,
            settings_text(language, "settings.remote.device", "Device"),
            div()
                .min_w_0()
                .text_size(rems(0.8125))
                .line_height(rems(1.125))
                .text_color(cx.theme().foreground)
                .truncate()
                .child(empty_label(&pairing.device_name))
                .into_any_element(),
            cx,
        ))
        .child(div().h(px(1.0)).w_full().bg(cx.theme().border))
        .child(remote_pending_pairing_row(
            HeroIconName::LockClosed,
            settings_text(language, "settings.remote.code", "Code"),
            div()
                .px(px(10.0))
                .py(px(3.0))
                .rounded(px(6.0))
                .bg(cx.theme().primary.opacity(0.14))
                .text_size(rems(1.0))
                .line_height(rems(1.25))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(cx.theme().primary)
                .child(pairing.code.clone())
                .into_any_element(),
            cx,
        ))
        .into_any_element()
}

pub(super) fn remote_pending_pairing_row(
    icon: HeroIconName,
    label: String,
    value: AnyElement,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(12.0))
        .py(px(11.0))
        .child(
            Icon::new(icon)
                .size_4()
                .flex_shrink_0()
                .text_color(cx.theme().muted_foreground),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(rems(0.8125))
                .line_height(rems(1.125))
                .text_color(cx.theme().muted_foreground)
                .child(label),
        )
        .child(value)
        .into_any_element()
}

pub(super) fn remote_pairing_qr(payload: &str) -> AnyElement {
    const OUTER_SIZE: f32 = 242.0;
    const QR_SIZE: f32 = 220.0;
    // Pair the trimmed payload with the lowest error-correction level: the QR is
    // shown on a clean screen at close range, so error-correction redundancy buys
    // little and a lower level keeps the version (and module count) down, making
    // the code larger-celled and easier for phones to scan.
    let Ok(code) = QrCode::with_error_correction_level(payload.as_bytes(), EcLevel::L) else {
        return div()
            .size(px(OUTER_SIZE))
            .rounded(px(12.0))
            .bg(color(0xFFFFFF))
            .into_any_element();
    };
    let width = code.width();
    let module_size = QR_SIZE / width as f32;

    div()
        .relative()
        .flex_none()
        .size(px(OUTER_SIZE))
        .rounded(px(12.0))
        .border_1()
        .border_color(color(theme::BORDER_SOFT))
        .bg(color(0xFFFFFF))
        .children(
            code.to_colors()
                .into_iter()
                .enumerate()
                .filter_map(|(index, module)| {
                    if module != QrColor::Dark {
                        return None;
                    }
                    let x = index % width;
                    let y = index / width;
                    Some(
                        div()
                            .absolute()
                            .left(px(11.0 + x as f32 * module_size))
                            .top(px(11.0 + y as f32 * module_size))
                            .size(px(module_size))
                            .bg(color(0x111827))
                            .into_any_element(),
                    )
                }),
        )
        .into_any_element()
}
pub(super) fn settings_remote_pane(
    settings: &SettingsSummary,
    remote: &RemoteSummary,
    saved_hosts: &[codux_runtime::remote::SavedRemoteHost],
    link_states: &std::collections::HashMap<String, codux_runtime::remote::ControllerLinkState>,
    link_paths: &std::collections::HashMap<String, codux_runtime::remote::ControllerLinkPath>,
    _selected_device_id: Option<&str>,
    language: &str,
    remote_reconnecting: bool,
    remote_pairing_creating: bool,
    _window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let mut device_rows: Vec<AnyElement> = remote
        .device_list
        .iter()
        .cloned()
        .map(|device| {
            let device_id = device.id.clone();
            let remove_id = device.id.clone();
            div()
                .id(SharedString::from(format!(
                    "settings-remote-device-{}",
                    device.id
                )))
                .min_h(px(64.0))
                .py(px(12.0))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(18.0))
                .cursor_pointer()
                .on_click(cx.listener(move |app, _event, window, cx| {
                    app.select_remote_device(device_id.clone(), window, cx)
                }))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                // Devices in this segment paired INTO this
                                // machine, so the peer is a controller.
                                .child(settings_status_tag(
                                    settings_text(
                                        language,
                                        "settings.remote.role.controller",
                                        "Controller",
                                    ),
                                    theme::ACCENT,
                                ))
                                .child(
                                    div()
                                        .min_w(px(64.0))
                                        .flex_1()
                                        .text_size(rems(0.9375))
                                        .line_height(rems(1.25))
                                        .text_color(color(theme::TEXT))
                                        .truncate()
                                        .child(empty_label(&device.name)),
                                ),
                        )
                        .child(
                            div()
                                .mt(px(3.0))
                                .text_size(rems(0.75))
                                .line_height(rems(1.0))
                                .text_color(color(theme::TEXT_DIM))
                                .truncate()
                                .child(device_type_label(&device.platform, language)),
                        ),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(if device.online.unwrap_or(false) {
                            settings_status_tag(
                                settings_text(
                                    language,
                                    "remote.status.connected_label",
                                    "Connected",
                                ),
                                theme::GREEN,
                            )
                        } else {
                            settings_status_tag(
                                settings_text(
                                    language,
                                    "remote.status.disconnected_label",
                                    "Disconnected",
                                ),
                                theme::TEXT_DIM,
                            )
                        })
                        .child(settings_icon_button_state(
                            SharedString::from(format!("settings-remote-remove-{}", device.id)),
                            HeroIconName::Trash,
                            false,
                            cx,
                            move |app, _event, window, cx| {
                                app.select_remote_device(remove_id.clone(), window, cx);
                                app.revoke_selected_remote_device(window, cx);
                            },
                        )),
                )
                .into_any_element()
        })
        .collect::<Vec<_>>();

    // Connected hosts (the desktops / headless agents this Mac pairs to as a
    // controller) share the same list, tagged "Host", with a Forget action.
    for host in saved_hosts {
        let device_id = host.device_id.clone();
        let name = if host.host_name.trim().is_empty() {
            host.host_id.clone()
        } else {
            host.host_name.clone()
        };
        device_rows.push(
            div()
                .id(SharedString::from(format!(
                    "settings-remote-host-{}",
                    host.device_id
                )))
                .min_h(px(64.0))
                .py(px(12.0))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(18.0))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                // This machine paired INTO these as a controller,
                                // so the peer is a host.
                                .child(settings_status_tag(
                                    settings_text(language, "settings.remote.role.host", "Host"),
                                    theme::TEXT_DIM,
                                ))
                                .child(
                                    div()
                                        .min_w(px(64.0))
                                        .flex_1()
                                        .text_size(rems(0.9375))
                                        .line_height(rems(1.25))
                                        .text_color(color(theme::TEXT))
                                        .truncate()
                                        .child(empty_label(&name)),
                                ),
                        )
                        .child(
                            div()
                                .mt(px(3.0))
                                .text_size(rems(0.75))
                                .line_height(rems(1.0))
                                .text_color(color(theme::TEXT_DIM))
                                .truncate()
                                .child(device_type_label(&host.platform, language)),
                        ),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(host_link_status_tag(
                            link_states.get(host.device_id.as_str()).copied(),
                            link_paths.get(host.device_id.as_str()).copied(),
                            language,
                        ))
                        .child(settings_icon_button_state(
                            SharedString::from(format!(
                                "settings-remote-forget-{}",
                                host.device_id
                            )),
                            HeroIconName::Trash,
                            false,
                            cx,
                            move |app, _event, _window, cx| {
                                app.forget_remote_host_device(device_id.clone(), cx)
                            },
                        )),
                )
                .into_any_element(),
        );
    }

    if device_rows.is_empty() {
        device_rows.push(
            div()
                .py(px(12.0))
                .text_size(rems(0.875))
                .line_height(rems(1.125))
                .text_color(color(theme::TEXT_DIM))
                .child(if remote.enabled {
                    settings_text(language, "settings.remote.no_devices", "No paired devices.")
                } else {
                    settings_text(
                        language,
                        "remote.devices.empty_hint",
                        "Pair a phone to control terminals on the go.",
                    )
                })
                .into_any_element(),
        );
    }

    div()
        .relative()
        .size_full()
        .child(settings_form(vec![
            settings_card(
                Some(settings_text(
                    language,
                    "settings.remote.server",
                    "Remote Host",
                )),
                None,
                vec![
                    settings_row(
                        settings_text(language, "settings.remote.enabled", "Enable Remote Host"),
                        None,
                        settings_toggle(
                            "settings-remote-enabled",
                            remote.enabled,
                            cx,
                            |app, window, cx| app.toggle_remote_host(window, cx),
                        ),
                    )
                    .into_any_element(),
                    {
                        // The custom URL/auth fields render as sub-content of the
                        // relay row (one card slot), and only when "custom" — so
                        // there's no empty slot drawing a stray separator.
                        let custom = (settings.remote_relay_preset == "custom").then(|| {
                            settings_remote_relay_custom_fields(settings, _window, cx, language)
                        });
                        let relay_row = settings_row(
                            settings_text(language, "settings.remote.relay_mode", "Relay Network"),
                            Some(settings_text(
                                language,
                                "settings.remote.relay_mode.help",
                                "Changing the relay requires pairing again.",
                            )),
                            settings_select_impl(
                                "settings-remote-relay-preset",
                                settings.remote_relay_preset.as_str(),
                                remote_relay_preset_options(language),
                                _window,
                                cx,
                                language,
                                |app, value, window, cx| {
                                    app.set_remote_relay_preset(value, window, cx)
                                },
                            ),
                        );
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child(relay_row)
                            .children(custom)
                            .into_any_element()
                    },
                    div()
                        .py(px(10.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .size(px(8.0))
                                .rounded_full()
                                .bg(color(remote_status_color(remote))),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .text_size(rems(0.75))
                                .line_height(rems(1.0))
                                .text_color(color(theme::TEXT_DIM))
                                .truncate()
                                .child(remote_status_label(remote, language)),
                        )
                        .child(settings_small_button_state(
                            "settings-remote-reconnect",
                            settings_text(language, "settings.remote.reconnect", "Reconnect"),
                            remote_reconnecting,
                            !remote.enabled,
                            cx,
                            |app, _event, window, cx| app.reconnect_remote(window, cx),
                        ))
                        .into_any_element(),
                ],
                cx,
            )
            .into_any_element(),
            remote_mobile_download_banner(language, cx),
            settings_card_with_actions(
                Some(settings_text(
                    language,
                    "settings.remote.devices",
                    "Devices",
                )),
                None,
                Some(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(div().child(remote_add_dropdown(
                            language,
                            remote_pairing_creating || !remote.enabled,
                            cx,
                        )))
                        .child(settings_icon_button_state(
                            "settings-remote-refresh",
                            HeroIconName::ArrowPath,
                            !remote.enabled,
                            cx,
                            |app, _event, window, cx| app.refresh_remote_devices(window, cx),
                        ))
                        .into_any_element(),
                ),
                device_rows,
                cx,
            )
            .into_any_element(),
        ]))
        .into_any_element()
}

pub(super) fn remote_mobile_download_banner(
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> AnyElement {
    let title = settings_text(
        language,
        "settings.remote.mobile_download.title",
        "Codux Mobile",
    );
    let description = settings_text(
        language,
        "settings.remote.mobile_download.description",
        "Download the mobile app to connect Codux and keep AI coding from your phone",
    );
    let action = settings_text(
        language,
        "settings.remote.mobile_download.action",
        "Get Mobile App",
    );

    div()
        .id("settings-remote-mobile-download")
        .min_h(px(72.0))
        .rounded(px(12.0))
        .border_1()
        .border_color(color(theme::ACCENT).opacity(0.28))
        .bg(color(theme::ACCENT).opacity(0.08))
        .px(px(18.0))
        .py(px(14.0))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .cursor_pointer()
        .hover(|style| style.bg(color(theme::ACCENT).opacity(0.12)))
        .on_click(cx.listener(|app, _event, _window, _cx| {
            let _ = app.runtime_service.open_url(CODUX_MOBILE_DOWNLOAD_URL);
        }))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .items_center()
                .gap(px(12.0))
                .child(
                    div()
                        .size(px(36.0))
                        .flex_none()
                        .rounded(px(10.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(color(theme::ACCENT).opacity(0.14))
                        .child(
                            Icon::new(HeroIconName::DevicePhoneMobile)
                                .size_4()
                                .text_color(color(theme::ACCENT)),
                        ),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(rems(0.875))
                                .line_height(rems(1.125))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(color(theme::TEXT))
                                .child(title),
                        )
                        .child(
                            div()
                                .mt(px(3.0))
                                .text_size(rems(0.75))
                                .line_height(rems(1.0625))
                                .text_color(color(theme::TEXT_DIM))
                                .child(description),
                        ),
                ),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.0))
                .text_size(rems(0.75))
                .line_height(rems(1.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(color(theme::ACCENT))
                .child(action)
                .child(
                    Icon::new(HeroIconName::ArrowTopRightOnSquare)
                        .size_3p5()
                        .text_color(color(theme::ACCENT)),
                ),
        )
        .into_any_element()
}
pub(super) fn settings_remote_relay_custom_fields(
    settings: &SettingsSummary,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
    language: &str,
) -> AnyElement {
    if settings.remote_relay_preset != "custom" {
        return div().into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(settings_row(
            settings_text(language, "settings.remote.relay_url", "Custom Relay URL"),
            Some(settings_text(
                language,
                "settings.remote.relay_url.help",
                "Leave empty to use the public network. Pair again after changing it",
            )),
            settings_remote_relay_url_editor(
                settings.remote_relay_url.as_str(),
                window,
                cx,
                language,
            ),
        ))
        .child(settings_row(
            settings_text(
                language,
                "settings.remote.relay_authentication",
                "Relay Authentication",
            ),
            Some(settings_text(
                language,
                "settings.remote.relay_authentication.help",
                "Optional Bearer token for custom Iroh relays. Pair again after changing it",
            )),
            settings_remote_relay_authentication_editor(
                settings.remote_relay_authentication.as_str(),
                window,
                cx,
            ),
        ))
        .into_any_element()
}
