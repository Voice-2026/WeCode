use super::*;
use codux_runtime::{i18n::translate, settings::locale_from_language_setting};
use gpui_component::resizable::{h_resizable, resizable_panel};

impl CoduxApp {
    pub(in crate::app) fn review_workspace_body(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let locale = locale_from_language_setting(&self.state.settings.language);
        let tr = |key: &str, fallback: &str| translate(&locale, key, fallback);
        let review_title = if self.git_review.mode == "taskBranch" {
            tr("worktree.review.title", "Worktree Review")
        } else {
            tr("worktree.review.audit_title", "Uncommitted Audit")
        };
        let review_subtitle = if self.git_review.mode == "taskBranch" {
            self.git_review
                .base_branch
                .as_ref()
                .map(|base| format!("{} <- {base}", self.state.git.branch))
                .unwrap_or_else(|| self.git_review.diff_stat.clone())
        } else if self.git_review.diff_stat.trim().is_empty() {
            self.state
                .selected_project
                .as_ref()
                .map(|project| project.path.clone())
                .unwrap_or_else(|| tr("worktree.review.audit_working_tree", "Working Tree"))
        } else {
            self.git_review.diff_stat.clone()
        };
        let changed_files_count = tr("worktree.review.changed_files_count_format", "%@ files")
            .replace("%@", &self.git_review.files.len().to_string());
        let selected_path = self
            .selected_git_file
            .as_deref()
            .filter(|path| self.git_review.files.iter().any(|file| file.path == *path));
        let selected_content = self
            .git_review_content
            .as_ref()
            .filter(|content| selected_path == Some(content.path.as_str()));
        let selected_rows = selected_content.and(self.git_review_aligned_rows.as_ref());
        let git_labels = Rc::new(sidebars::GitSidebarLabels::load(
            &self.state.settings.language,
        ));

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .bg(color(theme::BG_TERMINAL))
            .child(
                div()
                    .h(px(56.0))
                    .px_5()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().title_bar)
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .size(px(32.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_sm()
                                    .bg(color(theme::ACCENT).opacity(0.14))
                                    .text_color(color(theme::ACCENT))
                                    .child(Icon::new(HeroIconName::CodeBracket).size_4()),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .line_height(px(18.0))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(color(theme::TEXT))
                                            .child(review_title),
                                    )
                                    .child(
                                        div()
                                            .mt(px(2.0))
                                            .text_size(px(12.0))
                                            .line_height(px(16.0))
                                            .text_color(color(theme::TEXT_DIM))
                                            .truncate()
                                            .child(review_subtitle),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_size(px(12.0))
                            .line_height(px(16.0))
                            .text_color(color(theme::TEXT_DIM))
                            .child(changed_files_count),
                    ),
            )
            .child(
                h_resizable("git-review-workspace-split")
                    .child(
                        resizable_panel()
                            .size(px(360.0))
                            .size_range(px(260.0)..px(520.0))
                            .child(div().flex().flex_col().size_full().min_h_0().child(
                                git_review_file_list(
                                    &self.git_review,
                                    selected_path,
                                    &self.git_expanded_dirs,
                                    git_labels.clone(),
                                    cx,
                                ),
                            )),
                    )
                    .child(resizable_panel().size_range(px(520.0)..px(1600.0)).child(
                        div().size_full().min_w_0().child(git_review_workspace(
                            selected_path,
                            &self.git_review,
                            selected_content,
                            selected_rows,
                            git_labels,
                            self.git_review_code_scroll_handle.clone(),
                            cx,
                        )),
                    )),
            )
    }
}
