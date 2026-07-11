#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalMouseUiEvent {
    Press,
    Release,
    Move,
    Wheel,
}

fn terminal_mouse_ui_event_bytes(
    button: Option<MouseButton>,
    point: TerminalCellPoint,
    kind: TerminalMouseUiEvent,
    modifiers: Modifiers,
    mode: TerminalInputMode,
) -> Option<Vec<u8>> {
    wecode_terminal_core::terminal_mouse_input_bytes(wecode_terminal_core::TerminalMouseInput {
        action: terminal_mouse_core_action(kind),
        button: terminal_mouse_core_button(button, kind)?,
        row: point.row,
        col: point.col,
        modifiers: wecode_terminal_core::TerminalKeyInputModifiers {
            shift: modifiers.shift,
            alt: modifiers.alt,
            control: modifiers.control,
            platform: modifiers.platform,
        },
        mode,
    })
}

fn terminal_mouse_core_action(kind: TerminalMouseUiEvent) -> wecode_terminal_core::TerminalMouseAction {
    match kind {
        TerminalMouseUiEvent::Press | TerminalMouseUiEvent::Wheel => {
            wecode_terminal_core::TerminalMouseAction::Press
        }
        TerminalMouseUiEvent::Release => wecode_terminal_core::TerminalMouseAction::Release,
        TerminalMouseUiEvent::Move => wecode_terminal_core::TerminalMouseAction::Move,
    }
}

fn terminal_mouse_core_button(
    button: Option<MouseButton>,
    kind: TerminalMouseUiEvent,
) -> Option<Option<wecode_terminal_core::TerminalMouseButton>> {
    if matches!(kind, TerminalMouseUiEvent::Move) && button.is_none() {
        return Some(None);
    }
    let button = button?;
    let button = match button {
        MouseButton::Left => wecode_terminal_core::TerminalMouseButton::Left,
        MouseButton::Middle => wecode_terminal_core::TerminalMouseButton::Middle,
        MouseButton::Right => wecode_terminal_core::TerminalMouseButton::Right,
        MouseButton::Navigate(NavigationDirection::Back) if kind == TerminalMouseUiEvent::Wheel => {
            wecode_terminal_core::TerminalMouseButton::WheelUp
        }
        MouseButton::Navigate(NavigationDirection::Forward) if kind == TerminalMouseUiEvent::Wheel => {
            wecode_terminal_core::TerminalMouseButton::WheelDown
        }
        MouseButton::Navigate(_) => return None,
    };
    Some(Some(button))
}
