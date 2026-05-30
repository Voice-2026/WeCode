use super::*;

#[derive(Clone)]
struct SshCredentialOption {
    value: String,
    label: SharedString,
}

impl SshCredentialOption {
    fn new(value: &'static str, label: &'static str) -> Self {
        Self {
            value: value.to_string(),
            label: SharedString::from(label),
        }
    }
}

impl SelectItem for SshCredentialOption {
    type Value = String;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

fn ssh_credential_options() -> Vec<SshCredentialOption> {
    vec![
        SshCredentialOption::new("none", "无 / SSH Agent"),
        SshCredentialOption::new("password", "密码"),
        SshCredentialOption::new("privateKey", "私钥"),
    ]
}

pub(in crate::app) fn ssh_profile_editor_workspace(
    app: &CoduxApp,
    ssh_testing: bool,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    child_window_shell(if app.ssh_draft_id.is_some() {
        "编辑 SSH 配置"
    } else {
        "添加 SSH 配置"
    })
    .child(
        div()
            .flex_1()
            .min_h_0()
            .overflow_y_scrollbar()
            .p(px(18.0))
            .flex()
            .flex_col()
            .child(ssh_dialog_input(
                "name",
                "名称",
                &app.ssh_draft_name,
                "生产服务器",
                false,
                window,
                cx,
                |app, value, window, cx| app.set_ssh_draft_name(value, window, cx),
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap(px(8.0))
                    .mb(px(16.0))
                    .child(ssh_dialog_input(
                        "host",
                        "主机",
                        &app.ssh_draft_host,
                        "example.com",
                        false,
                        window,
                        cx,
                        |app, value, window, cx| app.set_ssh_draft_host(value, window, cx),
                    ))
                    .child(ssh_dialog_input(
                        "port",
                        "端口",
                        &app.ssh_draft_port,
                        "22",
                        false,
                        window,
                        cx,
                        |app, value, window, cx| app.set_ssh_draft_port(value, window, cx),
                    )),
            )
            .child(ssh_dialog_input(
                "username",
                "用户名",
                &app.ssh_draft_username,
                "root",
                false,
                window,
                cx,
                |app, value, window, cx| app.set_ssh_draft_username(value, window, cx),
            ))
            .child(ssh_dialog_select(
                &app.ssh_draft_credential_kind,
                window,
                cx,
            ))
            .when(app.ssh_draft_credential_kind == "password", |this| {
                this.child(ssh_dialog_input(
                    "password",
                    "密码",
                    &app.ssh_draft_password,
                    "保存到本地",
                    true,
                    window,
                    cx,
                    |app, value, window, cx| app.set_ssh_draft_password(value, window, cx),
                ))
            })
            .when(app.ssh_draft_credential_kind == "privateKey", |this| {
                this.child(ssh_private_key_path_input(
                    &app.ssh_draft_private_key_path,
                    window,
                    cx,
                ))
                .child(ssh_dialog_input(
                    "key-passphrase",
                    "私钥口令",
                    &app.ssh_draft_key_passphrase,
                    "可选，保存到本地",
                    true,
                    window,
                    cx,
                    |app, value, window, cx| app.set_ssh_draft_key_passphrase(value, window, cx),
                ))
            }),
    )
    .child(
        div()
            .h(px(62.0))
            .flex_shrink_0()
            .border_t_1()
            .border_color(color(theme::BORDER_SOFT).opacity(0.45))
            .px(px(18.0))
            .flex()
            .items_center()
            .justify_end()
            .gap(px(12.0))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        Button::new("ssh-editor-cancel")
                            .ghost()
                            .text_color(cx.theme().secondary_foreground)
                            .label("取消")
                            .on_click(cx.listener(|_app, _event, window, _cx| {
                                window.remove_window();
                            })),
                    )
                    .child(
                        Button::new("ssh-editor-test")
                            .secondary()
                            .loading(ssh_testing)
                            .disabled(ssh_testing)
                            .child(ssh_button_label(if ssh_testing {
                                "测试中"
                            } else {
                                "测试"
                            }))
                            .on_click(cx.listener(|app, _event, window, cx| {
                                app.test_ssh_profile_draft(window, cx)
                            })),
                    )
                    .child(
                        Button::new("ssh-editor-save")
                            .primary()
                            .child(ssh_button_label("保存"))
                            .on_click(cx.listener(|app, _event, window, cx| {
                                app.save_ssh_profile_draft(window, cx)
                            })),
                    ),
            ),
    )
}

fn ssh_dialog_input(
    id: &'static str,
    label: &'static str,
    value: &str,
    placeholder: &'static str,
    masked: bool,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
    action: impl Fn(&mut CoduxApp, String, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> impl IntoElement {
    let value = value.to_string();
    let state = window.use_keyed_state(SharedString::from(format!("ssh-input-{id}")), cx, {
        let value = value.clone();
        move |window, cx| {
            InputState::new(window, cx)
                .default_value(value.clone())
                .placeholder(placeholder)
                .masked(masked)
        }
    });
    state.update(cx, |state, cx| {
        if state.value().as_ref() != value.as_str() {
            state.set_value(value.clone(), window, cx);
        }
    });
    cx.subscribe_in(&state, window, move |app, state, event, window, cx| {
        if matches!(event, InputEvent::Change) {
            action(app, state.read(cx).value().to_string(), window, cx);
        }
    })
    .detach();

    div()
        .mb(px(16.0))
        .flex()
        .flex_col()
        .gap(px(5.0))
        .child(
            div()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(color(theme::TEXT_MUTED))
                .child(label),
        )
        .child(Input::new(&state).with_size(Size::Medium))
}

fn ssh_private_key_path_input(
    value: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let value = value.to_string();
    let state = window.use_keyed_state("ssh-input-private-key", cx, {
        let value = value.clone();
        move |window, cx| {
            InputState::new(window, cx)
                .default_value(value.clone())
                .placeholder("~/.ssh/id_ed25519")
        }
    });
    state.update(cx, |state, cx| {
        if state.value().as_ref() != value.as_str() {
            state.set_value(value.clone(), window, cx);
        }
    });
    cx.subscribe_in(&state, window, |app, state, event, window, cx| {
        if matches!(event, InputEvent::Change) {
            app.set_ssh_draft_private_key_path(state.read(cx).value().to_string(), window, cx);
        }
    })
    .detach();

    div()
        .mb(px(16.0))
        .flex()
        .flex_col()
        .gap(px(5.0))
        .child(
            div()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(color(theme::TEXT_MUTED))
                .child("私钥路径"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Input::new(&state).with_size(Size::Medium)),
                )
                .child(
                    Button::new("ssh-editor-choose-key")
                        .secondary()
                        .child(ssh_button_label("选择"))
                        .on_click(cx.listener(|app, _event, window, cx| {
                            app.choose_ssh_private_key_path(window, cx)
                        })),
                ),
        )
}

fn ssh_dialog_select(
    value: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let options = ssh_credential_options();
    let selected_index = options.iter().position(|item| item.value == value);
    let state = window.use_keyed_state("ssh-credential-select", cx, {
        let options = options.clone();
        move |window, cx| {
            SelectState::new(
                options,
                selected_index.map(|row| gpui_component::IndexPath::default().row(row)),
                window,
                cx,
            )
        }
    });
    state.update(cx, |state, cx| {
        let options = ssh_credential_options();
        let selected_index = options.iter().position(|item| item.value == value);
        state.set_items(options, window, cx);
        state.set_selected_index(
            selected_index.map(|row| gpui_component::IndexPath::default().row(row)),
            window,
            cx,
        );
    });
    cx.subscribe_in(&state, window, move |app, _, event, window, cx| {
        let SelectEvent::Confirm(selected) = event;
        if let Some(value) = selected.clone() {
            app.set_ssh_draft_credential_kind(value, window, cx);
        }
    })
    .detach();

    div()
        .mb(px(16.0))
        .flex()
        .flex_col()
        .gap(px(5.0))
        .child(
            div()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(color(theme::TEXT_MUTED))
                .child("凭据方式"),
        )
        .child(
            Select::new(&state)
                .placeholder("选择")
                .menu_width(px(220.0))
                .with_size(Size::Medium),
        )
}

fn ssh_button_label(label: &'static str) -> impl IntoElement {
    div().text_size(px(14.0)).line_height(px(18.0)).child(label)
}
