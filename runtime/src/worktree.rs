mod git_ops;
mod scan;
mod snapshot;
mod state;
#[cfg(test)]
mod tests;
mod types;

pub use types::*;

use crate::git::GitService;
use git_ops::*;
use scan::{ScannedWorktreeSnapshot, scan_git_worktrees};
use serde_json::{Map, Value};
use snapshot::{
    project_worktree_git_summary, project_worktree_snapshot, scanned_task_to_snapshot,
    scanned_worktree_to_snapshot,
};
use state::{
    StateFile, enrich_scanned_snapshot_from_state, merge_worktree_snapshot, raw_snapshot,
    save_raw_snapshot, selected_worktree_id_from_state,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

type GitRepository = git2::Repository;

pub struct WorktreeService {
    state_file: PathBuf,
}

impl WorktreeService {
    pub fn new(support_dir: PathBuf) -> Self {
        Self {
            state_file: support_dir.join("state.json"),
        }
    }
}

include!("worktree/service_summary.rs");
include!("worktree/service_state.rs");
include!("worktree/service_snapshot.rs");
include!("worktree/service_operations.rs");
