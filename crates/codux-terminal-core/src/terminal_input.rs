use libghostty_vt::{key as ghostty_key, mouse as ghostty_mouse};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInputMode {
    pub application_cursor: bool,
    pub alternate_screen: bool,
    pub alternate_scroll: bool,
    pub bracketed_paste: bool,
    pub focus_in_out: bool,
    pub mouse_tracking: bool,
    pub mouse_motion: bool,
    pub mouse_drag: bool,
    pub sgr_mouse: bool,
    pub utf8_mouse: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerminalKeyInputModifiers {
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
    pub platform: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalKeyInput<'a> {
    pub key: &'a str,
    pub key_char: Option<&'a str>,
    pub modifiers: TerminalKeyInputModifiers,
    pub mode: TerminalInputMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminalMouseInput {
    pub action: TerminalMouseAction,
    pub button: Option<TerminalMouseButton>,
    pub row: usize,
    pub col: usize,
    pub modifiers: TerminalKeyInputModifiers,
    pub mode: TerminalInputMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalMouseAction {
    Press,
    Release,
    Move,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalMouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

#[derive(Debug, PartialEq, Eq)]
enum TerminalKeyModifiers {
    None,
    Alt,
    Ctrl,
    Shift,
    Platform,
    CtrlShift,
    Other,
}

impl TerminalKeyModifiers {
    fn new(modifiers: TerminalKeyInputModifiers) -> Self {
        match (
            modifiers.alt,
            modifiers.control,
            modifiers.shift,
            modifiers.platform,
        ) {
            (false, false, false, false) => Self::None,
            (true, false, false, false) => Self::Alt,
            (false, true, false, false) => Self::Ctrl,
            (false, false, true, false) => Self::Shift,
            (false, false, false, true) => Self::Platform,
            (false, true, true, false) => Self::CtrlShift,
            _ => Self::Other,
        }
    }

    fn any(&self) -> bool {
        !matches!(self, Self::None)
    }
}

pub fn terminal_text_input_bytes(text: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    for c in text
        .chars()
        .filter(|c| !('\u{F700}'..='\u{F8FF}').contains(c))
    {
        match c {
            '\u{8}' => bytes.push(0x7f),
            '\n' | '\r' => bytes.push(b'\r'),
            _ => {
                let mut buffer = [0; 4];
                bytes.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
            }
        }
    }
    bytes
}

pub fn terminal_text_input(text: &str) -> String {
    String::from_utf8(terminal_text_input_bytes(text)).unwrap_or_default()
}

pub fn terminal_paste_input_bytes(text: &str, bracketed_paste: bool) -> Vec<u8> {
    if !bracketed_paste {
        return text.as_bytes().to_vec();
    }

    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut bytes = Vec::with_capacity(normalized.len() + 12);
    bytes.extend_from_slice(b"\x1b[200~");
    bytes.extend_from_slice(normalized.as_bytes());
    bytes.extend_from_slice(b"\x1b[201~");
    bytes
}

pub fn terminal_insert_input_bytes(text: &str) -> Vec<u8> {
    if text.chars().count() <= 1 {
        return text.as_bytes().to_vec();
    }
    terminal_paste_input_bytes(text, true)
}

pub fn terminal_insert_input(text: &str) -> String {
    String::from_utf8(terminal_insert_input_bytes(text)).unwrap_or_default()
}

pub fn terminal_key_input(input: TerminalKeyInput<'_>) -> Option<String> {
    String::from_utf8(terminal_key_input_bytes(input)?).ok()
}

pub fn terminal_selector_input(selector: &str, mode: TerminalInputMode) -> Option<String> {
    let key = terminal_selector_key(selector)?;
    terminal_key_input(TerminalKeyInput {
        key,
        key_char: None,
        modifiers: TerminalKeyInputModifiers::default(),
        mode,
    })
}

pub fn terminal_selector_input_bytes(selector: &str, mode: TerminalInputMode) -> Option<Vec<u8>> {
    let key = terminal_selector_key(selector)?;
    terminal_key_input_bytes(TerminalKeyInput {
        key,
        key_char: None,
        modifiers: TerminalKeyInputModifiers::default(),
        mode,
    })
}

pub fn terminal_key_input_bytes(input: TerminalKeyInput<'_>) -> Option<Vec<u8>> {
    if terminal_should_keep_platform_shortcut(input) {
        return None;
    }

    if input.modifiers.control
        && !input.modifiers.alt
        && !input.modifiers.platform
        && let Some(sequence) = control_key_char_sequence(input)
    {
        return Some(sequence);
    }

    let modifiers = TerminalKeyModifiers::new(input.modifiers);
    let key = terminal_normalize_key(input.key);
    let manual = match (key.as_str(), &modifiers) {
        ("tab", TerminalKeyModifiers::None) => Some("\x09"),
        ("escape", TerminalKeyModifiers::None) => Some("\x1b"),
        ("enter", TerminalKeyModifiers::None) => Some("\x0d"),
        ("enter", TerminalKeyModifiers::Shift) => Some("\x0a"),
        ("enter", TerminalKeyModifiers::Alt) => Some("\x1b\x0d"),
        ("backspace", TerminalKeyModifiers::None) | ("back", TerminalKeyModifiers::None) => {
            Some("\x7f")
        }
        ("tab", TerminalKeyModifiers::Shift) => Some("\x1b[Z"),
        ("backspace", TerminalKeyModifiers::Ctrl) => Some("\x08"),
        ("backspace", TerminalKeyModifiers::Alt) => Some("\x1b\x7f"),
        ("back", TerminalKeyModifiers::Alt) => Some("\x1b\x7f"),
        ("delete", TerminalKeyModifiers::Alt) => Some("\x1bd"),
        ("backspace", TerminalKeyModifiers::Platform) => Some("\x15"),
        ("back", TerminalKeyModifiers::Platform) => Some("\x15"),
        ("delete", TerminalKeyModifiers::Platform) => Some("\x0b"),
        ("backspace", TerminalKeyModifiers::Shift) => Some("\x7f"),
        ("space", TerminalKeyModifiers::Ctrl) => Some("\x00"),
        ("home", TerminalKeyModifiers::None) if input.mode.application_cursor => Some("\x1bOH"),
        ("home", TerminalKeyModifiers::None) => Some("\x1b[H"),
        ("end", TerminalKeyModifiers::None) if input.mode.application_cursor => Some("\x1bOF"),
        ("end", TerminalKeyModifiers::None) => Some("\x1b[F"),
        ("up", TerminalKeyModifiers::None) if input.mode.application_cursor => Some("\x1bOA"),
        ("up", TerminalKeyModifiers::None) => Some("\x1b[A"),
        ("down", TerminalKeyModifiers::None) if input.mode.application_cursor => Some("\x1bOB"),
        ("down", TerminalKeyModifiers::None) => Some("\x1b[B"),
        ("right", TerminalKeyModifiers::None) if input.mode.application_cursor => Some("\x1bOC"),
        ("right", TerminalKeyModifiers::None) => Some("\x1b[C"),
        ("left", TerminalKeyModifiers::None) if input.mode.application_cursor => Some("\x1bOD"),
        ("left", TerminalKeyModifiers::None) => Some("\x1b[D"),
        ("right", TerminalKeyModifiers::Alt) => Some("\x1bf"),
        ("left", TerminalKeyModifiers::Alt) => Some("\x1bb"),
        ("right", TerminalKeyModifiers::Platform) => Some("\x05"),
        ("left", TerminalKeyModifiers::Platform) => Some("\x01"),
        ("end", TerminalKeyModifiers::Platform) => Some("\x05"),
        ("home", TerminalKeyModifiers::Platform) => Some("\x01"),
        ("insert", TerminalKeyModifiers::None) => Some("\x1b[2~"),
        ("delete", TerminalKeyModifiers::None) => Some("\x1b[3~"),
        ("pageup", TerminalKeyModifiers::None) => Some("\x1b[5~"),
        ("pagedown", TerminalKeyModifiers::None) => Some("\x1b[6~"),
        ("f1", TerminalKeyModifiers::None) => Some("\x1bOP"),
        ("f2", TerminalKeyModifiers::None) => Some("\x1bOQ"),
        ("f3", TerminalKeyModifiers::None) => Some("\x1bOR"),
        ("f4", TerminalKeyModifiers::None) => Some("\x1bOS"),
        ("f5", TerminalKeyModifiers::None) => Some("\x1b[15~"),
        ("f6", TerminalKeyModifiers::None) => Some("\x1b[17~"),
        ("f7", TerminalKeyModifiers::None) => Some("\x1b[18~"),
        ("f8", TerminalKeyModifiers::None) => Some("\x1b[19~"),
        ("f9", TerminalKeyModifiers::None) => Some("\x1b[20~"),
        ("f10", TerminalKeyModifiers::None) => Some("\x1b[21~"),
        ("f11", TerminalKeyModifiers::None) => Some("\x1b[23~"),
        ("f12", TerminalKeyModifiers::None) => Some("\x1b[24~"),
        ("f13", TerminalKeyModifiers::None) => Some("\x1b[25~"),
        ("f14", TerminalKeyModifiers::None) => Some("\x1b[26~"),
        ("f15", TerminalKeyModifiers::None) => Some("\x1b[28~"),
        ("f16", TerminalKeyModifiers::None) => Some("\x1b[29~"),
        ("f17", TerminalKeyModifiers::None) => Some("\x1b[31~"),
        ("f18", TerminalKeyModifiers::None) => Some("\x1b[32~"),
        ("f19", TerminalKeyModifiers::None) => Some("\x1b[33~"),
        ("f20", TerminalKeyModifiers::None) => Some("\x1b[34~"),
        (key, TerminalKeyModifiers::Ctrl | TerminalKeyModifiers::CtrlShift) => ctrl_sequence(key),
        _ => None,
    };
    if let Some(sequence) = manual {
        return Some(sequence.as_bytes().to_vec());
    }

    if modifiers.any() {
        let modifier_code = terminal_modifier_code(input.modifiers);
        let modified = match key.as_str() {
            "up" => Some(format!("\x1b[1;{modifier_code}A")),
            "down" => Some(format!("\x1b[1;{modifier_code}B")),
            "right" => Some(format!("\x1b[1;{modifier_code}C")),
            "left" => Some(format!("\x1b[1;{modifier_code}D")),
            "f1" => Some(format!("\x1b[1;{modifier_code}P")),
            "f2" => Some(format!("\x1b[1;{modifier_code}Q")),
            "f3" => Some(format!("\x1b[1;{modifier_code}R")),
            "f4" => Some(format!("\x1b[1;{modifier_code}S")),
            "f5" => Some(format!("\x1b[15;{modifier_code}~")),
            "f6" => Some(format!("\x1b[17;{modifier_code}~")),
            "f7" => Some(format!("\x1b[18;{modifier_code}~")),
            "f8" => Some(format!("\x1b[19;{modifier_code}~")),
            "f9" => Some(format!("\x1b[20;{modifier_code}~")),
            "f10" => Some(format!("\x1b[21;{modifier_code}~")),
            "f11" => Some(format!("\x1b[23;{modifier_code}~")),
            "f12" => Some(format!("\x1b[24;{modifier_code}~")),
            "f13" => Some(format!("\x1b[25;{modifier_code}~")),
            "f14" => Some(format!("\x1b[26;{modifier_code}~")),
            "f15" => Some(format!("\x1b[28;{modifier_code}~")),
            "f16" => Some(format!("\x1b[29;{modifier_code}~")),
            "f17" => Some(format!("\x1b[31;{modifier_code}~")),
            "f18" => Some(format!("\x1b[32;{modifier_code}~")),
            "f19" => Some(format!("\x1b[33;{modifier_code}~")),
            "f20" => Some(format!("\x1b[34;{modifier_code}~")),
            "insert" => Some(format!("\x1b[2;{modifier_code}~")),
            "delete" => Some(format!("\x1b[3;{modifier_code}~")),
            "pageup" => Some(format!("\x1b[5;{modifier_code}~")),
            "pagedown" => Some(format!("\x1b[6;{modifier_code}~")),
            "end" => Some(format!("\x1b[1;{modifier_code}F")),
            "home" => Some(format!("\x1b[1;{modifier_code}H")),
            _ => None,
        };
        if let Some(sequence) = modified {
            return Some(sequence.into_bytes());
        }
    }

    if input.modifiers.alt
        && !input.modifiers.control
        && !input.modifiers.platform
        && key.is_ascii()
        && key.chars().count() == 1
    {
        let mut key = key;
        if input.modifiers.shift {
            key = key.to_ascii_uppercase();
        }
        return Some(format!("\x1b{key}").into_bytes());
    }

    None
}

pub fn terminal_mouse_input_bytes(input: TerminalMouseInput) -> Option<Vec<u8>> {
    if !input.mode.mouse_tracking {
        return None;
    }
    if matches!(input.action, TerminalMouseAction::Move)
        && !(input.mode.mouse_motion || input.mode.mouse_drag)
    {
        return None;
    }
    if input.mode.mouse_drag
        && matches!(input.action, TerminalMouseAction::Move)
        && input.button.is_none()
    {
        return None;
    }

    let mut encoder = ghostty_mouse::Encoder::new().ok()?;
    encoder
        .set_tracking_mode(terminal_mouse_tracking_mode(input.mode))
        .set_format(terminal_mouse_format(input.mode))
        .set_size(ghostty_mouse::EncoderSize {
            screen_width: (input.col.saturating_add(1)).try_into().unwrap_or(u32::MAX),
            screen_height: (input.row.saturating_add(1)).try_into().unwrap_or(u32::MAX),
            cell_width: 1,
            cell_height: 1,
            padding_top: 0,
            padding_bottom: 0,
            padding_right: 0,
            padding_left: 0,
        })
        .set_any_button_pressed(input.button.is_some());

    let mut event = ghostty_mouse::Event::new().ok()?;
    event
        .set_action(terminal_mouse_action(input.action))
        .set_button(terminal_mouse_button(input.button))
        .set_mods(ghostty_modifiers(input.modifiers))
        .set_position(ghostty_mouse::Position {
            x: input.col as f32,
            y: input.row as f32,
        });

    let mut bytes = Vec::new();
    encoder.encode_to_vec(&event, &mut bytes).ok()?;
    (!bytes.is_empty()).then_some(bytes)
}

pub fn terminal_is_copy_shortcut(input: TerminalKeyInput<'_>) -> bool {
    terminal_normalize_key(input.key) == "c"
        && input.modifiers.platform
        && !input.modifiers.control
        && !input.modifiers.alt
}

pub fn terminal_is_paste_shortcut(input: TerminalKeyInput<'_>) -> bool {
    terminal_normalize_key(input.key) == "v"
        && input.modifiers.platform
        && !input.modifiers.control
        && !input.modifiers.alt
}

fn terminal_selector_key(selector: &str) -> Option<&'static str> {
    match selector {
        "deleteBackward:" => Some("backspace"),
        "deleteForward:" => Some("delete"),
        "insertNewline:" => Some("enter"),
        "moveLeft:" => Some("left"),
        "moveRight:" => Some("right"),
        "moveUp:" => Some("up"),
        "moveDown:" => Some("down"),
        "moveToBeginningOfLine:" => Some("home"),
        "moveToEndOfLine:" => Some("end"),
        _ => None,
    }
}

fn terminal_should_keep_platform_shortcut(input: TerminalKeyInput<'_>) -> bool {
    if !input.modifiers.platform {
        return false;
    }

    let key = terminal_normalize_key(input.key);
    let bare_platform = !input.modifiers.control && !input.modifiers.alt && !input.modifiers.shift;
    let platform_alt = !input.modifiers.control && input.modifiers.alt && !input.modifiers.shift;
    let platform_shift = !input.modifiers.control && !input.modifiers.alt && input.modifiers.shift;

    matches!(
        (key.as_str(), bare_platform, platform_alt, platform_shift),
        ("h", true, _, _)
            | ("m", true, _, _)
            | ("q", true, _, _)
            | ("w", true, _, _)
            | ("`", true, _, _)
            | ("tab", true, _, _)
            | ("h", _, true, _)
            | ("m", _, true, _)
            | ("tab", _, _, true)
    )
}

fn terminal_normalize_key(key: &str) -> String {
    let normalized = key.to_ascii_lowercase();
    match normalized.as_str() {
        "return" | "kp_enter" | "numpadenter" | "numpad_enter" => "enter",
        "esc" => "escape",
        "backtab" | "iso_left_tab" => "tab",
        "del" => "delete",
        "pgup" | "page_up" => "pageup",
        "pgdn" | "page_down" => "pagedown",
        "arrowup" | "arrow_up" | "up_arrow" => "up",
        "arrowdown" | "arrow_down" | "down_arrow" => "down",
        "arrowleft" | "arrow_left" | "left_arrow" => "left",
        "arrowright" | "arrow_right" | "right_arrow" => "right",
        _ => normalized.as_str(),
    }
    .to_string()
}

fn control_key_char_sequence(input: TerminalKeyInput<'_>) -> Option<Vec<u8>> {
    let key_char = input.key_char?;
    let mut chars = key_char.chars();
    let ch = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if ch.is_control() {
        return Some(vec![ch as u8]);
    }
    ctrl_sequence(&ch.to_string()).map(|sequence| sequence.as_bytes().to_vec())
}

fn ctrl_sequence(key: &str) -> Option<&'static str> {
    match key {
        "a" | "A" => Some("\x01"),
        "b" | "B" => Some("\x02"),
        "c" | "C" => Some("\x03"),
        "d" | "D" => Some("\x04"),
        "e" | "E" => Some("\x05"),
        "f" | "F" => Some("\x06"),
        "g" | "G" => Some("\x07"),
        "h" | "H" => Some("\x08"),
        "i" | "I" => Some("\x09"),
        "j" | "J" => Some("\x0a"),
        "k" | "K" => Some("\x0b"),
        "l" | "L" => Some("\x0c"),
        "m" | "M" => Some("\x0d"),
        "n" | "N" => Some("\x0e"),
        "o" | "O" => Some("\x0f"),
        "p" | "P" => Some("\x10"),
        "q" | "Q" => Some("\x11"),
        "r" | "R" => Some("\x12"),
        "s" | "S" => Some("\x13"),
        "t" | "T" => Some("\x14"),
        "u" | "U" => Some("\x15"),
        "v" | "V" => Some("\x16"),
        "w" | "W" => Some("\x17"),
        "x" | "X" => Some("\x18"),
        "y" | "Y" => Some("\x19"),
        "z" | "Z" => Some("\x1a"),
        "@" => Some("\x00"),
        "[" => Some("\x1b"),
        "\\" => Some("\x1c"),
        "]" => Some("\x1d"),
        "^" => Some("\x1e"),
        "_" => Some("\x1f"),
        "?" => Some("\x7f"),
        _ => None,
    }
}

fn terminal_modifier_code(modifiers: TerminalKeyInputModifiers) -> u32 {
    let mut code = 0;
    if modifiers.shift {
        code |= 1;
    }
    if modifiers.alt {
        code |= 1 << 1;
    }
    if modifiers.control {
        code |= 1 << 2;
    }
    code + 1
}

fn terminal_mouse_tracking_mode(mode: TerminalInputMode) -> ghostty_mouse::TrackingMode {
    if mode.mouse_motion {
        ghostty_mouse::TrackingMode::Any
    } else if mode.mouse_drag {
        ghostty_mouse::TrackingMode::Button
    } else {
        ghostty_mouse::TrackingMode::Normal
    }
}

fn terminal_mouse_format(mode: TerminalInputMode) -> ghostty_mouse::Format {
    if mode.sgr_mouse {
        ghostty_mouse::Format::Sgr
    } else if mode.utf8_mouse {
        ghostty_mouse::Format::Utf8
    } else {
        ghostty_mouse::Format::X10
    }
}

fn terminal_mouse_action(action: TerminalMouseAction) -> ghostty_mouse::Action {
    match action {
        TerminalMouseAction::Press => ghostty_mouse::Action::Press,
        TerminalMouseAction::Release => ghostty_mouse::Action::Release,
        TerminalMouseAction::Move => ghostty_mouse::Action::Motion,
    }
}

fn terminal_mouse_button(button: Option<TerminalMouseButton>) -> Option<ghostty_mouse::Button> {
    match button {
        Some(TerminalMouseButton::Left) => Some(ghostty_mouse::Button::Left),
        Some(TerminalMouseButton::Middle) => Some(ghostty_mouse::Button::Middle),
        Some(TerminalMouseButton::Right) => Some(ghostty_mouse::Button::Right),
        Some(TerminalMouseButton::WheelUp) => Some(ghostty_mouse::Button::Four),
        Some(TerminalMouseButton::WheelDown) => Some(ghostty_mouse::Button::Five),
        None => None,
    }
}

fn ghostty_modifiers(modifiers: TerminalKeyInputModifiers) -> ghostty_key::Mods {
    let mut mods = ghostty_key::Mods::empty();
    if modifiers.shift {
        mods |= ghostty_key::Mods::SHIFT;
    }
    if modifiers.alt {
        mods |= ghostty_key::Mods::ALT;
    }
    if modifiers.control {
        mods |= ghostty_key::Mods::CTRL;
    }
    if modifiers.platform {
        mods |= ghostty_key::Mods::SUPER;
    }
    mods
}
