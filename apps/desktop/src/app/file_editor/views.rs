use super::*;

#[derive(Clone)]
pub(super) struct FileEditorTabDrag {
    pub(super) path: String,
    pub(super) tab: FileEditorTab,
    pub(super) active: bool,
}

impl Render for FileEditorTabDrag {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text_color = if self.tab.dirty {
            color(theme::TEXT)
        } else {
            cx.theme().secondary_foreground
        };
        file_editor_tab_base(text_color)
            .when(self.active, |this| {
                this.bg(color(theme::TEXT).opacity(0.07))
            })
            .child(file_editor_tab_content(
                self.tab.clone(),
                cx.theme().secondary_foreground,
            ))
    }
}

#[derive(Clone)]
pub(in crate::app) struct FileEditorWorkspaceSnapshot {
    pub(super) tabs: Vec<FileEditorTab>,
    pub(super) active_path: Option<String>,
    pub(super) active_preview_path: Option<String>,
    pub(super) single_window: bool,
    pub(super) active_tab: Option<FileEditorTab>,
    pub(super) active_editor: Option<gpui::Entity<InputState>>,
    pub(super) active_loading: bool,
    pub(super) markdown_preview: bool,
    /// True when this editor is rendered as the right-hand split panel (next to
    /// the terminal), so the tab bar shows its own dedicated "close split"
    /// control on the right.
    pub(super) split_active: bool,
}

impl PartialEq for FileEditorWorkspaceSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.tabs == other.tabs
            && self.active_path == other.active_path
            && self.active_preview_path == other.active_preview_path
            && self.single_window == other.single_window
            && self.active_tab == other.active_tab
            && self.active_editor.as_ref().map(|editor| editor.entity_id())
                == other
                    .active_editor
                    .as_ref()
                    .map(|editor| editor.entity_id())
            && self.active_loading == other.active_loading
            && self.markdown_preview == other.markdown_preview
            && self.split_active == other.split_active
    }
}

pub(in crate::app) struct FileEditorWorkspaceView {
    app_entity: gpui::Entity<CoduxApp>,
    snapshot: FileEditorWorkspaceSnapshot,
    chrome_view: Option<gpui::Entity<FileEditorChromeView>>,
    tab_bar_view: Option<gpui::Entity<FileEditorTabBarView>>,
    toolbar_view: Option<gpui::Entity<FileEditorToolbarView>>,
    content_view: Option<gpui::Entity<FileEditorContentView>>,
}

impl FileEditorWorkspaceView {
    pub(in crate::app) fn new(
        app_entity: gpui::Entity<CoduxApp>,
        snapshot: FileEditorWorkspaceSnapshot,
    ) -> Self {
        Self {
            app_entity,
            snapshot,
            chrome_view: None,
            tab_bar_view: None,
            toolbar_view: None,
            content_view: None,
        }
    }

    pub(in crate::app) fn set_snapshot(
        &mut self,
        snapshot: FileEditorWorkspaceSnapshot,
        cx: &mut Context<Self>,
    ) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }

    fn chrome_view(
        &mut self,
        tab_bar_view: gpui::Entity<FileEditorTabBarView>,
        toolbar_view: gpui::Entity<FileEditorToolbarView>,
        show_tab_bar: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<FileEditorChromeView> {
        if let Some(view) = &self.chrome_view {
            view.update(cx, |view, cx| {
                view.set_children(tab_bar_view, toolbar_view, show_tab_bar, cx)
            });
            return view.clone();
        }
        let view = cx.new(|_| FileEditorChromeView::new(tab_bar_view, toolbar_view, show_tab_bar));
        self.chrome_view = Some(view.clone());
        view
    }

    fn tab_bar_view(
        &mut self,
        snapshot: FileEditorTabBarSnapshot,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<FileEditorTabBarView> {
        if let Some(view) = &self.tab_bar_view {
            view.update(cx, |view, cx| view.set_snapshot(snapshot, cx));
            return view.clone();
        }
        let view = cx.new(|_| FileEditorTabBarView::new(self.app_entity.clone(), snapshot));
        self.tab_bar_view = Some(view.clone());
        view
    }

    fn toolbar_view(
        &mut self,
        snapshot: FileEditorToolbarSnapshot,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<FileEditorToolbarView> {
        if let Some(view) = &self.toolbar_view {
            view.update(cx, |view, cx| view.set_snapshot(snapshot, cx));
            return view.clone();
        }
        let view = cx.new(|_| FileEditorToolbarView::new(self.app_entity.clone(), snapshot));
        self.toolbar_view = Some(view.clone());
        view
    }

    fn content_view(
        &mut self,
        active_path: Option<String>,
        editor: Option<gpui::Entity<InputState>>,
        loading: bool,
        markdown_preview: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<FileEditorContentView> {
        if let Some(view) = &self.content_view {
            view.update(cx, |view, cx| {
                view.set_editor(active_path, editor, loading, markdown_preview, cx)
            });
            return view.clone();
        }
        let view =
            cx.new(|_| FileEditorContentView::new(active_path, editor, loading, markdown_preview));
        self.content_view = Some(view.clone());
        view
    }
}

impl Render for FileEditorWorkspaceView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.snapshot.clone();
        let tab_bar_view = self.tab_bar_view(
            FileEditorTabBarSnapshot {
                tabs: snapshot.tabs.clone(),
                active_path: snapshot.active_path.clone(),
                show_split_close: snapshot.split_active,
            },
            cx,
        );
        let toolbar_view = self.toolbar_view(
            FileEditorToolbarSnapshot {
                active_tab: snapshot.active_tab.clone(),
                markdown_preview: snapshot.markdown_preview,
                window_header: snapshot.single_window,
            },
            cx,
        );
        let chrome_view = self.chrome_view(tab_bar_view, toolbar_view, !snapshot.single_window, cx);
        let content_view = self.content_view(
            snapshot.active_preview_path.clone(),
            snapshot.active_editor.clone(),
            snapshot.active_loading,
            snapshot.markdown_preview,
            cx,
        );
        file_editor_workspace(
            self.app_entity.clone(),
            snapshot,
            chrome_view,
            content_view,
            window,
            cx,
        )
    }
}

pub(in crate::app) struct FileEditorChromeView {
    tab_bar_view: gpui::Entity<FileEditorTabBarView>,
    toolbar_view: gpui::Entity<FileEditorToolbarView>,
    show_tab_bar: bool,
}

impl FileEditorChromeView {
    fn new(
        tab_bar_view: gpui::Entity<FileEditorTabBarView>,
        toolbar_view: gpui::Entity<FileEditorToolbarView>,
        show_tab_bar: bool,
    ) -> Self {
        Self {
            tab_bar_view,
            toolbar_view,
            show_tab_bar,
        }
    }

    fn set_children(
        &mut self,
        tab_bar_view: gpui::Entity<FileEditorTabBarView>,
        toolbar_view: gpui::Entity<FileEditorToolbarView>,
        show_tab_bar: bool,
        cx: &mut Context<Self>,
    ) {
        if self.tab_bar_view.entity_id() == tab_bar_view.entity_id()
            && self.toolbar_view.entity_id() == toolbar_view.entity_id()
            && self.show_tab_bar == show_tab_bar
        {
            return;
        }
        self.tab_bar_view = tab_bar_view;
        self.toolbar_view = toolbar_view;
        self.show_tab_bar = show_tab_bar;
        cx.notify();
    }
}

impl Render for FileEditorChromeView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_none()
            .w_full()
            .when(self.show_tab_bar, |this| {
                this.child(gpui::AnyView::from(self.tab_bar_view.clone()))
            })
            .child(gpui::AnyView::from(self.toolbar_view.clone()))
    }
}

#[derive(Clone, PartialEq)]
struct FileEditorTabBarSnapshot {
    tabs: Vec<FileEditorTab>,
    active_path: Option<String>,
    show_split_close: bool,
}

pub(in crate::app) struct FileEditorTabBarView {
    pub(super) app_entity: gpui::Entity<CoduxApp>,
    snapshot: FileEditorTabBarSnapshot,
    tab_scroll_handle: ScrollHandle,
}

impl FileEditorTabBarView {
    fn new(app_entity: gpui::Entity<CoduxApp>, snapshot: FileEditorTabBarSnapshot) -> Self {
        Self {
            app_entity,
            snapshot,
            tab_scroll_handle: ScrollHandle::new(),
        }
    }

    fn set_snapshot(&mut self, snapshot: FileEditorTabBarSnapshot, cx: &mut Context<Self>) {
        if self.snapshot == snapshot {
            return;
        }
        if self.snapshot.active_path != snapshot.active_path
            && let Some(index) = snapshot
                .tabs
                .iter()
                .position(|tab| Some(tab.relative_path.as_str()) == snapshot.active_path.as_deref())
        {
            self.tab_scroll_handle.scroll_to_item(index);
        }
        self.snapshot = snapshot;
        cx.notify();
    }
}

impl Render for FileEditorTabBarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        file_editor_tab_bar(
            self.app_entity.clone(),
            self.snapshot.tabs.clone(),
            self.snapshot.active_path.clone(),
            self.snapshot.show_split_close,
            self.tab_scroll_handle.clone(),
            cx,
        )
    }
}

#[derive(Clone, PartialEq)]
struct FileEditorToolbarSnapshot {
    active_tab: Option<FileEditorTab>,
    markdown_preview: bool,
    window_header: bool,
}

pub(in crate::app) struct FileEditorToolbarView {
    pub(super) app_entity: gpui::Entity<CoduxApp>,
    snapshot: FileEditorToolbarSnapshot,
}

impl FileEditorToolbarView {
    fn new(app_entity: gpui::Entity<CoduxApp>, snapshot: FileEditorToolbarSnapshot) -> Self {
        Self {
            app_entity,
            snapshot,
        }
    }

    fn set_snapshot(&mut self, snapshot: FileEditorToolbarSnapshot, cx: &mut Context<Self>) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }

    pub(super) fn dispatch_active_file_editor_action(
        &self,
        action: impl gpui::Action + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let app_entity = self.app_entity.clone();
        cx.update_entity(&app_entity, |app, cx| {
            if let Some(editor) = app.active_file_editor_state() {
                editor.update(cx, |state, cx| state.focus(window, cx));
                window.dispatch_action(Box::new(action), cx);
            }
        });
    }
}

impl Render for FileEditorToolbarView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        file_editor_toolbar(
            self.app_entity.clone(),
            self.snapshot.active_tab.clone(),
            self.snapshot.markdown_preview,
            self.snapshot.window_header,
            cx,
        )
    }
}

pub(in crate::app) struct FileEditorContentView {
    active_path: Option<String>,
    editor: Option<gpui::Entity<InputState>>,
    loading: bool,
    markdown_preview: bool,
    markdown_state: Option<gpui::Entity<TextViewState>>,
    markdown_path: Option<String>,
}

impl FileEditorContentView {
    fn new(
        active_path: Option<String>,
        editor: Option<gpui::Entity<InputState>>,
        loading: bool,
        markdown_preview: bool,
    ) -> Self {
        Self {
            active_path,
            editor,
            loading,
            markdown_preview,
            markdown_state: None,
            markdown_path: None,
        }
    }

    fn set_editor(
        &mut self,
        active_path: Option<String>,
        editor: Option<gpui::Entity<InputState>>,
        loading: bool,
        markdown_preview: bool,
        cx: &mut Context<Self>,
    ) {
        if self.active_path == active_path
            && self.editor.as_ref().map(|editor| editor.entity_id())
                == editor.as_ref().map(|editor| editor.entity_id())
            && self.loading == loading
            && self.markdown_preview == markdown_preview
        {
            return;
        }
        self.active_path = active_path;
        self.editor = editor;
        self.loading = loading;
        self.markdown_preview = markdown_preview;
        cx.notify();
    }

    fn markdown_state(
        &mut self,
        path: Option<String>,
        content: &str,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<TextViewState> {
        if self.markdown_path != path {
            self.markdown_state = None;
            self.markdown_path = path;
        }
        if let Some(state) = &self.markdown_state {
            state.update(cx, |state, cx| state.set_text(content, cx));
            return state.clone();
        }
        let state = cx.new(|cx| TextViewState::markdown(content, cx));
        self.markdown_state = Some(state.clone());
        state
    }
}

impl Render for FileEditorContentView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let preview_kind = self
            .active_path
            .as_deref()
            .map(file_preview_kind_for_path)
            .unwrap_or(FilePreviewKind::Text);
        let show_markdown_preview =
            preview_kind == FilePreviewKind::Markdown && self.markdown_preview;
        let markdown_state = if show_markdown_preview {
            self.editor.clone().map(|editor| {
                let content = editor.read(cx).value().to_string();
                self.markdown_state(self.active_path.clone(), &content, cx)
            })
        } else {
            None
        };
        let show_text_editor = preview_kind == FilePreviewKind::Text
            || preview_kind == FilePreviewKind::Markdown && !show_markdown_preview;
        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .size_full()
            .when_some(
                self.active_path
                    .clone()
                    .filter(|_| preview_kind == FilePreviewKind::Image),
                |this, path| {
                    this.flex()
                        .items_center()
                        .justify_center()
                        .p(px(18.0))
                        .child(
                            img(PathBuf::from(path))
                                .max_w_full()
                                .max_h_full()
                                .object_fit(ObjectFit::Contain),
                        )
                },
            )
            .when_some(markdown_state, |this, markdown_state| {
                this.child(super::preview_render::file_preview_markdown(markdown_state))
            })
            .when_some(
                self.editor.clone().filter(|_| show_text_editor),
                |this, editor| {
                    this.child(
                        Input::new(&editor)
                            .appearance(false)
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_size(cx.theme().mono_font_size)
                            .size_full(),
                    )
                },
            )
            .when(
                show_text_editor && self.editor.is_none() && self.loading,
                |this| {
                    this.flex()
                        .items_center()
                        .justify_center()
                        .text_size(rems(0.8125))
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading file...")
                },
            )
    }
}

#[derive(Clone, PartialEq)]
pub(in crate::app) struct FilePreviewWindowSnapshot {
    pub(super) relative_path: Option<String>,
    pub(super) full_path: Option<String>,
    pub(super) kind: FilePreviewKind,
    pub(super) content: String,
    pub(super) error: Option<String>,
    pub(super) language: String,
}

pub(in crate::app) struct FilePreviewWindowView {
    pub(super) app_entity: gpui::Entity<CoduxApp>,
    snapshot: FilePreviewWindowSnapshot,
    markdown_state: Option<gpui::Entity<TextViewState>>,
    markdown_path: Option<String>,
    text_editor: Option<gpui::Entity<InputState>>,
    text_editor_path: Option<String>,
}

impl FilePreviewWindowView {
    pub(in crate::app) fn new(
        app_entity: gpui::Entity<CoduxApp>,
        snapshot: FilePreviewWindowSnapshot,
    ) -> Self {
        Self {
            app_entity,
            snapshot,
            markdown_state: None,
            markdown_path: None,
            text_editor: None,
            text_editor_path: None,
        }
    }

    pub(in crate::app) fn set_snapshot(
        &mut self,
        snapshot: FilePreviewWindowSnapshot,
        cx: &mut Context<Self>,
    ) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }

    fn markdown_state(
        &mut self,
        path: Option<String>,
        content: &str,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<TextViewState> {
        if self.markdown_path != path {
            self.markdown_state = None;
            self.markdown_path = path;
        }
        if let Some(state) = &self.markdown_state {
            state.update(cx, |state, cx| state.set_text(content, cx));
            return state.clone();
        }
        let state = cx.new(|cx| TextViewState::markdown(content, cx));
        self.markdown_state = Some(state.clone());
        state
    }

    fn text_editor(
        &mut self,
        path: Option<String>,
        content: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<InputState> {
        let language = path
            .as_deref()
            .map(file_language_for_path)
            .unwrap_or("text")
            .to_string();
        if self.text_editor_path != path {
            self.text_editor = None;
            self.text_editor_path = path;
        }
        if let Some(editor) = &self.text_editor {
            editor.update(cx, |editor, cx| {
                if editor.value().as_ref() != content.as_str() {
                    editor.set_value(content, window, cx);
                }
            });
            return editor.clone();
        }
        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(language)
                .folding(false)
                .multi_line(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    ..Default::default()
                })
                .default_value(content)
        });
        self.text_editor = Some(editor.clone());
        editor
    }
}

impl Render for FilePreviewWindowView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = self.snapshot.clone();
        let markdown_state =
            if snapshot.kind == FilePreviewKind::Markdown && snapshot.error.is_none() {
                Some(self.markdown_state(snapshot.relative_path.clone(), &snapshot.content, cx))
            } else {
                None
            };
        let text_editor = if matches!(
            snapshot.kind,
            FilePreviewKind::Markdown | FilePreviewKind::Text
        ) && snapshot.error.is_none()
        {
            Some(self.text_editor(
                snapshot.relative_path.clone(),
                snapshot.content.clone(),
                window,
                cx,
            ))
        } else {
            None
        };
        file_preview_window_workspace(
            self.app_entity.clone(),
            snapshot,
            markdown_state,
            text_editor,
            cx,
        )
    }
}
