use super::*;
use crate::app::quick_pick::QuickPickItem;

pub(in crate::app) fn show_create_git_branch_dialog(
    app_entity: gpui::Entity<WeCodeApp>,
    language: &str,
    local_branches: Vec<(String, bool)>,
    remote_branches: Vec<String>,
    window: &mut Window,
    cx: &mut App,
) {
    let locale = locale_from_language_setting(language);
    let tr = |key: &str, fallback: &str| translate(&locale, key, fallback);
    let new_branch_label = tr("git.branch.create_and_switch", "New Branch");
    let branch_name_placeholder = tr("git.branch.new.message", "Enter a new branch name.");
    let source_picker_label = tr("git.branch.select_source", "Select source branch");
    let current_label = tr("git.branch.current_label", "Current branch");
    let local_label = tr("git.branch.local", "Local branch");
    let remote_label = tr("git.branch.remote", "Remote branch");
    let action_picker_label = tr("git.branch.create_action", "Create branch");
    let create_and_switch_label = tr("git.branch.create_and_switch", "Create and switch");
    let create_only_label = tr("git.branch.create_only", "Create without switching");
    let current_branch = local_branches
        .iter()
        .find(|(_, is_current)| *is_current)
        .map(|(name, _)| name.clone())
        .unwrap_or_else(|| "HEAD".to_string());
    let title = format!("{new_branch_label} · {current_label}: {current_branch}");
    let entity = app_entity.clone();
    super::super::quick_input::show_quick_input(
        title,
        branch_name_placeholder,
        generated_git_branch_name(),
        false,
        move |name, window, cx| {
            let mut local_branches = local_branches.clone();
            local_branches.sort_by(|left, right| {
                right
                    .1
                    .cmp(&left.1)
                    .then_with(|| left.0.to_lowercase().cmp(&right.0.to_lowercase()))
            });
            let mut items = local_branches
                .iter()
                .map(|(branch, is_current)| {
                    QuickPickItem::new(branch.clone(), branch.clone())
                        .icon(Icon::new(if *is_current {
                            HeroIconName::Check
                        } else {
                            HeroIconName::ArrowPathRoundedSquare
                        }))
                        .description(if *is_current {
                            current_label.clone()
                        } else {
                            local_label.clone()
                        })
                })
                .collect::<Vec<_>>();
            let mut seen = local_branches
                .iter()
                .map(|(branch, _)| branch.clone())
                .collect::<HashSet<_>>();
            items.extend(remote_branches.iter().filter_map(|branch| {
                if !seen.insert(branch.clone()) {
                    return None;
                }
                Some(
                    QuickPickItem::new(branch.clone(), branch.clone())
                        .icon(Icon::new(HeroIconName::GlobeAlt))
                        .description(remote_label.clone()),
                )
            }));
            if items.is_empty() {
                entity.update(cx, |app, cx| app.create_git_branch(name, window, cx));
                return;
            }
            let entity = entity.clone();
            let action_picker_label = action_picker_label.clone();
            let create_and_switch_label = create_and_switch_label.clone();
            let create_only_label = create_only_label.clone();
            super::super::quick_pick::show_quick_pick(
                source_picker_label.clone(),
                items,
                move |source, window, cx| {
                    const CREATE_AND_SWITCH: &str = "create-and-switch";
                    const CREATE_ONLY: &str = "create-only";
                    let preview = format!("{source} → {name}");
                    let actions = vec![
                        QuickPickItem::new(CREATE_AND_SWITCH, create_and_switch_label.clone())
                            .icon(Icon::new(HeroIconName::ArrowRight))
                            .description(preview.clone()),
                        QuickPickItem::new(CREATE_ONLY, create_only_label.clone())
                            .icon(Icon::new(HeroIconName::Plus))
                            .description(preview),
                    ];
                    let entity = entity.clone();
                    let name = name.clone();
                    let source = source.to_string();
                    super::super::quick_pick::show_quick_pick(
                        action_picker_label.clone(),
                        actions,
                        move |action, window, cx| {
                            entity.update(cx, |app, cx| {
                                app.create_git_branch_from_with_checkout(
                                    name.clone(),
                                    source.clone(),
                                    action.as_ref() == CREATE_AND_SWITCH,
                                    window,
                                    cx,
                                );
                            });
                        },
                        window,
                        cx,
                    );
                },
                window,
                cx,
            );
        },
        window,
        cx,
    );
}

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
        self.create_git_branch_from_with_checkout(branch_name, from, true, _window, cx);
    }

    pub(in crate::app) fn create_git_branch_from_with_checkout(
        &mut self,
        branch_name: String,
        from: String,
        checkout: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let worker_branch = branch_name.clone();
        let worker_from = from.clone();
        let success_message = if checkout {
            format!("created and checked out Git branch {branch_name} from {from}")
        } else {
            format!("created Git branch {branch_name} from {from}")
        };
        self.run_simple_git_operation(
            format!("create-branch:{branch_name}"),
            false,
            success_message,
            "Git branch creation failed".to_string(),
            move |service, path| {
                service.create_project_git_branch_from(
                    &path,
                    &worker_branch,
                    Some(&worker_from),
                    checkout,
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
