//! git status reader for the headless host (git2).

use codux_runtime_core::git::{GitBranchSummary, GitStatusSummary};
use git2::{BranchType, Repository, Status, StatusOptions};
use serde_json::{json, Value};

pub fn git_status_summary(path: &str) -> GitStatusSummary {
    let repo = match Repository::open(path) {
        Ok(repo) => repo,
        Err(_) => {
            return GitStatusSummary {
                is_repository: false,
                ..Default::default()
            };
        }
    };
    let mut summary = GitStatusSummary {
        is_repository: true,
        ..Default::default()
    };

    if let Ok(head) = repo.head() {
        if let Ok(name) = head.shorthand() {
            summary.branch = name.to_string();
        }
    }

    let staged_mask = Status::INDEX_NEW
        | Status::INDEX_MODIFIED
        | Status::INDEX_DELETED
        | Status::INDEX_RENAMED
        | Status::INDEX_TYPECHANGE;
    let unstaged_mask = Status::WT_MODIFIED
        | Status::WT_DELETED
        | Status::WT_RENAMED
        | Status::WT_TYPECHANGE;

    let mut options = StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    if let Ok(statuses) = repo.statuses(Some(&mut options)) {
        for entry in statuses.iter() {
            let status = entry.status();
            let staged = status.intersects(staged_mask);
            let untracked = status.contains(Status::WT_NEW);
            let unstaged = status.intersects(unstaged_mask);
            if staged {
                summary.staged += 1;
            }
            if untracked {
                summary.untracked += 1;
            } else if unstaged {
                summary.unstaged += 1;
            }
            // Match the desktop host's GitFileStatus shape so a controller maps
            // both hosts uniformly.
            let index_status = if status.contains(Status::INDEX_NEW) {
                "A"
            } else if status.contains(Status::INDEX_MODIFIED) {
                "M"
            } else if status.contains(Status::INDEX_DELETED) {
                "D"
            } else if status.contains(Status::INDEX_RENAMED) {
                "R"
            } else if status.contains(Status::INDEX_TYPECHANGE) {
                "T"
            } else {
                ""
            };
            let worktree_status = if status.contains(Status::WT_NEW) {
                "?"
            } else if status.contains(Status::WT_MODIFIED) {
                "M"
            } else if status.contains(Status::WT_DELETED) {
                "D"
            } else if status.contains(Status::WT_RENAMED) {
                "R"
            } else if status.contains(Status::WT_TYPECHANGE) {
                "T"
            } else {
                ""
            };
            summary.changed_files.push(json!({
                "path": entry.path().unwrap_or_default(),
                "indexStatus": index_status,
                "worktreeStatus": worktree_status,
            }));
        }
    }

    if let Ok(branches) = repo.branches(Some(BranchType::Local)) {
        for branch in branches.flatten() {
            if let Ok(Some(name)) = branch.0.name() {
                summary.branches.push(GitBranchSummary {
                    name: name.to_string(),
                    is_current: branch.0.is_head(),
                });
            }
        }
    }

    summary
}


/// Changed files (GitFileStatus shape) under `directory` (project-relative).
pub fn path_status(repo_path: &str, directory: &str) -> Vec<Value> {
    let prefix = directory.trim_matches('/');
    git_status_summary(repo_path)
        .changed_files
        .into_iter()
        .filter(|entry| {
            prefix.is_empty()
                || entry
                    .get("path")
                    .and_then(Value::as_str)
                    .map(|path| path.starts_with(prefix))
                    .unwrap_or(false)
        })
        .collect()
}
