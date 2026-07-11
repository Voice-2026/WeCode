use super::*;
use crate::app::app_events::{ChildWindowUpdateKind, publish_child_window_update};
use crate::app::window_actions::{AuxiliaryWindowSlot, AuxiliaryWindowSpec};

mod terminal_accessors;
mod terminal_layout;
mod terminal_mount;
mod worktree_actions;

#[cfg(test)]
pub(in crate::app) use terminal_layout::restored_live_active_terminal_id;
pub(in crate::app) use terminal_layout::{
    TerminalLayoutSnapshot, active_terminal_slot_indices, terminal_runtime_summary_from_inputs,
};
