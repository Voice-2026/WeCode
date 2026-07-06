use super::*;
use crate::app::app_state::FileEditorTab;
use crate::app::ui_helpers::codux_tooltip_container;
use gpui_component::{
    input::{Redo, Search, TabSize, Undo},
    text::{TextView, TextViewState},
};
use std::collections::HashSet;

const FILE_EDITOR_TAB_BAR_HEIGHT: f32 = 38.0;
const FILE_EDITOR_TOOLBAR_HEIGHT: f32 = 56.0;
const FILE_EDITOR_CHROME_HEIGHT: f32 = FILE_EDITOR_TAB_BAR_HEIGHT + FILE_EDITOR_TOOLBAR_HEIGHT;

mod editor_render;
mod paths;
mod preview_render;
mod state;
mod tabs;
mod views;

use editor_render::{
    file_editor_tab_bar, file_editor_tab_base, file_editor_tab_content, file_editor_toolbar,
};
use paths::{
    changed_file_event_relative_paths, file_editor_i18n, file_editor_label, file_language_for_path,
};
use views::{
    FileEditorChromeView, FileEditorContentView, FileEditorTabBarView, FileEditorTabDrag,
    FileEditorToolbarView,
};

pub(in crate::app) use editor_render::file_editor_workspace;
pub(in crate::app) use paths::{
    FilePreviewKind, file_editor_window_title, file_preview_kind_for_path,
};
pub(in crate::app) use preview_render::file_preview_window_workspace;
pub(in crate::app) use views::{
    FileEditorWorkspaceSnapshot, FileEditorWorkspaceView, FilePreviewWindowSnapshot,
    FilePreviewWindowView,
};
