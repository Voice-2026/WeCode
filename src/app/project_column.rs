use super::*;

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
                    self.state.selected_project.is_some(),
                    !self.state.projects.is_empty(),
                    self.state.worktrees.selected_worktree_id.is_some(),
                    cx,
                ))
                .into_any_element()
        } else {
            base.flex_col()
                .child(project_tool_button(
                    IconName::Plus,
                    Some("Add"),
                    "project-add-footer",
                    cx,
                    |app, _event, window, cx| app.open_project_create_window(window, cx),
                ))
                .child(project_tool_button(
                    IconName::Settings,
                    Some("Settings"),
                    "project-settings-footer",
                    cx,
                    |app, _event, window, cx| app.open_settings_window(window, cx),
                ))
                .child(project_more_button(
                    Some("More"),
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
    label: Option<&'static str>,
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
    label: Option<&'static str>,
    has_project: bool,
    has_projects: bool,
    has_worktree: bool,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let app_entity = cx.entity();
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
            let reload_entity = app_entity.clone();
            let import_entity = app_entity.clone();
            let rename_entity = app_entity.clone();
            let move_up_entity = app_entity.clone();
            let move_down_entity = app_entity.clone();
            let close_entity = app_entity.clone();
            let close_all_entity = app_entity.clone();
            let sync_entity = app_entity.clone();
            let create_entity = app_entity.clone();
            let merge_entity = app_entity.clone();
            let remove_entity = app_entity.clone();
            let remove_branch_entity = app_entity.clone();

            menu.item(
                PopupMenuItem::new("刷新运行时")
                    .icon(IconName::Redo2)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&reload_entity, |app, cx| {
                            app.reload_runtime_state(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("导入文件夹")
                    .icon(IconName::FolderOpen)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&import_entity, |app, cx| {
                            app.open_project_folder_from_dialog(window, cx);
                        });
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new("编辑项目")
                    .icon(IconName::CaseSensitive)
                    .disabled(!has_project)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&rename_entity, |app, cx| {
                            app.rename_selected_project(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("项目上移")
                    .icon(IconName::ArrowUp)
                    .disabled(!has_project)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&move_up_entity, |app, cx| {
                            app.move_selected_project_up(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("项目下移")
                    .icon(IconName::ArrowDown)
                    .disabled(!has_project)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&move_down_entity, |app, cx| {
                            app.move_selected_project_down(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("关闭项目")
                    .icon(IconName::Close)
                    .disabled(!has_project)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&close_entity, |app, cx| {
                            app.close_selected_project(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("关闭所有项目")
                    .icon(IconName::Close)
                    .disabled(!has_projects)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&close_all_entity, |app, cx| {
                            app.close_all_projects(window, cx);
                        });
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new("同步 Worktree")
                    .icon(IconName::Redo2)
                    .disabled(!has_project)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&sync_entity, |app, cx| {
                            app.sync_worktrees_from_git(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("新建 Worktree")
                    .icon(IconName::Plus)
                    .disabled(!has_project)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&create_entity, |app, cx| {
                            app.create_worktree(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("合并当前 Worktree")
                    .icon(IconName::Undo2)
                    .disabled(!has_worktree)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&merge_entity, |app, cx| {
                            app.merge_selected_worktree(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("移除当前 Worktree")
                    .icon(IconName::Delete)
                    .disabled(!has_worktree)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&remove_entity, |app, cx| {
                            app.remove_selected_worktree(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("移除 Worktree 和分支")
                    .icon(IconName::Delete)
                    .disabled(!has_worktree)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&remove_branch_entity, |app, cx| {
                            app.remove_selected_worktree_and_branch(window, cx);
                        });
                    }),
            )
        })
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
