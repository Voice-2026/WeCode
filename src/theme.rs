use gpui::{App, Hsla, Window, rgb};
use gpui_component::{Colorize, Theme, ThemeMode};

pub const BG: u32 = 0x0F1117;
pub const BG_ELEVATED: u32 = 0x161A23;
pub const BG_PANEL: u32 = 0x1B202B;
pub const BG_TERMINAL: u32 = 0x11141A;
pub const BG_COLUMN: u32 = 0x131821;
pub const BG_HEADER: u32 = 0x181D27;
pub const BG_ROW_HOVER: u32 = 0x202736;
pub const BG_ROW_ACTIVE: u32 = 0x19314E;
pub const BORDER: u32 = 0x2A3040;
pub const BORDER_SOFT: u32 = 0x2D3545;
pub const TEXT: u32 = 0xE7EAF0;
pub const TEXT_MUTED: u32 = 0x97A1B3;
pub const TEXT_DIM: u32 = 0x687286;
// Prefer the system accent once GPUI exposes it directly; blue is the fallback.
pub const ACCENT: u32 = 0x2F80ED;
pub const ORANGE: u32 = 0xE6A35C;
pub const GREEN: u32 = 0x78D891;
pub const STATUS_BAR: u32 = 0x1C1F25;
pub fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}

pub fn apply_component_theme_for_name(
    theme_name: &str,
    mut window: Option<&mut Window>,
    cx: &mut App,
) {
    let mode = if is_light_component_theme(theme_name) {
        ThemeMode::Light
    } else {
        ThemeMode::Dark
    };
    Theme::change(mode, window.as_deref_mut(), cx);

    configure_component_theme(cx);
    if let Some(window) = window {
        window.refresh();
    }
}

fn is_light_component_theme(theme_name: &str) -> bool {
    let normalized = theme_name.to_ascii_lowercase();
    normalized.contains("light")
        || normalized.contains("day")
        || normalized.contains("latte")
        || normalized.contains("gruvbox material light")
        || normalized.contains("nord light")
}

fn configure_component_theme(cx: &mut App) {
    let is_dark = Theme::global(cx).is_dark();
    let theme = Theme::global_mut(cx);
    let (
        background,
        foreground,
        muted,
        muted_foreground,
        border,
        control_bg,
        control_hover,
        input,
        ring,
        accent_bg,
        accent,
        primary_hover,
        primary_active,
        primary_foreground,
        popover,
        sidebar,
        header,
        row_hover,
        title_bar,
        tab,
        tab_bar,
        tab_segmented,
        scrollbar_thumb,
        overlay,
        danger,
        warning,
        success,
        info,
    ) = if is_dark {
        (
            color(BG),
            color(TEXT),
            color(BG_PANEL),
            color(TEXT_MUTED),
            color(BORDER_SOFT),
            color(0xFFFFFF).opacity(0.055),
            color(0xFFFFFF).opacity(0.085),
            color(0xFFFFFF).opacity(0.075),
            color(ACCENT).opacity(0.30),
            color(BG_ROW_ACTIVE),
            color(ACCENT),
            color(0x4C9AFF),
            color(0x1F6FD1),
            color(0xF6FAFF),
            color(BG_ELEVATED),
            color(BG_COLUMN),
            color(BG_HEADER),
            color(BG_ROW_HOVER),
            color(BG_HEADER),
            color(BG),
            color(BG_HEADER),
            color(BG_COLUMN),
            color(BORDER),
            color(0x000000).opacity(0.42),
            color(0xF87171),
            color(ORANGE),
            color(GREEN),
            color(0x60A5FA),
        )
    } else {
        (
            color(0xF7F8FA),
            color(0x1F2430),
            color(0xEEF1F5),
            color(0x667085),
            color(0xD7DCE4),
            color(0xEEF1F5),
            color(0xE3E8F0),
            color(0xE1E6EE),
            color(ACCENT).opacity(0.46),
            color(0xE7F0FF),
            color(ACCENT),
            color(0x1F6FD1),
            color(0x1B5FC0),
            color(0xFFFFFF),
            color(0xFFFFFF),
            color(0xF0F3F7),
            color(0xF7F8FA),
            color(0xE9EDF3),
            color(0xF2F4F8),
            color(0xFFFFFF),
            color(0xF2F4F8),
            color(0xEEF1F5),
            color(0xAAB2C0),
            color(0x000000).opacity(0.28),
            color(0xDC2626),
            color(0xD97706),
            color(0x16A34A),
            color(0x2563EB),
        )
    };

    theme.shadow = false;
    theme.radius = gpui::px(6.0);
    theme.radius_lg = gpui::px(8.0);
    theme.background = background;
    theme.foreground = foreground;
    theme.muted = muted;
    theme.muted_foreground = muted_foreground;
    theme.border = border;
    theme.primary = accent;
    theme.primary_hover = primary_hover;
    theme.primary_active = primary_active;
    theme.primary_foreground = primary_foreground;
    theme.button_primary = theme.primary;
    theme.button_primary_hover = theme.primary_hover;
    theme.button_primary_active = theme.primary_active;
    theme.button_primary_foreground = theme.primary_foreground;
    theme.secondary = control_bg;
    theme.secondary_hover = row_hover;
    theme.secondary_foreground = foreground;
    theme.secondary_active = control_hover;
    theme.group_box = control_bg;
    theme.group_box_foreground = foreground;
    theme.accent = accent_bg;
    theme.accent_foreground = foreground;
    theme.input = input;
    theme.caret = accent;
    theme.ring = ring;
    theme.selection = accent.opacity(0.28);
    theme.danger = danger;
    theme.danger_hover = danger.mix(theme.transparent, 0.22);
    theme.danger_active = danger.mix(theme.transparent, 0.34);
    theme.danger_foreground = primary_foreground;
    theme.warning = warning;
    theme.warning_hover = warning.mix(theme.transparent, 0.22);
    theme.warning_active = warning.mix(theme.transparent, 0.34);
    theme.warning_foreground = primary_foreground;
    theme.success = success;
    theme.success_hover = success.mix(theme.transparent, 0.22);
    theme.success_active = success.mix(theme.transparent, 0.34);
    theme.success_foreground = primary_foreground;
    theme.info = info;
    theme.info_hover = info.mix(theme.transparent, 0.22);
    theme.info_active = info.mix(theme.transparent, 0.34);
    theme.info_foreground = primary_foreground;
    theme.link = if is_dark {
        color(0x8ABEFF)
    } else {
        color(0x1F6FD1)
    };
    theme.link_hover = if is_dark {
        color(0xB9D8FF)
    } else {
        color(0x164EA8)
    };
    theme.link_active = if is_dark {
        color(0x6CAAF8)
    } else {
        color(0x123E86)
    };
    theme.popover = popover;
    theme.popover_foreground = foreground;
    theme.drop_target = accent.opacity(0.16);
    theme.drag_border = accent.opacity(0.50);
    theme.tiles = muted;
    theme.title_bar = title_bar;
    theme.title_bar_border = border;
    theme.tab = tab;
    theme.tab_active = accent;
    theme.tab_active_foreground = primary_foreground;
    theme.tab_bar = tab_bar;
    theme.tab_bar_segmented = tab_segmented;
    theme.tab_foreground = muted_foreground;
    theme.colors.list = background;
    theme.list_hover = row_hover;
    theme.list_active = accent_bg;
    theme.list_active_border = accent.opacity(0.46);
    theme.list_head = header;
    theme.list_even = background;
    theme.table = background;
    theme.table_hover = row_hover;
    theme.table_active = accent_bg;
    theme.table_active_border = accent.opacity(0.46);
    theme.table_even = background;
    theme.table_head = header;
    theme.table_head_foreground = muted_foreground;
    theme.table_foot = header;
    theme.table_foot_foreground = muted_foreground;
    theme.table_row_border = border;
    theme.switch = row_hover;
    theme.switch_thumb = muted_foreground;
    theme.scrollbar = sidebar.opacity(0.0);
    theme.scrollbar_thumb = scrollbar_thumb;
    theme.scrollbar_thumb_hover = muted_foreground;
    theme.sidebar = sidebar;
    theme.sidebar_foreground = foreground;
    theme.sidebar_border = border;
    theme.sidebar_accent = accent_bg;
    theme.sidebar_accent_foreground = foreground;
    theme.sidebar_primary = accent;
    theme.sidebar_primary_foreground = primary_foreground;
    theme.skeleton = control_bg;
    theme.accordion = muted;
    theme.accordion_hover = control_hover;
    theme.overlay = overlay;
    theme.window_border = border;
}
