use super::*;

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
) -> impl IntoElement {
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
                .border_b_1()
                .border_color(color(theme::BORDER_SOFT))
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color(theme::TEXT_MUTED))
                .child(if editable {
                    if dirty {
                        "Editor · modified"
                    } else {
                        "Editor"
                    }
                } else {
                    "Preview"
                }),
        )
        .child(
            div()
                .flex_1()
                .p_3()
                .text_xs()
                .text_color(color(theme::TEXT))
                .children(preview.lines().take(80).map(|line| {
                    div()
                        .child(line.chars().take(120).collect::<String>())
                        .into_any_element()
                })),
        )
}
