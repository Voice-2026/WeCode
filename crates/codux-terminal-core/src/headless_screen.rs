use alacritty_terminal::{
    event::{Event, EventListener},
    grid::{Dimensions, Scroll},
    term::{Config as AlacrittyConfig, Term, TermMode, cell::Flags},
    vte::ansi::{Color, NamedColor, Processor},
};
use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TerminalScreenSnapshot {
    pub data: String,
    pub cols: usize,
    pub rows: usize,
    pub display_offset: usize,
    pub cells: Vec<TerminalScreenCellSnapshot>,
    pub cursor: TerminalScreenCursorSnapshot,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TerminalScreenCursorSnapshot {
    pub row: usize,
    pub col: usize,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TerminalScreenCellSnapshot {
    pub row: usize,
    pub col: usize,
    pub text: String,
    pub width: usize,
    pub fg: TerminalScreenColor,
    pub bg: TerminalScreenColor,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub hidden: bool,
    pub strikeout: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TerminalScreenColor {
    Default,
    Named { name: String },
    Rgb { r: u8, g: u8, b: u8 },
    Indexed { index: u8 },
}

pub struct HeadlessTerminalScreen {
    term: Term<HeadlessEventProxy>,
    parser: Processor,
    cols: usize,
    rows: usize,
    scrollback: usize,
}

impl HeadlessTerminalScreen {
    pub fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        let config = AlacrittyConfig {
            scrolling_history: scrollback,
            ..Default::default()
        };
        let size = HeadlessTermSize::new(cols, rows);
        Self {
            term: Term::new(config, &size, HeadlessEventProxy),
            parser: Processor::new(),
            cols,
            rows,
            scrollback,
        }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        self.parser.advance(&mut self.term, bytes);
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(20);
        let rows = rows.max(8);
        if self.cols == cols && self.rows == rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        self.term.resize(HeadlessTermSize::new(cols, rows));
    }

    pub fn scroll_lines(&mut self, lines: i32) {
        if lines == 0 {
            return;
        }
        self.term.scroll_display(Scroll::Delta(lines));
    }

    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    pub fn clear(&mut self) {
        *self = Self::new(self.cols, self.rows, self.scrollback);
    }

    pub fn snapshot(&self) -> TerminalScreenSnapshot {
        let (data, cells, cursor) = headless_screen_snapshot(&self.term);
        TerminalScreenSnapshot {
            data,
            cols: self.term.columns(),
            rows: self.term.screen_lines(),
            display_offset: self.term.grid().display_offset(),
            cells,
            cursor,
        }
    }
}

struct HeadlessTermSize {
    cols: usize,
    rows: usize,
}

impl HeadlessTermSize {
    fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows }
    }
}

impl Dimensions for HeadlessTermSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

#[derive(Clone)]
struct HeadlessEventProxy;

impl EventListener for HeadlessEventProxy {
    fn send_event(&self, _event: Event) {}
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct HeadlessCellStyle {
    fg: Color,
    bg: Color,
    flags: Flags,
}

impl Default for HeadlessCellStyle {
    fn default() -> Self {
        Self {
            fg: Color::Named(NamedColor::Foreground),
            bg: Color::Named(NamedColor::Background),
            flags: Flags::empty(),
        }
    }
}

fn headless_screen_snapshot(
    term: &Term<HeadlessEventProxy>,
) -> (
    String,
    Vec<TerminalScreenCellSnapshot>,
    TerminalScreenCursorSnapshot,
) {
    let content = term.renderable_content();
    let cols = term.columns();
    let rows = term.screen_lines();
    let mut rows_cells = vec![vec![None; cols]; rows];
    let display_offset = content.display_offset;
    let cursor = content.cursor;
    let cursor_visible = content.mode.contains(TermMode::SHOW_CURSOR);
    let mut snapshot_cells = Vec::new();

    for indexed in content.display_iter {
        let row = indexed.point.line.0 + display_offset as i32;
        if row < 0 {
            continue;
        }
        let row = row as usize;
        let col = indexed.point.column.0;
        if row >= rows || col >= cols {
            continue;
        }
        if indexed
            .cell
            .flags
            .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            || indexed.cell.c == '\0'
        {
            continue;
        }
        let mut text = indexed.cell.c.to_string();
        if let Some(zerowidth) = indexed.cell.zerowidth() {
            for ch in zerowidth {
                text.push(*ch);
            }
        }
        let width = if indexed.cell.flags.contains(Flags::WIDE_CHAR) {
            2
        } else {
            1
        };
        let style = HeadlessCellStyle {
            fg: indexed.cell.fg,
            bg: indexed.cell.bg,
            flags: headless_visual_flags(indexed.cell.flags),
        };
        snapshot_cells.push(TerminalScreenCellSnapshot {
            row,
            col,
            text: text.clone(),
            width,
            fg: terminal_screen_color(style.fg),
            bg: terminal_screen_color(style.bg),
            bold: style.flags.contains(Flags::BOLD),
            dim: style.flags.contains(Flags::DIM),
            italic: style.flags.contains(Flags::ITALIC),
            underline: style.flags.intersects(Flags::ALL_UNDERLINES),
            inverse: style.flags.contains(Flags::INVERSE),
            hidden: style.flags.contains(Flags::HIDDEN),
            strikeout: style.flags.contains(Flags::STRIKEOUT),
        });
        rows_cells[row][col] = Some(HeadlessScreenCell { text, width, style });
    }

    let mut output = String::new();
    output.push_str("\x1b[?25l\x1b[0m\x1b[H\x1b[2J");
    let mut current_style = HeadlessCellStyle::default();
    for (row_index, cells) in rows_cells.iter().enumerate() {
        let Some(last_col) = cells.iter().rposition(|cell| {
            cell.as_ref()
                .map(|cell| {
                    !cell.text.trim().is_empty() || cell.style != HeadlessCellStyle::default()
                })
                .unwrap_or(false)
        }) else {
            continue;
        };
        output.push_str(&format!("\x1b[{};1H", row_index + 1));
        let mut col = 0;
        while col <= last_col {
            match &cells[col] {
                Some(cell) => {
                    if cell.style != current_style {
                        output.push_str(&headless_style_sgr(cell.style));
                        current_style = cell.style;
                    }
                    output.push_str(&terminal_snapshot_text(&cell.text));
                    col += cell.width;
                }
                None => {
                    output.push(' ');
                    col += 1;
                }
            }
        }
    }
    if current_style != HeadlessCellStyle::default() {
        output.push_str("\x1b[0m");
    }

    let cursor_row = cursor.point.line.0 + display_offset as i32;
    let mut snapshot_cursor = TerminalScreenCursorSnapshot {
        row: 0,
        col: 0,
        visible: cursor_visible,
    };
    if cursor_row >= 0 {
        let cursor_row = (cursor_row as usize).min(rows.saturating_sub(1));
        let cursor_col = cursor.point.column.0.min(cols.saturating_sub(1));
        snapshot_cursor.row = cursor_row;
        snapshot_cursor.col = cursor_col;
        output.push_str(&format!("\x1b[{};{}H", cursor_row + 1, cursor_col + 1));
    }
    if cursor_visible {
        output.push_str("\x1b[?25h");
    }
    (output, snapshot_cells, snapshot_cursor)
}

#[derive(Clone)]
struct HeadlessScreenCell {
    text: String,
    width: usize,
    style: HeadlessCellStyle,
}

fn terminal_snapshot_text(text: &str) -> String {
    text.chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect()
}

fn headless_style_sgr(style: HeadlessCellStyle) -> String {
    let mut codes = vec!["0".to_string()];
    if style.flags.contains(Flags::BOLD) {
        codes.push("1".to_string());
    }
    if style.flags.contains(Flags::DIM) {
        codes.push("2".to_string());
    }
    if style.flags.contains(Flags::ITALIC) {
        codes.push("3".to_string());
    }
    if style.flags.intersects(Flags::ALL_UNDERLINES) {
        codes.push("4".to_string());
    }
    if style.flags.contains(Flags::INVERSE) {
        codes.push("7".to_string());
    }
    if style.flags.contains(Flags::HIDDEN) {
        codes.push("8".to_string());
    }
    if style.flags.contains(Flags::STRIKEOUT) {
        codes.push("9".to_string());
    }
    headless_color_sgr(style.fg, false, &mut codes);
    headless_color_sgr(style.bg, true, &mut codes);
    format!("\x1b[{}m", codes.join(";"))
}

fn headless_visual_flags(flags: Flags) -> Flags {
    flags
        & !(Flags::WIDE_CHAR
            | Flags::WIDE_CHAR_SPACER
            | Flags::LEADING_WIDE_CHAR_SPACER
            | Flags::WRAPLINE)
}

fn headless_color_sgr(color: Color, background: bool, codes: &mut Vec<String>) {
    match color {
        Color::Named(named) => {
            if let Some(code) = headless_named_color_sgr(named, background) {
                codes.push(code.to_string());
            }
        }
        Color::Spec(rgb) => {
            codes.push(if background { "48" } else { "38" }.to_string());
            codes.push("2".to_string());
            codes.push(rgb.r.to_string());
            codes.push(rgb.g.to_string());
            codes.push(rgb.b.to_string());
        }
        Color::Indexed(index) => {
            codes.push(if background { "48" } else { "38" }.to_string());
            codes.push("5".to_string());
            codes.push(index.to_string());
        }
    }
}

fn terminal_screen_color(color: Color) -> TerminalScreenColor {
    match color {
        Color::Named(named) => match named {
            NamedColor::Foreground | NamedColor::DimForeground | NamedColor::Background => {
                TerminalScreenColor::Default
            }
            NamedColor::Cursor => TerminalScreenColor::Named {
                name: "cursor".to_string(),
            },
            other => TerminalScreenColor::Named {
                name: format!("{other:?}"),
            },
        },
        Color::Spec(rgb) => TerminalScreenColor::Rgb {
            r: rgb.r,
            g: rgb.g,
            b: rgb.b,
        },
        Color::Indexed(index) => TerminalScreenColor::Indexed { index },
    }
}

fn headless_named_color_sgr(named: NamedColor, background: bool) -> Option<u16> {
    let base = if background { 40 } else { 30 };
    let bright = if background { 100 } else { 90 };
    let reset = if background { 49 } else { 39 };
    match named {
        NamedColor::Black | NamedColor::DimBlack => Some(base),
        NamedColor::Red | NamedColor::DimRed => Some(base + 1),
        NamedColor::Green | NamedColor::DimGreen => Some(base + 2),
        NamedColor::Yellow | NamedColor::DimYellow => Some(base + 3),
        NamedColor::Blue | NamedColor::DimBlue => Some(base + 4),
        NamedColor::Magenta | NamedColor::DimMagenta => Some(base + 5),
        NamedColor::Cyan | NamedColor::DimCyan => Some(base + 6),
        NamedColor::White | NamedColor::DimWhite => Some(base + 7),
        NamedColor::BrightBlack => Some(bright),
        NamedColor::BrightRed => Some(bright + 1),
        NamedColor::BrightGreen => Some(bright + 2),
        NamedColor::BrightYellow => Some(bright + 3),
        NamedColor::BrightBlue => Some(bright + 4),
        NamedColor::BrightMagenta => Some(bright + 5),
        NamedColor::BrightCyan => Some(bright + 6),
        NamedColor::BrightWhite | NamedColor::BrightForeground => Some(bright + 7),
        NamedColor::Foreground | NamedColor::DimForeground if !background => Some(reset),
        NamedColor::Background if background => Some(reset),
        NamedColor::Foreground | NamedColor::DimForeground | NamedColor::Background => None,
        NamedColor::Cursor => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redraws_current_screen_after_clear_and_cursor_moves() {
        let mut screen = HeadlessTerminalScreen::new(20, 4, 100);
        screen.process(b"old line\n\x1b[2J\x1b[Htop\x1b[3;5Hbottom");

        let snapshot = screen.snapshot();

        assert_eq!(snapshot.cols, 20);
        assert_eq!(snapshot.rows, 4);
        assert!(snapshot.data.contains("top"));
        assert!(snapshot.data.contains("bottom"));
        assert!(!snapshot.data.contains("old line"));
        assert!(snapshot.cells.iter().any(|cell| cell.text == "t"));
    }

    #[test]
    fn keeps_resize_state() {
        let mut screen = HeadlessTerminalScreen::new(20, 4, 100);
        screen.resize(30, 10);
        screen.process(b"ready");

        let snapshot = screen.snapshot();

        assert_eq!(snapshot.cols, 30);
        assert_eq!(snapshot.rows, 10);
        assert!(snapshot.data.contains("ready"));
    }

    #[test]
    fn preserves_wide_text_without_split_cells() {
        let mut screen = HeadlessTerminalScreen::new(40, 4, 100);
        screen.process("第 2003行 测 试 文 本".as_bytes());

        let snapshot = screen.snapshot();

        assert!(snapshot.data.contains("第 2003行 测 试 文 本"));
        assert!(
            snapshot
                .cells
                .iter()
                .any(|cell| cell.text == "第" && cell.width == 2)
        );
    }

    #[test]
    fn scrolls_viewport_through_history() {
        let mut screen = HeadlessTerminalScreen::new(20, 4, 100);
        screen.process(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix");
        assert_eq!(screen.snapshot().display_offset, 0);

        screen.scroll_lines(2);
        let scrolled = screen.snapshot();
        assert!(scrolled.display_offset > 0);
        assert!(scrolled.data.contains("two") || scrolled.data.contains("three"));

        screen.scroll_to_bottom();
        assert_eq!(screen.snapshot().display_offset, 0);
    }
}
