use super::*;

impl WeCodeApp {
    pub(in crate::app) fn checkout_selected_git_branch(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git checkout".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let Some(branch_name) = self.selected_git_branch.clone() else {
            self.status_message = "no selected Git branch".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        let worker_branch = branch_name.clone();
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: format!("checkout:{branch_name}"),
                cancellable: false,
            },
            move |service, path| service.checkout_project_git_branch(&path, &worker_branch),
            GitOperationCompletion {
                success_message: format!("checked out Git branch: {branch_name}"),
                failure_prefix: "Git checkout failed".to_string(),
                clear_commit_message: false,
                refresh_review: false,
                clear_selected_branch: false,
                selected_branch: Some(branch_name),
                ..Default::default()
            },
            cx,
        );
    }

    pub(in crate::app) fn checkout_git_remote_branch(
        &mut self,
        remote_branch: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git remote checkout".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        let worker_branch = remote_branch.clone();
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: format!("checkout-remote:{remote_branch}"),
                cancellable: false,
            },
            move |service, path| service.checkout_project_git_remote_branch(&path, &worker_branch),
            GitOperationCompletion {
                success_message: format!("checked out remote Git branch: {remote_branch}"),
                failure_prefix: "Git remote checkout failed".to_string(),
                clear_commit_message: false,
                refresh_review: false,
                clear_selected_branch: true,
                selected_branch: None,
                ..Default::default()
            },
            cx,
        );
    }

    pub(in crate::app) fn checkout_git_commit(
        &mut self,
        commit: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.run_git_commit_history_action("checkout", &commit, cx);
    }

    pub(in crate::app) fn revert_git_commit(
        &mut self,
        commit: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.run_git_commit_history_action("revert", &commit, cx);
    }

    pub(in crate::app) fn restore_git_commit(
        &mut self,
        commit: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.run_git_commit_history_action("restore", &commit, cx);
    }

    pub(in crate::app) fn run_git_commit_history_action(
        &mut self,
        action: &str,
        commit: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = format!("no selected project for Git {action}");
            self.invalidate_git_panel(cx);
            return;
        };
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        let action = action.to_string();
        let commit = commit.to_string();
        let worker_action = action.clone();
        let worker_commit = commit.clone();
        let success_message = match action.as_str() {
            "checkout" => format!("checked out Git commit: {commit}"),
            "revert" => format!("reverted Git commit: {commit}"),
            "restore" => format!("restored Git branch to commit: {commit}"),
            _ => format!("Git history action completed: {commit}"),
        };
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: format!("{action}:{commit}"),
                cancellable: false,
            },
            move |service, path| match worker_action.as_str() {
                "checkout" => service.checkout_project_git_commit(&path, &worker_commit),
                "revert" => service.revert_project_git_commit(&path, &worker_commit),
                "restore" => service.restore_project_git_commit(&path, &worker_commit, false),
                _ => Err(format!("unknown Git history action: {worker_action}")),
            },
            GitOperationCompletion {
                success_message,
                failure_prefix: format!("Git {action} commit failed"),
                clear_commit_message: false,
                refresh_review: true,
                clear_selected_branch: action == "checkout",
                selected_branch: None,
                ..Default::default()
            },
            cx,
        );
    }

    pub(in crate::app) fn create_git_branch(
        &mut self,
        branch_name: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git branch creation".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        let worker_branch = branch_name.clone();
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: format!("create-branch:{branch_name}"),
                cancellable: false,
            },
            move |service, path| service.create_project_git_branch(&path, &worker_branch, true),
            GitOperationCompletion {
                success_message: format!("created and checked out Git branch: {branch_name}"),
                failure_prefix: "Git branch creation failed".to_string(),
                clear_commit_message: false,
                refresh_review: false,
                clear_selected_branch: false,
                selected_branch: Some(branch_name),
                ..Default::default()
            },
            cx,
        );
    }

    pub(in crate::app) fn merge_git_branch(
        &mut self,
        branch_name: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git merge".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        let worker_branch = branch_name.clone();
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: format!("merge:{branch_name}"),
                cancellable: false,
            },
            move |service, path| service.merge_project_git_branch(&path, &worker_branch, false),
            GitOperationCompletion {
                success_message: format!("merged Git branch: {branch_name}"),
                failure_prefix: format!("Git merge {branch_name} failed"),
                clear_commit_message: false,
                refresh_review: true,
                clear_selected_branch: false,
                selected_branch: None,
                ..Default::default()
            },
            cx,
        );
    }

    pub(in crate::app) fn squash_merge_git_branch(
        &mut self,
        branch_name: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git squash merge".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        let worker_branch = branch_name.clone();
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: format!("squash-merge:{branch_name}"),
                cancellable: false,
            },
            move |service, path| service.merge_project_git_branch(&path, &worker_branch, true),
            GitOperationCompletion {
                success_message: format!("squash merged Git branch: {branch_name}"),
                failure_prefix: format!("Git squash merge {branch_name} failed"),
                clear_commit_message: false,
                refresh_review: true,
                clear_selected_branch: false,
                selected_branch: None,
                ..Default::default()
            },
            cx,
        );
    }

    pub(in crate::app) fn delete_selected_git_branch(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.state.selected_project.is_none() {
            self.status_message = "no selected project for Git branch deletion".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let Some(branch_name) = self.selected_git_branch.clone() else {
            self.status_message = "no selected Git branch to delete".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        if self.git_running_operation.is_some() {
            self.status_message = "Git operation is already running".to_string();
            self.invalidate_git_panel(cx);
            return;
        }

        let title = self.text("git.branch.delete_local", "Delete Local Branch");
        let message = self
            .text(
                "git.branch.delete.confirm_format",
                "Delete local branch %@?",
            )
            .replace("%@", &branch_name);
        let confirm_label = self.text("common.delete", "Delete");
        let cancel_label = self.text("common.cancel", "Cancel");
        let service = self.runtime_service.clone();
        self.status_message = "waiting for Git branch deletion confirmation".to_string();
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

            let _ = this.update(cx, |app, cx| match result {
                Ok(true) => app.delete_selected_git_branch_confirmed(branch_name, cx),
                Ok(false) => {
                    app.status_message = "Git branch deletion canceled".to_string();
                    app.invalidate_git_panel(cx);
                }
                Err(error) => {
                    app.status_message =
                        format!("failed to show branch deletion confirmation: {error}");
                    app.invalidate_git_panel(cx);
                }
            });
        })
        .detach();
        self.invalidate_git_panel(cx);
    }

    pub(in crate::app) fn delete_selected_git_branch_confirmed(
        &mut self,
        branch_name: String,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = &self.state.selected_project else {
            self.status_message = "no selected project for Git branch deletion".to_string();
            self.invalidate_git_panel(cx);
            return;
        };
        let project_id = project.id.clone();
        let project_path = project.path.clone();
        let worker_branch = branch_name.clone();
        self.start_project_git_operation(
            project_id,
            project_path,
            GitRunningOperation {
                label: format!("delete-branch:{branch_name}"),
                cancellable: false,
            },
            move |service, path| service.delete_project_git_branch(&path, &worker_branch, false),
            GitOperationCompletion {
                success_message: format!("deleted Git branch: {branch_name}"),
                failure_prefix: "Git branch deletion failed".to_string(),
                clear_commit_message: false,
                refresh_review: false,
                clear_selected_branch: false,
                selected_branch: None,
                ..Default::default()
            },
            cx,
        );
    }

    /// Shared confirm-dialog gate for destructive git actions.
    pub(in crate::app) fn create_git_branch_from(
        &mut self,
        branch_name: String,
        from: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let worker_branch = branch_name.clone();
        let worker_from = from.clone();
        self.run_simple_git_operation(
            format!("create-branch:{branch_name}"),
            false,
            format!("created Git branch {branch_name} from {from}"),
            "Git branch creation failed".to_string(),
            move |service, path| {
                service.create_project_git_branch_from(
                    &path,
                    &worker_branch,
                    Some(&worker_from),
                    true,
                )
            },
            cx,
        );
    }

    pub(in crate::app) fn rename_git_branch(
        &mut self,
        branch: String,
        new_name: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let worker_branch = branch.clone();
        let worker_new_name = new_name.clone();
        self.run_simple_git_operation(
            format!("rename-branch:{branch}"),
            false,
            format!("renamed Git branch {branch} to {new_name}"),
            "Git branch rename failed".to_string(),
            move |service, path| {
                service.rename_project_git_branch(&path, &worker_branch, &worker_new_name)
            },
            cx,
        );
    }

    pub(in crate::app) fn rebase_git_branch(
        &mut self,
        branch: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let worker_branch = branch.clone();
        self.run_simple_git_operation(
            format!("rebase:{branch}"),
            false,
            format!("rebased onto Git branch: {branch}"),
            format!("Git rebase onto {branch} failed"),
            move |service, path| service.rebase_project_git_branch(&path, &worker_branch),
            cx,
        );
    }
}
