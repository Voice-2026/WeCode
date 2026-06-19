//! Git domain for the headless host. Status/diff/stage/unstage/commit/discard
//! use git2 directly (ops.rs / status.rs); branch, merge, remote and network
//! operations shell out to the host's `git` CLI — which is both far less code
//! than reimplementing them in git2 and uses the host's configured credentials
//! (ssh-agent / credential helper) for push/pull/fetch. The desktop host serves
//! the same ops through its richer GitService.

mod ops;
mod status;

pub use status::git_status_summary;

use serde_json::{json, Value};
use std::process::Command;

fn arg<'a>(args: &'a Value, key: &str) -> &'a str {
    args.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn flag(args: &Value, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn paths(args: &Value) -> Vec<String> {
    args.get("paths")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Run a `git` subcommand in `repo`; surfaces stderr on failure.
fn git_cli(repo: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run git: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(if stderr.trim().is_empty() {
        format!("git {} failed", args.join(" "))
    } else {
        stderr.trim().to_string()
    })
}

/// Capture stdout of a `git` subcommand.
fn git_capture(repo: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run git: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// A mutating git op (`git.invoke`); the caller replies with refreshed status.
pub fn invoke(repo: &str, op: &str, args: &Value) -> Result<(), String> {
    match op {
        "stage" => ops::stage(repo, &paths(args)),
        "unstage" => ops::unstage(repo, &paths(args)),
        "discard" => ops::discard(repo, &paths(args)),
        "commit" => ops::commit(repo, arg(args, "message")),
        "init" => git_cli(repo, &["init"]),
        "checkout_branch" => git_cli(repo, &["checkout", arg(args, "branch")]),
        "checkout_remote_branch" => git_cli(repo, &["checkout", arg(args, "remoteBranch")]),
        "checkout_commit" => git_cli(repo, &["checkout", arg(args, "commit")]),
        "create_branch" => {
            if flag(args, "checkout") {
                git_cli(repo, &["checkout", "-b", arg(args, "branch")])
            } else {
                git_cli(repo, &["branch", arg(args, "branch")])
            }
        }
        "create_branch_from" => {
            let from = arg(args, "from");
            if from.is_empty() {
                return Err("A base branch is required.".to_string());
            }
            git_cli(repo, &["branch", arg(args, "branch"), from])?;
            if flag(args, "checkout") {
                git_cli(repo, &["checkout", arg(args, "branch")])
            } else {
                Ok(())
            }
        }
        "merge_branch" => {
            if flag(args, "squash") {
                git_cli(repo, &["merge", "--squash", arg(args, "branch")])
            } else {
                git_cli(repo, &["merge", "--no-edit", arg(args, "branch")])
            }
        }
        "delete_branch" => git_cli(
            repo,
            &[
                "branch",
                if flag(args, "force") { "-D" } else { "-d" },
                arg(args, "branch"),
            ],
        ),
        "amend" => git_cli(repo, &["commit", "--amend", "-m", arg(args, "message")]),
        "undo_last_commit" => git_cli(repo, &["reset", "--soft", "HEAD~1"]),
        "revert_commit" => git_cli(repo, &["revert", "--no-edit", arg(args, "commit")]),
        "restore_commit" => git_cli(repo, &["reset", "--hard", arg(args, "commit")]),
        "add_remote" => git_cli(repo, &["remote", "add", arg(args, "name"), arg(args, "url")]),
        "remove_remote" => git_cli(repo, &["remote", "remove", arg(args, "name")]),
        "append_gitignore" => append_gitignore(repo, &paths(args)),
        "fetch" => git_cli(repo, &["fetch", "--all", "--prune"]),
        "pull" => git_cli(repo, &["pull"]),
        "push" => git_cli(repo, &["push"]),
        "sync" => {
            git_cli(repo, &["pull", "--rebase"])?;
            git_cli(repo, &["push"])
        }
        "force_push" => git_cli(repo, &["push", "--force-with-lease"]),
        "push_remote" => git_cli(repo, &["push", arg(args, "remote")]),
        "push_remote_branch" => {
            let refspec = format!("{}:{}", arg(args, "localBranch"), arg(args, "remoteBranch"));
            git_cli(repo, &["push", arg(args, "remote"), &refspec])
        }
        other => Err(format!("git op '{other}' is not supported on this host.")),
    }
}

/// A read-only git query (`git.read`).
pub fn read(repo: &str, op: &str, args: &Value) -> Result<Value, String> {
    match op {
        "diff" => Ok(json!({ "diff": ops::diff(repo, arg(args, "filePath"))? })),
        "path_status" => Ok(json!({ "entries": status::path_status(repo, arg(args, "directoryPath")) })),
        "last_commit_message" => Ok(json!({
            "message": git_capture(repo, &["log", "-1", "--pretty=%B"]).unwrap_or_default().trim_end().to_string(),
        })),
        "head_pushed" => {
            // HEAD is pushed when it has an upstream and is not ahead of it.
            let pushed = git_capture(repo, &["rev-list", "--count", "@{u}..HEAD"])
                .map(|count| count.trim() == "0")
                .unwrap_or(false);
            Ok(json!({ "pushed": pushed }))
        }
        "stored_state" => {
            serde_json::to_value(git_status_summary(repo)).map_err(|error| error.to_string())
        }
        other => Err(format!("git read '{other}' is not supported on this host.")),
    }
}

fn append_gitignore(repo: &str, paths: &[String]) -> Result<(), String> {
    use std::io::Write;
    let gitignore = std::path::Path::new(repo).join(".gitignore");
    let mut existing = std::fs::read_to_string(&gitignore).unwrap_or_default();
    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    for path in paths {
        existing.push_str(path);
        existing.push('\n');
    }
    let mut file = std::fs::File::create(&gitignore).map_err(|error| error.to_string())?;
    file.write_all(existing.as_bytes())
        .map_err(|error| error.to_string())
}
