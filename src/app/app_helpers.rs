use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(test)]
use codux_runtime::ssh::SSHProfileSummary;
use codux_runtime::{git::GitSummary, runtime_state::FileEntry};

#[cfg(test)]
use super::shell_utils::shell_join;

pub(in crate::app) fn git_remote_action_label(action: &str) -> String {
    if let Some(remote) = action.strip_prefix("push:") {
        return format!("push to {remote}");
    }
    if let Some(remote_branch) = action.strip_prefix("push-branch:") {
        return format!("push to {remote_branch}");
    }

    match action {
        "force-push" => "force push".to_string(),
        _ => action.to_string(),
    }
}

pub(in crate::app) fn normalized_git_action_paths(paths: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for path in paths {
        let path = path.trim().trim_start_matches('/').to_string();
        if path.is_empty() || !seen.insert(path.clone()) {
            continue;
        }
        normalized.push(path);
    }
    normalized
}

pub(in crate::app) fn file_search_status_message(index: usize, count: usize) -> String {
    if count == 0 {
        "file search has no matches".to_string()
    } else {
        format!("file search match {} of {count}", index + 1)
    }
}

#[cfg(test)]
pub(in crate::app) fn ssh_connect_command(profile: &SSHProfileSummary) -> String {
    shell_join(vec!["codux-ssh".to_string(), profile.id.clone()])
}

pub(in crate::app) fn generated_git_commit_message(git: &GitSummary) -> String {
    let changed = git.staged + git.unstaged + git.untracked;
    if git.staged > 0 {
        format!("Update {} staged file{}", git.staged, plural(git.staged))
    } else if changed > 0 {
        format!("Update {} changed file{}", changed, plural(changed))
    } else {
        "Update project files".to_string()
    }
}

pub(in crate::app) fn generated_git_branch_name() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("codux-gpui-{timestamp}")
}

pub(in crate::app) fn generated_project_child_name(files: &[FileEntry], directory: bool) -> String {
    let prefix = if directory {
        "codux-folder"
    } else {
        "codux-file"
    };
    let suffix = if directory { "" } else { ".txt" };
    for index in 1..1000 {
        let name = format!("{prefix}-{index}{suffix}");
        if !files.iter().any(|file| file.name == name) {
            return name;
        }
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{prefix}-{timestamp}{suffix}")
}

pub(in crate::app) const PROJECT_BADGE_COLORS: &[&str] = &[
    "#0A84FF", "#8C52FF", "#4C8BF5", "#15B8A6", "#32C766", "#FFB020", "#FF7A59", "#FF5C8A",
    "#7B61FF", "#00A3FF", "#6D9F71",
];

pub(in crate::app) fn project_badge_text_from_name(name: &str) -> Option<String> {
    let badge = name
        .chars()
        .filter(|character| !character.is_whitespace())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    (!badge.is_empty()).then_some(badge)
}

pub(in crate::app) fn join_relative_child_path(parent: &str, name: &str) -> String {
    let parent = parent.trim().trim_matches('/');
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

pub(in crate::app) fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
