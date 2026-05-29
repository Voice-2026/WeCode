use super::*;
use codux_runtime::{
    i18n::translate, runtime_paths::app_display_name, settings::locale_from_language_setting,
};

impl CoduxApp {
    pub(super) fn project_column(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let collapsed = self.project_column_collapsed;
        div()
            .flex()
            .flex_col()
            .w(px(if collapsed { 80.0 } else { 232.0 }))
            .h_full()
            .bg(color(theme::BG_COLUMN))
            .border_r_1()
            .border_color(color(theme::BORDER_SOFT))
            .child(project_column_header(collapsed, cx))
            .child(
                div()
                    .id("project-list-scroll")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .px(if collapsed { px(7.0) } else { px(10.0) })
                    .pt(if collapsed { px(10.0) } else { px(10.0) })
                    .overflow_y_scrollbar()
                    .children(self.state.projects.iter().cloned().map(|project| {
                        let project_id = project.id.clone();
                        project_row(
                            &project,
                            self.state
                                .selected_project
                                .as_ref()
                                .map(|selected| selected.id == project.id)
                                .unwrap_or(false),
                            cx,
                            project_id,
                            collapsed,
                        )
                        .into_any_element()
                    })),
            )
            .child(self.project_tools(collapsed, cx))
    }

    fn project_tools(&self, collapsed: bool, cx: &mut Context<Self>) -> impl IntoElement {
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
                    cx,
                    |app, _event, window, cx| app.open_project_create_window(window, cx),
                ))
                .child(project_tool_button(
                    IconName::Settings,
                    None,
                    "project-settings-footer",
                    cx,
                    |app, _event, window, cx| app.open_settings_window(window, cx),
                ))
                .child(project_more_button(
                    None,
                    &self.state.settings.language,
                    self.state.selected_project.is_some(),
                    !self.state.projects.is_empty(),
                    self.state.worktrees.selected_worktree_id.is_some(),
                    cx,
                ))
                .into_any_element()
        } else {
            let language = self.state.settings.language.as_str();
            base.flex_col()
                .child(project_tool_button(
                    IconName::Plus,
                    Some(project_column_text(
                        language,
                        "sidebar.footer.add_project",
                        "Add Project",
                    )),
                    "project-add-footer",
                    cx,
                    |app, _event, window, cx| app.open_project_create_window(window, cx),
                ))
                .child(project_tool_button(
                    IconName::Settings,
                    Some(project_column_text(language, "menu.settings", "Settings")),
                    "project-settings-footer",
                    cx,
                    |app, _event, window, cx| app.open_settings_window(window, cx),
                ))
                .child(project_more_button(
                    Some(project_column_text(language, "sidebar.footer.help", "Help")),
                    language,
                    self.state.selected_project.is_some(),
                    !self.state.projects.is_empty(),
                    self.state.worktrees.selected_worktree_id.is_some(),
                    cx,
                ))
                .into_any_element()
        }
    }
}

fn project_column_header(collapsed: bool, cx: &mut Context<CoduxApp>) -> impl IntoElement {
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
            .child(project_column_toggle_button(collapsed, cx))
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
            .child(project_column_toggle_button(collapsed, cx))
            .into_any_element()
    }
}

fn project_tool_button(
    icon: IconName,
    label: Option<String>,
    id: &'static str,
    cx: &mut Context<CoduxApp>,
    on_click: impl Fn(&mut CoduxApp, &gpui::ClickEvent, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> impl IntoElement {
    let button = Button::new(SharedString::from(format!("project-tool-{id}")))
        .ghost()
        .text_color(cx.theme().secondary_foreground)
        .w(if label.is_some() { px(212.0) } else { px(40.0) });

    let button = if label.is_some() {
        button.justify_start()
    } else {
        button
    };

    button
        .on_click(cx.listener(on_click))
        .child(
            div()
                .w(px(20.0))
                .flex()
                .justify_center()
                .text_color(cx.theme().secondary_foreground)
                .child(Icon::new(icon).text_color(cx.theme().secondary_foreground)),
        )
        .children(label.map(|label| {
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(cx.theme().secondary_foreground)
                .child(label)
                .into_any_element()
        }))
}

fn project_more_button(
    label: Option<String>,
    language: &str,
    _has_project: bool,
    _has_projects: bool,
    _has_worktree: bool,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let app_entity = cx.entity();
    let about_label = project_column_text(language, "menu.app.about_format", "About %@")
        .replace("%@", app_display_name());
    let updates_label = project_column_text(language, "about.updates", "Check for Updates");
    let diagnostics_label = project_column_text(
        language,
        "menu.help.export_diagnostics",
        "Export Diagnostics...",
    );
    let runtime_log_label =
        project_column_text(language, "menu.help.open_runtime_log", "Open Runtime Log");
    let live_log_label = project_column_text(language, "menu.help.open_live_log", "Open Live Log");
    let devtools_label =
        project_column_text(language, "menu.help.developer_tools", "Developer Tools");
    let website_label = project_column_text(language, "menu.help.website", "Website");
    let github_label = project_column_text(language, "menu.help.github", "GitHub");
    let button = Button::new("project-tool-project-more-footer")
        .ghost()
        .text_color(cx.theme().secondary_foreground)
        .w(if label.is_some() { px(212.0) } else { px(40.0) });
    let button = if label.is_some() {
        button.justify_start()
    } else {
        button
    };

    button
        .child(
            div()
                .w(px(20.0))
                .flex()
                .justify_center()
                .text_color(cx.theme().secondary_foreground)
                .child(Icon::new(IconName::Ellipsis).text_color(cx.theme().secondary_foreground)),
        )
        .children(label.map(|label| {
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(cx.theme().secondary_foreground)
                .child(label)
                .into_any_element()
        }))
        .dropdown_menu(move |menu, _window, _cx| {
            let about_entity = app_entity.clone();
            let update_entity = app_entity.clone();
            let diagnostics_entity = app_entity.clone();
            let runtime_log_entity = app_entity.clone();
            let live_log_entity = app_entity.clone();
            let inspector_entity = app_entity.clone();
            let website_entity = app_entity.clone();
            let github_entity = app_entity.clone();

            menu.item(
                PopupMenuItem::new(about_label.clone())
                    .icon(IconName::Info)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&about_entity, |app, cx| {
                            app.open_about_window(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new(updates_label.clone())
                    .icon(IconName::Redo2)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&update_entity, |app, cx| {
                            app.reload_update(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new(diagnostics_label.clone())
                    .icon(IconName::File)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&diagnostics_entity, |app, cx| {
                            let _ = window;
                            app.export_diagnostics(cx);
                        });
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new(runtime_log_label.clone())
                    .icon(IconName::File)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&runtime_log_entity, |app, cx| {
                            let _ = window;
                            app.open_runtime_log(cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new(live_log_label.clone())
                    .icon(IconName::File)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&live_log_entity, |app, cx| {
                            let _ = window;
                            app.open_live_log(cx);
                        });
                    }),
            )
            .when(cfg!(debug_assertions), |menu| {
                menu.item(
                    PopupMenuItem::new(devtools_label.clone())
                        .icon(IconName::Inspector)
                        .on_click(move |_, window, cx| {
                            cx.update_entity(&inspector_entity, |_app, cx| {
                                window.toggle_inspector(cx);
                            });
                        }),
                )
            })
            .separator()
            .item(
                PopupMenuItem::new(website_label.clone())
                    .icon(IconName::ExternalLink)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&website_entity, |app, cx| {
                            let _ = window;
                            app.open_codux_website(cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new(github_label.clone())
                    .icon(IconName::Github)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&github_entity, |app, cx| {
                            let _ = window;
                            app.open_codux_github(cx);
                        });
                    }),
            )
        })
}

fn project_column_text(language: &str, key: &str, fallback: &str) -> String {
    let locale = locale_from_language_setting(language);
    translate(&locale, key, fallback)
}

fn project_column_toggle_button(collapsed: bool, cx: &mut Context<CoduxApp>) -> impl IntoElement {
    header_icon_button(
        "project-column-toggle",
        if collapsed {
            IconName::PanelLeftOpen
        } else {
            IconName::PanelLeftClose
        },
        cx,
        |app, _event, window, cx| app.toggle_project_column(window, cx),
    )
}

fn project_row(
    project: &ProjectInfo,
    active: bool,
    cx: &mut Context<CoduxApp>,
    project_id: String,
    collapsed: bool,
) -> impl IntoElement {
    if collapsed {
        return div()
            .id(SharedString::from(format!("project-{}", project.id)))
            .w_full()
            .h(px(48.0))
            .mb(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .on_click(cx.listener(move |app, _event, window, cx| {
                app.select_project(project_id.clone(), window, cx)
            }))
            .child(
                div()
                    .w(px(46.0))
                    .h(px(46.0))
                    .rounded(px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(color(if active {
                        theme::BG_ROW_HOVER
                    } else {
                        theme::BG_COLUMN
                    }))
                    .hover(|style| {
                        style.bg(color(if active {
                            theme::BG_ROW_HOVER
                        } else {
                            theme::BG_ROW_HOVER
                        }))
                    })
                    .child(project_icon(project, active)),
            );
    }

    div()
        .id(SharedString::from(format!("project-{}", project.id)))
        .flex()
        .items_center()
        .gap_2()
        .h(px(52.0))
        .mb(px(8.0))
        .px(px(8.0))
        .rounded(px(8.0))
        .bg(color(if active {
            theme::BG_ROW_HOVER
        } else {
            theme::BG_COLUMN
        }))
        .cursor_pointer()
        .hover(|style| style.bg(color(theme::BG_ROW_HOVER)))
        .on_click(cx.listener(move |app, _event, window, cx| {
            app.select_project(project_id.clone(), window, cx)
        }))
        .child(project_icon(project, active))
        .child(
            div()
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
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
        )
}

fn project_icon(project: &ProjectInfo, active: bool) -> impl IntoElement {
    let (from, to, text) = match project
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
        .bg(linear_gradient(
            150.0,
            linear_color_stop(color(from), 0.0),
            linear_color_stop(color(to), 1.0),
        ))
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
