use super::*;
use gpui::{Anchor, Rems};
use gpui_component::{
    Disableable, Sizable,
    button::Button,
    menu::{DropdownMenu, PopupMenuItem},
};

const WECODE_SELECT_TEXT_SIZE: Rems = Rems(0.875);
const WECODE_SELECT_LINE_HEIGHT: Rems = Rems(1.125);

#[derive(Clone)]
pub(in crate::app) struct WeCodeSelectOption {
    pub(in crate::app) value: String,
    pub(in crate::app) label: SharedString,
}

impl WeCodeSelectOption {
    pub(in crate::app) fn new(value: impl Into<String>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

pub(in crate::app) fn wecode_select(
    id: impl Into<String>,
    value: &str,
    options: Vec<WeCodeSelectOption>,
    placeholder: impl Into<SharedString>,
    width: impl Into<Length> + Clone,
    menu_width: Pixels,
    disabled: bool,
    _window: &mut Window,
    cx: &mut Context<WeCodeApp>,
    action: impl Fn(&mut WeCodeApp, String, &mut Window, &mut Context<WeCodeApp>) + 'static,
) -> AnyElement {
    let id = id.into();
    let selected_index = options.iter().position(|item| item.value == value);
    let selected_label = selected_index
        .and_then(|index| options.get(index))
        .map(|item| item.label.clone())
        .unwrap_or_else(|| placeholder.into());
    let action: Rc<dyn Fn(&mut WeCodeApp, String, &mut Window, &mut Context<WeCodeApp>)> =
        Rc::new(action);
    let selected_value = value.to_string();
    let app_entity = cx.entity();

    Button::new(SharedString::from(format!("wecode-select-trigger-{id}")))
        .outline()
        .with_size(gpui_component::Size::Medium)
        .disabled(disabled)
        .w(width)
        .min_w(px(180.0))
        .child(
            div()
                .flex()
                .w_full()
                .min_w_0()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .text_size(WECODE_SELECT_TEXT_SIZE)
                        .line_height(WECODE_SELECT_LINE_HEIGHT)
                        .text_color(if selected_index.is_some() {
                            color(theme::TEXT)
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child(selected_label),
                )
                .child(
                    Icon::new(HeroIconName::ChevronDown)
                        .size_3()
                        .flex_shrink_0()
                        .text_color(if disabled {
                            cx.theme().foreground.opacity(0.3)
                        } else {
                            cx.theme().foreground.opacity(0.5)
                        }),
                ),
        )
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _window, _cx| {
            options.iter().fold(
                menu.min_w(menu_width).max_w(menu_width).scrollable(true),
                |menu, item| {
                    let value = item.value.clone();
                    let selected = value == selected_value;
                    let action = action.clone();
                    let app_entity = app_entity.clone();
                    menu.item(
                        PopupMenuItem::new(item.label.clone())
                            .checked(selected)
                            .on_click(move |_, window, cx| {
                                cx.update_entity(&app_entity, |app, cx| {
                                    action(app, value.clone(), window, cx);
                                    cx.notify();
                                });
                            }),
                    )
                },
            )
        })
        .into_any_element()
}
