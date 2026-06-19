//! Worktree listing for the headless host via the `git worktree` CLI. The
//! desktop has a richer WorktreeService (tasks, default/selected state in
//! state.json); the agent reports the real git worktrees so a controller's
//! worktree panel populates. Mutations (create/merge/remove) follow the same
//! CLI approach as the git domain.

use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::process::Command;

/// A `worktree.list` reply: the project's real git worktrees, mapped into the
/// shape the controller deserializes into `WorktreeInfo`.
pub fn worktree_list_payload(project_id: &str, project_path: &str) -> Value {
    let mut worktrees = Vec::new();
    if let Ok(output) = Command::new("git")
        .arg("-C")
        .arg(project_path)
        .args(["worktree", "list", "--porcelain"])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut path: Option<String> = None;
            let mut branch = String::new();
            for line in text.lines() {
                if let Some(value) = line.strip_prefix("worktree ") {
                    path = Some(value.to_string());
                    branch = String::new();
                } else if let Some(value) = line.strip_prefix("branch ") {
                    branch = value.trim_start_matches("refs/heads/").to_string();
                } else if line.trim().is_empty() {
                    if let Some(path) = path.take() {
                        let is_default = worktrees.is_empty();
                        worktrees.push(worktree_entry(project_id, &path, &branch, is_default));
                    }
                }
            }
            if let Some(path) = path.take() {
                let is_default = worktrees.is_empty();
                worktrees.push(worktree_entry(project_id, &path, &branch, is_default));
            }
        }
    }
    let selected = worktrees
        .first()
        .and_then(|entry| entry.get("id").cloned())
        .unwrap_or(Value::Null);
    json!({
        "projectId": project_id,
        "selectedWorktreeId": selected,
        "worktrees": worktrees,
        "tasks": [],
        "available": true,
        "baseBranches": [],
        "defaultBaseBranch": "",
        "error": Value::Null,
    })
}

/// Create a worktree (`git worktree add`) at a sibling `.worktrees/<branch>` dir.
pub fn worktree_create(
    project_path: &str,
    branch_name: &str,
    base_branch: Option<&str>,
) -> Result<(), String> {
    if branch_name.trim().is_empty() {
        return Err("A branch name is required.".to_string());
    }
    let target = worktree_target_path(project_path, branch_name);
    let mut args: Vec<&str> = vec!["worktree", "add", &target, "-b", branch_name];
    if let Some(base) = base_branch.filter(|value| !value.trim().is_empty()) {
        args.push(base);
    }
    run_git(project_path, &args)
}

/// Remove a worktree (`git worktree remove`), optionally deleting its branch.
pub fn worktree_remove(
    project_path: &str,
    worktree_path: &str,
    remove_branch: bool,
) -> Result<(), String> {
    let branch = remove_branch
        .then(|| {
            Command::new("git")
                .arg("-C")
                .arg(worktree_path)
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        })
        .flatten();
    run_git(project_path, &["worktree", "remove", "--force", worktree_path])?;
    if let Some(branch) = branch.filter(|value| !value.is_empty() && value != "HEAD") {
        let _ = run_git(project_path, &["branch", "-D", &branch]);
    }
    Ok(())
}

/// Merge a worktree's branch into `base_branch` (in the main worktree),
/// optionally removing the worktree + branch afterwards.
pub fn worktree_merge(
    project_path: &str,
    worktree_path: &str,
    base_branch: Option<&str>,
    remove_branch: bool,
) -> Result<(), String> {
    let branch = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty() && value != "HEAD")
        .ok_or_else(|| "Could not resolve the worktree's branch.".to_string())?;
    if let Some(base) = base_branch.filter(|value| !value.trim().is_empty()) {
        run_git(project_path, &["checkout", base])?;
    }
    run_git(project_path, &["merge", "--no-edit", &branch])?;
    if remove_branch {
        let _ = run_git(project_path, &["worktree", "remove", "--force", worktree_path]);
        let _ = run_git(project_path, &["branch", "-D", &branch]);
    }
    Ok(())
}

fn worktree_target_path(project_path: &str, branch_name: &str) -> String {
    let safe = branch_name.replace(['/', '\\'], "-");
    format!("{}/.worktrees/{safe}", project_path.trim_end_matches('/'))
}

fn run_git(repo: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run git: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn worktree_entry(project_id: &str, path: &str, branch: &str, is_default: bool) -> Value {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    let id = format!("wt-{:016x}", hasher.finish());
    let name = if branch.is_empty() {
        Path::new(path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("worktree")
            .to_string()
    } else {
        branch.to_string()
    };
    json!({
        "id": id,
        "projectId": project_id,
        "name": name,
        "branch": branch,
        "path": path,
        "status": "active",
        "isDefault": is_default,
        "exists": Path::new(path).exists(),
        "gitSummary": {
            "changes": changed_file_count(path),
            "incoming": 0,
            "outgoing": 0,
            "additions": 0,
            "deletions": 0,
        },
    })
}

fn changed_file_count(path: &str) -> usize {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count()
        })
        .unwrap_or(0)
}
