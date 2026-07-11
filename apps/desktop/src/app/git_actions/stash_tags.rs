use super::*;

impl WeCodeApp {
    pub(in crate::app) fn fetch_prune_project_git(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.run_simple_git_operation(
            "fetch-prune".to_string(),
            true,
            "fetched and pruned remote updates".to_string(),
            "Git fetch (prune) failed".to_string(),
            move |service, path| service.fetch_prune_project_git(&path),
            cx,
        );
    }

    pub(in crate::app) fn delete_git_remote_branch(
        &mut self,
        remote_branch: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = self.text("git.branch.delete_remote", "Delete Remote Branch");
        let message = self
            .text(
                "git.branch.delete_remote.confirm_format",
                "Delete remote branch %@?",
            )
            .replace("%@", &remote_branch);
        let confirm_label = self.text("common.delete", "Delete");
        self.confirm_git_action(
            title,
            message,
            confirm_label,
            move |app, cx| {
                let worker_branch = remote_branch.clone();
                app.run_simple_git_operation(
                    format!("delete-remote-branch:{remote_branch}"),
                    true,
                    format!("deleted remote Git branch: {remote_branch}"),
                    "Git remote branch deletion failed".to_string(),
                    move |service, path| {
                        service.delete_project_git_remote_branch(&path, &worker_branch)
                    },
                    cx,
                );
            },
            cx,
        );
    }

    pub(in crate::app) fn stash_git(
        &mut self,
        message: Option<String>,
        include_untracked: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let worker_message = message.clone();
        self.run_simple_git_operation(
            "stash".to_string(),
            false,
            "stashed working tree changes".to_string(),
            "Git stash failed".to_string(),
            move |service, path| {
                service.stash_project_git(&path, worker_message.as_deref(), include_untracked)
            },
            cx,
        );
    }

    pub(in crate::app) fn apply_git_stash(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.run_simple_git_operation(
            format!("stash-apply:{index}"),
            false,
            format!("applied Git stash @{{{index}}}"),
            "Git stash apply failed".to_string(),
            move |service, path| service.apply_project_git_stash(&path, index),
            cx,
        );
    }

    pub(in crate::app) fn pop_git_stash(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.run_simple_git_operation(
            format!("stash-pop:{index}"),
            false,
            format!("popped Git stash @{{{index}}}"),
            "Git stash pop failed".to_string(),
            move |service, path| service.pop_project_git_stash(&path, index),
            cx,
        );
    }

    pub(in crate::app) fn drop_git_stash(
        &mut self,
        index: usize,
        stash_label: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = self.text("git.stash.drop", "Drop Stash");
        let message = self
            .text("git.stash.drop.confirm_format", "Drop stash %@?")
            .replace("%@", &stash_label);
        let confirm_label = self.text("common.delete", "Delete");
        self.confirm_git_action(
            title,
            message,
            confirm_label,
            move |app, cx| {
                app.run_simple_git_operation(
                    format!("stash-drop:{index}"),
                    false,
                    format!("dropped Git stash @{{{index}}}"),
                    "Git stash drop failed".to_string(),
                    move |service, path| service.drop_project_git_stash(&path, index),
                    cx,
                );
            },
            cx,
        );
    }

    pub(in crate::app) fn drop_all_git_stashes(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = self.text("git.stash.drop_all", "Drop All Stashes");
        let message = self.text("git.stash.drop_all.confirm", "Drop all stashes?");
        let confirm_label = self.text("common.delete", "Delete");
        self.confirm_git_action(
            title,
            message,
            confirm_label,
            move |app, cx| {
                app.run_simple_git_operation(
                    "stash-drop-all".to_string(),
                    false,
                    "dropped all Git stashes".to_string(),
                    "Git stash drop failed".to_string(),
                    move |service, path| service.drop_all_project_git_stashes(&path),
                    cx,
                );
            },
            cx,
        );
    }

    pub(in crate::app) fn create_git_tag(
        &mut self,
        name: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let worker_name = name.clone();
        self.run_simple_git_operation(
            format!("create-tag:{name}"),
            false,
            format!("created Git tag: {name}"),
            "Git tag creation failed".to_string(),
            move |service, path| service.create_project_git_tag(&path, &worker_name, None),
            cx,
        );
    }

    pub(in crate::app) fn delete_git_tag(
        &mut self,
        name: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let worker_name = name.clone();
        self.run_simple_git_operation(
            format!("delete-tag:{name}"),
            false,
            format!("deleted Git tag: {name}"),
            "Git tag deletion failed".to_string(),
            move |service, path| service.delete_project_git_tag(&path, &worker_name),
            cx,
        );
    }

    pub(in crate::app) fn push_git_tags(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.run_simple_git_operation(
            "push-tags".to_string(),
            true,
            "pushed Git tags".to_string(),
            "Git tag push failed".to_string(),
            move |service, path| service.push_project_git_tags(&path, None),
            cx,
        );
    }

    pub(in crate::app) fn delete_git_remote_tag(
        &mut self,
        name: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = self.text("git.tag.delete_remote", "Delete Remote Tag");
        let message = self
            .text(
                "git.tag.delete_remote.confirm_format",
                "Delete remote tag %@?",
            )
            .replace("%@", &name);
        let confirm_label = self.text("common.delete", "Delete");
        self.confirm_git_action(
            title,
            message,
            confirm_label,
            move |app, cx| {
                let worker_name = name.clone();
                app.run_simple_git_operation(
                    format!("delete-remote-tag:{name}"),
                    true,
                    format!("deleted remote Git tag: {name}"),
                    "Git remote tag deletion failed".to_string(),
                    move |service, path| {
                        service.delete_project_git_remote_tag(&path, None, &worker_name)
                    },
                    cx,
                );
            },
            cx,
        );
    }
}
