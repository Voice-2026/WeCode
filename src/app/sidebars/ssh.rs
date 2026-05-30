use super::*;

pub(in crate::app) fn ssh_section(
    ssh: &SSHSummary,
    selected_profile_id: Option<&str>,
    scroll_handle: UniformListScrollHandle,
    _window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let profiles = Rc::new(ssh.profiles.clone());
    let selected_profile_id = selected_profile_id.map(str::to_string);
    let error_row = ssh.error.as_ref().map(|error| {
        div()
            .mt(px(12.0))
            .p(px(12.0))
            .rounded(px(8.0))
            .bg(ai_stats_surface(cx))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .text_color(color(theme::ACCENT))
            .child(format!("error: {error}"))
            .into_any_element()
    });

    div()
        .flex()
        .flex_1()
        .h_full()
        .min_h_0()
        .flex_col()
        .relative()
        .child(assistant_panel_header(
            "SSH",
            IconName::SquareTerminal,
            header_icon_button(
                "ssh-add-profile",
                IconName::Plus,
                cx,
                |app, _event, window, cx| app.open_ssh_profile_dialog(window, cx),
            ),
        ))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .p(px(12.0))
                .relative()
                .overflow_y_scrollbar()
                .child(if profiles.is_empty() {
                    ssh_empty_state(cx).into_any_element()
                } else {
                    let _ = scroll_handle;
                    div()
                        .flex()
                        .flex_col()
                        .children(profiles.iter().cloned().map(|profile| {
                            ssh_profile_row(profile, selected_profile_id.as_deref(), cx)
                                .into_any_element()
                        }))
                        .into_any_element()
                })
                .children(error_row),
        )
}

fn ssh_empty_state(cx: &mut Context<CoduxApp>) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .text_center()
        .gap(px(10.0))
        .child(
            div()
                .size(px(44.0))
                .rounded(px(12.0))
                .flex()
                .items_center()
                .justify_center()
                .bg(ai_stats_surface(cx))
                .child(
                    Icon::new(IconName::SquareTerminal)
                        .size_5()
                        .text_color(color(theme::TEXT_MUTED)),
                ),
        )
        .child(
            div()
                .text_size(px(13.0))
                .line_height(px(18.0))
                .text_color(color(theme::TEXT_MUTED))
                .child("暂无 SSH 配置"),
        )
}

fn ssh_profile_row(
    profile: SSHProfileSummary,
    selected_profile_id: Option<&str>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let active = selected_profile_id
        .map(|id| id == profile.id)
        .unwrap_or(false);
    let profile_id = profile.id.clone();
    let connect_profile_id = profile.id.clone();
    let right_click_profile_id = profile.id.clone();
    let menu_profile_id = profile.id.clone();
    let hover_surface = ai_stats_track_surface(cx);
    let app_entity = cx.entity();
    div()
        .id(SharedString::from(format!("ssh-profile-{}", profile.id)))
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .mb(px(10.0))
        .p(px(12.0))
        .rounded(px(8.0))
        .bg(if active {
            ai_stats_track_surface(cx)
        } else {
            color(0xFFFFFF).opacity(0.0)
        })
        .cursor_pointer()
        .hover(move |style| style.bg(hover_surface))
        .on_click(cx.listener(move |app, _event, window, cx| {
            app.select_ssh_profile(profile_id.clone(), window, cx)
        }))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |app, _event, window, cx| {
                app.select_ssh_profile(right_click_profile_id.clone(), window, cx)
            }),
        )
        .child(
            div()
                .size(px(40.0))
                .rounded(px(8.0))
                .flex()
                .items_center()
                .justify_center()
                .bg(color(theme::ORANGE).opacity(0.14))
                .child(
                    Icon::new(IconName::SquareTerminal)
                        .size_4()
                        .text_color(color(theme::ORANGE)),
                ),
        )
        .child(
            div()
                .ml(px(12.0))
                .min_w_0()
                .flex()
                .flex_1()
                .flex_col()
                .child(
                    div()
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .text_color(color(theme::TEXT))
                        .truncate()
                        .child(profile.name),
                )
                .child(
                    div()
                        .mt(px(4.0))
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(color(theme::TEXT_MUTED))
                        .truncate()
                        .child(profile.endpoint),
                ),
        )
        .child(
            Button::new(SharedString::from(format!("ssh-connect-{}", profile.id)))
                .compact()
                .ghost()
                .text_color(cx.theme().secondary_foreground)
                .icon(
                    Icon::new(IconName::ExternalLink)
                        .size_3p5()
                        .text_color(cx.theme().secondary_foreground),
                )
                .tooltip("连接")
                .on_click(cx.listener(move |app, _event, window, cx| {
                    cx.stop_propagation();
                    app.select_ssh_profile(connect_profile_id.clone(), window, cx);
                    app.connect_selected_ssh_profile(window, cx);
                })),
        )
        .context_menu(move |menu, _window, _cx| {
            let open_entity = app_entity.clone();
            let open_profile_id = menu_profile_id.clone();
            let edit_entity = app_entity.clone();
            let edit_profile_id = menu_profile_id.clone();
            let remove_entity = app_entity.clone();
            let remove_profile_id = menu_profile_id.clone();

            menu.item(
                PopupMenuItem::new("打开")
                    .icon(IconName::ExternalLink)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&open_entity, |app, cx| {
                            app.select_ssh_profile(open_profile_id.clone(), window, cx);
                            app.connect_selected_ssh_profile(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("编辑")
                    .icon(IconName::CaseSensitive)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&edit_entity, |app, cx| {
                            app.select_ssh_profile(edit_profile_id.clone(), window, cx);
                            app.open_selected_ssh_profile_editor(edit_profile_id.clone(), cx);
                        });
                    }),
            )
            .separator()
            .item(PopupMenuItem::new("移除").icon(IconName::Delete).on_click(
                move |_, window, cx| {
                    cx.update_entity(&remove_entity, |app, cx| {
                        app.select_ssh_profile(remove_profile_id.clone(), window, cx);
                        app.delete_selected_ssh_profile(window, cx);
                    });
                },
            ))
        })
}
