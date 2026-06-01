use gpui::{App, Hsla, SharedString, TitlebarOptions, Window, point, px, rgb};
use gpui_component::{Colorize, Theme, ThemeMode};
use std::sync::atomic::{AtomicU32, Ordering};

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

static DYNAMIC_BG: AtomicU32 = AtomicU32::new(BG);
static DYNAMIC_BG_ELEVATED: AtomicU32 = AtomicU32::new(BG_ELEVATED);
static DYNAMIC_BG_PANEL: AtomicU32 = AtomicU32::new(BG_PANEL);
static DYNAMIC_BG_TERMINAL: AtomicU32 = AtomicU32::new(BG_TERMINAL);
static DYNAMIC_BG_COLUMN: AtomicU32 = AtomicU32::new(BG_COLUMN);
static DYNAMIC_BG_HEADER: AtomicU32 = AtomicU32::new(BG_HEADER);
static DYNAMIC_BG_ROW_HOVER: AtomicU32 = AtomicU32::new(BG_ROW_HOVER);
static DYNAMIC_BG_ROW_ACTIVE: AtomicU32 = AtomicU32::new(BG_ROW_ACTIVE);
static DYNAMIC_BORDER: AtomicU32 = AtomicU32::new(BORDER);
static DYNAMIC_BORDER_SOFT: AtomicU32 = AtomicU32::new(BORDER_SOFT);
static DYNAMIC_TEXT: AtomicU32 = AtomicU32::new(TEXT);
static DYNAMIC_TEXT_MUTED: AtomicU32 = AtomicU32::new(TEXT_MUTED);
static DYNAMIC_TEXT_DIM: AtomicU32 = AtomicU32::new(TEXT_DIM);
static DYNAMIC_ACCENT: AtomicU32 = AtomicU32::new(ACCENT);
static DYNAMIC_STATUS_BAR: AtomicU32 = AtomicU32::new(STATUS_BAR);

pub fn color(hex: u32) -> Hsla {
    rgb(dynamic_color(hex)).into()
}

pub fn fixed_color(hex: u32) -> Hsla {
    raw_color(hex)
}

fn raw_color(hex: u32) -> Hsla {
    rgb(hex).into()
}

fn dynamic_color(hex: u32) -> u32 {
    match hex {
        BG => DYNAMIC_BG.load(Ordering::Relaxed),
        BG_ELEVATED => DYNAMIC_BG_ELEVATED.load(Ordering::Relaxed),
        BG_PANEL => DYNAMIC_BG_PANEL.load(Ordering::Relaxed),
        BG_TERMINAL => DYNAMIC_BG_TERMINAL.load(Ordering::Relaxed),
        BG_COLUMN => DYNAMIC_BG_COLUMN.load(Ordering::Relaxed),
        BG_HEADER => DYNAMIC_BG_HEADER.load(Ordering::Relaxed),
        BG_ROW_HOVER => DYNAMIC_BG_ROW_HOVER.load(Ordering::Relaxed),
        BG_ROW_ACTIVE => DYNAMIC_BG_ROW_ACTIVE.load(Ordering::Relaxed),
        BORDER => DYNAMIC_BORDER.load(Ordering::Relaxed),
        BORDER_SOFT => DYNAMIC_BORDER_SOFT.load(Ordering::Relaxed),
        TEXT => DYNAMIC_TEXT.load(Ordering::Relaxed),
        TEXT_MUTED => DYNAMIC_TEXT_MUTED.load(Ordering::Relaxed),
        TEXT_DIM => DYNAMIC_TEXT_DIM.load(Ordering::Relaxed),
        ACCENT => DYNAMIC_ACCENT.load(Ordering::Relaxed),
        STATUS_BAR => DYNAMIC_STATUS_BAR.load(Ordering::Relaxed),
        _ => hex,
    }
}

fn set_dynamic_color(cell: &AtomicU32, value: u32) {
    cell.store(value, Ordering::Relaxed);
}

fn rgba_to_u32(value: Hsla) -> u32 {
    let rgba = value.to_rgb();
    let channel = |component: f32| -> u32 { (component.clamp(0.0, 1.0) * 255.0).round() as u32 };
    (channel(rgba.r) << 16) | (channel(rgba.g) << 8) | channel(rgba.b)
}

fn mix_hex(foreground: u32, background: u32, background_ratio: f32) -> u32 {
    let background_ratio = background_ratio.clamp(0.0, 1.0);
    let foreground_ratio = 1.0 - background_ratio;
    let channel = |shift: u32| -> u32 {
        let foreground_channel = ((foreground >> shift) & 0xFF) as f32;
        let background_channel = ((background >> shift) & 0xFF) as f32;
        (foreground_channel * foreground_ratio + background_channel * background_ratio).round()
            as u32
    };
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

fn mix_towards(color: Hsla, target: Hsla, amount: f32) -> Hsla {
    raw_color(mix_hex(rgba_to_u32(color), rgba_to_u32(target), amount))
}

pub fn codux_titlebar(title: impl Into<SharedString>) -> TitlebarOptions {
    TitlebarOptions {
        title: Some(title.into()),
        appears_transparent: true,
        traffic_light_position: codux_traffic_light_position(),
    }
}

#[cfg(target_os = "macos")]
fn codux_traffic_light_position() -> Option<gpui::Point<gpui::Pixels>> {
    Some(point(px(14.0), px(16.0)))
}

#[cfg(not(target_os = "macos"))]
fn codux_traffic_light_position() -> Option<gpui::Point<gpui::Pixels>> {
    None
}

pub fn apply_component_theme(
    theme_name: &str,
    theme_color: &str,
    mut window: Option<&mut Window>,
    cx: &mut App,
) {
    let terminal = terminal_theme_palette(theme_name);
    let mode = if terminal.is_light {
        ThemeMode::Light
    } else {
        ThemeMode::Dark
    };
    Theme::change(mode, window.as_deref_mut(), cx);

    configure_component_theme(cx, terminal, theme_color_value(theme_color));
    cx.refresh_windows();
    if let Some(window) = window {
        window.refresh();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TerminalThemePalette {
    pub background: u32,
    pub foreground: u32,
    pub cursor: u32,
    pub selection: u32,
    pub black: u32,
    pub red: u32,
    pub green: u32,
    pub yellow: u32,
    pub blue: u32,
    pub magenta: u32,
    pub cyan: u32,
    pub white: u32,
    pub bright_black: u32,
    pub bright_red: u32,
    pub bright_green: u32,
    pub bright_yellow: u32,
    pub bright_blue: u32,
    pub bright_magenta: u32,
    pub bright_cyan: u32,
    pub bright_white: u32,
    pub muted_foreground: u32,
    pub is_light: bool,
}

impl TerminalThemePalette {
    fn auto() -> Self {
        Self::from_colors(
            false, 0x100F0F, 0xCECDC3, 0xCECDC3, 0x403E3C, 0x100F0F, 0xAF3029, 0x66800B, 0xAD8301,
            0x205EA6, 0x5E409D, 0x24837B, 0xCECDC3, 0x575653, 0xD14D41, 0x879A39, 0xD0A215,
            0x4385BE, 0x8B7EC8, 0x3AA99F, 0xFFFCF0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_colors(
        is_light: bool,
        background: u32,
        foreground: u32,
        cursor: u32,
        selection: u32,
        black: u32,
        red: u32,
        green: u32,
        yellow: u32,
        blue: u32,
        magenta: u32,
        cyan: u32,
        white: u32,
        bright_black: u32,
        bright_red: u32,
        bright_green: u32,
        bright_yellow: u32,
        bright_blue: u32,
        bright_magenta: u32,
        bright_cyan: u32,
        bright_white: u32,
    ) -> Self {
        Self {
            background,
            foreground,
            cursor,
            selection,
            black,
            red,
            green,
            yellow,
            blue,
            magenta,
            cyan,
            white,
            bright_black,
            bright_red,
            bright_green,
            bright_yellow,
            bright_blue,
            bright_magenta,
            bright_cyan,
            bright_white,
            muted_foreground: mix_hex(foreground, background, if is_light { 0.42 } else { 0.36 }),
            is_light,
        }
    }
}

pub fn terminal_theme_palette(theme_name: &str) -> TerminalThemePalette {
    match normalize_theme_name(theme_name).as_str() {
        "auto" => TerminalThemePalette::auto(),
        "tokyonight storm" | "tokyo night storm" => TerminalThemePalette::from_colors(
            false, 0x24283B, 0xC0CAF5, 0xC0CAF5, 0x364A82, 0x1D202F, 0xF7768E, 0x9ECE6A, 0xE0AF68,
            0x7AA2F7, 0xBB9AF7, 0x7DCFFF, 0xC0CAF5, 0x565F89, 0xFF7A93, 0xB9F27C, 0xFFCB6B,
            0x7DA6FF, 0xC8A7FF, 0x90E0FF, 0xDFE5FF,
        ),
        "tokyonight night" | "tokyo night night" => TerminalThemePalette::from_colors(
            false, 0x1A1B26, 0xC0CAF5, 0xC0CAF5, 0x33467C, 0x15161E, 0xF7768E, 0x9ECE6A, 0xE0AF68,
            0x7AA2F7, 0xBB9AF7, 0x7DCFFF, 0xC0CAF5, 0x565F89, 0xFF7A93, 0xB9F27C, 0xFFCB6B,
            0x7DA6FF, 0xC8A7FF, 0x90E0FF, 0xDFE5FF,
        ),
        "catppuccin mocha" => TerminalThemePalette::from_colors(
            false, 0x1E1E2E, 0xCDD6F4, 0xF5E0DC, 0x45475A, 0x181825, 0xF38BA8, 0xA6E3A1, 0xF9E2AF,
            0x89B4FA, 0xCBA6F7, 0x94E2D5, 0xBAC2DE, 0x585B70, 0xF38BA8, 0xA6E3A1, 0xF9E2AF,
            0x89B4FA, 0xCBA6F7, 0x94E2D5, 0xA6ADC8,
        ),
        "catppuccin latte" => TerminalThemePalette::from_colors(
            true, 0xEFF1F5, 0x4C4F69, 0xDC8A78, 0xCCD0DA, 0x5C5F77, 0xD20F39, 0x40A02B, 0xDF8E1D,
            0x1E66F5, 0x8839EF, 0x179299, 0xACB0BE, 0x6C6F85, 0xD20F39, 0x40A02B, 0xDF8E1D,
            0x1E66F5, 0x8839EF, 0x179299, 0xBCC0CC,
        ),
        "rose pine moon" => TerminalThemePalette::from_colors(
            false, 0x232136, 0xE0DEF4, 0xC4A7E7, 0x393552, 0x393552, 0xEB6F92, 0x9CCFD8, 0xF6C177,
            0x3E8FB0, 0xC4A7E7, 0xEA9A97, 0xE0DEF4, 0x6E6A86, 0xEB6F92, 0x9CCFD8, 0xF6C177,
            0x3E8FB0, 0xC4A7E7, 0xEA9A97, 0xE0DEF4,
        ),
        "kanagawa wave" => TerminalThemePalette::from_colors(
            false, 0x1F1F28, 0xDCD7BA, 0xC8C093, 0x2D4F67, 0x090618, 0xC34043, 0x76946A, 0xC0A36E,
            0x7E9CD8, 0x957FB8, 0x6A9589, 0xC8C093, 0x727169, 0xE82424, 0x98BB6C, 0xE6C384,
            0x7FB4CA, 0x938AA9, 0x7AA89F, 0xDCD7BA,
        ),
        "material ocean" => TerminalThemePalette::from_colors(
            false, 0x0F111A, 0x8F93A2, 0xFFCC00, 0x1F2233, 0x000000, 0xFF5370, 0xC3E88D, 0xFFCB6B,
            0x82AAFF, 0xC792EA, 0x89DDFF, 0xFFFFFF, 0x546E7A, 0xFF869A, 0xDDFFA7, 0xFFD98F,
            0x9CC4FF, 0xD6A8FF, 0xA6EAFF, 0xFFFFFF,
        ),
        "ayu mirage" => TerminalThemePalette::from_colors(
            false, 0x1F2430, 0xCBCCC6, 0xFFCC66, 0x33415E, 0x191E2A, 0xF28779, 0xBAE67E, 0xFFD580,
            0x73D0FF, 0xD4BFFF, 0x95E6CB, 0xC7C7C7, 0x686868, 0xF28779, 0xBAE67E, 0xFFD580,
            0x73D0FF, 0xD4BFFF, 0x95E6CB, 0xFFFFFF,
        ),
        "dracula" | "dracula+" => TerminalThemePalette::from_colors(
            false, 0x282A36, 0xF8F8F2, 0xF8F8F2, 0x44475A, 0x21222C, 0xFF5555, 0x50FA7B, 0xF1FA8C,
            0xBD93F9, 0xFF79C6, 0x8BE9FD, 0xF8F8F2, 0x6272A4, 0xFF6E6E, 0x69FF94, 0xFFFFA5,
            0xD6ACFF, 0xFF92DF, 0xA4FFFF, 0xFFFFFF,
        ),
        "github dark" => TerminalThemePalette::from_colors(
            false, 0x0D1117, 0xC9D1D9, 0xC9D1D9, 0x264F78, 0x484F58, 0xFF7B72, 0x3FB950, 0xD29922,
            0x58A6FF, 0xBC8CFF, 0x39C5CF, 0xB1BAC4, 0x6E7681, 0xFFA198, 0x56D364, 0xE3B341,
            0x79C0FF, 0xD2A8FF, 0x56D4DD, 0xF0F6FC,
        ),
        "gruvbox dark" => TerminalThemePalette::from_colors(
            false, 0x282828, 0xEBDBB2, 0xFABD2F, 0x504945, 0x282828, 0xCC241D, 0x98971A, 0xD79921,
            0x458588, 0xB16286, 0x689D6A, 0xA89984, 0x928374, 0xFB4934, 0xB8BB26, 0xFABD2F,
            0x83A598, 0xD3869B, 0x8EC07C, 0xEBDBB2,
        ),
        "gruvbox material dark" => TerminalThemePalette::from_colors(
            false, 0x1D2021, 0xD4BE98, 0xD4BE98, 0x3C3836, 0x32302F, 0xEA6962, 0xA9B665, 0xD8A657,
            0x7DAEA3, 0xD3869B, 0x89B482, 0xD4BE98, 0x665C54, 0xEA6962, 0xA9B665, 0xD8A657,
            0x7DAEA3, 0xD3869B, 0x89B482, 0xDDC7A1,
        ),
        "nord" => TerminalThemePalette::from_colors(
            false, 0x2E3440, 0xD8DEE9, 0xD8DEE9, 0x4C566A, 0x3B4252, 0xBF616A, 0xA3BE8C, 0xEBCB8B,
            0x81A1C1, 0xB48EAD, 0x88C0D0, 0xE5E9F0, 0x4C566A, 0xBF616A, 0xA3BE8C, 0xEBCB8B,
            0x81A1C1, 0xB48EAD, 0x8FBCBB, 0xECEFF4,
        ),
        "tokyonight day" | "tokyo night day" => TerminalThemePalette::from_colors(
            true, 0xE1E2E7, 0x3760BF, 0x3760BF, 0xB7C1E3, 0xE9E9ED, 0xF52A65, 0x587539, 0x8C6C3E,
            0x2E7DE9, 0x9854F1, 0x007197, 0x6172B0, 0xA1A6C5, 0xF52A65, 0x587539, 0x8C6C3E,
            0x2E7DE9, 0x9854F1, 0x007197, 0x3760BF,
        ),
        "github light" => TerminalThemePalette::from_colors(
            true, 0xFFFFFF, 0x24292F, 0x0969DA, 0xB6D7FF, 0x24292F, 0xCF222E, 0x116329, 0x4D2D00,
            0x0969DA, 0x8250DF, 0x1B7C83, 0x6E7781, 0x57606A, 0xA40E26, 0x1A7F37, 0x9A6700,
            0x218BFF, 0xA475F9, 0x3192AA, 0xF6F8FA,
        ),
        "flexoki dark" => TerminalThemePalette::from_colors(
            false, 0x100F0F, 0xCECDC3, 0xCECDC3, 0x403E3C, 0x100F0F, 0xAF3029, 0x66800B, 0xAD8301,
            0x205EA6, 0x5E409D, 0x24837B, 0xCECDC3, 0x575653, 0xD14D41, 0x879A39, 0xD0A215,
            0x4385BE, 0x8B7EC8, 0x3AA99F, 0xFFFCF0,
        ),
        "flexoki light" => TerminalThemePalette::from_colors(
            true, 0xFFFCF0, 0x100F0F, 0x100F0F, 0xE6E4D9, 0x100F0F, 0xAF3029, 0x66800B, 0xAD8301,
            0x205EA6, 0x5E409D, 0x24837B, 0xCECDC3, 0x6F6E69, 0xD14D41, 0x879A39, 0xD0A215,
            0x4385BE, 0x8B7EC8, 0x3AA99F, 0xFFFCF0,
        ),
        "gruvbox light" => TerminalThemePalette::from_colors(
            true, 0xFBF1C7, 0x3C3836, 0x3C3836, 0xD5C4A1, 0xFBF1C7, 0xCC241D, 0x98971A, 0xD79921,
            0x458588, 0xB16286, 0x689D6A, 0x7C6F64, 0x928374, 0x9D0006, 0x79740E, 0xB57614,
            0x076678, 0x8F3F71, 0x427B58, 0x3C3836,
        ),
        "gruvbox material light" => TerminalThemePalette::from_colors(
            true, 0xFBF1C7, 0x654735, 0x654735, 0xD5C4A1, 0xFBF1C7, 0xC14A4A, 0x6C782E, 0xB47109,
            0x45707A, 0x945E80, 0x4C7A5D, 0x654735, 0x928374, 0xC14A4A, 0x6C782E, 0xB47109,
            0x45707A, 0x945E80, 0x4C7A5D, 0x3C3836,
        ),
        "nord light" => TerminalThemePalette::from_colors(
            true, 0xECEFF4, 0x2E3440, 0x2E3440, 0xD8DEE9, 0x3B4252, 0xBF616A, 0xA3BE8C, 0xD08770,
            0x5E81AC, 0xB48EAD, 0x8FBCBB, 0xE5E9F0, 0x4C566A, 0xBF616A, 0xA3BE8C, 0xEBCB8B,
            0x81A1C1, 0xB48EAD, 0x88C0D0, 0xECEFF4,
        ),
        "atom one light" => TerminalThemePalette::from_colors(
            true, 0xFAFAFA, 0x383A42, 0x526FFF, 0xE5E5E6, 0x383A42, 0xE45649, 0x50A14F, 0xC18401,
            0x4078F2, 0xA626A4, 0x0184BC, 0xA0A1A7, 0x696C77, 0xE45649, 0x50A14F, 0xC18401,
            0x4078F2, 0xA626A4, 0x0184BC, 0xF0F0F0,
        ),
        _ => {
            let normalized = normalize_theme_name(theme_name);
            if normalized.contains("day")
                || normalized.contains("latte")
                || normalized.contains("light")
            {
                terminal_theme_palette("Catppuccin Latte")
            } else if normalized.contains("nord") {
                terminal_theme_palette("Nord")
            } else if normalized.contains("night")
                || normalized.contains("mocha")
                || normalized.contains("moon")
                || normalized.contains("wave")
                || normalized.contains("dark")
            {
                terminal_theme_palette("Tokyo Night Night")
            } else {
                TerminalThemePalette::auto()
            }
        }
    }
}

fn normalize_theme_name(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace(['_', '-'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn theme_color_value(theme_color: &str) -> u32 {
    match theme_color.to_ascii_lowercase().as_str() {
        "sky" => 0x0EA5E9,
        "cyan" => 0x06B6D4,
        "teal" => 0x14B8A6,
        "emerald" | "moss" => 0x10B981,
        "green" | "sage" => 0x22C55E,
        "lime" => 0x84CC16,
        "amber" | "gold" => 0xF59E0B,
        "orange" | "burnt" => 0xF97316,
        "red" | "crimson" => 0xEF4444,
        "rose" | "plum" => 0xF43F5E,
        "pink" => 0xEC4899,
        "fuchsia" => 0xD946EF,
        "purple" => 0xA855F7,
        "violet" | "iris" | "lavender" => 0x8B5CF6,
        "indigo" => 0x6366F1,
        _ => 0x3B82F6,
    }
}

fn configure_component_theme(cx: &mut App, terminal: TerminalThemePalette, accent_hex: u32) {
    let is_dark = !terminal.is_light;
    let theme = Theme::global_mut(cx);
    let terminal_background = raw_color(terminal.background);
    let accent_color = raw_color(accent_hex);
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
        task_column,
    ) = if is_dark {
        let selection = raw_color(terminal.selection);
        let current_line = raw_color(terminal.bright_black);
        let terminal_black = raw_color(terminal.black);
        let app_surface = mix_towards(terminal_background, terminal_black, 0.36);
        let column_surface = mix_towards(app_surface, terminal_background, 0.48);
        let header_surface = mix_towards(app_surface, selection, 0.10);
        (
            app_surface,
            raw_color(terminal.foreground),
            mix_towards(terminal_background, selection, 0.34),
            raw_color(terminal.muted_foreground),
            mix_towards(terminal_background, current_line, 0.30),
            raw_color(0xFFFFFF).opacity(0.055),
            raw_color(0xFFFFFF).opacity(0.085),
            raw_color(0xFFFFFF).opacity(0.075),
            accent_color.opacity(0.30),
            accent_color.opacity(0.17),
            accent_color,
            mix_towards(accent_color, raw_color(0xFFFFFF), 0.18),
            mix_towards(accent_color, raw_color(0x000000), 0.16),
            raw_color(0xF6FAFF),
            mix_towards(app_surface, terminal_black, 0.18),
            app_surface,
            header_surface,
            mix_towards(terminal_background, selection, 0.58),
            app_surface,
            terminal_background,
            header_surface,
            mix_towards(column_surface, selection, 0.08),
            mix_towards(terminal_background, current_line, 0.42),
            raw_color(0x000000).opacity(0.42),
            raw_color(0xF87171),
            color(ORANGE),
            color(GREEN),
            raw_color(0x60A5FA),
            column_surface,
        )
    } else {
        (
            mix_towards(terminal_background, raw_color(0x000000), 0.035),
            raw_color(terminal.foreground),
            mix_towards(terminal_background, raw_color(0x000000), 0.035),
            raw_color(terminal.muted_foreground),
            mix_towards(terminal_background, raw_color(0x000000), 0.12),
            raw_color(0x000000).opacity(0.055),
            raw_color(0x000000).opacity(0.085),
            raw_color(0x000000).opacity(0.075),
            accent_color.opacity(0.34),
            accent_color.opacity(0.12),
            accent_color,
            mix_towards(accent_color, raw_color(0x000000), 0.12),
            mix_towards(accent_color, raw_color(0x000000), 0.22),
            raw_color(0xFFFFFF),
            mix_towards(terminal_background, raw_color(0xFFFFFF), 0.82),
            mix_towards(terminal_background, raw_color(0x000000), 0.070),
            mix_towards(terminal_background, raw_color(0x000000), 0.055),
            mix_towards(terminal_background, raw_color(0x000000), 0.075),
            mix_towards(terminal_background, raw_color(0x000000), 0.055),
            terminal_background,
            mix_towards(terminal_background, raw_color(0x000000), 0.045),
            raw_color(0x000000).opacity(0.055),
            mix_towards(terminal_background, raw_color(0x000000), 0.26),
            raw_color(0x000000).opacity(0.28),
            raw_color(0xDC2626),
            raw_color(0xD97706),
            raw_color(0x16A34A),
            raw_color(0x2563EB),
            mix_towards(terminal_background, raw_color(0x000000), 0.045),
        )
    };
    let hover_surface = if is_dark {
        raw_color(0xFFFFFF).opacity(0.10)
    } else {
        raw_color(0x000000).opacity(0.07)
    };

    set_dynamic_color(&DYNAMIC_BG, rgba_to_u32(background));
    set_dynamic_color(&DYNAMIC_BG_ELEVATED, rgba_to_u32(popover));
    set_dynamic_color(&DYNAMIC_BG_PANEL, rgba_to_u32(muted));
    set_dynamic_color(&DYNAMIC_BG_TERMINAL, terminal.background);
    set_dynamic_color(&DYNAMIC_BG_COLUMN, rgba_to_u32(task_column));
    set_dynamic_color(&DYNAMIC_BG_HEADER, rgba_to_u32(header));
    set_dynamic_color(&DYNAMIC_BG_ROW_HOVER, rgba_to_u32(row_hover));
    set_dynamic_color(&DYNAMIC_BG_ROW_ACTIVE, rgba_to_u32(accent_bg));
    set_dynamic_color(&DYNAMIC_BORDER, rgba_to_u32(border));
    set_dynamic_color(&DYNAMIC_BORDER_SOFT, rgba_to_u32(border));
    set_dynamic_color(&DYNAMIC_TEXT, terminal.foreground);
    set_dynamic_color(&DYNAMIC_TEXT_MUTED, terminal.muted_foreground);
    set_dynamic_color(&DYNAMIC_TEXT_DIM, rgba_to_u32(muted_foreground));
    set_dynamic_color(&DYNAMIC_ACCENT, accent_hex);
    set_dynamic_color(&DYNAMIC_STATUS_BAR, rgba_to_u32(title_bar));

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
    theme.secondary_hover = hover_surface;
    theme.secondary_foreground = foreground;
    theme.secondary_active = control_hover;
    theme.group_box = control_bg;
    theme.group_box_foreground = foreground;
    theme.accent = accent_bg;
    theme.accent_foreground = foreground;
    theme.input = input;
    theme.caret = accent;
    theme.ring = ring;
    theme.selection = accent.opacity(if is_dark { 0.28 } else { 0.20 });
    let highlight_style = std::sync::Arc::make_mut(&mut theme.highlight_theme)
        .style
        .clone();
    let mut highlight_style = highlight_style;
    highlight_style.editor_background = Some(background);
    highlight_style.editor_active_line = Some(if is_dark {
        raw_color(0x000000).opacity(0.20)
    } else {
        raw_color(0x000000).opacity(0.055)
    });
    highlight_style.editor_line_number = Some(if is_dark {
        raw_color(0xFFFFFF).opacity(0.32)
    } else {
        raw_color(0x000000).opacity(0.34)
    });
    highlight_style.editor_active_line_number = Some(foreground);
    std::sync::Arc::make_mut(&mut theme.highlight_theme).style = highlight_style;
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
        mix_towards(accent_color, color(0xFFFFFF), 0.34)
    } else {
        accent_color
    };
    theme.link_hover = if is_dark {
        mix_towards(accent_color, color(0xFFFFFF), 0.50)
    } else {
        mix_towards(accent_color, color(0x000000), 0.16)
    };
    theme.link_active = if is_dark {
        accent_color
    } else {
        mix_towards(accent_color, color(0x000000), 0.28)
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
    theme.list_hover = hover_surface;
    theme.list_active = accent_bg;
    theme.list_active_border = accent.opacity(if is_dark { 0.46 } else { 0.36 });
    theme.list_head = header;
    theme.list_even = background;
    theme.table = background;
    theme.table_hover = hover_surface;
    theme.table_active = accent_bg;
    theme.table_active_border = accent.opacity(if is_dark { 0.46 } else { 0.36 });
    theme.table_even = background;
    theme.table_head = header;
    theme.table_head_foreground = muted_foreground;
    theme.table_foot = header;
    theme.table_foot_foreground = muted_foreground;
    theme.table_row_border = border;
    theme.switch = control_hover;
    theme.switch_thumb = background;
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
    theme.accordion_hover = hover_surface;
    theme.overlay = overlay;
    theme.window_border = border;
}
