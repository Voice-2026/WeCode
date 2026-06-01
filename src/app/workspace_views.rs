use super::*;
use crate::app::ui_helpers::codux_tooltip;

pub(in crate::app) struct WorkspaceColumnView {
    toolbar_view: gpui::Entity<WorkspaceToolbarView>,
    body_view: gpui::Entity<WorkspaceBodyView>,
    assistant_view: gpui::Entity<WorkspaceAssistantView>,
}

impl WorkspaceColumnView {
    pub(in crate::app) fn new(
        toolbar_view: gpui::Entity<WorkspaceToolbarView>,
        body_view: gpui::Entity<WorkspaceBodyView>,
        assistant_view: gpui::Entity<WorkspaceAssistantView>,
    ) -> Self {
        Self {
            toolbar_view,
            body_view,
            assistant_view,
        }
    }
}

impl Render for WorkspaceColumnView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .flex_basis(px(0.0))
            .min_w_0()
            .min_h_0()
            .w_full()
            .h_full()
            .bg(color(theme::BG_TERMINAL))
            .child(
                div().flex().flex_none().w_full().h(px(44.0)).child(
                    gpui::AnyView::from(self.toolbar_view.clone())
                        .cached(gpui::StyleRefinement::default().flex().w_full().h(px(44.0))),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .w_full()
                    .h_full()
                    .min_w_0()
                    .min_h_0()
                    .child(gpui::AnyView::from(self.body_view.clone()))
                    .child(gpui::AnyView::from(self.assistant_view.clone())),
            )
    }
}

pub(in crate::app) struct WorkspaceToolbarView {
    app_entity: gpui::Entity<CoduxApp>,
    _observe_app: Option<Subscription>,
}

impl WorkspaceToolbarView {
    pub(in crate::app) fn new(app_entity: gpui::Entity<CoduxApp>) -> Self {
        Self {
            app_entity,
            _observe_app: None,
        }
    }

    pub(in crate::app) fn observe_app(&mut self, cx: &mut Context<Self>) {
        if self._observe_app.is_none() {
            self._observe_app = Some(cx.observe(&self.app_entity, |_, _, cx| cx.notify()));
        }
    }
}

impl Render for WorkspaceToolbarView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .w_full()
            .h_full()
            .child(self.app_entity.update(cx, |app, cx| {
                app.workspace_toolbar(window, cx).into_any_element()
            }))
    }
}

pub(in crate::app) struct WorkspaceBodyView {
    app_entity: gpui::Entity<CoduxApp>,
    terminal_workspace_view: Option<gpui::Entity<TerminalWorkspaceView>>,
    _observe_app: Option<Subscription>,
}

impl WorkspaceBodyView {
    pub(in crate::app) fn new(app_entity: gpui::Entity<CoduxApp>) -> Self {
        Self {
            app_entity,
            terminal_workspace_view: None,
            _observe_app: None,
        }
    }

    pub(in crate::app) fn observe_app(&mut self, cx: &mut Context<Self>) {
        if self._observe_app.is_none() {
            self._observe_app = Some(cx.observe(&self.app_entity, |_, _, cx| cx.notify()));
        }
    }
}

impl Render for WorkspaceBodyView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let app_entity = self.app_entity.clone();
        self.app_entity.update(cx, |app, app_cx| {
            if app.workspace_view == WorkspaceView::Terminal {
                let snapshot = app.terminal_workspace_snapshot();
                let terminal_view = if let Some(view) = &self.terminal_workspace_view {
                    view.update(app_cx, |view, cx| view.set_snapshot(snapshot, cx));
                    view.clone()
                } else {
                    let view =
                        app_cx.new(|_| TerminalWorkspaceView::new(app_entity.clone(), snapshot));
                    self.terminal_workspace_view = Some(view.clone());
                    view
                };
                gpui::AnyView::from(terminal_view).into_any_element()
            } else {
                app.workspace_body(window, app_cx).into_any_element()
            }
        })
    }
}

pub(in crate::app) struct WorkspaceAssistantView {
    app_entity: gpui::Entity<CoduxApp>,
}

impl WorkspaceAssistantView {
    pub(in crate::app) fn new(app_entity: gpui::Entity<CoduxApp>) -> Self {
        Self { app_entity }
    }
}

impl Render for WorkspaceAssistantView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.app_entity.update(cx, |app, cx| {
            app.assistant_column(window, cx).into_any_element()
        })
    }
}

impl CoduxApp {
    pub(in crate::app) fn workspace_column_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<WorkspaceColumnView> {
        if let Some(view) = &self.workspace_column_view {
            return view.clone();
        }
        let toolbar_view = self.workspace_toolbar_view(cx);
        let body_view = self.workspace_body_view(cx);
        let assistant_view = self.workspace_assistant_view(cx);
        let view = cx.new(|_| WorkspaceColumnView::new(toolbar_view, body_view, assistant_view));
        self.workspace_column_view = Some(view.clone());
        view
    }

    pub(in crate::app) fn workspace_toolbar_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<WorkspaceToolbarView> {
        if let Some(view) = &self.workspace_toolbar_view {
            return view.clone();
        }
        let app_entity = cx.entity();
        let view = cx.new(|_| WorkspaceToolbarView::new(app_entity));
        view.update(cx, |view, cx| view.observe_app(cx));
        self.workspace_toolbar_view = Some(view.clone());
        view
    }

    pub(in crate::app) fn workspace_body_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<WorkspaceBodyView> {
        if let Some(view) = &self.workspace_body_view {
            return view.clone();
        }
        let app_entity = cx.entity();
        let view = cx.new(|_| WorkspaceBodyView::new(app_entity));
        view.update(cx, |view, cx| view.observe_app(cx));
        self.workspace_body_view = Some(view.clone());
        view
    }

    pub(in crate::app) fn workspace_assistant_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::Entity<WorkspaceAssistantView> {
        if let Some(view) = &self.workspace_assistant_view {
            return view.clone();
        }
        let app_entity = cx.entity();
        let view = cx.new(|_| WorkspaceAssistantView::new(app_entity));
        self.workspace_assistant_view = Some(view.clone());
        view
    }
}

#[derive(Clone, PartialEq)]
pub(in crate::app) struct TerminalWorkspaceSnapshot {
    loading: bool,
    main_panes: Vec<TerminalPaneViewSnapshot>,
    bottom_tabs: Vec<TerminalBottomTabViewSnapshot>,
    active_bottom: Option<TerminalPaneViewSnapshot>,
}

#[derive(Clone)]
struct TerminalPaneViewSnapshot {
    view: Option<gpui::Entity<TerminalView>>,
}

impl PartialEq for TerminalPaneViewSnapshot {
    fn eq(&self, other: &Self) -> bool {
        match (&self.view, &other.view) {
            (Some(left), Some(right)) => left.entity_id() == right.entity_id(),
            (None, None) => true,
            _ => false,
        }
    }
}

#[derive(Clone, PartialEq)]
struct TerminalBottomTabViewSnapshot {
    id: usize,
    label: String,
    active: bool,
}

pub(in crate::app) struct TerminalWorkspaceView {
    app_entity: gpui::Entity<CoduxApp>,
    snapshot: TerminalWorkspaceSnapshot,
}

impl TerminalWorkspaceView {
    fn new(app_entity: gpui::Entity<CoduxApp>, snapshot: TerminalWorkspaceSnapshot) -> Self {
        Self {
            app_entity,
            snapshot,
        }
    }

    fn set_snapshot(&mut self, snapshot: TerminalWorkspaceSnapshot, cx: &mut Context<Self>) {
        if self.snapshot == snapshot {
            return;
        }
        self.snapshot = snapshot;
        cx.notify();
    }
}

impl Render for TerminalWorkspaceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_bottom_tabs = !self.snapshot.bottom_tabs.is_empty();
        let main = terminal_main_split_area(
            self.app_entity.clone(),
            self.snapshot.main_panes.clone(),
            cx,
        );
        let bottom = terminal_bottom_tabs_area(
            self.app_entity.clone(),
            self.snapshot.bottom_tabs.clone(),
            self.snapshot.active_bottom.clone(),
            cx,
        );

        let base = div()
            .flex()
            .flex_col()
            .flex_1()
            .flex_basis(px(0.0))
            .min_w_0()
            .min_h_0()
            .w_full()
            .h_full()
            .bg(color(theme::BG_TERMINAL));

        if !has_bottom_tabs {
            return base
                .child(
                    div()
                        .flex_1()
                        .flex_basis(px(0.0))
                        .min_w_0()
                        .min_h_0()
                        .w_full()
                        .child(main),
                )
                .child(div().h(px(40.0)).child(bottom));
        }

        base.child(
            v_resizable("workspace-terminal-split")
                .child(
                    resizable_panel()
                        .size(px(420.0))
                        .size_range(px(220.0)..px(900.0))
                        .child(main),
                )
                .child(
                    resizable_panel()
                        .size(px(220.0))
                        .size_range(px(44.0)..px(520.0))
                        .child(bottom),
                ),
        )
    }
}

impl CoduxApp {
    pub(in crate::app) fn terminal_workspace_snapshot(&self) -> TerminalWorkspaceSnapshot {
        let main_panes = self
            .main_terminal()
            .map(|tab| {
                tab.panes
                    .iter()
                    .map(|slot| TerminalPaneViewSnapshot {
                        view: slot.pane.as_ref().map(|pane| pane.view.clone()),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let bottom_tabs = self
            .bottom_terminals()
            .map(|terminal| TerminalBottomTabViewSnapshot {
                id: terminal.id,
                label: terminal.label.clone(),
                active: terminal.id == self.active_terminal_id,
            })
            .collect::<Vec<_>>();

        let active_bottom = self
            .active_bottom_terminal()
            .and_then(|tab| tab.panes.first())
            .map(|slot| TerminalPaneViewSnapshot {
                view: slot.pane.as_ref().map(|pane| pane.view.clone()),
            });

        TerminalWorkspaceSnapshot {
            loading: self.terminal_layout_loading,
            main_panes,
            bottom_tabs,
            active_bottom,
        }
    }
}

fn terminal_main_split_area(
    app_entity: gpui::Entity<CoduxApp>,
    panes: Vec<TerminalPaneViewSnapshot>,
    cx: &mut Context<TerminalWorkspaceView>,
) -> AnyElement {
    if panes.is_empty() {
        return div()
            .flex_1()
            .size_full()
            .bg(color(theme::BG_TERMINAL))
            .into_any_element();
    }

    let pane_count = panes.len();
    div()
        .flex()
        .flex_1()
        .flex_basis(px(0.0))
        .size_full()
        .min_w_0()
        .min_h_0()
        .overflow_hidden()
        .children(panes.into_iter().enumerate().map(move |(index, slot)| {
            terminal_pane(app_entity.clone(), index, pane_count, slot, cx)
        }))
        .into_any_element()
}

fn terminal_pane(
    app_entity: gpui::Entity<CoduxApp>,
    index: usize,
    pane_count: usize,
    slot: TerminalPaneViewSnapshot,
    cx: &mut Context<TerminalWorkspaceView>,
) -> AnyElement {
    let close_id = SharedString::from(format!("terminal-pane-close-{index}"));
    let float_id = SharedString::from(format!("terminal-pane-float-{index}"));
    let add_id = SharedString::from(format!("terminal-pane-add-{index}"));

    div()
        .relative()
        .group("terminal-pane")
        .flex()
        .flex_col()
        .flex_1()
        .flex_basis(px(0.0))
        .size_full()
        .min_w_0()
        .min_h_0()
        .overflow_hidden()
        .border_l_1()
        .border_color(color(if index == 0 {
            theme::BG_TERMINAL
        } else {
            theme::BORDER_SOFT
        }))
        .child(
            div()
                .flex_1()
                .flex_basis(px(0.0))
                .min_w_0()
                .min_h_0()
                .child(match slot.view {
                    Some(view) => gpui::AnyView::from(view).into_any_element(),
                    None => div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(color(theme::TEXT_DIM))
                        .child("Terminal mounting...")
                        .into_any_element(),
                }),
        )
        .child(
            div()
                .absolute()
                .top_2()
                .right_2()
                .flex()
                .items_center()
                .gap_1()
                .child(terminal_pane_control_button(
                    app_entity.clone(),
                    float_id,
                    HeroIconName::ArrowTopRightOnSquare,
                    "浮窗",
                    pane_count > 1,
                    cx,
                    move |app, window, cx| app.float_terminal_pane(index, window, cx),
                ))
                .child(terminal_pane_control_button(
                    app_entity.clone(),
                    add_id,
                    HeroIconName::Plus,
                    "新建分屏",
                    true,
                    cx,
                    |app, window, cx| app.split_terminal(window, cx),
                ))
                .child(terminal_pane_control_button(
                    app_entity,
                    close_id,
                    HeroIconName::XMark,
                    "关闭分屏",
                    pane_count > 1,
                    cx,
                    move |app, window, cx| app.close_terminal_pane(index, window, cx),
                )),
        )
        .into_any_element()
}

fn terminal_bottom_tabs_area(
    app_entity: gpui::Entity<CoduxApp>,
    tabs: Vec<TerminalBottomTabViewSnapshot>,
    active: Option<TerminalPaneViewSnapshot>,
    cx: &mut Context<TerminalWorkspaceView>,
) -> AnyElement {
    let has_bottom_tabs = active.is_some();

    div()
        .flex()
        .flex_col()
        .size_full()
        .min_w_0()
        .min_h_0()
        .child(
            div()
                .h(px(40.0))
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .px_2()
                .border_t_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .when(!has_bottom_tabs, |this| {
                            this.child(
                                div()
                                    .px_2()
                                    .text_xs()
                                    .line_height(px(16.0))
                                    .text_color(cx.theme().secondary_foreground)
                                    .child("终端"),
                            )
                        })
                        .children(tabs.into_iter().map(|tab| {
                            terminal_bottom_tab_button(app_entity.clone(), tab, cx)
                                .into_any_element()
                        })),
                )
                .child(terminal_bottom_add_button(app_entity.clone(), cx)),
        )
        .when_some(active, |this, tab| {
            this.child(div().flex_1().min_h_0().child(terminal_bottom_content(tab)))
        })
        .into_any_element()
}

fn terminal_bottom_content(tab: TerminalPaneViewSnapshot) -> AnyElement {
    div()
        .size_full()
        .min_h_0()
        .child(match tab.view {
            Some(view) => gpui::AnyView::from(view).into_any_element(),
            None => div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(color(theme::TEXT_DIM))
                .child("Terminal mounting...")
                .into_any_element(),
        })
        .into_any_element()
}

fn terminal_pane_control_button(
    app_entity: gpui::Entity<CoduxApp>,
    id: SharedString,
    icon: HeroIconName,
    tooltip: &'static str,
    enabled: bool,
    cx: &mut Context<TerminalWorkspaceView>,
    on_click: impl Fn(&mut CoduxApp, &mut Window, &mut Context<CoduxApp>) + 'static,
) -> AnyElement {
    let text_color = if enabled {
        cx.theme().secondary_foreground
    } else {
        color(theme::TEXT_DIM)
    };
    let button = div()
        .id(id)
        .size(px(28.0))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_color(text_color)
        .tooltip(move |window, cx| codux_tooltip(tooltip, window, cx))
        .child(Icon::new(icon).size_3p5().text_color(text_color));

    if enabled {
        button
            .cursor_pointer()
            .hover(|style| style.bg(cx.theme().secondary_hover))
            .on_click(cx.listener(move |_view, _event, window, cx| {
                cx.stop_propagation();
                window.prevent_default();
                app_entity.update(cx, |app, app_cx| on_click(app, window, app_cx));
            }))
            .into_any_element()
    } else {
        button.opacity(0.45).into_any_element()
    }
}

fn terminal_bottom_tab_button(
    app_entity: gpui::Entity<CoduxApp>,
    tab: TerminalBottomTabViewSnapshot,
    cx: &mut Context<TerminalWorkspaceView>,
) -> impl IntoElement {
    let terminal_id = tab.id;
    div()
        .id(SharedString::from(format!(
            "terminal-bottom-tab-{terminal_id}"
        )))
        .h(px(32.0))
        .px_3()
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .rounded_md()
        .cursor_pointer()
        .text_color(if tab.active {
            cx.theme().foreground
        } else {
            cx.theme().secondary_foreground
        })
        .bg(if tab.active {
            cx.theme().secondary_hover
        } else {
            cx.theme().transparent
        })
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .on_click(cx.listener({
            let app_entity = app_entity.clone();
            move |_view, _event, window, cx| {
                app_entity.update(cx, |app, app_cx| {
                    app.select_terminal_tab(terminal_id, window, app_cx)
                });
            }
        }))
        .child(div().text_xs().line_height(px(14.0)).child(tab.label))
        .child(
            div()
                .id(SharedString::from(format!(
                    "terminal-bottom-tab-close-{terminal_id}"
                )))
                .size(px(20.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .text_color(cx.theme().secondary_foreground)
                .hover(|style| style.bg(cx.theme().secondary_hover))
                .on_click(cx.listener(move |_view, _event, window, cx| {
                    cx.stop_propagation();
                    window.prevent_default();
                    app_entity.update(cx, |app, app_cx| {
                        app.close_terminal_tab(terminal_id, window, app_cx)
                    });
                }))
                .child(Icon::new(HeroIconName::XMark).size_3()),
        )
}

fn terminal_bottom_add_button(
    app_entity: gpui::Entity<CoduxApp>,
    cx: &mut Context<TerminalWorkspaceView>,
) -> impl IntoElement {
    div()
        .id("terminal-bottom-tab-add")
        .size(px(26.0))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded_sm()
        .cursor_pointer()
        .text_color(cx.theme().secondary_foreground)
        .hover(|style| style.bg(cx.theme().secondary_hover))
        .on_click(cx.listener(move |_view, _event, window, cx| {
            app_entity.update(cx, |app, app_cx| app.add_terminal_tab(window, app_cx));
        }))
        .child(Icon::new(HeroIconName::Plus).size_3p5())
}
