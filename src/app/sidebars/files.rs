use super::*;
use gpui_component::input::{Input, InputEvent, InputState};

pub(in crate::app) fn file_directory_option(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub(in crate::app) fn current_directory_suffix(value: &str) -> String {
    file_directory_option(value)
        .map(|directory| format!(" / {directory}"))
        .unwrap_or_default()
}

pub(in crate::app) fn parent_relative_directory(value: &str) -> String {
    let mut parts = value
        .split('/')
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>();
    parts.pop();
    parts.join("/")
}

pub(in crate::app) fn file_section(
    project_name: &str,
    files: &[FileEntry],
    tree_children: &HashMap<String, Vec<FileEntry>>,
    expanded_dirs: &HashSet<String>,
    directory_path: &str,
    selected_entry: Option<&str>,
    draft_kind: Option<FileNameDraftKind>,
    draft_value: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let directory_path = directory_path.to_string();
    let has_selected_entry = selected_entry.is_some();
    let rows = file_tree_rows(files, tree_children, expanded_dirs, selected_entry, 0, cx);

    div()
        .flex()
        .min_h_0()
        .flex_col()
        .child(assistant_panel_header(
            "文件",
            IconName::Folder,
            div()
                .flex()
                .items_center()
                .child(assistant_header_icon_button(
                    "file-sidebar-new-file",
                    IconName::File,
                    cx,
                    |app, _event, window, cx| app.create_project_file(window, cx),
                ))
                .child(assistant_header_icon_button(
                    "file-sidebar-new-dir",
                    IconName::FolderClosed,
                    cx,
                    |app, _event, window, cx| app.create_project_directory(window, cx),
                ))
                .child(assistant_header_icon_button(
                    "file-sidebar-import",
                    IconName::ExternalLink,
                    cx,
                    |app, _event, window, cx| app.import_external_file_entries(window, cx),
                ))
                .child(assistant_header_icon_button(
                    "file-sidebar-refresh",
                    IconName::Redo2,
                    cx,
                    |app, _event, window, cx| app.reload_project_files(window, cx),
                ))
                .child(file_actions_menu_button(has_selected_entry, cx)),
        ))
        .child(
            div()
                .flex_1()
                .min_h_0()
                .p(px(12.0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .h(px(26.0))
                        .px(px(10.0))
                        .flex()
                        .items_center()
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(theme::TEXT_MUTED))
                        .truncate()
                        .child(if directory_path.trim().is_empty() {
                            project_name.to_string()
                        } else {
                            format!("{project_name} / {directory_path}")
                        }),
                )
                .child(
                    div()
                        .mt(px(4.0))
                        .when_some(draft_kind, |this, kind| {
                            this.child(file_name_draft_row(kind, draft_value, window, cx))
                        })
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scrollbar()
                        .flex()
                        .flex_col()
                        .child(if files.is_empty() {
                            div()
                                .p(px(10.0))
                                .text_size(px(12.0))
                                .line_height(px(16.0))
                                .text_color(color(theme::TEXT_DIM))
                                .child("暂无文件")
                                .into_any_element()
                        } else {
                            div().flex().flex_col().children(rows).into_any_element()
                        }),
                ),
        )
}

fn file_name_draft_row(
    kind: FileNameDraftKind,
    value: &str,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let value = value.to_string();
    let placeholder = match kind {
        FileNameDraftKind::CreateFile => "filename.txt",
        FileNameDraftKind::CreateDirectory => "folder",
        FileNameDraftKind::Rename => "new name",
    };
    let input_state = window.use_keyed_state(
        SharedString::from(format!("file-name-draft-{kind:?}")),
        cx,
        |window, cx| {
            InputState::new(window, cx)
                .default_value(value.clone())
                .placeholder(placeholder)
        },
    );
    input_state.update(cx, |state, cx| {
        if state.value().as_ref() != value {
            state.set_value(value.clone(), window, cx);
        }
    });
    cx.subscribe_in(&input_state, window, |app, state, event, window, cx| {
        if matches!(event, InputEvent::Change) {
            app.set_file_name_draft_value(state.read(cx).value().to_string(), window, cx);
        }
    })
    .detach();

    div()
        .mb(px(6.0))
        .px(px(6.0))
        .py(px(5.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .bg(ai_stats_surface(cx))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(Input::new(&input_state).with_size(gpui_component::Size::Small)),
        )
        .child(header_icon_button(
            "file-name-draft-confirm",
            IconName::Check,
            cx,
            |app, _event, window, cx| app.confirm_file_name_draft(window, cx),
        ))
        .child(header_icon_button(
            "file-name-draft-cancel",
            IconName::Close,
            cx,
            |app, _event, window, cx| app.cancel_file_name_draft(window, cx),
        ))
}

fn file_actions_menu_button(
    has_selected_entry: bool,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let app_entity = cx.entity();

    Button::new("file-sidebar-actions")
        .compact()
        .ghost()
        .text_color(cx.theme().secondary_foreground)
        .icon(
            Icon::new(IconName::Ellipsis)
                .size_3p5()
                .text_color(cx.theme().secondary_foreground),
        )
        .tooltip("更多")
        .dropdown_menu(move |menu, _window, _cx| {
            let open_entity = app_entity.clone();
            let reveal_entity = app_entity.clone();
            let copy_entity = app_entity.clone();
            let rename_entity = app_entity.clone();
            let delete_entity = app_entity.clone();

            menu.item(
                PopupMenuItem::new("打开")
                    .icon(IconName::ExternalLink)
                    .disabled(!has_selected_entry)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&open_entity, |app, cx| {
                            app.open_selected_file_entry(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("在文件管理器中显示")
                    .icon(IconName::FolderOpen)
                    .disabled(!has_selected_entry)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&reveal_entity, |app, cx| {
                            app.reveal_selected_file_entry(window, cx);
                        });
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new("复制")
                    .icon(IconName::Copy)
                    .disabled(!has_selected_entry)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&copy_entity, |app, cx| {
                            app.copy_selected_file_entry(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("重命名")
                    .icon(IconName::CaseSensitive)
                    .disabled(!has_selected_entry)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&rename_entity, |app, cx| {
                            app.rename_selected_file_entry(window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new("删除")
                    .icon(IconName::Delete)
                    .disabled(!has_selected_entry)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&delete_entity, |app, cx| {
                            app.delete_selected_file_entry(window, cx);
                        });
                    }),
            )
        })
}

fn file_tree_rows(
    files: &[FileEntry],
    tree_children: &HashMap<String, Vec<FileEntry>>,
    expanded_dirs: &HashSet<String>,
    selected_entry: Option<&str>,
    depth: usize,
    cx: &mut Context<CoduxApp>,
) -> Vec<AnyElement> {
    let mut rows = Vec::new();
    for file in files {
        let active = selected_entry
            .map(|path| path == file.relative_path)
            .unwrap_or(false);
        let expanded = expanded_dirs.contains(&file.relative_path);
        rows.push(
            file_tree_entry_row(file.clone(), active, expanded, depth, cx).into_any_element(),
        );
        if expanded {
            if let Some(children) = tree_children.get(&file.relative_path) {
                rows.extend(file_tree_rows(
                    children,
                    tree_children,
                    expanded_dirs,
                    selected_entry,
                    depth + 1,
                    cx,
                ));
            }
        }
    }
    rows
}

fn file_tree_entry_row(
    file: FileEntry,
    active: bool,
    expanded: bool,
    depth: usize,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let entry = file.clone();
    let is_dir = matches!(file.kind, FileKind::Directory);
    let hover_surface = ai_stats_track_surface(cx);
    let icon = if is_dir {
        if expanded {
            IconName::FolderOpen
        } else {
            IconName::FolderClosed
        }
    } else {
        IconName::File
    };
    let indent = px(8.0 + depth as f32 * 14.0);

    div()
        .id(SharedString::from(format!(
            "file-tree-{}",
            file.relative_path
        )))
        .h(px(24.0))
        .pl(indent)
        .pr(px(8.0))
        .flex()
        .items_center()
        .bg(if active {
            hover_surface
        } else {
            color(0xFFFFFF).opacity(0.0)
        })
        .cursor_pointer()
        .hover(move |style| style.bg(hover_surface))
        .on_click(cx.listener(move |app, _event, window, cx| {
            app.open_file_entry(entry.clone(), window, cx)
        }))
        .child(
            div()
                .w(px(18.0))
                .mr(px(4.0))
                .flex()
                .items_center()
                .justify_center()
                .child(if is_dir {
                    Icon::new(if expanded {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                    .size_3()
                    .text_color(color(theme::TEXT_DIM))
                    .into_any_element()
                } else {
                    div().into_any_element()
                }),
        )
        .child(Icon::new(icon).size_3p5().text_color(color(if is_dir {
            theme::ACCENT
        } else {
            theme::TEXT_DIM
        })))
        .child(
            div()
                .ml(px(8.0))
                .min_w_0()
                .text_size(px(14.0))
                .line_height(px(18.0))
                .text_color(color(if is_dir {
                    theme::TEXT_MUTED
                } else {
                    theme::TEXT_DIM
                }))
                .truncate()
                .child(file.name),
        )
}

pub(in crate::app) fn file_preview_workspace(
    preview: &str,
    editable: bool,
    dirty: bool,
    search_open: bool,
    search_query: &str,
    search_match_index: usize,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let match_lines = file_search_match_lines(preview, search_query);
    let active_line = match_lines.get(search_match_index).copied();
    let match_count = match_lines.len();

    div()
        .flex()
        .flex_col()
        .flex_1()
        .child(
            div()
                .h(px(34.0))
                .px_3()
                .flex()
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(color(theme::BORDER_SOFT))
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(theme::TEXT_MUTED))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(if editable {
                            if dirty {
                                "Editor · modified"
                            } else {
                                "Editor"
                            }
                        } else {
                            "Preview"
                        })
                        .when(search_open && !search_query.trim().is_empty(), |this| {
                            this.child(
                                div()
                                    .h(px(20.0))
                                    .px(px(7.0))
                                    .rounded(px(5.0))
                                    .bg(color(theme::TEXT).opacity(0.08))
                                    .text_size(px(12.0))
                                    .line_height(px(16.0))
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(color(theme::TEXT_MUTED))
                                    .flex()
                                    .items_center()
                                    .child(if match_count == 0 {
                                        "0".to_string()
                                    } else {
                                        format!("{}/{}", search_match_index + 1, match_count)
                                    }),
                            )
                        }),
                )
                .child(header_icon_button(
                    "file-preview-search",
                    IconName::Search,
                    cx,
                    |app, _event, _window, cx| app.open_file_search(cx),
                )),
        )
        .when(search_open, |this| {
            this.child(file_preview_search_bar(
                search_query,
                match_count,
                search_match_index,
                editable,
                window,
                cx,
            ))
        })
        .child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_y_scrollbar()
                .p_3()
                .text_xs()
                .text_color(color(theme::TEXT))
                .children(preview.lines().enumerate().map(|(index, line)| {
                    let matched = match_lines.binary_search(&index).is_ok();
                    let active = active_line == Some(index);
                    div()
                        .min_h(px(18.0))
                        .px(px(4.0))
                        .rounded(px(4.0))
                        .when(matched, |this| {
                            this.bg(color(if active { theme::ACCENT } else { theme::TEXT })
                                .opacity(if active { 0.18 } else { 0.08 }))
                        })
                        .child(line.chars().take(160).collect::<String>())
                        .into_any_element()
                })),
        )
}

fn file_preview_search_bar(
    search_query: &str,
    match_count: usize,
    search_match_index: usize,
    editable: bool,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let query_value = search_query.to_string();
    let input_state = window.use_keyed_state("file-preview-search-query", cx, |window, cx| {
        InputState::new(window, cx).placeholder("Find")
    });
    input_state.update(cx, |state, cx| {
        if state.value().as_ref() != query_value {
            state.set_value(query_value.clone(), window, cx);
        }
    });
    cx.subscribe_in(&input_state, window, |app, state, event, window, cx| {
        if matches!(event, InputEvent::Change) {
            app.set_file_search_query(state.read(cx).value().to_string(), window, cx);
        }
    })
    .detach();

    div()
        .flex_shrink_0()
        .px_3()
        .py(px(8.0))
        .border_b_1()
        .border_color(color(theme::BORDER_SOFT))
        .bg(color(theme::BG_HEADER).opacity(0.76))
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .w(px(260.0))
                .child(Input::new(&input_state).with_size(gpui_component::Size::Small)),
        )
        .child(
            div()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(color(theme::TEXT_DIM))
                .min_w(px(58.0))
                .child(if search_query.trim().is_empty() {
                    "Find".to_string()
                } else if match_count == 0 {
                    "0 matches".to_string()
                } else {
                    format!("{}/{}", search_match_index + 1, match_count)
                }),
        )
        .child(header_icon_button(
            "file-preview-search-prev",
            IconName::ChevronUp,
            cx,
            |app, _event, window, cx| app.select_previous_file_search_match(window, cx),
        ))
        .child(header_icon_button(
            "file-preview-search-next",
            IconName::ChevronDown,
            cx,
            |app, _event, window, cx| app.select_next_file_search_match(window, cx),
        ))
        .child(
            div()
                .flex_1()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(color(theme::TEXT_DIM))
                .child(if editable { "" } else { "Read only" }),
        )
        .child(header_icon_button(
            "file-preview-search-close",
            IconName::Close,
            cx,
            |app, _event, window, cx| app.close_file_search(window, cx),
        ))
}

fn file_search_match_lines(preview: &str, query: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }

    preview
        .lines()
        .enumerate()
        .filter_map(|(index, line)| line.to_lowercase().contains(&query).then_some(index))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::file_search_match_lines;

    #[test]
    fn file_search_match_lines_are_case_insensitive_and_limited() {
        let preview = "Alpha\nbeta\nALPHA beta\n";
        assert_eq!(file_search_match_lines(preview, "alpha"), vec![0, 2]);
        assert_eq!(file_search_match_lines(preview, " BETA "), vec![1, 2]);
        assert!(file_search_match_lines(preview, "").is_empty());
    }
}
