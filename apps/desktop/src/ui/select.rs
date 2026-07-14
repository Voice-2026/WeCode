use gpui::{
    AnyElement, App, AppContext, Entity, IntoElement, Length, Pixels, SharedString, Styled, Window,
    px,
};
use gpui_component::{
    IndexPath, Sizable, Size,
    searchable_list::{SearchableListItem, SearchableVec},
    select::{Select, SelectEvent, SelectState},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectOption {
    pub(crate) value: String,
    pub(crate) label: SharedString,
}

impl SelectOption {
    pub(crate) fn new(value: impl Into<String>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

impl SearchableListItem for SelectOption {
    type Value = String;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

pub(crate) type UiSelectState = SelectState<SearchableVec<SelectOption>>;
pub(crate) type UiSelectEvent = SelectEvent<SearchableVec<SelectOption>>;

pub(crate) fn new_select_state(
    options: Vec<SelectOption>,
    selected_value: &str,
    window: &mut Window,
    cx: &mut App,
) -> Entity<UiSelectState> {
    let selected_index = options
        .iter()
        .position(|option| option.value == selected_value)
        .map(IndexPath::new);
    cx.new(|cx| SelectState::new(SearchableVec::new(options), selected_index, window, cx))
}

pub(crate) fn sync_select_state(
    state: &Entity<UiSelectState>,
    options: Vec<SelectOption>,
    selected_value: &str,
    window: &mut Window,
    cx: &mut App,
) {
    state.update(cx, |state, cx| {
        state.set_items(SearchableVec::new(options), window, cx);
        state.set_selected_value(&selected_value.to_string(), window, cx);
    });
}

pub(crate) fn select_control(
    state: &Entity<UiSelectState>,
    placeholder: impl Into<SharedString>,
    width: impl Into<Length> + Clone,
    menu_width: Pixels,
    disabled: bool,
) -> AnyElement {
    Select::new(state)
        .placeholder(placeholder)
        .menu_width(menu_width)
        .disabled(disabled)
        .with_size(Size::Medium)
        .w(width)
        .min_w(px(180.0))
        .into_any_element()
}
