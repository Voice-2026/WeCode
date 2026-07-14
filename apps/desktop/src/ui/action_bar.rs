use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, div};

use super::tokens::{FORM_ACTION_GAP, FORM_ACTION_PADDING_TOP};

pub(crate) fn form_action_bar(
    actions: impl IntoIterator<Item = AnyElement>,
    _cx: &App,
) -> AnyElement {
    div()
        .w_full()
        .pt(FORM_ACTION_PADDING_TOP)
        .flex()
        .justify_end()
        .gap(FORM_ACTION_GAP)
        .children(actions)
        .into_any_element()
}
