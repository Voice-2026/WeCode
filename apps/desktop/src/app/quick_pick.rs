//! A centered, searchable Quick Pick overlay (VS Code style), built on
//! gpui-component's `List` + `Dialog`. Used by the git menu to pick a branch or
//! remote without deep cascading submenus.

use std::rc::Rc;

use gpui::{
    App, AppContext as _, Context, ParentElement as _, SharedString, Styled as _, Task, Window,
    div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    Icon, IndexPath, WindowExt as _,
    list::{List, ListDelegate, ListItem, ListState},
};

/// One selectable row in a [`show_quick_pick`] overlay.
#[derive(Clone)]
pub struct QuickPickItem {
    pub id: SharedString,
    pub icon: Option<Icon>,
    pub label: SharedString,
}

impl QuickPickItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            icon: None,
            label: label.into(),
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

type OnConfirm = Rc<dyn Fn(SharedString, &mut Window, &mut App)>;

struct QuickPickDelegate {
    all: Vec<QuickPickItem>,
    filtered: Vec<QuickPickItem>,
    selected: Option<IndexPath>,
    on_confirm: OnConfirm,
}

impl ListDelegate for QuickPickDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.filtered.len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        let needle = query.trim().to_lowercase();
        self.filtered = if needle.is_empty() {
            self.all.clone()
        } else {
            self.all
                .iter()
                .filter(|item| item.label.to_lowercase().contains(&needle))
                .cloned()
                .collect()
        };
        Task::ready(())
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let item = self.filtered.get(ix.row)?;
        Some(
            ListItem::new(ix).child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .when_some(item.icon.clone(), |this, icon| this.child(icon))
                    .child(item.label.clone()),
            ),
        )
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected = ix;
        cx.notify();
    }

    fn confirm(&mut self, _secondary: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let Some(ix) = self.selected else { return };
        let Some(item) = self.filtered.get(ix.row) else {
            return;
        };
        let id = item.id.clone();
        (self.on_confirm.clone())(id, window, cx);
        window.close_dialog(cx);
    }

    // Escape is handled by the hosting Dialog (List::Cancel re-propagates to it).
}

/// Show a centered, searchable Quick Pick overlay. `on_confirm` receives the
/// chosen item's `id`; the overlay dismisses on Enter/click (after the
/// callback), Escape, or click-outside. Requires the window root to render
/// `Root::render_dialog_layer` (see `app_render`).
pub fn show_quick_pick(
    placeholder: impl Into<SharedString>,
    items: Vec<QuickPickItem>,
    on_confirm: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    window: &mut Window,
    cx: &mut App,
) {
    let on_confirm: OnConfirm = Rc::new(on_confirm);
    let state = cx.new(|cx| {
        ListState::new(
            QuickPickDelegate {
                filtered: items.clone(),
                all: items,
                selected: Some(IndexPath::default()),
                on_confirm,
            },
            window,
            cx,
        )
        .searchable(true)
    });

    let list = state.clone();
    let placeholder = placeholder.into();
    window.open_dialog(cx, move |dialog, _window, _cx| {
        dialog.close_button(false).w(px(560.)).child(
            div()
                .h(px(360.))
                .w_full()
                .child(List::new(&list).search_placeholder(placeholder.clone())),
        )
    });

    // `open_dialog` focuses the dialog handle; move focus into the search input
    // so typing + Up/Down + Enter work immediately.
    state.update(cx, |list, cx| list.focus(window, cx));
}
