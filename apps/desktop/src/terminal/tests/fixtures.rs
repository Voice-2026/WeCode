use super::super::*;

pub(super) fn keystroke(key: &str) -> Keystroke {
    Keystroke {
        key: key.to_string(),
        key_char: None,
        modifiers: Modifiers::default(),
    }
}

pub(super) fn modified_key(
    key: &str,
    shift: bool,
    alt: bool,
    control: bool,
    platform: bool,
) -> Keystroke {
    modified_key_with_function(key, shift, alt, control, platform, false)
}

pub(super) fn modified_key_with_function(
    key: &str,
    shift: bool,
    alt: bool,
    control: bool,
    platform: bool,
    function: bool,
) -> Keystroke {
    Keystroke {
        key: key.to_string(),
        key_char: None,
        modifiers: Modifiers {
            shift,
            alt,
            control,
            platform,
            function,
        },
    }
}

pub(super) fn key_char(key: &str, key_char: &str) -> Keystroke {
    Keystroke {
        key: key.to_string(),
        key_char: Some(key_char.to_string()),
        modifiers: Modifiers::default(),
    }
}

pub(super) fn modified_key_with_char(
    key: &str,
    key_char: &str,
    shift: bool,
    alt: bool,
    control: bool,
    platform: bool,
) -> Keystroke {
    let mut keystroke = modified_key(key, shift, alt, control, platform);
    keystroke.key_char = Some(key_char.to_string());
    keystroke
}

pub(super) fn normal_mode() -> TerminalInputMode {
    TerminalInputMode::default()
}

pub(super) fn app_cursor_mode() -> TerminalInputMode {
    TerminalInputMode {
        application_cursor: true,
        ..TerminalInputMode::default()
    }
}

pub(super) fn alternate_scroll_mode() -> TerminalInputMode {
    TerminalInputMode {
        alternate_screen: true,
        alternate_scroll: true,
        ..TerminalInputMode::default()
    }
}

pub(super) fn bytes(keystroke: Keystroke, mode: TerminalInputMode) -> Vec<u8> {
    keystroke_to_bytes(&keystroke, mode).expect("keystroke should map to terminal bytes")
}

pub(super) fn row_text(content: &TerminalContent, line: i32) -> String {
    content
        .cells
        .iter()
        .filter(|cell| cell.point.line == line)
        .map(|cell| cell.cell.text.as_str())
        .collect()
}

pub(super) fn test_cell(
    fg: TerminalScreenColor,
    bg: TerminalScreenColor,
    bold: bool,
    inverse: bool,
) -> TerminalScreenCellSnapshot {
    TerminalScreenCellSnapshot {
        row: 0,
        col: 0,
        text: "x".to_string(),
        width: 1,
        fg,
        bg,
        bold,
        dim: false,
        italic: false,
        underline: false,
        inverse,
        hidden: false,
        strikeout: false,
    }
}
