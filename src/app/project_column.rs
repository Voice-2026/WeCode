use super::*;
use codux_runtime::{i18n::translate, settings::locale_from_language_setting};

impl Render for ProjectColumnView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsed = self.collapsed;
        let (projects, selected_project_id) = self.project_store.update(cx, |store, _cx| {
            (store.projects.clone(), store.selected_project_id.clone())
        });
        let app_entity = self.app_entity.clone();
        let scroll_handle = self.scroll_handle.clone();

        div()
            .flex()
            .flex_col()
            .w(px(if collapsed { 80.0 } else { 232.0 }))
            .h_full()
            .bg(color(theme::BG_COLUMN))
            .border_r_1()
            .border_color(color(theme::BORDER_SOFT))
            .child(project_column_header(
                collapsed,
                app_entity.clone(),
                window,
                cx,
            ))
            .child(
                div()
                    .id("project-list-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .px(if collapsed { px(7.0) } else { px(10.0) })
                    .pt(if collapsed { px(10.0) } else { px(10.0) })
                    .pb(if collapsed { px(10.0) } else { px(10.0) })
                    .relative()
                    .overflow_hidden()
                    .child(codux_uniform_list(
                        "project-list",
                        projects,
                        scroll_handle,
                        None,
                        cx,
                        move |project, _index, window, _cx| {
                            let project_id = project.id.clone();
                            let active = selected_project_id
                                .as_deref()
                                .map(|selected| selected == project.id)
                                .unwrap_or(false);
                            div()
                                .w_full()
                                .pb(px(4.0))
                                .child(project_row(
                                    project,
                                    active,
                                    app_entity.clone(),
                                    project_id,
                                    collapsed,
                                    window,
                                ))
                                .into_any_element()
                        },
                    )),
            )
            .child(project_tools_snapshot(
                collapsed,
                self.language.as_str(),
                self.has_project,
                self.has_projects,
                self.has_worktree,
                self.app_entity.clone(),
                window,
                cx,
            ))
    }
}

fn project_column_header(
    collapsed: bool,
    app_entity: gpui::Entity<CoduxApp>,
    window: &mut Window,
    cx: &mut Context<ProjectColumnView>,
) -> impl IntoElement {
    if collapsed {
        div()
            .h(px(74.0))
            .px(px(26.0))
            .pt(px(30.0))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, |_event, window, _cx| {
                window.start_window_move();
            })
            .child(project_column_toggle_button(
                collapsed, app_entity, window, cx,
            ))
            .into_any_element()
    } else {
        div()
            .h(px(44.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .justify_end()
            .on_mouse_down(MouseButton::Left, |_event, window, _cx| {
                window.start_window_move();
            })
            .border_b_1()
            .border_color(color(theme::BORDER_SOFT))
            .child(project_column_toggle_button(
                collapsed, app_entity, window, cx,
            ))
            .into_any_element()
    }
}

fn project_tools_snapshot(
    collapsed: bool,
    language: &str,
    has_project: bool,
    has_projects: bool,
    has_worktree: bool,
    app_entity: gpui::Entity<CoduxApp>,
    window: &mut Window,
    cx: &mut Context<ProjectColumnView>,
) -> AnyElement {
    let base = div()
        .flex()
        .flex_shrink_0()
        .gap(if collapsed { px(10.0) } else { px(4.0) })
        .px(if collapsed { px(20.0) } else { px(10.0) })
        .py_3();

    if collapsed {
        base.flex_col()
            .items_center()
            .child(project_tool_button(
                IconName::Plus,
                None,
                "project-add-footer",
                app_entity.clone(),
                window,
                cx,
                |app, _event, window, cx| app.open_project_create_window(window, cx),
            ))
            .child(project_tool_button(
                IconName::Settings,
                None,
                "project-settings-footer",
                app_entity.clone(),
                window,
                cx,
                |app, _event, window, cx| app.open_settings_window(window, cx),
            ))
            .child(project_more_button(
                None,
                language,
                has_project,
                has_projects,
                has_worktree,
                app_entity,
                cx,
            ))
            .into_any_element()
    } else {
        base.flex_col()
            .items_start()
            .child(project_tool_button(
                IconName::Plus,
                Some(project_column_text(
                    language,
                    "sidebar.footer.add_project",
                    "Add Project",
                )),
                "project-add-footer",
                app_entity.clone(),
                window,
                cx,
                |app, _event, window, cx| app.open_project_create_window(window, cx),
            ))
            .child(project_tool_button(
                IconName::Settings,
                Some(project_column_text(language, "menu.settings", "Settings")),
                "project-settings-footer",
                app_entity.clone(),
                window,
                cx,
                |app, _event, window, cx| app.open_settings_window(window, cx),
            ))
            .child(project_more_button(
                Some(project_column_text(language, "sidebar.footer.help", "Help")),
                language,
                has_project,
                has_projects,
                has_worktree,
                app_entity,
                cx,
            ))
            .into_any_element()
    }
}

fn project_tool_button(
    icon: IconName,
    label: Option<String>,
    id: &'static str,
    app_entity: gpui::Entity<CoduxApp>,
    window: &mut Window,
    cx: &mut Context<ProjectColumnView>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> impl IntoElement {
    let has_label = label.is_some();
    let button = Button::new(SharedString::from(format!("project-tool-{id}")))
        .ghost()
        .text_color(cx.theme().secondary_foreground)
        .w(if has_label { px(212.0) } else { px(40.0) });

    let button = if has_label {
        button.justify_start()
    } else {
        button
    };

    button
        .on_click(window.listener_for(&app_entity, on_click))
        .child(project_tool_content(icon, label, cx))
}

fn project_tool_content(
    icon: IconName,
    label: Option<String>,
    cx: &mut Context<ProjectColumnView>,
) -> AnyElement {
    if let Some(label) = label {
        div()
            .w_full()
            .flex()
            .items_center()
            .justify_start()
            .gap(px(16.0))
            .child(
                div()
                    .w(px(20.0))
                    .flex()
                    .justify_center()
                    .text_color(cx.theme().secondary_foreground)
                    .child(Icon::new(icon).text_color(cx.theme().secondary_foreground)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().secondary_foreground)
                    .child(label),
            )
            .into_any_element()
    } else {
        div()
            .w_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(20.0))
                    .flex()
                    .justify_center()
                    .text_color(cx.theme().secondary_foreground)
                    .child(Icon::new(icon).text_color(cx.theme().secondary_foreground)),
            )
            .into_any_element()
    }
}

fn project_more_button(
    label: Option<String>,
    language: &str,
    _has_project: bool,
    _has_projects: bool,
    _has_worktree: bool,
    app_entity: gpui::Entity<CoduxApp>,
    cx: &mut Context<ProjectColumnView>,
) -> impl IntoElement {
    let language = language.to_string();
    let has_label = label.is_some();
    let button = Button::new("project-tool-project-more-footer")
        .ghost()
        .text_color(cx.theme().secondary_foreground)
        .w(if has_label { px(212.0) } else { px(40.0) });
    let button = if has_label {
        button.justify_start()
    } else {
        button
    };

    button
        .child(project_tool_content(IconName::Ellipsis, label, cx))
        .on_click(move |_, window, cx| {
            let position = window.mouse_position();
            cx.update_entity(&app_entity, |app, cx| {
                app.open_project_help_native_menu(language.clone(), position, window, cx);
            });
        })
}

fn project_column_text(language: &str, key: &str, fallback: &str) -> String {
    let locale = locale_from_language_setting(language);
    translate(&locale, key, fallback)
}

fn project_column_toggle_button(
    collapsed: bool,
    app_entity: gpui::Entity<CoduxApp>,
    window: &mut Window,
    cx: &mut Context<ProjectColumnView>,
) -> impl IntoElement {
    let icon = if collapsed {
        IconName::PanelLeftOpen
    } else {
        IconName::PanelLeftClose
    };
    Button::new("project-column-toggle")
        .ghost()
        .text_color(cx.theme().secondary_foreground)
        .icon(Icon::new(icon).text_color(cx.theme().secondary_foreground))
        .on_click(window.listener_for(&app_entity, |app, _event, window, cx| {
            app.toggle_project_column(window, cx)
        }))
}

fn project_row(
    project: ProjectInfo,
    active: bool,
    app_entity: gpui::Entity<CoduxApp>,
    project_id: String,
    collapsed: bool,
    window: &mut Window,
) -> AnyElement {
    if collapsed {
        return div()
            .id(SharedString::from(format!("project-{}", project.id)))
            .w_full()
            .h(px(44.0))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id(SharedString::from(format!("project-icon-{}", project.id)))
                    .w(px(44.0))
                    .h(px(44.0))
                    .rounded(px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .when(active, |this| this.bg(color(theme::BG_ROW_HOVER)))
                    .hover(|style| style.bg(color(theme::BG_ROW_HOVER)))
                    .on_click(
                        window.listener_for(&app_entity, move |app, _event, window, cx| {
                            app.select_project(project_id.clone(), window, cx)
                        }),
                    )
                    .child(project_icon(&project, active)),
            )
            .into_any_element();
    }

    div()
        .id(SharedString::from(format!("project-{}", project.id)))
        .w_full()
        .min_w_0()
        .h(px(52.0))
        .flex()
        .flex_col()
        .justify_start()
        .child(
            div()
                .id(SharedString::from(format!(
                    "project-row-inner-{}",
                    project.id
                )))
                .flex()
                .items_center()
                .gap_2()
                .h(px(52.0))
                .w_full()
                .min_w_0()
                .px(px(8.0))
                .rounded(px(8.0))
                .when(active, |this| this.bg(color(theme::BG_ROW_HOVER)))
                .cursor_pointer()
                .hover(|style| style.bg(color(theme::BG_ROW_HOVER)))
                .on_click(
                    window.listener_for(&app_entity, move |app, _event, window, cx| {
                        app.select_project(project_id.clone(), window, cx)
                    }),
                )
                .child(project_icon(&project, active))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .min_w_0()
                        .flex_1()
                        .overflow_hidden()
                        .child(
                            div()
                                .text_sm()
                                .text_color(color(if project.exists {
                                    theme::TEXT
                                } else {
                                    theme::TEXT_DIM
                                }))
                                .truncate()
                                .child(project.name.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(color(theme::TEXT_DIM))
                                .truncate()
                                .child(project.path.clone()),
                        ),
                ),
        )
        .into_any_element()
}

fn project_icon(project: &ProjectInfo, active: bool) -> impl IntoElement {
    let (background, _accent, text) = match project
        .badge_color_hex
        .as_deref()
        .and_then(project_icon_hex_color)
    {
        Some(base) => project_custom_icon_palette(base, active),
        None => project_icon_palette(&project.id, active),
    };
    let symbol_icon = project
        .badge_symbol
        .as_deref()
        .and_then(project_badge_symbol_icon);
    let badge = project_badge_label(project);

    div()
        .w(px(36.0))
        .h(px(36.0))
        .rounded(px(8.0))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .bg(color(background))
        .text_size(px(14.0))
        .line_height(px(14.0))
        .text_color(color(text))
        .font_weight(FontWeight::BOLD)
        .child(match symbol_icon {
            Some(icon) => Icon::new(icon)
                .size_4()
                .text_color(color(text))
                .into_any_element(),
            None => div().child(badge).into_any_element(),
        })
}

fn project_icon_palette(key: &str, active: bool) -> (u32, u32, u32) {
    let active_palettes = [
        (0x39D77A, 0x2CC96D, 0xF6FFF9),
        (0x5276E8, 0x4265CC, 0xEEF3FF),
        (0xF18A5C, 0xD96D45, 0xFFF4ED),
        (0x9B72F4, 0x7755D7, 0xF6F1FF),
        (0x35C7D7, 0x269CAD, 0xF0FDFF),
    ];
    let inactive_palettes = [
        (0x4A8664, 0x3A7458, 0xD6EBDD),
        (0x4A63B8, 0x3F56A1, 0xD8DEF6),
        (0xA7694F, 0x8F5A43, 0xF2DCD2),
        (0x7358A8, 0x624B94, 0xE2D9F3),
        (0x44838B, 0x39747D, 0xD8EFF2),
    ];
    let index = key
        .bytes()
        .fold(0usize, |acc, byte| acc.wrapping_add(byte as usize))
        % active_palettes.len();

    if active {
        active_palettes[index]
    } else {
        inactive_palettes[index]
    }
}

fn project_custom_icon_palette(base: u32, active: bool) -> (u32, u32, u32) {
    if active {
        (mix_rgb(base, 0xFFFFFF, 18), base, 0xFFFFFF)
    } else {
        (
            mix_rgb(base, 0x4A5260, 58),
            mix_rgb(base, 0x242A35, 52),
            0xE3E8EF,
        )
    }
}

fn mix_rgb(base: u32, other: u32, other_percent: u8) -> u32 {
    let other_percent = other_percent.min(100) as u32;
    let base_percent = 100 - other_percent;
    let channel = |shift: u32| {
        let base_value = (base >> shift) & 0xFF;
        let other_value = (other >> shift) & 0xFF;
        ((base_value * base_percent + other_value * other_percent) / 100) & 0xFF
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

fn project_icon_hex_color(value: &str) -> Option<u32> {
    let value = value.trim().trim_start_matches('#');
    if value.len() == 6 {
        u32::from_str_radix(value, 16).ok()
    } else {
        None
    }
}

fn project_badge_symbol_icon(symbol: &str) -> Option<IconName> {
    match symbol {
        "terminal" => Some(IconName::SquareTerminal),
        "folder" => Some(IconName::Folder),
        "shippingbox" | "shippingbox.fill" | "cube.box" | "laptopcomputer" => Some(IconName::Bot),
        "hammer" => Some(IconName::Settings2),
        "server.rack" | "globe" => Some(IconName::Globe),
        "bolt" | "sparkles" => Some(IconName::Star),
        "wrench" | "paintpalette" => Some(IconName::Settings),
        "doc.text" => Some(IconName::File),
        "book" => Some(IconName::BookOpen),
        "person.2" => Some(IconName::CircleUser),
        _ => None,
    }
}

fn project_badge_label(project: &ProjectInfo) -> String {
    let badge = project.badge.trim();
    if badge.is_empty() {
        return project_initial(&project.name);
    }
    badge.chars().take(2).collect::<String>().to_uppercase()
}

fn project_initial(name: &str) -> String {
    name.chars()
        .find(|ch| ch.is_alphanumeric())
        .map(|ch| ch.to_uppercase().collect::<String>())
        .unwrap_or_else(|| "C".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with_badge(badge: &str) -> ProjectInfo {
        ProjectInfo {
            id: "project-a".to_string(),
            name: "Project A".to_string(),
            path: "/workspace/project-a".to_string(),
            exists: true,
            badge: badge.to_string(),
            badge_symbol: None,
            badge_color_hex: None,
            git_default_push_remote_name: None,
        }
    }

    #[test]
    fn project_badge_label_prefers_runtime_badge() {
        assert_eq!(project_badge_label(&project_with_badge("cd")), "CD");
        assert_eq!(project_badge_label(&project_with_badge("项目")), "项目");
        assert_eq!(project_badge_label(&project_with_badge(" ")), "P");
    }

    #[test]
    fn project_icon_hex_color_accepts_saved_project_colors() {
        assert_eq!(project_icon_hex_color("#0A84FF"), Some(0x0A84FF));
        assert_eq!(project_icon_hex_color("FFB020"), Some(0xFFB020));
        assert_eq!(project_icon_hex_color("bad"), None);
    }
}
