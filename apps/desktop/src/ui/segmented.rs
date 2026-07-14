use gpui::{AnyElement, App, AppContext, Entity, IntoElement, Window};
use gpui_component::{
    ActiveTheme, Disableable, Selectable, Sizable, Size,
    button::{Button, ButtonCustomVariant, ButtonGroup, ButtonVariants},
};

#[derive(Clone)]
pub(crate) struct SegmentedOption {
    pub(crate) value: String,
    pub(crate) label: gpui::SharedString,
    pub(crate) disabled: bool,
}

impl SegmentedOption {
    pub(crate) fn new(value: impl Into<String>, label: impl Into<gpui::SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub(crate) fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

pub(crate) fn segmented_control<V: 'static>(
    id: impl Into<String>,
    selected_value: &str,
    options: Vec<SegmentedOption>,
    view: Entity<V>,
    action: impl Fn(&mut V, String, &mut Window, &mut gpui::Context<V>) + 'static,
    cx: &App,
) -> AnyElement {
    let id = id.into();
    let action = std::rc::Rc::new(action);
    let selected_variant = ButtonCustomVariant::new(cx)
        .color(cx.theme().primary)
        .hover(cx.theme().primary.opacity(0.08))
        .active(cx.theme().input_background());
    let values = options
        .iter()
        .map(|option| option.value.clone())
        .collect::<Vec<_>>();
    let group = options.into_iter().fold(
        ButtonGroup::new(id)
            .compact()
            .outline()
            .with_size(Size::Small),
        |group, option| {
            let selected = option.value == selected_value;
            let button = Button::new(option.value.clone())
                .selected(selected)
                .disabled(option.disabled)
                .label(option.label);
            group.child(if selected {
                button.custom(selected_variant)
            } else {
                button
            })
        },
    );

    group
        .on_click(move |selected, window, cx: &mut App| {
            let Some(value) = selected
                .first()
                .and_then(|index| values.get(*index))
                .cloned()
            else {
                return;
            };
            let action = action.clone();
            cx.update_entity(&view, |view, cx| {
                action(view, value, window, cx);
                cx.notify();
            });
        })
        .into_any_element()
}
