use super::*;
use gpui::{ClickEvent, ListSizingBehavior, Pixels};
use gpui_component::input::{Input, InputEvent, InputState};
use std::ops::Range;

#[derive(Clone)]
struct GitSidebarLabels {
    remote_name: String,
    remote_url: String,
    add_remote: String,
    add: String,
    commit_message: String,
    commit: String,
    commit_push: String,
    commit_sync: String,
    load_last_commit_message: String,
    amend_last_commit: String,
    undo_last_commit: String,
    no_repository: String,
    no_repository_description: String,
    init_repository: String,
    clone_repository: String,
    staged: String,
    staged_empty: String,
    changed: String,
    changed_empty: String,
    untracked: String,
    untracked_empty: String,
    history: String,
    history_empty: String,
    tree_limit: String,
    stage: String,
    unstage: String,
    open_diff: String,
    discard_changes: String,
    add_gitignore: String,
    no_selected_file: String,
    open_file: String,
    empty_diff: String,
    checkout_commit: String,
    revert_commit: String,
    restore_commit: String,
}

impl GitSidebarLabels {
    fn load(language: &str) -> Self {
        let locale = locale_from_language_setting(language);
        let tr = |key: &str, fallback: &str| translate(&locale, key, fallback);
        Self {
            remote_name: tr("git.remote.name", "Remote Name"),
            remote_url: tr("git.remote.add.url_message", "Remote Repository URL"),
            add_remote: tr("git.remote.add", "Add Remote"),
            add: tr("common.add", "Add"),
            commit_message: tr("git.commit.message.placeholder", "Enter Commit Message"),
            commit: tr("git.commit.action", "Commit"),
            commit_push: tr("git.commit.action_push", "Commit and Push"),
            commit_sync: tr("git.commit.action_sync", "Commit and Sync"),
            load_last_commit_message: tr(
                "git.history.edit_last_commit_message",
                "Load Last Commit Message",
            ),
            amend_last_commit: tr(
                "git.history.edit_last_commit_message",
                "Edit Last Commit Message",
            ),
            undo_last_commit: tr("git.history.undo_last_commit", "Undo Last Commit"),
            no_repository: tr("git.empty.no_repository", "No Repository"),
            no_repository_description: tr(
                "git.empty.description",
                "Initialize a repository or clone a remote repository to view commits, diffs, and branches.",
            ),
            init_repository: tr("git.empty.initialize_repository", "Initialize Repository"),
            clone_repository: tr(
                "git.empty.clone_remote_repository",
                "Clone Remote Repository",
            ),
            staged: tr("git.files.staged", "Staged"),
            staged_empty: tr("git.files.staged.empty", "No staged changes"),
            changed: tr("git.files.changes", "Changes"),
            changed_empty: tr("git.files.changes.empty", "No worktree changes"),
            untracked: tr("git.files.untracked", "Untracked"),
            untracked_empty: tr("git.files.untracked.empty", "No untracked files"),
            history: tr("git.history.title", "Git History"),
            history_empty: tr("git.history.empty", "No commit history"),
            tree_limit: tr(
                "git.files.tree_limit_format",
                "Showing the first %@ items. Expand a directory to view its children.",
            ),
            stage: tr("git.files.stage", "Stage"),
            unstage: tr("git.files.unstage", "Unstage"),
            open_diff: tr("git.diff.open", "Open Diff"),
            discard_changes: tr("git.files.discard_changes", "Discard Changes"),
            add_gitignore: tr("git.ignore.add", "Add to .gitignore"),
            no_selected_file: tr("git.diff.select_file", "Select a file to view its diff."),
            open_file: tr("git.diff.open_file", "Open File"),
            empty_diff: tr("git.diff.empty", "No Diff to Display"),
            checkout_commit: tr("git.history.checkout_commit", "Checkout This Commit"),
            revert_commit: tr("git.history.revert_commit", "Revert This Commit"),
            restore_commit: tr("git.history.restore_local", "Restore to This Commit"),
        }
    }
}

pub(in crate::app) fn git_section(
    git: &GitSummary,
    expanded_sections: &HashSet<String>,
    expanded_dirs: &HashSet<String>,
    tree_children: &HashMap<String, Vec<GitFileStatus>>,
    selected_file: Option<&str>,
    selected_files: &HashSet<String>,
    selected_branch: Option<&str>,
    default_push_remote: Option<&str>,
    clone_remote_url: &str,
    language: &str,
    remote_editor_open: bool,
    remote_name: &str,
    remote_url: &str,
    running_operation: Option<&GitRunningOperation>,
    commit_message: &str,
    commit_message_revision: u64,
    files_scroll_handle: VirtualListScrollHandle,
    history_scroll_handle: VirtualListScrollHandle,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let labels = Rc::new(GitSidebarLabels::load(language));
    let branch = if git.branch.trim().is_empty() {
        "HEAD"
    } else {
        git.branch.as_str()
    };

    div()
        .flex()
        .flex_1()
        .h_full()
        .min_h_0()
        .flex_col()
        .child(git_panel_header(
            git,
            branch,
            selected_branch,
            default_push_remote,
            language,
            running_operation,
            cx,
        ))
        .child(if git.is_repository {
            git_repository_panel(
                git,
                expanded_sections,
                expanded_dirs,
                tree_children,
                selected_file,
                selected_files,
                remote_editor_open,
                remote_name,
                remote_url,
                labels.clone(),
                commit_message,
                commit_message_revision,
                files_scroll_handle,
                history_scroll_handle,
                window,
                cx,
            )
            .into_any_element()
        } else {
            git_empty_repository_panel(clone_remote_url, labels, window, cx).into_any_element()
        })
}

fn git_panel_header(
    git: &GitSummary,
    branch: &str,
    _selected_branch: Option<&str>,
    default_push_remote: Option<&str>,
    language: &str,
    running_operation: Option<&GitRunningOperation>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let branches = git.branches.clone();
    let remote_branches = git.remote_branches.clone();
    let remotes = git.remotes.clone();
    let default_push_remote = default_push_remote.map(str::to_string);
    let language = language.to_string();
    let current_branch = branch.to_string();
    let upstream = git.upstream.clone();
    let has_commits = !git.commits.is_empty();
    let app_entity = cx.entity();

    div()
        .h(px(44.0))
        .px_3()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(color(theme::BORDER_SOFT))
        .child(
            div().flex().items_center().min_w_0().child(
                Button::new("git-sidebar-branch-menu")
                    .compact()
                    .ghost()
                    .text_color(cx.theme().foreground)
                    .child(
                        div()
                            .h(px(24.0))
                            .flex()
                            .items_center()
                            .gap_1()
                            .min_w_0()
                            .child(
                                div()
                                    .max_w(px(132.0))
                                    .text_size(px(14.0))
                                    .line_height(px(18.0))
                                    .truncate()
                                    .child(branch.to_string()),
                            )
                            .child(
                                Icon::new(IconName::ChevronDown)
                                    .size_3()
                                    .text_color(color(theme::TEXT_DIM)),
                            ),
                    )
                    .dropdown_menu(move |menu, window, cx| {
                        git_branch_dropdown_menu(
                            menu,
                            window,
                            cx,
                            branches.clone(),
                            remote_branches.clone(),
                            remotes.clone(),
                            default_push_remote.clone(),
                            current_branch.clone(),
                            upstream.clone(),
                            has_commits,
                            language.clone(),
                            app_entity.clone(),
                        )
                    }),
            ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .child(assistant_header_icon_button(
                    "git-sidebar-ai",
                    IconName::Asterisk,
                    cx,
                    |app, _event, window, cx| app.generate_git_commit_message_with_ai(window, cx),
                ))
                .when_some(running_operation, |this, operation| {
                    if operation.cancellable {
                        this.child(assistant_header_icon_button(
                            "git-sidebar-cancel",
                            IconName::CircleX,
                            cx,
                            move |app, _event, window, cx| {
                                app.cancel_project_git(window, cx);
                            },
                        ))
                    } else {
                        this.child(
                            Button::new("git-sidebar-running")
                                .compact()
                                .ghost()
                                .text_color(cx.theme().secondary_foreground)
                                .icon(
                                    Icon::new(IconName::LoaderCircle)
                                        .size_3p5()
                                        .text_color(cx.theme().secondary_foreground),
                                ),
                        )
                    }
                })
                .child(assistant_header_icon_button(
                    "git-sidebar-refresh",
                    IconName::Redo2,
                    cx,
                    |app, _event, window, cx| app.reload_project_git(window, cx),
                )),
        )
}

fn git_branch_dropdown_menu(
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
    branches: Vec<GitBranchSummary>,
    remote_branches: Vec<String>,
    remotes: Vec<GitRemoteSummary>,
    default_push_remote: Option<String>,
    current_branch: String,
    upstream: Option<String>,
    has_commits: bool,
    language: String,
    app_entity: gpui::Entity<CoduxApp>,
) -> PopupMenu {
    let labels = Rc::new(GitBranchMenuLabels::load(&language));
    let create_entity = app_entity.clone();
    let menu = menu
        .item(
            PopupMenuItem::new(labels.new_branch.clone())
                .icon(IconName::Plus)
                .on_click(move |_, window, cx| {
                    cx.update_entity(&create_entity, |app, cx| {
                        app.create_git_branch(window, cx);
                    });
                }),
        )
        .separator();

    let local_branches = branches.clone();
    let local_entity = app_entity.clone();
    let local_labels = labels.clone();
    let menu = menu.submenu_with_icon(
        Some(Icon::new(IconName::Github)),
        labels.local_branches.clone(),
        window,
        cx,
        move |menu, window, cx| {
            if local_branches.is_empty() {
                return menu.item(
                    PopupMenuItem::new(local_labels.local_empty.clone())
                        .icon(IconName::Github)
                        .disabled(true),
                );
            }

            local_branches.iter().take(40).fold(menu, |menu, branch| {
                let branch_name = branch.name.clone();
                let is_current = branch.is_current;
                let submenu_entity = local_entity.clone();
                let submenu_labels = local_labels.clone();
                menu.submenu_with_icon(
                    Some(Icon::new(if is_current {
                        IconName::Check
                    } else {
                        IconName::Github
                    })),
                    branch.name.clone(),
                    window,
                    cx,
                    move |menu, _window, _cx| {
                        let switch_branch = branch_name.clone();
                        let switch_entity = submenu_entity.clone();
                        let merge_branch = branch_name.clone();
                        let merge_entity = submenu_entity.clone();
                        let squash_branch = branch_name.clone();
                        let squash_entity = submenu_entity.clone();
                        let delete_branch = branch_name.clone();
                        let delete_entity = submenu_entity.clone();

                        menu.item(
                            PopupMenuItem::new(submenu_labels.switch_branch.clone())
                                .icon(IconName::Check)
                                .disabled(is_current)
                                .on_click(move |_, window, cx| {
                                    cx.update_entity(&switch_entity, |app, cx| {
                                        app.select_git_branch(switch_branch.clone(), window, cx);
                                        app.checkout_selected_git_branch(window, cx);
                                    });
                                }),
                        )
                        .separator()
                        .item(
                            PopupMenuItem::new(submenu_labels.merge_current.clone())
                                .icon(IconName::Redo2)
                                .disabled(is_current)
                                .on_click(move |_, window, cx| {
                                    cx.update_entity(&merge_entity, |app, cx| {
                                        app.merge_git_branch(merge_branch.clone(), window, cx);
                                    });
                                }),
                        )
                        .item(
                            PopupMenuItem::new(submenu_labels.squash_merge.clone())
                                .icon(IconName::Redo)
                                .disabled(is_current)
                                .on_click(move |_, window, cx| {
                                    cx.update_entity(&squash_entity, |app, cx| {
                                        app.squash_merge_git_branch(
                                            squash_branch.clone(),
                                            window,
                                            cx,
                                        );
                                    });
                                }),
                        )
                        .separator()
                        .item(
                            PopupMenuItem::new(submenu_labels.delete_local.clone())
                                .icon(IconName::Delete)
                                .disabled(is_current)
                                .on_click(move |_, window, cx| {
                                    cx.update_entity(&delete_entity, |app, cx| {
                                        app.select_git_branch(delete_branch.clone(), window, cx);
                                        app.delete_selected_git_branch(window, cx);
                                    });
                                }),
                        )
                    },
                )
            })
        },
    );

    let merge_branches = branches.clone();
    let merge_entity = app_entity.clone();
    let merge_labels = labels.clone();
    let menu = menu.submenu(
        labels.merge_current.clone(),
        window,
        cx,
        move |menu, _window, _cx| {
            let candidates = merge_branches
                .iter()
                .filter(|branch| !branch.is_current)
                .take(40)
                .collect::<Vec<_>>();
            if candidates.is_empty() {
                return menu.item(
                    PopupMenuItem::new(merge_labels.merge_empty.clone())
                        .icon(IconName::Redo2)
                        .disabled(true),
                );
            }

            candidates.into_iter().fold(menu, |menu, branch| {
                let branch_name = branch.name.clone();
                let app_entity = merge_entity.clone();
                menu.item(
                    PopupMenuItem::new(branch.name.clone())
                        .icon(IconName::Redo2)
                        .on_click(move |_, window, cx| {
                            cx.update_entity(&app_entity, |app, cx| {
                                app.merge_git_branch(branch_name.clone(), window, cx);
                            });
                        }),
                )
            })
        },
    );

    let remote_items = remotes.clone();
    let remote_entity = app_entity.clone();
    let default_remote = default_push_remote.clone();
    let push_to_default_remote = default_push_remote.clone();
    let remote_labels = labels.clone();
    let menu = menu.submenu(
        labels.remotes.clone(),
        window,
        cx,
        move |menu, window, cx| {
            let add_entity = remote_entity.clone();
            let menu = menu.item(
                PopupMenuItem::new(remote_labels.add_remote.clone())
                    .icon(IconName::Plus)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&add_entity, |app, cx| {
                            app.open_git_remote_editor(window, cx);
                        });
                    }),
            );

            if remote_items.is_empty() {
                return menu.separator().item(
                    PopupMenuItem::new(remote_labels.no_remotes.clone())
                        .icon(IconName::Globe)
                        .disabled(true),
                );
            }

            remote_items.iter().fold(menu, |menu, remote| {
                let is_default = push_to_default_remote
                    .as_deref()
                    .map(|name| name == remote.name)
                    .unwrap_or(false);
                let remote_name = remote.name.clone();
                let remote_url = remote.url.clone();
                let set_entity = remote_entity.clone();
                let remove_entity = remote_entity.clone();
                let item_labels = remote_labels.clone();
                menu.submenu_with_icon(
                    Some(Icon::new(if is_default {
                        IconName::Check
                    } else {
                        IconName::Globe
                    })),
                    remote.name.clone(),
                    window,
                    cx,
                    move |menu, _window, _cx| {
                        let set_remote = remote_name.clone();
                        let set_entity = set_entity.clone();
                        let remove_remote = remote_name.clone();
                        let remove_entity = remove_entity.clone();
                        let copy_url = remote_url.clone();

                        menu.item(
                            PopupMenuItem::new(item_labels.set_default.clone())
                                .icon(IconName::Check)
                                .checked(is_default)
                                .on_click(move |_, window, cx| {
                                    let next_remote = if is_default {
                                        None
                                    } else {
                                        Some(set_remote.clone())
                                    };
                                    cx.update_entity(&set_entity, |app, cx| {
                                        app.set_project_default_push_remote(
                                            next_remote,
                                            window,
                                            cx,
                                        );
                                    });
                                }),
                        )
                        .separator()
                        .item(
                            PopupMenuItem::new(item_labels.copy_url.clone())
                                .icon(IconName::Copy)
                                .on_click(move |_, _window, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        copy_url.clone(),
                                    ));
                                }),
                        )
                        .item(
                            PopupMenuItem::new(item_labels.remove_remote.clone())
                                .icon(IconName::Delete)
                                .on_click(move |_, window, cx| {
                                    cx.update_entity(&remove_entity, |app, cx| {
                                        app.remove_project_git_remote(
                                            remove_remote.clone(),
                                            window,
                                            cx,
                                        );
                                    });
                                }),
                        )
                    },
                )
            })
        },
    );

    let remote_branch_items = remote_branches.clone();
    let remote_branch_groups = group_remote_branches(&remote_branch_items, upstream.as_deref());
    let remote_branch_entity = app_entity.clone();
    let has_remotes = !remotes.is_empty();
    let can_use_current_branch_remote = upstream.is_some()
        && current_branch != "HEAD"
        && current_branch != "uninitialized"
        && !current_branch.trim().is_empty();
    let remote_branch_labels = labels.clone();
    let menu = menu.submenu(
        labels.remote_branches.clone(),
        window,
        cx,
        move |menu, window, cx| {
            let fetch_entity = remote_branch_entity.clone();
            let menu = menu.item(
                PopupMenuItem::new(remote_branch_labels.refresh_remote_branches.clone())
                    .icon(IconName::Redo2)
                    .disabled(!has_remotes)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&fetch_entity, |app, cx| {
                            app.fetch_project_git(window, cx);
                        });
                    }),
            );

            if remote_branch_groups.is_empty() {
                return menu.separator().item(
                    PopupMenuItem::new(remote_branch_labels.remote_branches_empty.clone())
                        .icon(IconName::ArrowDown)
                        .disabled(true),
                );
            }

            remote_branch_groups
                .iter()
                .fold(menu.separator(), |menu, group| {
                    let group = group.clone();
                    let group_entity = remote_branch_entity.clone();
                    let group_labels = remote_branch_labels.clone();
                    menu.submenu(group.remote.clone(), window, cx, move |menu, window, cx| {
                        group.branches.iter().fold(menu, |menu, branch| {
                            let remote_branch = format!("{}/{}", group.remote, branch.name);
                            let checkout_branch = remote_branch.clone();
                            let checkout_entity = group_entity.clone();
                            let push_branch = remote_branch.clone();
                            let push_entity = group_entity.clone();
                            let branch_labels = group_labels.clone();
                            menu.submenu(
                                branch.name.clone(),
                                window,
                                cx,
                                move |menu, _window, _cx| {
                                    let checkout_branch = checkout_branch.clone();
                                    let checkout_entity = checkout_entity.clone();
                                    let push_branch = push_branch.clone();
                                    let push_entity = push_entity.clone();
                                    menu.item(
                                        PopupMenuItem::new(
                                            branch_labels.checkout_remote_branch.clone(),
                                        )
                                        .icon(IconName::ArrowDown)
                                        .on_click(
                                            move |_, window, cx| {
                                                cx.update_entity(&checkout_entity, |app, cx| {
                                                    app.checkout_git_remote_branch(
                                                        checkout_branch.clone(),
                                                        window,
                                                        cx,
                                                    );
                                                });
                                            },
                                        ),
                                    )
                                    .item(
                                        PopupMenuItem::new(branch_labels.push_here.clone())
                                            .icon(IconName::ArrowUp)
                                            .on_click(move |_, window, cx| {
                                                cx.update_entity(&push_entity, |app, cx| {
                                                    app.push_project_git_remote_branch(
                                                        push_branch.clone(),
                                                        window,
                                                        cx,
                                                    );
                                                });
                                            }),
                                    )
                                },
                            )
                        })
                    })
                })
        },
    );

    let fetch_entity = app_entity.clone();
    let pull_entity = app_entity.clone();
    let push_entity = app_entity.clone();
    let menu = menu
        .separator()
        .item(
            PopupMenuItem::new(labels.fetch.clone())
                .icon(IconName::ArrowDown)
                .disabled(!has_remotes)
                .on_click(move |_, window, cx| {
                    cx.update_entity(&fetch_entity, |app, cx| {
                        app.fetch_project_git(window, cx);
                    });
                }),
        )
        .item(
            PopupMenuItem::new(labels.pull.clone())
                .icon(IconName::ArrowDown)
                .disabled(!can_use_current_branch_remote)
                .on_click(move |_, window, cx| {
                    cx.update_entity(&pull_entity, |app, cx| {
                        app.pull_project_git(window, cx);
                    });
                }),
        )
        .item(
            PopupMenuItem::new(labels.push.clone())
                .icon(IconName::ArrowUp)
                .on_click(move |_, window, cx| {
                    cx.update_entity(&push_entity, |app, cx| {
                        app.push_project_git(window, cx);
                    });
                })
                .disabled(!can_use_current_branch_remote),
        );

    let push_remotes = remotes.clone();
    let push_remote_entity = app_entity.clone();
    let push_to_labels = labels.clone();
    let menu = menu.submenu(
        labels.push_to.clone(),
        window,
        cx,
        move |menu, _window, _cx| {
            if push_remotes.is_empty() {
                return menu.item(
                    PopupMenuItem::new(push_to_labels.no_remotes.clone())
                        .icon(IconName::Globe)
                        .disabled(true),
                );
            }

            push_remotes.iter().fold(menu, |menu, remote| {
                let is_default = default_remote
                    .as_deref()
                    .map(|name| name == remote.name)
                    .unwrap_or(false);
                let remote_name = remote.name.clone();
                let label = if remote.url.trim().is_empty() {
                    remote.name.clone()
                } else {
                    format!("{}\n{}", remote.name, remote.url)
                };
                let app_entity = push_remote_entity.clone();
                menu.item(
                    PopupMenuItem::new(label)
                        .icon(if is_default {
                            IconName::Check
                        } else {
                            IconName::ArrowUp
                        })
                        .on_click(move |_, window, cx| {
                            cx.update_entity(&app_entity, |app, cx| {
                                app.push_project_git_remote(remote_name.clone(), window, cx);
                            });
                        }),
                )
            })
        },
    );

    let force_push_entity = app_entity.clone();
    let undo_entity = app_entity.clone();
    let edit_entity = app_entity.clone();
    let reveal_entity = app_entity.clone();
    menu.separator()
        .item(
            PopupMenuItem::new(labels.force_push.clone())
                .icon(IconName::TriangleAlert)
                .disabled(!can_use_current_branch_remote)
                .on_click(move |_, window, cx| {
                    cx.update_entity(&force_push_entity, |app, cx| {
                        app.force_push_project_git(window, cx);
                    });
                }),
        )
        .item(
            PopupMenuItem::new(labels.undo_last_commit.clone())
                .icon(IconName::Undo2)
                .disabled(!has_commits)
                .on_click(move |_, window, cx| {
                    cx.update_entity(&undo_entity, |app, cx| {
                        app.undo_last_git_commit(window, cx);
                    });
                }),
        )
        .item(
            PopupMenuItem::new(labels.edit_last_commit_message.clone())
                .icon(IconName::Redo)
                .disabled(!has_commits)
                .on_click(move |_, window, cx| {
                    cx.update_entity(&edit_entity, |app, cx| {
                        app.load_last_git_commit_message(window, cx);
                    });
                }),
        )
        .item(
            PopupMenuItem::new(labels.show_repository.clone())
                .icon(IconName::FolderOpen)
                .on_click(move |_, window, cx| {
                    cx.update_entity(&reveal_entity, |app, cx| {
                        app.reveal_selected_project_in_file_manager(window, cx);
                    });
                }),
        )
}

#[derive(Clone)]
struct GitBranchMenuLabels {
    new_branch: String,
    local_branches: String,
    local_empty: String,
    switch_branch: String,
    merge_current: String,
    squash_merge: String,
    delete_local: String,
    merge_empty: String,
    remote_branches: String,
    refresh_remote_branches: String,
    remote_branches_empty: String,
    checkout_remote_branch: String,
    push_here: String,
    remotes: String,
    add_remote: String,
    no_remotes: String,
    set_default: String,
    copy_url: String,
    remove_remote: String,
    fetch: String,
    pull: String,
    push: String,
    push_to: String,
    force_push: String,
    undo_last_commit: String,
    edit_last_commit_message: String,
    show_repository: String,
}

impl GitBranchMenuLabels {
    fn load(language: &str) -> Self {
        let locale = locale_from_language_setting(language);
        let tr = |key: &str, fallback: &str| translate(&locale, key, fallback);
        Self {
            new_branch: tr("git.branch.create_and_switch", "New Branch"),
            local_branches: tr("git.branch.local", "Local Branches"),
            local_empty: tr("git.branch.local.empty", "No local branches"),
            switch_branch: tr("git.branch.switch", "Switch Branch"),
            merge_current: tr("git.branch.merge_current", "Merge into Current Branch"),
            squash_merge: tr(
                "git.branch.squash_merge",
                "Squash Merge into Current Branch",
            ),
            delete_local: tr("git.branch.delete_local", "Delete Local Branch"),
            merge_empty: tr("git.branch.merge.empty", "No branches to merge"),
            remote_branches: tr("git.remote.branches", "Remote Branches"),
            refresh_remote_branches: tr("git.remote.branches.refresh", "Refresh Remote Branches"),
            remote_branches_empty: tr("git.remote.branches.empty", "No remote branches"),
            checkout_remote_branch: tr(
                "git.remote.branch.checkout_local",
                "Checkout as Local Branch",
            ),
            push_here: tr("git.remote.branch.push_here", "Push to This Branch"),
            remotes: tr("git.remote.remotes", "Remotes"),
            add_remote: tr("git.remote.add", "Add Remote"),
            no_remotes: tr("git.remote.empty", "No remotes"),
            set_default: tr("git.remote.set_default", "Set as Default"),
            copy_url: tr("git.remote.copy_url", "Copy URL"),
            remove_remote: tr("git.remote.remove", "Remove Remote"),
            fetch: tr("git.remote.fetch", "Fetch"),
            pull: tr("git.remote.pull", "Pull"),
            push: tr("git.remote.push", "Push"),
            push_to: tr("git.remote.push_to", "Push To..."),
            force_push: tr("git.remote.force_push", "Force Push"),
            undo_last_commit: tr("git.history.undo_last_commit", "Undo Last Commit"),
            edit_last_commit_message: tr(
                "git.history.edit_last_commit_message",
                "Edit Last Commit Message",
            ),
            show_repository: tr(
                "git.repository.show_in_finder",
                "Show Repository in File Manager",
            ),
        }
    }
}

#[derive(Clone)]
struct RemoteBranchGroup {
    remote: String,
    branches: Vec<RemoteBranchItem>,
}

#[derive(Clone)]
struct RemoteBranchItem {
    name: String,
    is_upstream: bool,
}

fn group_remote_branches(values: &[String], upstream: Option<&str>) -> Vec<RemoteBranchGroup> {
    let mut groups: BTreeMap<String, Vec<RemoteBranchItem>> = BTreeMap::new();
    for value in values {
        let Some((remote, branch)) = value.split_once('/') else {
            continue;
        };
        if remote.is_empty() || branch.is_empty() || branch == "HEAD" {
            continue;
        }
        let branches = groups.entry(remote.to_string()).or_default();
        if branches.iter().any(|item| item.name == branch) {
            continue;
        }
        branches.push(RemoteBranchItem {
            name: branch.to_string(),
            is_upstream: upstream == Some(value.as_str()),
        });
    }

    groups
        .into_iter()
        .map(|(remote, mut branches)| {
            branches.sort_by(|left, right| {
                right
                    .is_upstream
                    .cmp(&left.is_upstream)
                    .then_with(|| left.name.cmp(&right.name))
            });
            RemoteBranchGroup { remote, branches }
        })
        .collect()
}

fn git_repository_panel(
    git: &GitSummary,
    expanded_sections: &HashSet<String>,
    expanded_dirs: &HashSet<String>,
    tree_children: &HashMap<String, Vec<GitFileStatus>>,
    selected_file: Option<&str>,
    selected_files: &HashSet<String>,
    remote_editor_open: bool,
    remote_name: &str,
    remote_url: &str,
    labels: Rc<GitSidebarLabels>,
    commit_message: &str,
    commit_message_revision: u64,
    files_scroll_handle: VirtualListScrollHandle,
    history_scroll_handle: VirtualListScrollHandle,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let staged = git
        .changed_files
        .iter()
        .filter(|file| is_git_staged_file(file))
        .cloned()
        .collect::<Vec<_>>();
    let changed = git
        .changed_files
        .iter()
        .filter(|file| is_git_worktree_file(file))
        .cloned()
        .collect::<Vec<_>>();
    let untracked = git
        .changed_files
        .iter()
        .filter(|file| is_git_untracked_file(file))
        .cloned()
        .collect::<Vec<_>>();

    div()
        .flex()
        .flex_1()
        .min_h_0()
        .flex_col()
        .child(git_commit_panel(
            commit_message,
            commit_message_revision,
            labels.clone(),
            window,
            cx,
        ))
        .when(remote_editor_open, |this| {
            this.child(git_remote_editor_panel(
                remote_name,
                remote_url,
                labels.clone(),
                window,
                cx,
            ))
        })
        .child(
            v_resizable("git-sidebar-file-history-split")
                .child(
                    resizable_panel()
                        .size_range(px(160.0)..px(900.0))
                        .child(git_files_panel(
                            &staged,
                            &changed,
                            &untracked,
                            expanded_sections,
                            expanded_dirs,
                            tree_children,
                            selected_file,
                            selected_files,
                            labels.clone(),
                            files_scroll_handle,
                            cx,
                        )),
                )
                .child(
                    resizable_panel()
                        .size(px(260.0))
                        .size_range(px(180.0)..px(420.0))
                        .child(git_history_panel(git, labels, history_scroll_handle, cx)),
                ),
        )
}

fn git_remote_editor_panel(
    remote_name: &str,
    remote_url: &str,
    labels: Rc<GitSidebarLabels>,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let name_value = remote_name.to_string();
    let name_state = window.use_keyed_state("git-remote-name", cx, |window, cx| {
        InputState::new(window, cx)
            .default_value(name_value.clone())
            .placeholder(labels.remote_name.clone())
    });
    name_state.update(cx, |state, cx| {
        if state.value().as_ref() != remote_name {
            state.set_value(remote_name.to_string(), window, cx);
        }
    });
    cx.subscribe_in(&name_state, window, |app, state, event, window, cx| {
        if matches!(event, InputEvent::Change) {
            app.set_git_remote_name(state.read(cx).value().to_string(), window, cx);
        }
    })
    .detach();

    let url_value = remote_url.to_string();
    let url_state = window.use_keyed_state("git-remote-url", cx, |window, cx| {
        InputState::new(window, cx)
            .default_value(url_value.clone())
            .placeholder(labels.remote_url.clone())
    });
    url_state.update(cx, |state, cx| {
        if state.value().as_ref() != remote_url {
            state.set_value(remote_url.to_string(), window, cx);
        }
    });
    cx.subscribe_in(&url_state, window, |app, state, event, window, cx| {
        if matches!(event, InputEvent::Change) {
            app.set_git_remote_url(state.read(cx).value().to_string(), window, cx);
        }
    })
    .detach();

    div()
        .flex_shrink_0()
        .border_b_1()
        .border_color(color(theme::BORDER_SOFT))
        .p(px(12.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .text_color(color(theme::TEXT))
                        .child(labels.add_remote.clone()),
                )
                .child(
                    Button::new("git-remote-editor-close")
                        .compact()
                        .ghost()
                        .text_color(cx.theme().secondary_foreground)
                        .icon(Icon::new(IconName::Close).size_3p5())
                        .on_click(cx.listener(|app, _event, window, cx| {
                            app.close_git_remote_editor(window, cx)
                        })),
                ),
        )
        .child(
            div()
                .mt(px(10.0))
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(96.0))
                        .child(Input::new(&name_state).with_size(gpui_component::Size::Small)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(Input::new(&url_state).with_size(gpui_component::Size::Small)),
                )
                .child(
                    Button::new("git-remote-editor-add")
                        .compact()
                        .secondary()
                        .disabled(remote_name.trim().is_empty() || remote_url.trim().is_empty())
                        .text_color(cx.theme().secondary_foreground)
                        .label(labels.add.clone())
                        .on_click(cx.listener(|app, _event, window, cx| {
                            app.add_project_git_remote(window, cx)
                        })),
                ),
        )
}

fn git_commit_panel(
    commit_message: &str,
    commit_message_revision: u64,
    labels: Rc<GitSidebarLabels>,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let button_bg = color(theme::ACCENT).opacity(0.70);
    let app_entity = cx.entity();
    let value = commit_message.to_string();
    let input_state = window.use_keyed_state(
        SharedString::from(format!("git-commit-message-{commit_message_revision}")),
        cx,
        |window, cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .rows(3)
                .default_value(value.clone())
                .placeholder(labels.commit_message.clone())
        },
    );
    cx.subscribe_in(&input_state, window, |app, state, event, window, cx| {
        if matches!(event, InputEvent::Change) {
            app.set_git_commit_message(state.read(cx).value().to_string(), window, cx);
        }
    })
    .detach();

    div()
        .h(px(162.0))
        .flex_shrink_0()
        .p(px(12.0))
        .border_b_1()
        .border_color(color(theme::BORDER_SOFT))
        .child(
            Input::new(&input_state)
                .with_size(gpui_component::Size::Medium)
                .h(px(86.0)),
        )
        .child(
            div()
                .id("git-sidebar-commit-button")
                .mt(px(12.0))
                .h(px(34.0))
                .rounded(px(8.0))
                .flex()
                .items_center()
                .overflow_hidden()
                .bg(button_bg)
                .text_color(color(0xFFFFFF))
                .cursor_pointer()
                .on_click(cx.listener(|app, _event, window, cx| app.commit_staged_git(window, cx)))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .child(labels.commit.clone()),
                )
                .child(
                    Button::new("git-sidebar-commit-actions")
                        .h(px(34.0))
                        .w(px(44.0))
                        .compact()
                        .primary()
                        .text_color(color(0xFFFFFF))
                        .bg(color(theme::ACCENT).opacity(0.18))
                        .icon(
                            Icon::new(IconName::ChevronDown)
                                .size_3()
                                .text_color(color(0xFFFFFF)),
                        )
                        .dropdown_menu(move |menu, _window, _cx| {
                            let commit_entity = app_entity.clone();
                            let push_entity = app_entity.clone();
                            let sync_entity = app_entity.clone();
                            let load_last_entity = app_entity.clone();
                            let amend_entity = app_entity.clone();
                            let undo_entity = app_entity.clone();
                            menu.item(
                                PopupMenuItem::new(labels.commit.clone())
                                    .icon(IconName::Check)
                                    .on_click(move |_, window, cx| {
                                        cx.update_entity(&commit_entity, |app, cx| {
                                            app.commit_staged_git(window, cx);
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new(labels.commit_push.clone())
                                    .icon(IconName::ArrowUp)
                                    .on_click(move |_, window, cx| {
                                        cx.update_entity(&push_entity, |app, cx| {
                                            app.commit_and_push_git(window, cx);
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new(labels.commit_sync.clone())
                                    .icon(IconName::Redo2)
                                    .on_click(move |_, window, cx| {
                                        cx.update_entity(&sync_entity, |app, cx| {
                                            app.commit_and_sync_git(window, cx);
                                        });
                                    }),
                            )
                            .separator()
                            .item(
                                PopupMenuItem::new(labels.load_last_commit_message.clone())
                                    .icon(IconName::Copy)
                                    .on_click(move |_, window, cx| {
                                        cx.update_entity(&load_last_entity, |app, cx| {
                                            app.load_last_git_commit_message(window, cx);
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new(labels.amend_last_commit.clone())
                                    .icon(IconName::Redo2)
                                    .on_click(move |_, window, cx| {
                                        cx.update_entity(&amend_entity, |app, cx| {
                                            app.amend_last_git_commit(window, cx);
                                        });
                                    }),
                            )
                            .item(
                                PopupMenuItem::new(labels.undo_last_commit.clone())
                                    .icon(IconName::Undo2)
                                    .on_click(move |_, window, cx| {
                                        cx.update_entity(&undo_entity, |app, cx| {
                                            app.undo_last_git_commit(window, cx);
                                        });
                                    }),
                            )
                        }),
                ),
        )
}

fn git_files_panel(
    staged: &[GitFileStatus],
    changed: &[GitFileStatus],
    untracked: &[GitFileStatus],
    expanded_sections: &HashSet<String>,
    expanded_dirs: &HashSet<String>,
    tree_children: &HashMap<String, Vec<GitFileStatus>>,
    selected_file: Option<&str>,
    selected_files: &HashSet<String>,
    labels: Rc<GitSidebarLabels>,
    scroll_handle: VirtualListScrollHandle,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let rows = Rc::new(git_status_virtual_rows(
        staged,
        changed,
        untracked,
        expanded_sections,
        expanded_dirs,
        tree_children,
        selected_file,
        selected_files,
        &labels,
    ));
    let item_sizes = Rc::new(
        rows.iter()
            .map(|row| size(px(1.0), row.height()))
            .collect::<Vec<_>>(),
    );
    div()
        .flex()
        .flex_col()
        .size_full()
        .min_h_0()
        .relative()
        .overflow_hidden()
        .child(
            v_virtual_list(
                cx.entity().clone(),
                "git-files-list",
                item_sizes,
                move |_app, visible_range: Range<usize>, _window, cx| {
                    visible_range
                        .filter_map(|index| {
                            rows.get(index)
                                .cloned()
                                .map(|row: GitStatusVirtualRow| row.render(cx))
                        })
                        .collect::<Vec<_>>()
                },
            )
            .track_scroll(&scroll_handle)
            .with_sizing_behavior(ListSizingBehavior::Auto),
        )
        .vertical_scrollbar(&scroll_handle)
}

fn git_empty_repository_panel(
    clone_remote_url: &str,
    labels: Rc<GitSidebarLabels>,
    window: &mut Window,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let value = clone_remote_url.to_string();
    let input_state = window.use_keyed_state("git-clone-remote-url", cx, |window, cx| {
        InputState::new(window, cx)
            .default_value(value.clone())
            .placeholder(labels.remote_url.clone())
    });
    input_state.update(cx, |state, cx| {
        if state.value().as_ref() != clone_remote_url {
            state.set_value(clone_remote_url.to_string(), window, cx);
        }
    });
    cx.subscribe_in(&input_state, window, |app, state, event, window, cx| {
        if matches!(event, InputEvent::Change) {
            app.set_git_clone_remote_url(state.read(cx).value().to_string(), window, cx);
        }
    })
    .detach();

    div()
        .flex_1()
        .min_h_0()
        .flex()
        .items_center()
        .justify_center()
        .p(px(28.0))
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .text_center()
                .child(
                    div()
                        .size(px(84.0))
                        .rounded_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(color(theme::ORANGE).opacity(0.12))
                        .text_color(color(theme::ORANGE))
                        .child(Icon::new(IconName::Folder).size_8()),
                )
                .child(
                    div()
                        .mt(px(18.0))
                        .text_size(px(18.0))
                        .line_height(px(24.0))
                        .text_color(color(theme::TEXT))
                        .child(labels.no_repository.clone()),
                )
                .child(
                    div()
                        .mt(px(10.0))
                        .max_w(px(280.0))
                        .text_size(px(14.0))
                        .line_height(px(22.0))
                        .text_color(color(theme::TEXT_MUTED))
                        .child(labels.no_repository_description.clone()),
                )
                .child(
                    div()
                        .mt(px(22.0))
                        .w_full()
                        .max_w(px(300.0))
                        .child(Input::new(&input_state).with_size(gpui_component::Size::Medium)),
                )
                .child(
                    div()
                        .mt(px(12.0))
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Button::new("git-init-repo")
                                .primary()
                                .text_color(color(0xFFFFFF))
                                .on_click(cx.listener(|app, _event, window, cx| {
                                    app.init_project_git(window, cx)
                                }))
                                .child(labels.init_repository.clone()),
                        )
                        .child(
                            Button::new("git-clone-repo")
                                .secondary()
                                .text_color(cx.theme().secondary_foreground)
                                .on_click(cx.listener(|app, _event, window, cx| {
                                    app.clone_project_git(window, cx)
                                }))
                                .child(labels.clone_repository.clone()),
                        ),
                ),
        )
}

#[derive(Clone)]
enum GitStatusVirtualRow {
    GroupHeader {
        id: &'static str,
        title: String,
        count: usize,
        files: Vec<GitFileStatus>,
        expanded: bool,
        first: bool,
    },
    Spacer {
        height: f32,
    },
    Empty {
        text: String,
    },
    Dir {
        section_id: &'static str,
        name: String,
        path: String,
        expanded: bool,
        depth: usize,
    },
    File {
        file: GitFileStatus,
        active: bool,
        selected_files: HashSet<String>,
        depth: usize,
        labels: Rc<GitFileMenuLabels>,
    },
    Limit {
        count: usize,
        text: String,
    },
}

const GIT_STATUS_GROUP_TOP_PADDING: f32 = 4.0;
const GIT_STATUS_GROUP_BOTTOM_PADDING: f32 = 8.0;

impl GitStatusVirtualRow {
    fn height(&self) -> Pixels {
        match self {
            Self::GroupHeader { .. } => px(40.0),
            Self::Spacer { height } => px(*height),
            Self::Empty { .. } => px(42.0),
            Self::Dir { .. } | Self::File { .. } => px(24.0),
            Self::Limit { .. } => px(32.0),
        }
    }

    fn render(self, cx: &mut Context<CoduxApp>) -> AnyElement {
        match self {
            Self::GroupHeader {
                id,
                title,
                count,
                files,
                expanded,
                first,
            } => git_status_group_header(id, title, count, files, expanded, first, cx)
                .into_any_element(),
            Self::Spacer { height } => div().h(px(height)).into_any_element(),
            Self::Empty { text } => div()
                .px_3()
                .py_3()
                .text_size(px(14.0))
                .line_height(px(18.0))
                .text_color(color(theme::TEXT_DIM))
                .child(text)
                .into_any_element(),
            Self::Dir {
                section_id,
                name,
                path,
                expanded,
                depth,
            } => {
                git_status_dir_row(section_id, &name, &path, expanded, depth, cx).into_any_element()
            }
            Self::File {
                file,
                active,
                selected_files,
                depth,
                labels,
            } => {
                let selected_path = active.then(|| file.path.clone());
                git_status_file_row(
                    file,
                    selected_path.as_deref(),
                    &selected_files,
                    depth,
                    labels,
                    cx,
                )
                .into_any_element()
            }
            Self::Limit { count, text } => div()
                .px_3()
                .py_2()
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(color(theme::TEXT_DIM))
                .child(text.replace("%@", &count.to_string()))
                .into_any_element(),
        }
    }
}

fn git_status_virtual_rows(
    staged: &[GitFileStatus],
    changed: &[GitFileStatus],
    untracked: &[GitFileStatus],
    expanded_sections: &HashSet<String>,
    expanded_dirs: &HashSet<String>,
    tree_children: &HashMap<String, Vec<GitFileStatus>>,
    selected_file: Option<&str>,
    selected_files: &HashSet<String>,
    labels: &GitSidebarLabels,
) -> Vec<GitStatusVirtualRow> {
    let mut rows = Vec::new();
    let file_menu_labels = Rc::new(GitFileMenuLabels::from(labels));
    append_git_status_group_virtual_rows(
        "staged",
        labels.staged.clone(),
        staged,
        expanded_sections,
        expanded_dirs,
        tree_children,
        selected_file,
        selected_files,
        labels.staged_empty.clone(),
        labels.tree_limit.clone(),
        file_menu_labels.clone(),
        rows.is_empty(),
        &mut rows,
    );
    append_git_status_group_virtual_rows(
        "changed",
        labels.changed.clone(),
        changed,
        expanded_sections,
        expanded_dirs,
        tree_children,
        selected_file,
        selected_files,
        labels.changed_empty.clone(),
        labels.tree_limit.clone(),
        file_menu_labels.clone(),
        rows.is_empty(),
        &mut rows,
    );
    append_git_status_group_virtual_rows(
        "untracked",
        labels.untracked.clone(),
        untracked,
        expanded_sections,
        expanded_dirs,
        tree_children,
        selected_file,
        selected_files,
        labels.untracked_empty.clone(),
        labels.tree_limit.clone(),
        file_menu_labels,
        rows.is_empty(),
        &mut rows,
    );
    rows
}

fn append_git_status_group_virtual_rows(
    id: &'static str,
    title: String,
    files: &[GitFileStatus],
    expanded_sections: &HashSet<String>,
    expanded_dirs: &HashSet<String>,
    tree_children: &HashMap<String, Vec<GitFileStatus>>,
    selected_file: Option<&str>,
    selected_files: &HashSet<String>,
    empty_text: String,
    tree_limit: String,
    file_menu_labels: Rc<GitFileMenuLabels>,
    first: bool,
    rows: &mut Vec<GitStatusVirtualRow>,
) {
    let expanded = expanded_sections.contains(id);
    rows.push(GitStatusVirtualRow::GroupHeader {
        id,
        title,
        count: files.len(),
        files: files.to_vec(),
        expanded,
        first,
    });
    if !expanded {
        return;
    }
    rows.push(GitStatusVirtualRow::Spacer {
        height: GIT_STATUS_GROUP_TOP_PADDING,
    });
    if files.is_empty() {
        rows.push(GitStatusVirtualRow::Empty { text: empty_text });
        rows.push(GitStatusVirtualRow::Spacer {
            height: GIT_STATUS_GROUP_BOTTOM_PADDING,
        });
        return;
    }
    let start_len = rows.len();
    append_git_status_virtual_directory_rows(
        id,
        "",
        files,
        0,
        expanded_dirs,
        tree_children,
        selected_file,
        selected_files,
        file_menu_labels,
        rows,
    );
    let appended = rows.len().saturating_sub(start_len);
    if appended >= MAX_GIT_STATUS_TREE_ROWS {
        rows.push(GitStatusVirtualRow::Limit {
            count: appended,
            text: tree_limit,
        });
    }
    rows.push(GitStatusVirtualRow::Spacer {
        height: GIT_STATUS_GROUP_BOTTOM_PADDING,
    });
}

fn append_git_status_virtual_directory_rows(
    section_id: &'static str,
    base_path: &str,
    files: &[GitFileStatus],
    depth: usize,
    expanded_dirs: &HashSet<String>,
    tree_children: &HashMap<String, Vec<GitFileStatus>>,
    selected_file: Option<&str>,
    selected_files: &HashSet<String>,
    file_menu_labels: Rc<GitFileMenuLabels>,
    rows: &mut Vec<GitStatusVirtualRow>,
) {
    if rows.len() >= MAX_GIT_STATUS_TREE_ROWS {
        return;
    }

    let (dirs, direct_files) = collect_immediate_git_status_entries(section_id, base_path, files);

    for (name, dir) in dirs {
        if rows.len() >= MAX_GIT_STATUS_TREE_ROWS {
            return;
        }
        let tree_key = git_status_tree_key(section_id, &dir.path);
        let expanded = expanded_dirs.contains(&tree_key);
        rows.push(GitStatusVirtualRow::Dir {
            section_id,
            name,
            path: dir.path.clone(),
            expanded,
            depth,
        });
        if expanded {
            if let Some(children) = tree_children.get(&tree_key) {
                append_git_status_virtual_directory_rows(
                    section_id,
                    &dir.path,
                    children,
                    depth + 1,
                    expanded_dirs,
                    tree_children,
                    selected_file,
                    selected_files,
                    file_menu_labels.clone(),
                    rows,
                );
            }
        }
    }
    for file in direct_files {
        if rows.len() >= MAX_GIT_STATUS_TREE_ROWS {
            return;
        }
        let active = selected_file
            .map(|path| path == file.path.as_str())
            .unwrap_or(false);
        rows.push(GitStatusVirtualRow::File {
            file,
            active,
            selected_files: selected_files.clone(),
            depth,
            labels: file_menu_labels.clone(),
        });
    }
}

fn git_status_group_header(
    id: &'static str,
    title: String,
    count: usize,
    _files: Vec<GitFileStatus>,
    expanded: bool,
    first: bool,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("git-status-group-{id}")))
        .w_full()
        .min_w_0()
        .h(px(40.0))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .border_color(cx.theme().border)
        .when(!first, |this| this.border_t_1())
        .bg(cx.theme().list_head)
        .cursor_pointer()
        .on_click(
            cx.listener(move |app, _event, _window, cx| app.toggle_git_status_section(id, cx)),
        )
        .child(
            div()
                .flex()
                .flex_1()
                .items_center()
                .min_w_0()
                .gap_2()
                .child(
                    Icon::new(if expanded {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                    .size_3p5()
                    .text_color(color(theme::TEXT_DIM)),
                )
                .child(
                    div()
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .text_color(cx.theme().muted_foreground)
                        .child(title),
                )
                .child(
                    div()
                        .px_1p5()
                        .h(px(18.0))
                        .min_w(px(18.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(5.0))
                        .bg(cx.theme().secondary)
                        .text_size(px(12.0))
                        .line_height(px(14.0))
                        .text_color(cx.theme().muted_foreground)
                        .child(count.to_string()),
                ),
        )
}

struct GitImmediateDir {
    path: String,
    count: usize,
}

const MAX_GIT_STATUS_TREE_ROWS: usize = 600;

fn collect_immediate_git_status_entries(
    section_id: &'static str,
    base_path: &str,
    files: &[GitFileStatus],
) -> (BTreeMap<String, GitImmediateDir>, Vec<GitFileStatus>) {
    let mut dirs = BTreeMap::<String, GitImmediateDir>::new();
    let mut direct_files = Vec::<GitFileStatus>::new();
    for file in files {
        if !git_status_matches_section(section_id, file) {
            continue;
        }
        let Some(relative_path) = relative_git_status_path(base_path, &file.path) else {
            continue;
        };
        let relative_path = relative_path.trim_end_matches('/');
        if relative_path.is_empty() {
            continue;
        }
        if let Some((dir_name, _rest)) = relative_path.split_once('/') {
            let dir_path = join_git_path(base_path, dir_name);
            dirs.entry(dir_name.to_string())
                .and_modify(|dir| dir.count += 1)
                .or_insert(GitImmediateDir {
                    path: dir_path,
                    count: 1,
                });
        } else if file.path.ends_with('/') {
            let dir_path = join_git_path(base_path, relative_path);
            dirs.entry(relative_path.to_string())
                .and_modify(|dir| dir.count += 1)
                .or_insert(GitImmediateDir {
                    path: dir_path,
                    count: 1,
                });
        } else {
            direct_files.push(file.clone());
        }
    }
    (dirs, direct_files)
}

fn git_status_matches_section(section_id: &'static str, file: &GitFileStatus) -> bool {
    match section_id {
        "staged" => is_git_staged_file(file),
        "changed" => is_git_worktree_file(file),
        "untracked" => is_git_untracked_file(file),
        _ => true,
    }
}

fn relative_git_status_path<'a>(base_path: &str, file_path: &'a str) -> Option<&'a str> {
    let base_path = base_path.trim_matches('/');
    if base_path.is_empty() {
        return Some(file_path);
    }
    file_path
        .strip_prefix(base_path)
        .and_then(|path| path.strip_prefix('/'))
}

fn join_git_path(base_path: &str, name: &str) -> String {
    let base_path = base_path.trim_matches('/');
    if base_path.is_empty() {
        name.to_string()
    } else {
        format!("{base_path}/{name}")
    }
}

fn git_status_tree_key(section_id: &str, path: &str) -> String {
    format!("{section_id}:{}", path.trim_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_file(path: &str, index_status: &str, worktree_status: &str) -> GitFileStatus {
        GitFileStatus {
            path: path.to_string(),
            index_status: index_status.to_string(),
            worktree_status: worktree_status.to_string(),
        }
    }

    #[test]
    fn git_tree_collects_only_immediate_rows_for_current_directory() {
        let files = vec![
            git_file("src/main.rs", " ", "M"),
            git_file("src/nested/lib.rs", " ", "M"),
            git_file("README.md", " ", "M"),
            git_file("bulk/", "?", "?"),
        ];

        let (root_dirs, root_files) = collect_immediate_git_status_entries("changed", "", &files);
        assert_eq!(root_dirs.keys().cloned().collect::<Vec<_>>(), vec!["src"]);
        assert_eq!(
            root_files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            vec!["README.md"]
        );

        let (src_dirs, src_files) = collect_immediate_git_status_entries("changed", "src", &files);
        assert_eq!(src_dirs.keys().cloned().collect::<Vec<_>>(), vec!["nested"]);
        assert_eq!(
            src_files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            vec!["src/main.rs"]
        );
    }

    #[test]
    fn git_tree_keeps_untracked_directory_as_lazy_child() {
        let files = vec![
            git_file("bulk/", "?", "?"),
            git_file("bulk/nested/a.txt", "?", "?"),
        ];

        let (root_dirs, root_files) = collect_immediate_git_status_entries("untracked", "", &files);
        assert_eq!(root_dirs["bulk"].path, "bulk");
        assert!(root_files.is_empty());

        let (bulk_dirs, bulk_files) =
            collect_immediate_git_status_entries("untracked", "bulk", &files);
        assert_eq!(
            bulk_dirs.keys().cloned().collect::<Vec<_>>(),
            vec!["nested"]
        );
        assert!(bulk_files.is_empty());
    }

    #[test]
    fn git_tree_keys_scope_same_directory_by_section() {
        assert_eq!(git_status_tree_key("changed", "src"), "changed:src");
        assert_eq!(git_status_tree_key("untracked", "src"), "untracked:src");
        assert_ne!(
            git_status_tree_key("changed", "src"),
            git_status_tree_key("untracked", "src")
        );
    }
}

fn git_status_dir_row(
    section_id: &'static str,
    name: &str,
    path: &str,
    expanded: bool,
    depth: usize,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let directory_path = path.to_string();
    let directory_section = section_id.to_string();

    div()
        .id(SharedString::from(format!(
            "git-sidebar-dir-{section_id}-{path}"
        )))
        .w_full()
        .min_w_0()
        .h(px(24.0))
        .pl(px(18.0 + depth as f32 * 18.0))
        .pr_3()
        .flex()
        .items_center()
        .text_color(color(theme::TEXT_MUTED))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().list_hover))
        .on_click(cx.listener(move |app, _event, _window, cx| {
            app.toggle_git_status_dir(directory_section.clone(), directory_path.clone(), cx)
        }))
        .child(
            div()
                .flex()
                .flex_1()
                .items_center()
                .min_w_0()
                .child(
                    Icon::new(if expanded {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                    .size_3(),
                )
                .child(
                    Icon::new(IconName::Folder)
                        .size_4()
                        .ml(px(8.0))
                        .text_color(color(theme::ACCENT)),
                )
                .child(
                    div()
                        .ml(px(8.0))
                        .min_w_0()
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .truncate()
                        .child(name.to_string()),
                ),
        )
}

fn git_status_file_row(
    file: GitFileStatus,
    selected_file: Option<&str>,
    selected_files: &HashSet<String>,
    depth: usize,
    labels: Rc<GitFileMenuLabels>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let status = git_file_status_label(&file);
    let status_color = git_file_status_color(&status);
    let can_stage = is_git_worktree_file(&file) || is_git_untracked_file(&file);
    let can_unstage = is_git_staged_file(&file);
    let can_discard = is_git_worktree_file(&file) || is_git_untracked_file(&file);
    let active = selected_file.map(|path| path == file.path).unwrap_or(false)
        || selected_files.contains(&file.path);
    let file_path = file.path.clone();
    let menu_file_path = file.path.clone();
    let file_name = file
        .path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(file.path.as_str())
        .to_string();
    let is_dir_status = file.path.ends_with('/');
    let app_entity = cx.entity();

    div()
        .id(SharedString::from(format!(
            "git-sidebar-file-{}",
            file.path
        )))
        .w_full()
        .min_w_0()
        .h(px(24.0))
        .pl(px(46.0 + depth as f32 * 18.0))
        .pr_3()
        .flex()
        .items_center()
        .justify_between()
        .bg(if active {
            cx.theme().list_hover
        } else {
            cx.theme().transparent
        })
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().list_hover))
        .on_click(cx.listener(move |app, event: &ClickEvent, window, cx| {
            if event.modifiers().shift {
                app.toggle_git_file_selection(file_path.clone(), cx);
            } else {
                app.select_git_file(file_path.clone(), window, cx)
            }
        }))
        .child(
            div()
                .flex()
                .flex_1()
                .items_center()
                .min_w_0()
                .text_color(color(theme::TEXT_MUTED))
                .child(
                    Icon::new(if is_dir_status {
                        IconName::Folder
                    } else {
                        IconName::File
                    })
                    .size_3p5(),
                )
                .child(
                    div()
                        .ml(px(8.0))
                        .min_w_0()
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .truncate()
                        .child(file_name),
                ),
        )
        .child(
            div().ml_2().flex().items_center().gap_1().child(
                div()
                    .min_w(px(18.0))
                    .text_size(px(14.0))
                    .line_height(px(18.0))
                    .text_color(color(status_color))
                    .child(status),
            ),
        )
        .context_menu(move |menu, _window, _cx| {
            let stage_entity = app_entity.clone();
            let stage_path = menu_file_path.clone();
            let unstage_entity = app_entity.clone();
            let unstage_path = menu_file_path.clone();
            let discard_entity = app_entity.clone();
            let discard_path = menu_file_path.clone();
            let ignore_entity = app_entity.clone();
            let ignore_path = menu_file_path.clone();
            let diff_entity = app_entity.clone();
            let diff_path = menu_file_path.clone();

            let menu = if can_stage {
                menu.item(
                    git_context_menu_item(labels.stage.clone(), IconName::Plus).on_click(
                        move |_, window, cx| {
                            cx.update_entity(&stage_entity, |app, cx| {
                                app.select_git_file(stage_path.clone(), window, cx);
                                app.stage_git_paths(
                                    app.selected_git_action_paths(&stage_path),
                                    window,
                                    cx,
                                );
                            });
                        },
                    ),
                )
            } else {
                menu
            };
            let menu = if can_unstage {
                menu.item(
                    git_context_menu_item(labels.unstage.clone(), IconName::Minus).on_click(
                        move |_, window, cx| {
                            cx.update_entity(&unstage_entity, |app, cx| {
                                app.select_git_file(unstage_path.clone(), window, cx);
                                app.unstage_git_paths(
                                    app.selected_git_action_paths(&unstage_path),
                                    window,
                                    cx,
                                );
                            });
                        },
                    ),
                )
            } else {
                menu
            };
            let menu = if !is_dir_status {
                menu.item(
                    git_context_menu_item(labels.open_diff.clone(), IconName::ExternalLink)
                        .on_click(move |_, window, cx| {
                            cx.update_entity(&diff_entity, |app, cx| {
                                app.open_git_diff_window(diff_path.clone(), window, cx);
                            });
                        }),
                )
            } else {
                menu
            };
            let menu = if can_discard {
                menu.separator().item(
                    git_context_menu_item(labels.discard_changes.clone(), IconName::Undo).on_click(
                        move |_, window, cx| {
                            cx.update_entity(&discard_entity, |app, cx| {
                                app.select_git_file(discard_path.clone(), window, cx);
                                app.discard_git_paths(
                                    app.selected_git_action_paths(&discard_path),
                                    window,
                                    cx,
                                );
                            });
                        },
                    ),
                )
            } else {
                menu
            };
            if is_git_untracked_file(&file) || is_dir_status {
                menu.item(
                    git_context_menu_item(labels.add_gitignore.clone(), IconName::Close).on_click(
                        move |_, window, cx| {
                            cx.update_entity(&ignore_entity, |app, cx| {
                                app.append_project_gitignore_paths(
                                    app.selected_git_action_paths(&ignore_path),
                                    window,
                                    cx,
                                );
                            });
                        },
                    ),
                )
            } else {
                menu
            }
        })
}

#[derive(Clone)]
struct GitFileMenuLabels {
    stage: String,
    unstage: String,
    open_diff: String,
    discard_changes: String,
    add_gitignore: String,
}

impl From<&GitSidebarLabels> for GitFileMenuLabels {
    fn from(labels: &GitSidebarLabels) -> Self {
        Self {
            stage: labels.stage.clone(),
            unstage: labels.unstage.clone(),
            open_diff: labels.open_diff.clone(),
            discard_changes: labels.discard_changes.clone(),
            add_gitignore: labels.add_gitignore.clone(),
        }
    }
}

fn git_context_menu_item(label: String, icon: IconName) -> PopupMenuItem {
    PopupMenuItem::element(move |_window, cx| {
        div()
            .flex()
            .items_center()
            .min_w(px(132.0))
            .text_size(px(14.0))
            .line_height(px(18.0))
            .text_color(cx.theme().foreground)
            .child(
                Icon::new(icon.clone())
                    .size_3p5()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(div().ml(px(10.0)).child(label.clone()))
    })
}

pub(in crate::app) fn git_diff_window_workspace(
    selected_path: Option<&str>,
    diff: &str,
    error: Option<&str>,
    language: &str,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let labels = GitSidebarLabels::load(language);
    let file_path = selected_path
        .unwrap_or(labels.no_selected_file.as_str())
        .to_string();
    let open_path = file_path.clone();

    div()
        .flex()
        .flex_col()
        .size_full()
        .bg(color(theme::BG))
        .child(
            div()
                .h(px(52.0))
                .px_4()
                .flex()
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(color(theme::BORDER_SOFT))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .line_height(px(18.0))
                                .truncate()
                                .text_color(color(theme::TEXT))
                                .child("Diff"),
                        )
                        .child(
                            div()
                                .mt(px(2.0))
                                .text_size(px(12.0))
                                .line_height(px(16.0))
                                .truncate()
                                .text_color(color(theme::TEXT_DIM))
                                .child(file_path),
                        ),
                )
                .child(
                    Button::new("git-diff-window-open-file")
                        .secondary()
                        .compact()
                        .text_color(cx.theme().secondary_foreground)
                        .disabled(selected_path.is_none())
                        .on_click(cx.listener(move |app, _event, _window, cx| {
                            app.open_git_diff_window_file(open_path.clone(), cx);
                        }))
                        .child(
                            div()
                                .h(px(22.0))
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .text_size(px(12.0))
                                .line_height(px(16.0))
                                .child(Icon::new(IconName::ExternalLink).size_3())
                                .child(labels.open_file.clone()),
                        ),
                ),
        )
        .when_some(error.map(str::to_string), |this, error| {
            this.child(
                div()
                    .mx_4()
                    .mt_3()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(color(theme::ORANGE).opacity(0.35))
                    .bg(color(theme::ORANGE).opacity(0.12))
                    .px_3()
                    .py_2()
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .text_color(color(theme::ORANGE))
                    .child(error),
            )
        })
        .child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_y_scrollbar()
                .bg(color(theme::BG_TERMINAL))
                .px_4()
                .py_3()
                .children(if diff.trim().is_empty() {
                    vec![
                        div()
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(14.0))
                            .line_height(px(18.0))
                            .text_color(color(theme::TEXT_DIM))
                            .child(labels.empty_diff.clone())
                            .into_any_element(),
                    ]
                } else {
                    diff.lines()
                        .map(|line| git_diff_line_row(line).into_any_element())
                        .collect::<Vec<_>>()
                }),
        )
}

fn git_diff_line_row(line: &str) -> impl IntoElement {
    let line_color = if line.starts_with('+') && !line.starts_with("+++") {
        theme::GREEN
    } else if line.starts_with('-') && !line.starts_with("---") {
        0xF87171
    } else if line.starts_with("@@") {
        theme::ACCENT
    } else {
        theme::TEXT_MUTED
    };

    div()
        .min_h(px(18.0))
        .text_size(px(12.0))
        .line_height(px(18.0))
        .font_family("SF Mono")
        .text_color(color(line_color))
        .child(line.to_string())
}

fn git_history_panel(
    git: &GitSummary,
    labels: Rc<GitSidebarLabels>,
    scroll_handle: VirtualListScrollHandle,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let commits = Rc::new(git.commits.clone());
    let commit_count = commits.len();
    let menu_labels = Rc::new(GitHistoryMenuLabels::from(labels.as_ref()));
    let item_sizes = Rc::new(vec![size(px(1.0), px(44.0)); commit_count]);
    div()
        .size_full()
        .min_h_0()
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(38.0))
                .flex_shrink_0()
                .px_3()
                .flex()
                .items_center()
                .bg(cx.theme().list_head)
                .text_size(px(14.0))
                .line_height(px(18.0))
                .text_color(cx.theme().muted_foreground)
                .child(labels.history.clone()),
        )
        .child(if git.commits.is_empty() {
            div()
                .flex_1()
                .px_3()
                .py_4()
                .text_size(px(14.0))
                .line_height(px(18.0))
                .text_color(color(theme::TEXT_DIM))
                .child(labels.history_empty.clone())
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_h_0()
                .relative()
                .overflow_hidden()
                .py(px(6.0))
                .child(
                    v_virtual_list(
                        cx.entity().clone(),
                        "git-history-list",
                        item_sizes,
                        move |_app, visible_range: Range<usize>, _window, cx| {
                            visible_range
                                .filter_map(|index| {
                                    commits.get(index).cloned().map(|commit| {
                                        git_history_timeline_row(
                                            &commit,
                                            index == 0,
                                            index == 0,
                                            index + 1 >= commit_count,
                                            menu_labels.clone(),
                                            cx,
                                        )
                                        .into_any_element()
                                    })
                                })
                                .collect::<Vec<_>>()
                        },
                    )
                    .track_scroll(&scroll_handle)
                    .with_sizing_behavior(ListSizingBehavior::Auto),
                )
                .vertical_scrollbar(&scroll_handle)
                .into_any_element()
        })
}

fn git_history_timeline_row(
    commit: &GitCommitSummary,
    active: bool,
    is_first: bool,
    is_last: bool,
    labels: Rc<GitHistoryMenuLabels>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let title = commit.title.clone();
    let author = commit.author.clone();
    let relative_time = commit.relative_time.clone();
    let hash = commit.hash.clone();
    let menu_hash = hash.clone();
    let app_entity = cx.entity();
    let context_entity = app_entity.clone();
    let context_hash = menu_hash.clone();
    let tooltip = format!(
        "{}\n{}\n{} · {}",
        commit.hash, commit.title, commit.author, commit.relative_time
    );

    div()
        .id(SharedString::from(format!("git-history-{}", commit.hash)))
        .w_full()
        .min_w_0()
        .relative()
        .h(px(44.0))
        .px_3()
        .py(px(4.0))
        .flex()
        .gap_2()
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        .hover(|style| style.bg(cx.theme().list_hover))
        .child(
            div()
                .w(px(18.0))
                .h(px(36.0))
                .relative()
                .flex_shrink_0()
                .when(!is_first, |this| {
                    this.child(
                        div()
                            .absolute()
                            .left(px(8.5))
                            .top(px(-4.0))
                            .h(px(13.0))
                            .w(px(1.0))
                            .bg(color(0x7A8599).opacity(0.82)),
                    )
                })
                .when(!is_last, |this| {
                    this.child(
                        div()
                            .absolute()
                            .left(px(8.5))
                            .top(px(21.0))
                            .bottom(px(-4.0))
                            .w(px(1.0))
                            .bg(color(0x7A8599).opacity(0.82)),
                    )
                })
                .child(
                    div()
                        .absolute()
                        .left(px(2.5))
                        .top(px(12.0))
                        .size(px(12.0))
                        .rounded_full()
                        .border_1()
                        .border_color(color(theme::BG_COLUMN))
                        .bg(color(if active {
                            theme::ACCENT
                        } else {
                            theme::TEXT_DIM
                        })),
                ),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .min_w_0()
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .text_size(px(14.0))
                                .line_height(px(18.0))
                                .text_color(color(theme::TEXT))
                                .truncate()
                                .child(title),
                        )
                        .child(if active {
                            div()
                                .rounded(px(6.0))
                                .px_2()
                                .py(px(2.0))
                                .bg(color(theme::ACCENT).opacity(0.16))
                                .text_size(px(12.0))
                                .line_height(px(14.0))
                                .text_color(color(theme::ACCENT))
                                .child("HEAD->main")
                                .into_any_element()
                        } else {
                            div().into_any_element()
                        }),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(color(theme::TEXT_DIM))
                        .truncate()
                        .child(format!("{author} · {relative_time} · {hash}")),
                ),
        )
        .context_menu(move |menu, _window, _cx| {
            let checkout_hash = context_hash.clone();
            let revert_hash = context_hash.clone();
            let restore_hash = context_hash.clone();
            let checkout_entity = context_entity.clone();
            let revert_entity = context_entity.clone();
            let restore_entity = context_entity.clone();
            menu.item(
                PopupMenuItem::new(labels.checkout_commit.clone())
                    .icon(IconName::Github)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&checkout_entity, |app, cx| {
                            app.checkout_git_commit(checkout_hash.clone(), window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new(labels.revert_commit.clone())
                    .icon(IconName::Undo2)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&revert_entity, |app, cx| {
                            app.revert_git_commit(revert_hash.clone(), window, cx);
                        });
                    }),
            )
            .item(
                PopupMenuItem::new(labels.restore_commit.clone())
                    .icon(IconName::Redo2)
                    .on_click(move |_, window, cx| {
                        cx.update_entity(&restore_entity, |app, cx| {
                            app.restore_git_commit(restore_hash.clone(), window, cx);
                        });
                    }),
            )
        })
}

#[derive(Clone)]
struct GitHistoryMenuLabels {
    checkout_commit: String,
    revert_commit: String,
    restore_commit: String,
}

impl From<&GitSidebarLabels> for GitHistoryMenuLabels {
    fn from(labels: &GitSidebarLabels) -> Self {
        Self {
            checkout_commit: labels.checkout_commit.clone(),
            revert_commit: labels.revert_commit.clone(),
            restore_commit: labels.restore_commit.clone(),
        }
    }
}

fn is_git_staged_file(file: &GitFileStatus) -> bool {
    let index = file.index_status.trim();
    !index.is_empty() && index != "?"
}

fn is_git_worktree_file(file: &GitFileStatus) -> bool {
    !is_git_untracked_file(file) && !file.worktree_status.trim().is_empty()
}

fn is_git_untracked_file(file: &GitFileStatus) -> bool {
    file.worktree_status == "?" && (file.index_status == "?" || file.index_status.trim().is_empty())
}

fn git_file_status_label(file: &GitFileStatus) -> String {
    if is_git_untracked_file(file) {
        "A".to_string()
    } else {
        let status = format!(
            "{}{}",
            file.index_status.trim(),
            file.worktree_status.trim()
        );
        if status.is_empty() {
            "M".to_string()
        } else {
            status
        }
    }
}

fn git_file_status_color(status: &str) -> u32 {
    match status.chars().next().unwrap_or('?') {
        'A' | '?' => theme::GREEN,
        'D' => theme::ACCENT,
        'M' => theme::ORANGE,
        'R' | 'C' | 'T' => theme::ORANGE,
        _ => theme::TEXT_DIM,
    }
}

pub(in crate::app) fn git_workspace_section(
    git: &GitSummary,
    selected_file: Option<&str>,
    selected_branch: Option<&str>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let status_rows = vec![
        format!(
            "repository: {}",
            if git.is_repository { "yes" } else { "no" }
        ),
        format!("branch: {}", git.branch),
        format!("upstream: {}", git.upstream.as_deref().unwrap_or("none")),
        format!("ahead / behind: {} / {}", git.ahead, git.behind),
        format!(
            "staged / unstaged / untracked: {} / {} / {}",
            git.staged, git.unstaged, git.untracked
        ),
    ];
    let commit_rows = if git.commits.is_empty() {
        vec!["no recent commits".to_string()]
    } else {
        git.commits
            .iter()
            .take(8)
            .map(|commit| {
                format!(
                    "{} {} · {} · {}",
                    commit.hash, commit.title, commit.author, commit.relative_time
                )
            })
            .collect()
    };

    div()
        .flex()
        .flex_col()
        .child(section("Repository", status_rows))
        .child(git_changed_files_section(
            &git.changed_files,
            selected_file,
            cx,
        ))
        .child(section("Recent Commits", commit_rows))
        .child(git_branches_section(&git.branches, selected_branch, cx))
        .child(section(
            "Remotes",
            git.remotes
                .iter()
                .take(6)
                .map(|remote| format!("{} {}", remote.name, remote.url))
                .collect(),
        ))
}

fn git_branches_section(
    branches: &[GitBranchSummary],
    selected_branch: Option<&str>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let rows = if branches.is_empty() {
        vec![
            div()
                .px_2()
                .py_1()
                .text_xs()
                .text_color(color(theme::TEXT_DIM))
                .child("no local branches")
                .into_any_element(),
        ]
    } else {
        branches
            .iter()
            .take(20)
            .cloned()
            .map(|branch| git_branch_row(branch, selected_branch, cx).into_any_element())
            .collect()
    };
    div()
        .flex()
        .flex_col()
        .mx_3()
        .mt_3()
        .rounded_sm()
        .border_1()
        .border_color(color(theme::BORDER))
        .bg(color(theme::BG_ELEVATED))
        .child(
            div()
                .h(px(30.0))
                .px_2()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(color(theme::BORDER_SOFT))
                .text_xs()
                .text_color(color(theme::TEXT_MUTED))
                .child("Branches"),
        )
        .children(rows)
}

fn git_branch_row(
    branch: GitBranchSummary,
    selected_branch: Option<&str>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let active = selected_branch
        .map(|name| name == branch.name)
        .unwrap_or(branch.is_current);
    let branch_name = branch.name.clone();
    div()
        .id(SharedString::from(format!("git-branch-{}", branch.name)))
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_2()
        .py_1()
        .border_t_1()
        .border_color(color(theme::BORDER_SOFT))
        .bg(color(if active {
            theme::BG_PANEL
        } else {
            theme::BG_ELEVATED
        }))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().list_hover))
        .on_click(cx.listener(move |app, _event, window, cx| {
            app.select_git_branch(branch_name.clone(), window, cx)
        }))
        .child(
            div()
                .text_xs()
                .text_color(color(theme::TEXT))
                .truncate()
                .child(branch.name),
        )
        .child(
            div()
                .text_xs()
                .text_color(color(if branch.is_current {
                    theme::ACCENT
                } else {
                    theme::TEXT_DIM
                }))
                .child(if branch.is_current { "current" } else { "" }),
        )
}

fn git_changed_files_section(
    files: &[GitFileStatus],
    selected_file: Option<&str>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let rows = if files.is_empty() {
        vec![
            div()
                .px_2()
                .py_1()
                .text_xs()
                .text_color(color(theme::TEXT_DIM))
                .child("no changed files")
                .into_any_element(),
        ]
    } else {
        files
            .iter()
            .take(24)
            .cloned()
            .map(|file| git_changed_file_row(file, selected_file, cx).into_any_element())
            .collect()
    };
    div()
        .flex()
        .flex_col()
        .mx_3()
        .mt_3()
        .rounded_sm()
        .border_1()
        .border_color(color(theme::BORDER))
        .bg(color(theme::BG_ELEVATED))
        .child(
            div()
                .h(px(30.0))
                .px_2()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(color(theme::BORDER_SOFT))
                .text_xs()
                .text_color(color(theme::TEXT_MUTED))
                .child("Changed Files"),
        )
        .children(rows)
}

fn git_changed_file_row(
    file: GitFileStatus,
    selected_file: Option<&str>,
    cx: &mut Context<CoduxApp>,
) -> impl IntoElement {
    let active = selected_file.map(|path| path == file.path).unwrap_or(false);
    let file_path = file.path.clone();
    div()
        .id(SharedString::from(format!("git-file-{}", file.path)))
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_2()
        .py_1()
        .border_t_1()
        .border_color(color(theme::BORDER_SOFT))
        .bg(color(if active {
            theme::BG_PANEL
        } else {
            theme::BG_ELEVATED
        }))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().list_hover))
        .on_click(cx.listener(move |app, _event, window, cx| {
            app.select_git_file(file_path.clone(), window, cx)
        }))
        .child(
            div()
                .text_xs()
                .text_color(color(theme::TEXT))
                .truncate()
                .child(file.path),
        )
        .child(
            div()
                .text_xs()
                .text_color(color(if active {
                    theme::ACCENT
                } else {
                    theme::TEXT_DIM
                }))
                .child(format!("{}{}", file.index_status, file.worktree_status)),
        )
}

pub(in crate::app) fn git_diff_workspace(diff: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .mx_3()
        .mt_3()
        .rounded_sm()
        .border_1()
        .border_color(color(theme::BORDER))
        .bg(color(theme::BG_TERMINAL))
        .child(
            div()
                .h(px(30.0))
                .px_2()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(color(theme::BORDER_SOFT))
                .text_xs()
                .text_color(color(theme::TEXT_MUTED))
                .child("Diff Preview"),
        )
        .child(
            div()
                .p_2()
                .text_xs()
                .text_color(color(theme::TEXT))
                .children(diff.lines().take(40).map(|line| {
                    div()
                        .child(line.chars().take(110).collect::<String>())
                        .into_any_element()
                })),
        )
}

pub(in crate::app) fn git_review_workspace(
    selected_path: Option<&str>,
    diff: &str,
    content: Option<&GitReviewContentSummary>,
) -> impl IntoElement {
    let selected_path = selected_path.unwrap_or("No file selected");
    div()
        .flex()
        .flex_col()
        .size_full()
        .min_h_0()
        .child(
            div()
                .h(px(44.0))
                .px_3()
                .flex()
                .items_center()
                .justify_between()
                .border_b_1()
                .border_color(color(theme::BORDER_SOFT))
                .child(
                    div()
                        .min_w_0()
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .text_color(color(theme::TEXT))
                        .truncate()
                        .child(selected_path.to_string()),
                )
                .when_some(content, |this, content| {
                    this.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .text_color(color(theme::TEXT_DIM))
                            .child(format!("+{}", content.added_lines.len()))
                            .child(format!("-{}", content.deleted_lines.len())),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .child(git_review_content_panel(
                    "Base",
                    content.and_then(|item| item.base_content.as_deref()),
                ))
                .child(git_review_content_panel(
                    "Worktree",
                    content.map(|item| item.worktree_content.as_str()),
                )),
        )
        .child(
            div()
                .h(px(190.0))
                .flex_shrink_0()
                .border_t_1()
                .border_color(color(theme::BORDER_SOFT))
                .overflow_y_scrollbar()
                .child(git_diff_workspace(diff)),
        )
}

fn git_review_content_panel(title: &'static str, content: Option<&str>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .border_r_1()
        .border_color(color(theme::BORDER_SOFT))
        .child(
            div()
                .h(px(30.0))
                .px_2()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(color(theme::BORDER_SOFT))
                .text_size(px(12.0))
                .line_height(px(16.0))
                .text_color(color(theme::TEXT_MUTED))
                .child(title),
        )
        .child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_y_scrollbar()
                .bg(color(theme::BG_TERMINAL))
                .p_2()
                .text_size(px(12.0))
                .line_height(px(18.0))
                .text_color(color(theme::TEXT))
                .children(content.unwrap_or("").lines().take(160).map(|line| {
                    div()
                        .child(line.chars().take(130).collect::<String>())
                        .into_any_element()
                })),
        )
}
