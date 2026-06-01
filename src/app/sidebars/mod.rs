use super::*;

mod ai;
mod files;
mod git;
mod ssh;

use ai::ai_stats_sidebar;
pub(in crate::app) use ai::memory_manager_window_workspace;
pub(in crate::app) use files::{FileTreeRow, file_section};
pub(in crate::app) use files::{clipboard_external_paths, file_tree_rows};
pub(in crate::app) use git::git_section;
pub(in crate::app) use ssh::ssh_section;

pub(in crate::app) use files::{
    current_directory_suffix, file_directory_option, file_preview_workspace,
    parent_relative_directory,
};
pub(in crate::app) use git::{
    git_diff_window_workspace, git_review_workspace, git_workspace_section,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AssistantPanel {
    AIStats,
    SSH,
    FileManager,
    Git,
}

impl CoduxApp {
    pub(super) fn assistant_column(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let Some(panel) = self.assistant_panel else {
            return div().into_any_element();
        };
        if self.state.selected_project.is_none() && panel != AssistantPanel::SSH {
            return div().into_any_element();
        }

        div()
            .flex()
            .flex_col()
            .w(px(318.0))
            .h_full()
            .bg(color(theme::BG_COLUMN))
            .border_l_1()
            .border_color(cx.theme().sidebar_border)
            .child(match panel {
                AssistantPanel::AIStats => div()
                    .flex()
                    .flex_1()
                    .h_full()
                    .min_h_0()
                    .flex_col()
                    .child(ai_stats_sidebar(
                        &self.state.ai_global_history,
                        &self.state.ai_history,
                        self.state
                            .selected_project
                            .as_ref()
                            .map(|project| project.id.as_str()),
                        &self.state.settings.statistics_mode,
                        &self.state.ai_runtime_state,
                        &self.state.settings.language,
                        cx,
                    ))
                    .into_any_element(),
                AssistantPanel::SSH => div()
                    .flex()
                    .flex_1()
                    .h_full()
                    .min_h_0()
                    .flex_col()
                    .child(ssh_section(
                        &self.state.ssh,
                        self.selected_ssh_profile_id.as_deref(),
                        self.ssh_scroll_handle.clone(),
                        &self.state.settings.language,
                        window,
                        cx,
                    ))
                    .into_any_element(),
                AssistantPanel::FileManager => div()
                    .flex()
                    .flex_1()
                    .h_full()
                    .min_h_0()
                    .flex_col()
                    .child(
                        gpui::AnyView::from(self.file_sidebar_view(cx)).cached(
                            gpui::StyleRefinement::default()
                                .flex()
                                .flex_col()
                                .w_full()
                                .h_full()
                                .min_h_0(),
                        ),
                    )
                    .into_any_element(),
                AssistantPanel::Git => div()
                    .flex()
                    .flex_1()
                    .h_full()
                    .min_h_0()
                    .flex_col()
                    .child(git_section(
                        &self.state.git,
                        &self.git_expanded_sections,
                        &self.git_expanded_dirs,
                        &self.git_tree_children,
                        self.selected_git_file.as_deref(),
                        &self.selected_git_files,
                        self.selected_git_branch.as_deref(),
                        self.state
                            .selected_project
                            .as_ref()
                            .and_then(|project| project.git_default_push_remote_name.as_deref()),
                        &self.git_clone_remote_url,
                        &self.state.settings.language,
                        self.git_remote_editor_open,
                        &self.git_remote_name,
                        &self.git_remote_url,
                        self.git_running_operation.as_ref(),
                        &self.git_commit_message,
                        self.git_commit_message_revision,
                        self.git_files_scroll_handle.clone(),
                        self.git_history_scroll_handle.clone(),
                        window,
                        cx,
                    ))
                    .into_any_element(),
            })
            .into_any_element()
    }
}

impl CoduxApp {
    pub(in crate::app) fn file_sidebar_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<FileSidebarView> {
        let project_name = self
            .state
            .selected_project
            .as_ref()
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "Project".to_string());
        let files = self.state.files.clone();
        let tree_children = self.file_tree_children.clone();
        let expanded_dirs = self.file_tree_expanded_dirs.clone();
        let file_directory = self.file_directory.clone();
        let selected_entry = self.selected_file_entry.clone();
        let selected_entries = self.selected_file_entries.clone();
        let draft_kind = self.file_name_draft_kind;
        let draft_target = self.file_name_draft_target.clone();
        let draft_value = self.file_name_draft_value.clone();
        let draft_select_all = self.file_name_draft_select_all;
        let scroll_handle = self.file_tree_scroll_handle.clone();
        let language = self.state.settings.language.clone();

        if let Some(view) = &self.file_sidebar_view {
            view.update(cx, |view, cx| {
                let changed = view.project_name != project_name
                    || view.files != files
                    || view.tree_children != tree_children
                    || view.expanded_dirs != expanded_dirs
                    || view.file_directory != file_directory
                    || view.selected_entry != selected_entry
                    || view.selected_entries != selected_entries
                    || view.draft_kind != draft_kind
                    || view.draft_target != draft_target
                    || view.draft_value != draft_value
                    || view.draft_select_all != draft_select_all
                    || view.language != language;
                if !changed {
                    return;
                }
                let rows = Rc::new(file_tree_rows(
                    &files,
                    &tree_children,
                    &expanded_dirs,
                    selected_entry.as_deref(),
                    &selected_entries,
                    draft_kind,
                    draft_target.as_deref(),
                    &draft_value,
                    0,
                ));
                view.project_name = project_name;
                view.files = files;
                view.tree_children = tree_children;
                view.expanded_dirs = expanded_dirs;
                view.file_directory = file_directory;
                view.selected_entry = selected_entry;
                view.selected_entries = selected_entries;
                view.draft_kind = draft_kind;
                view.draft_target = draft_target;
                view.draft_value = draft_value;
                view.draft_select_all = draft_select_all;
                view.language = language;
                view.scroll_handle = scroll_handle;
                view.rows = rows;
                cx.notify();
            });
            return view.clone();
        }

        let app_entity = cx.entity();
        let rows = Rc::new(file_tree_rows(
            &files,
            &tree_children,
            &expanded_dirs,
            selected_entry.as_deref(),
            &selected_entries,
            draft_kind,
            draft_target.as_deref(),
            &draft_value,
            0,
        ));
        let view = cx.new(|cx| FileSidebarView {
            app_entity: app_entity.clone(),
            focus_handle: cx.focus_handle(),
            project_name,
            files,
            tree_children,
            expanded_dirs,
            file_directory,
            selected_entry,
            selected_entries,
            draft_kind,
            draft_target,
            draft_value,
            draft_select_all,
            language,
            rows,
            scroll_handle,
        });
        self.file_sidebar_view = Some(view.clone());
        view
    }
}

pub(in crate::app) struct FileSidebarView {
    app_entity: gpui::Entity<CoduxApp>,
    focus_handle: FocusHandle,
    project_name: String,
    files: Vec<FileEntry>,
    tree_children: HashMap<String, Vec<FileEntry>>,
    expanded_dirs: HashSet<String>,
    file_directory: String,
    selected_entry: Option<String>,
    selected_entries: HashSet<String>,
    draft_kind: Option<FileNameDraftKind>,
    draft_target: Option<String>,
    draft_value: String,
    draft_select_all: bool,
    language: String,
    rows: Rc<Vec<FileTreeRow>>,
    scroll_handle: UniformListScrollHandle,
}

impl Render for FileSidebarView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        file_section(
            self.app_entity.clone(),
            self.focus_handle.clone(),
            &self.project_name,
            &self.files,
            &self.tree_children,
            &self.expanded_dirs,
            &self.file_directory,
            self.selected_entry.as_deref(),
            &self.selected_entries,
            self.draft_kind,
            &self.draft_value,
            self.draft_select_all,
            self.rows.clone(),
            self.scroll_handle.clone(),
            &self.language,
            window,
            cx,
        )
        .into_any_element()
    }
}

fn assistant_panel_header(
    title: impl Into<SharedString>,
    icon: HeroIconName,
    action: impl IntoElement,
) -> impl IntoElement {
    let title = title.into();
    div()
        .h(px(44.0))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(color(theme::BORDER_SOFT))
        .bg(color(theme::BG_HEADER))
        .child(
            div()
                .flex()
                .items_center()
                .child(
                    Icon::new(icon)
                        .size_4()
                        .text_color(color(theme::TEXT_MUTED)),
                )
                .child(
                    div()
                        .ml(px(8.0))
                        .text_size(px(14.0))
                        .line_height(px(18.0))
                        .text_color(color(theme::TEXT))
                        .child(title),
                ),
        )
        .child(action)
}

fn ai_stats_surface(cx: &mut Context<CoduxApp>) -> gpui::Hsla {
    cx.theme().secondary
}

fn ai_stats_track_surface(cx: &mut Context<CoduxApp>) -> gpui::Hsla {
    cx.theme().secondary_hover
}
