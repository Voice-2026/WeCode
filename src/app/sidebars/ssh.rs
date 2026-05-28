use super::*;

pub(in crate::app) fn ssh_section(
    ssh: &SSHSummary,
    selected_profile_id: Option<&str>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let profile_rows = if ssh.profiles.is_empty() {
        vec![
            div()
                .p(px(12.0))
                .rounded(px(8.0))
                .bg(ai_stats_surface(cx))
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(color(theme::TEXT_DIM))
                .child("暂无 SSH 配置")
                .into_any_element(),
        ]
    } else {
        ssh.profiles
            .iter()
            .cloned()
            .map(|profile| ssh_profile_row(profile, selected_profile_id, cx).into_any_element())
            .collect::<Vec<_>>()
    };
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
        .min_h_0()
        .flex_col()
        .child(assistant_panel_header(
            "SSH",
            IconName::SquareTerminal,
            header_icon_button(
                "ssh-add-profile",
                IconName::Plus,
                cx,
                |app, _event, window, cx| app.open_ssh_settings_window(window, cx),
            ),
        ))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_y_scrollbar()
                .p(px(12.0))
                .flex()
                .flex_col()
                .children(profile_rows)
                .children(error_row),
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
    let hover_surface = ai_stats_track_surface(cx);
    div()
        .id(SharedString::from(format!("ssh-profile-{}", profile.id)))
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
                        .font_weight(FontWeight::SEMIBOLD)
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
}
