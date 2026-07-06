use super::*;

pub(in crate::app) struct ReviewWorkspaceView {
    app_entity: gpui::Entity<CoduxApp>,
    snapshot: super::workspace_review::ReviewWorkspaceSnapshot,
    file_list_view: Option<gpui::Entity<ReviewFileListView>>,
    diff_content_view: Option<gpui::Entity<ReviewDiffContentView>>,
}

pub(in crate::app) struct StatsWorkspaceView {
    app_entity: gpui::Entity<CoduxApp>,
    snapshot: super::workspace_stats::StatsWorkspaceSnapshot,
    scroll_handle: gpui::ScrollHandle,
    container_width: Option<Pixels>,
    project_table: gpui::Entity<
        gpui_component::table::TableState<super::workspace_stats::StatsProjectTableDelegate>,
    >,
}

impl StatsWorkspaceView {
    pub(super) fn new(
        app_entity: gpui::Entity<CoduxApp>,
        snapshot: super::workspace_stats::StatsWorkspaceSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let project_table = cx.new(|cx| {
            gpui_component::table::TableState::new(
                super::workspace_stats::StatsProjectTableDelegate::new(
                    snapshot.project_rows(),
                    snapshot.language().to_string(),
                ),
                window,
                cx,
            )
            .col_selectable(false)
            .col_movable(false)
        });
        Self {
            app_entity,
            snapshot,
            scroll_handle: gpui::ScrollHandle::default(),
            container_width: None,
            project_table,
        }
    }

    pub(in crate::app) fn set_snapshot(
        &mut self,
        snapshot: super::workspace_stats::StatsWorkspaceSnapshot,
        cx: &mut Context<Self>,
    ) {
        if self.snapshot == snapshot {
            return;
        }
        self.project_table.update(cx, |table, cx| {
            table
                .delegate_mut()
                .set_rows(snapshot.project_rows(), snapshot.language().to_string());
            table.refresh(cx);
            cx.notify();
        });
        self.snapshot = snapshot;
        cx.notify();
    }
}

impl Render for StatsWorkspaceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::workspace_stats::stats_workspace_body(
            self.app_entity.clone(),
            self.project_table.clone(),
            self.scroll_handle.clone(),
            self.snapshot.clone(),
            self.container_width,
            cx,
        )
        .on_prepaint({
            let view = cx.entity();
            move |bounds, _, cx| {
                view.update(cx, |view, cx| {
                    let width = bounds.size.width;
                    if view
                        .container_width
                        .is_none_or(|recorded| (recorded - width).abs() > px(1.0))
                    {
                        view.container_width = Some(width);
                        cx.notify();
                    }
                });
            }
        })
        .into_any_element()
    }
}

impl ReviewWorkspaceView {
    pub(super) fn new(
        app_entity: gpui::Entity<CoduxApp>,
        snapshot: super::workspace_review::ReviewWorkspaceSnapshot,
    ) -> Self {
        Self {
            app_entity,
            snapshot,
            file_list_view: None,
            diff_content_view: None,
        }
    }

    pub(in crate::app) fn set_snapshot(
        &mut self,
        snapshot: super::workspace_review::ReviewWorkspaceSnapshot,
        cx: &mut Context<Self>,
    ) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }
}

impl Render for ReviewWorkspaceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.snapshot.clone();
        let file_list_snapshot = snapshot.file_list_snapshot();
        let diff_content_snapshot = snapshot.diff_content_snapshot();
        let file_list_view = self.review_file_list_view(file_list_snapshot, cx);
        let diff_content_view = self.review_diff_content_view(diff_content_snapshot, cx);
        super::workspace_review::review_workspace_body(
            snapshot,
            file_list_view,
            diff_content_view,
            cx,
        )
        .into_any_element()
    }
}

impl ReviewWorkspaceView {
    fn review_file_list_view(
        &mut self,
        snapshot: super::workspace_review::ReviewFileListSnapshot,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<ReviewFileListView> {
        if let Some(view) = &self.file_list_view {
            view.update(cx, |view, cx| view.set_snapshot(snapshot, cx));
            return view.clone();
        }
        let view = cx.new(|_| ReviewFileListView::new(self.app_entity.clone(), snapshot));
        self.file_list_view = Some(view.clone());
        view
    }

    fn review_diff_content_view(
        &mut self,
        snapshot: super::workspace_review::ReviewDiffContentSnapshot,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<ReviewDiffContentView> {
        if let Some(view) = &self.diff_content_view {
            view.update(cx, |view, cx| view.set_snapshot(snapshot, cx));
            return view.clone();
        }
        let view = cx.new(|_| ReviewDiffContentView::new(snapshot));
        self.diff_content_view = Some(view.clone());
        view
    }
}

pub(in crate::app) struct ReviewFileListView {
    app_entity: gpui::Entity<CoduxApp>,
    snapshot: super::workspace_review::ReviewFileListSnapshot,
}

impl ReviewFileListView {
    fn new(
        app_entity: gpui::Entity<CoduxApp>,
        snapshot: super::workspace_review::ReviewFileListSnapshot,
    ) -> Self {
        Self {
            app_entity,
            snapshot,
        }
    }

    fn set_snapshot(
        &mut self,
        snapshot: super::workspace_review::ReviewFileListSnapshot,
        cx: &mut Context<Self>,
    ) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }
}

impl Render for ReviewFileListView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.snapshot.clone();
        super::workspace_review::review_file_list(self.app_entity.clone(), snapshot, cx)
            .into_any_element()
    }
}

pub(in crate::app) struct ReviewDiffContentView {
    snapshot: super::workspace_review::ReviewDiffContentSnapshot,
}

impl ReviewDiffContentView {
    fn new(snapshot: super::workspace_review::ReviewDiffContentSnapshot) -> Self {
        Self { snapshot }
    }

    fn set_snapshot(
        &mut self,
        snapshot: super::workspace_review::ReviewDiffContentSnapshot,
        cx: &mut Context<Self>,
    ) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }
}

impl Render for ReviewDiffContentView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.snapshot.clone();
        super::workspace_review::review_diff_content(snapshot, cx).into_any_element()
    }
}
