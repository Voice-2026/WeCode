// Worktree create/remove/merge git2 operations now live in the shared
// `wecode_git::worktree` engine (both remote hosts call it). Only the shared id
// helpers stay here for the desktop's listing/summary code.
pub(super) use wecode_runtime_core::worktree::{worktree_display_name, worktree_uuid};
