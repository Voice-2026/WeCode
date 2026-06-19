//! Local git mutations the headless host runs with git2.

use git2::build::CheckoutBuilder;
use git2::{DiffFormat, DiffOptions, IndexAddOption, Repository, Signature};
use std::path::Path;

/// Stage the given project-relative paths (adds, modifies, deletes).
pub fn stage(repo_path: &str, paths: &[String]) -> Result<(), String> {
    let repo = Repository::open(repo_path).map_err(|error| error.to_string())?;
    let mut index = repo.index().map_err(|error| error.to_string())?;
    index
        .add_all(paths.iter(), IndexAddOption::DEFAULT, None)
        .map_err(|error| error.to_string())?;
    index.write().map_err(|error| error.to_string())
}

/// Unstage the given paths (reset their index entries to HEAD).
pub fn unstage(repo_path: &str, paths: &[String]) -> Result<(), String> {
    let repo = Repository::open(repo_path).map_err(|error| error.to_string())?;
    match repo.head().ok().and_then(|head| head.peel_to_commit().ok()) {
        Some(commit) => repo
            .reset_default(Some(commit.as_object()), paths.iter())
            .map_err(|error| error.to_string()),
        None => {
            // No commits yet: drop the entries from the index.
            let mut index = repo.index().map_err(|error| error.to_string())?;
            for path in paths {
                let _ = index.remove_path(Path::new(path));
            }
            index.write().map_err(|error| error.to_string())
        }
    }
}

/// Commit the staged index with `message`.
pub fn commit(repo_path: &str, message: &str) -> Result<(), String> {
    let repo = Repository::open(repo_path).map_err(|error| error.to_string())?;
    let mut index = repo.index().map_err(|error| error.to_string())?;
    let tree_oid = index.write_tree().map_err(|error| error.to_string())?;
    let tree = repo.find_tree(tree_oid).map_err(|error| error.to_string())?;
    let signature = repo
        .signature()
        .or_else(|_| Signature::now("Codux", "codux@local"))
        .map_err(|error| error.to_string())?;
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &parents)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Discard worktree changes for the given tracked paths (checkout from HEAD).
pub fn discard(repo_path: &str, paths: &[String]) -> Result<(), String> {
    let repo = Repository::open(repo_path).map_err(|error| error.to_string())?;
    let mut checkout = CheckoutBuilder::new();
    checkout.force();
    for path in paths {
        checkout.path(path);
    }
    repo.checkout_head(Some(&mut checkout))
        .map_err(|error| error.to_string())
}

/// A unified diff (HEAD → working tree, including the index) for one path.
pub fn diff(repo_path: &str, path: &str) -> Result<String, String> {
    let repo = Repository::open(repo_path).map_err(|error| error.to_string())?;
    let head_tree = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_tree().ok());
    let mut options = DiffOptions::new();
    options
        .pathspec(path)
        .include_untracked(true)
        .recurse_untracked_dirs(true);
    let diff = repo
        .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut options))
        .map_err(|error| error.to_string())?;
    let mut out = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        if matches!(line.origin(), '+' | '-' | ' ') {
            out.push(line.origin());
        }
        out.push_str(std::str::from_utf8(line.content()).unwrap_or_default());
        true
    })
    .map_err(|error| error.to_string())?;
    Ok(out)
}
