use super::*;

impl WeCodeApp {
    fn spawn_file_move_conflict_check(
        &mut self,
        paths: Vec<String>,
        target_directory_path: String,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for file move".to_string();
            self.invalidate_file_panel(cx);
            return;
        };
        let project_path = project.path.clone();
        let runtime_service = self.runtime_service.clone();
        let generation = self.project_switch_generation;
        let scope_key = super::app_state::current_worktree_scope_key(&self.state);
        self.file_mutation_generation = self.file_mutation_generation.wrapping_add(1);
        let mutation_generation = self.file_mutation_generation;
        let target_directory_for_read = target_directory_path.clone();
        let worker_paths = paths.clone();
        self.status_message = format!(
            "checking move conflicts for {} file item{}",
            paths.len(),
            plural(paths.len())
        );
        self.invalidate_file_panel(cx);
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx| {
            let result = wecode_runtime::async_runtime::run_limited_blocking_with_priority(
                wecode_runtime::async_runtime::BLOCKING_PRIORITY_FOREGROUND + generation,
                move || {
                    let target_entries = runtime_service.try_reload_project_files(
                        &project_path,
                        file_directory_option(&target_directory_for_read),
                    )?;
                    let target_names = target_entries
                        .iter()
                        .map(|entry| entry.name.as_str())
                        .collect::<HashSet<_>>();
                    let conflicts = worker_paths
                        .iter()
                        .filter_map(|path| {
                            let name = path.rsplit('/').next()?;
                            target_names
                                .contains(name)
                                .then(|| join_relative_child_path(&target_directory_for_read, name))
                        })
                        .collect::<Vec<_>>();
                    Ok::<_, String>(if conflicts.is_empty() {
                        FileMoveConflictCheckResult::Clear
                    } else {
                        FileMoveConflictCheckResult::Conflicts(conflicts)
                    })
                },
            )
            .await
            .ok()
            .unwrap_or_else(|| Err("file move conflict check failed".to_string()));

            let _ = this.update(cx, |app, cx| {
                let current_key = super::app_state::current_worktree_scope_key(&app.state);
                if app.project_switch_generation != generation
                    || current_key != scope_key
                    || app.file_mutation_generation != mutation_generation
                {
                    app.invalidate_file_panel(cx);
                    return;
                }
                app.apply_file_move_conflict_check_result(paths, target_directory_path, result, cx);
            });
        })
        .detach();
    }

    fn apply_file_move_conflict_check_result(
        &mut self,
        paths: Vec<String>,
        target_directory_path: String,
        result: Result<FileMoveConflictCheckResult, String>,
        cx: &mut Context<Self>,
    ) {
        let conflicts = match result {
            Ok(FileMoveConflictCheckResult::Clear) => {
                self.move_file_entries_to_directory_confirmed(
                    paths,
                    target_directory_path,
                    false,
                    cx,
                );
                return;
            }
            Ok(FileMoveConflictCheckResult::Conflicts(conflicts)) => conflicts,
            Err(error) => {
                self.status_message = format!("failed to check file move conflicts: {error}");
                self.invalidate_file_panel(cx);
                return;
            }
        };

        let title = if conflicts.len() == 1 {
            self.text("files.move.conflict_one_format", "Overwrite \"%@\"?")
                .replace("%@", &conflicts[0])
        } else {
            self.text(
                "files.move.conflict_many_format",
                "Overwrite %d file items?",
            )
            .replace("%d", &conflicts.len().to_string())
        };
        let message = self.text(
            "files.move.conflict.message",
            "The destination already contains file items with the same name. Overwriting will replace the destination items.",
        );
        let confirm_label = self.text("files.move.conflict.confirm", "Overwrite");
        let cancel_label = self.text("common.cancel", "Cancel");
        let service = self.runtime_service.clone();
        let generation = self.project_switch_generation;
        let scope_key = super::app_state::current_worktree_scope_key(&self.state);
        let mutation_generation = self.file_mutation_generation;
        self.status_message = "waiting for file move confirmation".to_string();
        let timer = cx.background_executor().clone();
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx| {
            timer.timer(Duration::from_millis(120)).await;
            let result = wecode_runtime::async_runtime::spawn_blocking(move || {
                service.localized_confirm_dialog(LocalizedConfirmDialogRequest {
                    title,
                    message,
                    confirm_label,
                    cancel_label,
                })
            })
            .await
            .map_err(|error| error.to_string())
            .and_then(|result| result);

            let _ = this.update(cx, |app, cx| {
                if app.project_switch_generation != generation
                    || super::app_state::current_worktree_scope_key(&app.state) != scope_key
                    || app.file_mutation_generation != mutation_generation
                {
                    app.invalidate_file_panel(cx);
                    return;
                }
                match result {
                    Ok(true) => app.move_file_entries_to_directory_confirmed(
                        paths,
                        target_directory_path,
                        true,
                        cx,
                    ),
                    Ok(false) => {
                        app.status_message = "file move canceled".to_string();
                        app.invalidate_file_panel(cx);
                    }
                    Err(error) => {
                        app.status_message = format!("failed to show move confirmation: {error}");
                        app.invalidate_file_panel(cx);
                    }
                }
            });
        })
        .detach();
        self.invalidate_file_panel(cx);
    }

    pub(in crate::app) fn move_file_entries_to_directory(
        &mut self,
        paths: Vec<String>,
        target_directory_path: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut paths = paths
            .into_iter()
            .filter(|path| path != &target_directory_path)
            .filter(|path| !target_directory_path.starts_with(&format!("{path}/")))
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();
        let original_len = paths.len();
        paths.retain(|path| {
            parent_relative_directory(path) != target_directory_path.trim_matches('/')
        });
        if paths.is_empty() {
            self.status_message = if original_len == 0 {
                "no movable file item selected".to_string()
            } else {
                "file item is already in that directory".to_string()
            };
            self.invalidate_file_panel(cx);
            return;
        }

        self.spawn_file_move_conflict_check(paths, target_directory_path, cx);
    }

    pub(in crate::app) fn move_file_entries_to_directory_confirmed(
        &mut self,
        paths: Vec<String>,
        target_directory_path: String,
        overwrite: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for file move".to_string();
            self.invalidate_file_panel(cx);
            return;
        };
        if paths.is_empty() {
            self.status_message = "no movable file item selected".to_string();
            self.invalidate_file_panel(cx);
            return;
        }
        let project_path = project.path.clone();
        let file_directory = self.file_directory.clone();
        let directory = file_directory_option(&file_directory).map(str::to_string);
        let expanded_dirs = self
            .file_tree_expanded_dirs
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let runtime_service = self.runtime_service.clone();
        let count = paths.len();
        self.spawn_file_mutation(
            format!("moving {count} file item{}", plural(count)),
            "file move task failed",
            cx,
            move || {
                let mut moved = Vec::new();
                let mut files = None;
                for path in &paths {
                    let (next_files, moved_path) = if overwrite {
                        runtime_service.move_project_file_entry_overwrite(
                            &project_path,
                            path,
                            &target_directory_path,
                            directory.as_deref(),
                        )
                    } else {
                        runtime_service.move_project_file_entry(
                            &project_path,
                            path,
                            &target_directory_path,
                            directory.as_deref(),
                        )
                    }
                    .map_err(|error| format!("failed to move {path}: {error}"))?;
                    files = Some(next_files);
                    moved.push(moved_path);
                }
                let files = files.unwrap_or_default();
                let next_expanded = file_mutation_prune_expanded(&expanded_dirs, &paths);
                let file_tree_children =
                    load_file_mutation_children(&runtime_service, &project_path, &next_expanded)
                        .map_err(|error| file_mutation_refresh_error("file items moved", error))?;
                let git = runtime_service.reload_project_git(&project_path);
                let selection = if moved.len() == 1 {
                    FileMutationSelection::Single(moved[0].clone())
                } else {
                    FileMutationSelection::Multiple(moved.clone())
                };
                Ok(FileMutationResult {
                    files,
                    file_tree_children,
                    expanded_dirs: next_expanded.into_iter().collect(),
                    selection,
                    preview: None,
                    git,
                    status: format!("moved {} file item{}", moved.len(), plural(moved.len())),
                    clear_draft: false,
                    saved_editor_path: None,
                    saved_editor_content: None,
                })
            },
        );
    }
    pub(in crate::app) fn request_delete_selected_file_entries(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut paths = if self.selected_file_entries.is_empty() {
            self.selected_file_entry
                .clone()
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            self.selected_file_entries
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        };
        paths.sort();
        paths.dedup();
        if paths.is_empty() {
            self.status_message = "no selected file entry to delete".to_string();
        } else {
            let title = if paths.len() == 1 {
                self.text("files.delete.confirm_one_format", "Delete \"%@\"?")
                    .replace("%@", &paths[0])
            } else {
                self.text("files.delete.confirm_many_format", "Delete %d file items?")
                    .replace("%d", &paths.len().to_string())
            };
            let message = self.text(
                "files.delete.confirm.message",
                "Deleted file items will be moved to Trash.",
            );
            let confirm_label = self.text("common.delete", "Delete");
            let cancel_label = self.text("common.cancel", "Cancel");
            let service = self.runtime_service.clone();
            self.file_mutation_generation = self.file_mutation_generation.wrapping_add(1);
            let mutation_generation = self.file_mutation_generation;
            let generation = self.project_switch_generation;
            let scope_key = super::app_state::current_worktree_scope_key(&self.state);
            self.status_message = "waiting for file deletion confirmation".to_string();
            let timer = cx.background_executor().clone();
            cx.spawn(async move |this: gpui::WeakEntity<Self>, cx| {
                timer.timer(Duration::from_millis(120)).await;
                let result = wecode_runtime::async_runtime::spawn_blocking(move || {
                    service.localized_confirm_dialog(LocalizedConfirmDialogRequest {
                        title,
                        message,
                        confirm_label,
                        cancel_label,
                    })
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);

                let _ = this.update(cx, |app, cx| {
                    if app.project_switch_generation != generation
                        || super::app_state::current_worktree_scope_key(&app.state) != scope_key
                        || app.file_mutation_generation != mutation_generation
                    {
                        app.invalidate_file_panel(cx);
                        return;
                    }
                    match result {
                        Ok(true) => app.delete_file_entries(paths, cx),
                        Ok(false) => {
                            app.status_message = "file deletion canceled".to_string();
                            app.invalidate_file_panel(cx);
                        }
                        Err(error) => {
                            app.status_message =
                                format!("failed to show delete confirmation: {error}");
                            app.invalidate_file_panel(cx);
                        }
                    }
                });
            })
            .detach();
            self.invalidate_file_panel(cx);
            return;
        }
        self.invalidate_file_panel(cx);
    }

    pub(in crate::app) fn delete_file_entries(
        &mut self,
        paths: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for file deletion".to_string();
            self.invalidate_file_panel(cx);
            return;
        };
        if paths.is_empty() {
            self.status_message = "no selected file entry to delete".to_string();
            self.invalidate_file_panel(cx);
            return;
        }
        let project_path = project.path.clone();
        let file_directory = self.file_directory.clone();
        let directory = file_directory_option(&file_directory).map(str::to_string);
        let expanded_dirs = self
            .file_tree_expanded_dirs
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let runtime_service = self.runtime_service.clone();
        let count = paths.len();
        self.spawn_file_mutation(
            format!("moving {count} file item{} to trash", plural(count)),
            "file deletion task failed",
            cx,
            move || {
                let mut files = None;
                for entry_path in &paths {
                    files = Some(runtime_service.delete_project_file_entry(
                        &project_path,
                        entry_path,
                        directory.as_deref(),
                    )?);
                }
                let files = files.unwrap_or_default();
                let next_expanded = file_mutation_prune_expanded(&expanded_dirs, &paths);
                let file_tree_children =
                    load_file_mutation_children(&runtime_service, &project_path, &next_expanded)
                        .map_err(|error| {
                            file_mutation_refresh_error("file items deleted", error)
                        })?;
                let git = runtime_service.reload_project_git(&project_path);
                Ok(FileMutationResult {
                    files,
                    file_tree_children,
                    expanded_dirs: next_expanded.into_iter().collect(),
                    selection: FileMutationSelection::Clear,
                    preview: Some(("select a file to preview it".to_string(), false, false)),
                    git,
                    status: format!("moved {count} file item{} to trash", plural(count)),
                    clear_draft: false,
                    saved_editor_path: None,
                    saved_editor_content: None,
                })
            },
        );
    }
}
