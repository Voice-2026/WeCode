use super::*;

impl CoduxApp {
    pub(in crate::app) fn normalize_selected_git_file(&mut self) {
        let selected_still_exists = self
            .selected_git_file
            .as_deref()
            .map(|path| {
                self.git_review.files.iter().any(|file| file.path == path)
                    || self
                        .state
                        .git
                        .changed_files
                        .iter()
                        .any(|file| file.path == path)
            })
            .unwrap_or(false);
        if !selected_still_exists {
            self.selected_git_file = None;
            self.selected_git_files.clear();
            self.git_diff_preview = "select a changed file to preview its diff".to_string();
            self.clear_git_review_derived_content();
        }
    }

    pub(in crate::app) fn refresh_git_review_for_project(&mut self, project_path: &str) {
        self.git_review = self
            .runtime_service
            .reload_project_git_review(project_path, self.git_review.base_branch.as_deref());
        merge_git_review_status_files(&mut self.git_review, &self.state.git);
    }

    pub(in crate::app) fn normalize_selected_git_branch(&mut self) {
        let selected_still_exists = self
            .selected_git_branch
            .as_deref()
            .map(|name| {
                self.state
                    .git
                    .branches
                    .iter()
                    .any(|branch| branch.name == name)
            })
            .unwrap_or(false);
        if selected_still_exists {
            return;
        }
        self.selected_git_branch = self
            .state
            .git
            .branches
            .iter()
            .find(|branch| branch.is_current)
            .or_else(|| self.state.git.branches.first())
            .map(|branch| branch.name.clone());
    }

    pub(in crate::app) fn select_git_branch(
        &mut self,
        branch_name: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .state
            .git
            .branches
            .iter()
            .any(|branch| branch.name == branch_name)
        {
            self.selected_git_branch = Some(branch_name.clone());
            self.status_message = format!("selected Git branch: {branch_name}");
        } else {
            self.normalize_selected_git_branch();
            self.status_message = "Git branch is no longer available".to_string();
        }
        self.invalidate_git_panel(cx);
    }

    pub(in crate::app) fn stage_git_paths(
        &mut self,
        paths: Vec<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_git_paths_stage(paths, true, cx);
    }

    pub(in crate::app) fn unstage_git_paths(
        &mut self,
        paths: Vec<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_git_paths_stage(paths, false, cx);
    }

    pub(in crate::app) fn discard_git_paths(
        &mut self,
        paths: Vec<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.discard_git_paths_inner(paths, cx);
    }

    /// Confirm-then-discard for the menu's "Discard All Changes".
    pub(in crate::app) fn discard_all_git_changes(
        &mut self,
        paths: Vec<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = self.text("git.files.discard_all", "Discard All");
        let message = self.text(
            "git.files.discard_all.confirm",
            "Discard all worktree changes?",
        );
        let confirm_label = self.text("common.delete", "Delete");
        self.confirm_git_action(
            title,
            message,
            confirm_label,
            move |app, cx| app.discard_git_paths_inner(paths, cx),
            cx,
        );
    }

    fn discard_git_paths_inner(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git discard".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let paths = normalized_git_action_paths(paths);
        if paths.is_empty() {
            self.status_message = "no Git files to discard".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let count = paths.len();
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: format!("discard-batch:{count}"),
                cancellable: false,
            },
            move |service, path| service.discard_project_git_paths(&path, &paths),
            GitOperationCompletion {
                success_message: format!("discarded {count} Git file paths"),
                failure_prefix: "failed to discard Git file paths".to_string(),
                clear_git_diff_preview: true,
                clear_git_tree_state: true,
                refresh_review: true,
                ..Default::default()
            },
            cx,
        );
    }

    pub(in crate::app) fn append_project_gitignore_paths(
        &mut self,
        paths: Vec<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for .gitignore".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let paths = normalized_git_action_paths(paths);
        if paths.is_empty() {
            self.status_message = "no Git paths to ignore".to_string();
            self.invalidate_git_panel(cx);
            return;
        }

        let count = paths.len();
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: format!("ignore-batch:{count}"),
                cancellable: false,
            },
            move |service, path| service.append_project_gitignore(&path, &paths),
            GitOperationCompletion {
                success_message: format!("added {count} Git paths to .gitignore"),
                failure_prefix: "failed to update .gitignore".to_string(),
                clear_git_diff_preview: true,
                clear_git_tree_state: true,
                refresh_review: true,
                ..Default::default()
            },
            cx,
        );
    }

    pub(in crate::app) fn update_git_paths_stage(
        &mut self,
        paths: Vec<String>,
        stage: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git file operation".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let paths = normalized_git_action_paths(paths);
        if paths.is_empty() {
            self.status_message = "no Git files selected".to_string();
            self.invalidate_git_panel(cx);
            return;
        }

        let count = paths.len();
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        let label = if stage { "stage" } else { "unstage" };
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: format!("{label}-batch:{count}"),
                cancellable: false,
            },
            move |service, path| {
                if stage {
                    service.stage_project_git_paths(&path, &paths)
                } else {
                    service.unstage_project_git_paths(&path, &paths)
                }
            },
            GitOperationCompletion {
                success_message: format!(
                    "{} {count} Git file paths",
                    if stage { "staged" } else { "unstaged" }
                ),
                failure_prefix: format!(
                    "failed to {} Git file paths",
                    if stage { "stage" } else { "unstage" }
                ),
                clear_git_tree_state: true,
                refresh_review: true,
                ..Default::default()
            },
            cx,
        );
    }

    pub(in crate::app) fn commit_staged_git(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_git_with_action("commit", cx);
    }

    pub(in crate::app) fn commit_and_push_git(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_git_with_action("commitAndPush", cx);
    }

    pub(in crate::app) fn commit_and_sync_git(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_git_with_action("commitAndSync", cx);
    }

    pub(in crate::app) fn commit_git_with_action(&mut self, action: &str, cx: &mut Context<Self>) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git commit".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        let message = self
            .git_commit_message
            .trim()
            .to_string()
            .chars()
            .take(500)
            .collect::<String>();
        let message = if message.is_empty() {
            generated_git_commit_message(&self.state.git)
        } else {
            message
        };
        let action = action.to_string();
        let worker_action = action.clone();
        let worker_message = message.clone();
        let success_message = match action.as_str() {
            "commitAndPush" => format!("committed and pushed staged changes: {message}"),
            "commitAndSync" => format!("committed and synced staged changes: {message}"),
            _ => format!("committed staged changes: {message}"),
        };
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: action.clone(),
                cancellable: false,
            },
            move |service, path| match worker_action.as_str() {
                "commit" => service.commit_project_git(&path, &worker_message),
                "commitAndPush" | "commitAndSync" => {
                    service.commit_project_git_action(&path, &worker_message, &worker_action)
                }
                _ => Err(format!("unknown Git commit action: {worker_action}")),
            },
            GitOperationCompletion {
                success_message,
                failure_prefix: "failed to commit staged changes".to_string(),
                clear_commit_message: true,
                refresh_review: true,
                clear_selected_branch: false,
                selected_branch: None,
                ..Default::default()
            },
            cx,
        );
    }

    pub(in crate::app) fn load_last_git_commit_message(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git commit message".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        match self
            .runtime_service
            .read_project_git_last_commit_message(&project.path)
        {
            Ok(message) if !message.trim().is_empty() => {
                self.git_commit_message = message;
                self.git_commit_message_revision =
                    self.git_commit_message_revision.saturating_add(1);
                self.status_message = "loaded last Git commit message".to_string();
            }
            Ok(_) => {
                self.status_message = "last Git commit has no summary".to_string();
            }
            Err(error) => {
                self.status_message = format!("failed to load last Git commit message: {error}");
            }
        }
        self.invalidate_git_panel(cx);
    }

    pub(in crate::app) fn amend_last_git_commit(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git amend".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        let draft_message = self
            .git_commit_message
            .trim()
            .to_string()
            .chars()
            .take(500)
            .collect::<String>();
        let message = if draft_message.is_empty() {
            match self
                .runtime_service
                .read_project_git_last_commit_message(&project_path)
            {
                Ok(message) if !message.trim().is_empty() => message,
                Ok(_) => {
                    self.status_message = "last Git commit has no summary".to_string();
                    self.invalidate_git_panel(cx);
                    return;
                }
                Err(error) => {
                    self.status_message =
                        format!("failed to load last Git commit message: {error}");
                    self.invalidate_git_panel(cx);
                    return;
                }
            }
        } else {
            draft_message
        };

        let worker_message = message.clone();
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: "amend".to_string(),
                cancellable: false,
            },
            move |service, path| service.amend_project_git_last_commit(&path, &worker_message),
            GitOperationCompletion {
                success_message: format!("amended last Git commit: {message}"),
                failure_prefix: "failed to amend last Git commit".to_string(),
                clear_commit_message: true,
                refresh_review: true,
                clear_selected_branch: false,
                selected_branch: None,
                ..Default::default()
            },
            cx,
        );
    }

    pub(in crate::app) fn undo_last_git_commit(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git undo".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: "undo".to_string(),
                cancellable: false,
            },
            |service, path| service.undo_project_git_last_commit(&path),
            GitOperationCompletion {
                success_message: "undid last Git commit".to_string(),
                failure_prefix: "failed to undo last Git commit".to_string(),
                clear_commit_message: false,
                refresh_review: true,
                clear_selected_branch: false,
                selected_branch: None,
                ..Default::default()
            },
            cx,
        );
    }
}
