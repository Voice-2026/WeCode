use std::sync::mpsc;
use std::thread;

use libghostty_vt::{
    RenderState, Terminal, TerminalOptions,
    render::{CellIterator, CursorVisualStyle, RowIterator, Snapshot},
    screen::{Cell, CellContentTag, CellWide},
    style::{RgbColor, Style, Underline},
    terminal::{Mode, ScrollViewport},
};
use serde::Serialize;

use crate::TerminalInputMode;

const GHOSTTY_CELL_WIDTH_PX: u32 = 10;
const GHOSTTY_CELL_HEIGHT_PX: u32 = 20;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalScreenSnapshot {
    pub data: String,
    pub cols: usize,
    pub rows: usize,
    pub total_lines: usize,
    pub display_offset: usize,
    pub scroll_pixel_offset: f64,
    pub application_cursor: bool,
    pub input_mode: TerminalInputMode,
    pub cells: Vec<TerminalScreenCellSnapshot>,
    pub cursor: TerminalScreenCursorSnapshot,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TerminalScreenCursorSnapshot {
    pub row: usize,
    pub col: usize,
    pub visible: bool,
    pub shape: TerminalScreenCursorShape,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalScreenCursorShape {
    #[default]
    Block,
    Beam,
    Underline,
    HollowBlock,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TerminalScreenCellSnapshot {
    pub row: i32,
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
    engine: GhosttyTerminalScreenEngine,
    pending_scroll_pixels: f64,
}

impl HeadlessTerminalScreen {
    pub fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        Self {
            engine: GhosttyTerminalScreenEngine::new(cols, rows, scrollback),
            pending_scroll_pixels: 0.0,
        }
    }

    pub fn process(&mut self, bytes: &[u8]) {
        self.engine.process(bytes);
    }

    pub fn replace_with_keyframe(&mut self, bytes: &[u8]) {
        self.clear();
        self.process(bytes);
        self.process(b"\x1b[3J");
        self.scroll_to_bottom();
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.pending_scroll_pixels = 0.0;
        self.engine.resize(cols, rows);
    }

    pub fn scroll_lines(&mut self, lines: i32) {
        if lines == 0 {
            return;
        }
        self.pending_scroll_pixels = 0.0;
        self.engine.scroll_lines(lines);
    }

    pub fn scroll_pixels(&mut self, pixels: f64, cell_height: f64) {
        if !pixels.is_finite() || pixels == 0.0 || !cell_height.is_finite() || cell_height <= 0.0 {
            return;
        }
        self.pending_scroll_pixels += pixels;
        let requested_lines = (self.pending_scroll_pixels / cell_height).trunc() as i32;
        if requested_lines != 0 {
            let previous_offset = self.engine.display_offset() as i32;
            self.engine.scroll_lines(requested_lines);
            let applied_lines = self.engine.display_offset() as i32 - previous_offset;
            self.pending_scroll_pixels -= applied_lines as f64 * cell_height;
            if applied_lines != requested_lines
                && ((requested_lines > 0 && self.pending_scroll_pixels > 0.0)
                    || (requested_lines < 0 && self.pending_scroll_pixels < 0.0))
            {
                self.pending_scroll_pixels = 0.0;
            }
        }
        if self.engine.display_offset() == 0 && self.pending_scroll_pixels < 0.0 {
            self.pending_scroll_pixels = 0.0;
        }
        if self.pending_scroll_pixels > 0.0 && !self.engine.has_history_above_viewport() {
            self.pending_scroll_pixels = 0.0;
        }
    }

    pub fn settle_pixel_scroll(&mut self) {
        // Pixel scrolling intentionally allows the viewport to stop between
        // terminal rows. Snapping here makes every drag look like a row-based
        // rebound; true bounds are already clamped in `scroll_pixels`.
    }

    pub fn scroll_to_bottom(&mut self) {
        self.pending_scroll_pixels = 0.0;
        self.engine.scroll_to_bottom();
    }

    pub fn display_offset(&self) -> usize {
        self.engine.display_offset()
    }

    pub fn clear(&mut self) {
        self.engine.clear();
        self.pending_scroll_pixels = 0.0;
    }

    pub fn snapshot(&self) -> TerminalScreenSnapshot {
        self.engine.snapshot(self.pending_scroll_pixels)
    }
}

struct GhosttyTerminalScreenEngine {
    tx: mpsc::Sender<GhosttyScreenCommand>,
}

impl GhosttyTerminalScreenEngine {
    fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("codux-ghostty-screen".to_string())
            .spawn(move || {
                GhosttyScreenWorker::new(cols, rows, scrollback).run(rx);
            })
            .expect("failed to spawn ghostty screen worker");
        Self { tx }
    }

    fn clear(&mut self) {
        self.send(GhosttyScreenCommand::Clear);
    }

    fn send(&self, command: GhosttyScreenCommand) {
        let _ = self.tx.send(command);
    }

    fn request<R: Default>(
        &self,
        build: impl FnOnce(mpsc::Sender<R>) -> GhosttyScreenCommand,
    ) -> R {
        let (tx, rx) = mpsc::channel();
        if self.tx.send(build(tx)).is_err() {
            return R::default();
        }
        rx.recv().unwrap_or_default()
    }

    fn process(&mut self, bytes: &[u8]) {
        if !bytes.is_empty() {
            self.send(GhosttyScreenCommand::Process(bytes.to_vec()));
        }
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        self.send(GhosttyScreenCommand::Resize { cols, rows });
    }

    fn scroll_lines(&mut self, lines: i32) {
        if lines != 0 {
            self.send(GhosttyScreenCommand::ScrollLines(lines));
        }
    }

    fn scroll_to_bottom(&mut self) {
        self.send(GhosttyScreenCommand::ScrollToBottom);
    }

    fn display_offset(&self) -> usize {
        self.request(GhosttyScreenCommand::DisplayOffset)
    }

    fn snapshot(&self, scroll_pixel_offset: f64) -> TerminalScreenSnapshot {
        self.request(|reply| GhosttyScreenCommand::Snapshot {
            scroll_pixel_offset,
            reply,
        })
    }

    fn has_history_above_viewport(&self) -> bool {
        self.request(GhosttyScreenCommand::HasHistoryAboveViewport)
    }
}

enum GhosttyScreenCommand {
    Process(Vec<u8>),
    Resize {
        cols: usize,
        rows: usize,
    },
    ScrollLines(i32),
    ScrollToBottom,
    DisplayOffset(mpsc::Sender<usize>),
    HasHistoryAboveViewport(mpsc::Sender<bool>),
    Snapshot {
        scroll_pixel_offset: f64,
        reply: mpsc::Sender<TerminalScreenSnapshot>,
    },
    Clear,
}

struct GhosttyScreenWorker {
    terminal: Terminal<'static, 'static>,
    render_state: RenderState<'static>,
    cols: usize,
    rows: usize,
    scrollback: usize,
}

impl GhosttyScreenWorker {
    fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        let cols = cols.max(1);
        let rows = rows.max(1);
        let terminal = Terminal::new(TerminalOptions {
            cols: cols.try_into().unwrap_or(u16::MAX),
            rows: rows.try_into().unwrap_or(u16::MAX),
            max_scrollback: scrollback,
        })
        .expect("failed to create ghostty terminal");
        let render_state = RenderState::new().expect("failed to create ghostty render state");
        Self {
            terminal,
            render_state,
            cols,
            rows,
            scrollback,
        }
    }

    fn run(mut self, rx: mpsc::Receiver<GhosttyScreenCommand>) {
        while let Ok(command) = rx.recv() {
            match command {
                GhosttyScreenCommand::Process(bytes) => self.terminal.vt_write(&bytes),
                GhosttyScreenCommand::Resize { cols, rows } => self.resize(cols, rows),
                GhosttyScreenCommand::ScrollLines(lines) => self.scroll_lines(lines),
                GhosttyScreenCommand::ScrollToBottom => {
                    self.terminal.scroll_viewport(ScrollViewport::Bottom);
                }
                GhosttyScreenCommand::DisplayOffset(reply) => {
                    let _ = reply.send(self.display_offset());
                }
                GhosttyScreenCommand::HasHistoryAboveViewport(reply) => {
                    let _ = reply.send(self.has_history_above_viewport());
                }
                GhosttyScreenCommand::Snapshot {
                    scroll_pixel_offset,
                    reply,
                } => {
                    let _ = reply.send(self.snapshot(scroll_pixel_offset));
                }
                GhosttyScreenCommand::Clear => {
                    self = Self::new(self.cols, self.rows, self.scrollback);
                }
            }
        }
    }

    fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(1);
        let rows = rows.max(1);
        if self.cols == cols && self.rows == rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        let _ = self.terminal.resize(
            cols.try_into().unwrap_or(u16::MAX),
            rows.try_into().unwrap_or(u16::MAX),
            GHOSTTY_CELL_WIDTH_PX,
            GHOSTTY_CELL_HEIGHT_PX,
        );
    }

    fn scroll_lines(&mut self, lines: i32) {
        if lines == 0 {
            return;
        }
        self.terminal
            .scroll_viewport(ScrollViewport::Delta(-(lines as isize)));
    }

    fn display_offset(&self) -> usize {
        self.terminal
            .scrollbar()
            .ok()
            .map(ghostty_display_offset)
            .unwrap_or(0)
    }

    fn has_history_above_viewport(&self) -> bool {
        self.display_offset() < self.terminal.scrollback_rows().unwrap_or(0)
    }

    fn snapshot(&mut self, scroll_pixel_offset: f64) -> TerminalScreenSnapshot {
        let terminal = &self.terminal;
        let scrollbar = terminal.scrollbar().ok();
        let display_offset = scrollbar.map(ghostty_display_offset).unwrap_or(0);
        let total_lines = scrollbar
            .map(|scrollbar| scrollbar.total as usize)
            .unwrap_or_else(|| terminal.total_rows().unwrap_or(self.rows))
            .max(self.rows);
        let snapshot = match self.render_state.update(terminal) {
            Ok(snapshot) => snapshot,
            Err(_) => {
                return TerminalScreenSnapshot {
                    cols: self.cols,
                    rows: self.rows,
                    total_lines: self.rows,
                    scroll_pixel_offset,
                    ..Default::default()
                };
            }
        };

        let cols = snapshot.cols().map(usize::from).unwrap_or(self.cols);
        let rows = snapshot.rows().map(usize::from).unwrap_or(self.rows);
        let cursor = ghostty_cursor_snapshot(&snapshot);
        let cells = ghostty_snapshot_cells(&snapshot, cols, rows);
        let data = terminal_snapshot_data(cols, rows, &cells, &cursor);
        TerminalScreenSnapshot {
            data,
            cols,
            rows,
            total_lines,
            display_offset,
            scroll_pixel_offset,
            application_cursor: terminal.mode(Mode::DECCKM).unwrap_or(false),
            input_mode: ghostty_input_mode(terminal),
            cells,
            cursor,
        }
    }
}

fn ghostty_input_mode(terminal: &Terminal<'_, '_>) -> TerminalInputMode {
    TerminalInputMode {
        application_cursor: terminal.mode(Mode::DECCKM).unwrap_or(false),
        alternate_screen: terminal.mode(Mode::ALT_SCREEN).unwrap_or(false)
            || terminal.mode(Mode::ALT_SCREEN_SAVE).unwrap_or(false)
            || terminal.mode(Mode::ALT_SCREEN_LEGACY).unwrap_or(false),
        alternate_scroll: terminal.mode(Mode::ALT_SCROLL).unwrap_or(false),
        bracketed_paste: terminal.mode(Mode::BRACKETED_PASTE).unwrap_or(false),
        focus_in_out: terminal.mode(Mode::FOCUS_EVENT).unwrap_or(false),
        mouse_tracking: terminal.is_mouse_tracking().unwrap_or(false),
        mouse_motion: terminal.mode(Mode::ANY_MOUSE).unwrap_or(false),
        mouse_drag: terminal.mode(Mode::BUTTON_MOUSE).unwrap_or(false),
        sgr_mouse: terminal.mode(Mode::SGR_MOUSE).unwrap_or(false),
        utf8_mouse: terminal.mode(Mode::UTF8_MOUSE).unwrap_or(false),
    }
}

fn ghostty_display_offset(scrollbar: libghostty_vt::ffi::GhosttyTerminalScrollbar) -> usize {
    scrollbar
        .total
        .saturating_sub(scrollbar.offset.saturating_add(scrollbar.len)) as usize
}

fn ghostty_snapshot_cells(
    snapshot: &Snapshot<'_, '_>,
    cols: usize,
    rows: usize,
) -> Vec<TerminalScreenCellSnapshot> {
    let mut cells = Vec::new();
    let mut row_iterator = match RowIterator::new() {
        Ok(iterator) => iterator,
        Err(_) => return cells,
    };
    let mut cell_iterator = match CellIterator::new() {
        Ok(iterator) => iterator,
        Err(_) => return cells,
    };
    let Ok(mut row_iteration) = row_iterator.update(snapshot) else {
        return cells;
    };

    let mut row_index = 0usize;
    while let Some(row) = row_iteration.next() {
        if row_index >= rows {
            break;
        }
        let Ok(mut cell_iteration) = cell_iterator.update(row) else {
            row_index += 1;
            continue;
        };
        for col in 0..cols {
            if cell_iteration
                .select(col.try_into().unwrap_or(u16::MAX))
                .is_err()
            {
                continue;
            }
            let Ok(raw_cell) = cell_iteration.raw_cell() else {
                continue;
            };
            let wide = raw_cell.wide().unwrap_or(CellWide::Narrow);
            if matches!(wide, CellWide::SpacerTail | CellWide::SpacerHead) {
                continue;
            }
            let text = match cell_iteration.graphemes() {
                Ok(graphemes) => graphemes
                    .into_iter()
                    .filter(|ch| *ch != '\0' && !ch.is_control())
                    .collect::<String>(),
                Err(_) => String::new(),
            };
            let style = cell_iteration.style().unwrap_or_default();
            let fg = style_color(style.fg_color);
            let bg = cell_background_color(raw_cell).unwrap_or_else(|| style_color(style.bg_color));
            if text.is_empty()
                && bg == TerminalScreenColor::Default
                && !ghostty_style_has_visuals(style)
            {
                continue;
            }
            cells.push(TerminalScreenCellSnapshot {
                row: row_index as i32,
                col,
                text,
                width: if wide == CellWide::Wide { 2 } else { 1 },
                fg,
                bg,
                bold: style.bold,
                dim: style.faint,
                italic: style.italic,
                underline: style.underline != Underline::None,
                inverse: style.inverse,
                hidden: style.invisible,
                strikeout: style.strikethrough,
            });
        }
        row_index += 1;
    }
    cells
}

fn ghostty_cursor_snapshot(snapshot: &Snapshot<'_, '_>) -> TerminalScreenCursorSnapshot {
    let viewport = snapshot.cursor_viewport().ok().flatten();
    let style = snapshot
        .cursor_visual_style()
        .unwrap_or(CursorVisualStyle::Block);
    TerminalScreenCursorSnapshot {
        row: viewport.map(|cursor| cursor.y as usize).unwrap_or(0),
        col: viewport.map(|cursor| cursor.x as usize).unwrap_or(0),
        visible: snapshot.cursor_visible().unwrap_or(false) && viewport.is_some(),
        shape: ghostty_cursor_shape(style),
    }
}

fn ghostty_cursor_shape(style: CursorVisualStyle) -> TerminalScreenCursorShape {
    match style {
        CursorVisualStyle::Bar => TerminalScreenCursorShape::Beam,
        CursorVisualStyle::Underline => TerminalScreenCursorShape::Underline,
        CursorVisualStyle::BlockHollow => TerminalScreenCursorShape::HollowBlock,
        CursorVisualStyle::Block => TerminalScreenCursorShape::Block,
        _ => TerminalScreenCursorShape::Block,
    }
}

fn ghostty_style_has_visuals(style: Style) -> bool {
    style.bold
        || style.italic
        || style.faint
        || style.blink
        || style.inverse
        || style.invisible
        || style.strikethrough
        || style.overline
        || style.underline != Underline::None
        || style_color(style.fg_color) != TerminalScreenColor::Default
        || style_color(style.bg_color) != TerminalScreenColor::Default
}

fn style_color(color: libghostty_vt::style::StyleColor) -> TerminalScreenColor {
    match color {
        libghostty_vt::style::StyleColor::None => TerminalScreenColor::Default,
        libghostty_vt::style::StyleColor::Palette(index) => {
            TerminalScreenColor::Indexed { index: index.0 }
        }
        libghostty_vt::style::StyleColor::Rgb(color) => color.into(),
    }
}

fn cell_background_color(cell: Cell) -> Option<TerminalScreenColor> {
    match cell.content_tag().ok()? {
        CellContentTag::BgColorPalette => cell
            .bg_color_palette()
            .ok()
            .map(|index| TerminalScreenColor::Indexed { index: index.0 }),
        CellContentTag::BgColorRgb => cell.bg_color_rgb().ok().map(TerminalScreenColor::from),
        CellContentTag::Codepoint | CellContentTag::CodepointGrapheme => None,
    }
}

impl From<RgbColor> for TerminalScreenColor {
    fn from(color: RgbColor) -> Self {
        Self::Rgb {
            r: color.r,
            g: color.g,
            b: color.b,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct SnapshotCellStyle {
    fg: TerminalScreenColor,
    bg: TerminalScreenColor,
    bold: bool,
    dim: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
    hidden: bool,
    strikeout: bool,
}

impl Default for SnapshotCellStyle {
    fn default() -> Self {
        Self {
            fg: TerminalScreenColor::Default,
            bg: TerminalScreenColor::Default,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            inverse: false,
            hidden: false,
            strikeout: false,
        }
    }
}

impl From<&TerminalScreenCellSnapshot> for SnapshotCellStyle {
    fn from(cell: &TerminalScreenCellSnapshot) -> Self {
        Self {
            fg: cell.fg.clone(),
            bg: cell.bg.clone(),
            bold: cell.bold,
            dim: cell.dim,
            italic: cell.italic,
            underline: cell.underline,
            inverse: cell.inverse,
            hidden: cell.hidden,
            strikeout: cell.strikeout,
        }
    }
}

#[derive(Clone)]
struct SnapshotScreenCell {
    text: String,
    width: usize,
    style: SnapshotCellStyle,
}

fn terminal_snapshot_data(
    cols: usize,
    rows: usize,
    cells: &[TerminalScreenCellSnapshot],
    cursor: &TerminalScreenCursorSnapshot,
) -> String {
    let mut rows_cells = vec![vec![None; cols]; rows];
    for cell in cells {
        if cell.row < 0 || cell.row as usize >= rows || cell.col >= cols {
            continue;
        }
        rows_cells[cell.row as usize][cell.col] = Some(SnapshotScreenCell {
            text: cell.text.clone(),
            width: cell.width,
            style: SnapshotCellStyle::from(cell),
        });
    }

    let mut output = String::new();
    output.push_str("\x1b[?25l\x1b[0m\x1b[H\x1b[2J");
    let mut current_style = SnapshotCellStyle::default();
    for (row_index, row_cells) in rows_cells.iter().enumerate() {
        let Some(last_col) = row_cells.iter().rposition(|cell| {
            cell.as_ref()
                .map(|cell| {
                    !cell.text.trim().is_empty() || cell.style != SnapshotCellStyle::default()
                })
                .unwrap_or(false)
        }) else {
            continue;
        };
        output.push_str(&format!("\x1b[{};1H", row_index + 1));
        let mut col = 0;
        while col <= last_col {
            match &row_cells[col] {
                Some(cell) => {
                    if cell.style != current_style {
                        output.push_str(&snapshot_style_sgr(cell.style.clone()));
                        current_style = cell.style.clone();
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
    if current_style != SnapshotCellStyle::default() {
        output.push_str("\x1b[0m");
    }
    if cursor.visible {
        output.push_str(&format!("\x1b[{};{}H", cursor.row + 1, cursor.col + 1));
        output.push_str("\x1b[?25h");
    }
    output
}

fn terminal_snapshot_text(text: &str) -> String {
    text.chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect()
}

fn snapshot_style_sgr(style: SnapshotCellStyle) -> String {
    let mut codes = vec!["0".to_string()];
    if style.bold {
        codes.push("1".to_string());
    }
    if style.dim {
        codes.push("2".to_string());
    }
    if style.italic {
        codes.push("3".to_string());
    }
    if style.underline {
        codes.push("4".to_string());
    }
    if style.inverse {
        codes.push("7".to_string());
    }
    if style.hidden {
        codes.push("8".to_string());
    }
    if style.strikeout {
        codes.push("9".to_string());
    }
    snapshot_color_sgr(&style.fg, false, &mut codes);
    snapshot_color_sgr(&style.bg, true, &mut codes);
    format!("\x1b[{}m", codes.join(";"))
}

fn snapshot_color_sgr(color: &TerminalScreenColor, background: bool, codes: &mut Vec<String>) {
    match color {
        TerminalScreenColor::Default | TerminalScreenColor::Named { .. } => {
            codes.push(if background { "49" } else { "39" }.to_string());
        }
        TerminalScreenColor::Rgb { r, g, b } => {
            codes.push(if background { "48" } else { "38" }.to_string());
            codes.push("2".to_string());
            codes.push(r.to_string());
            codes.push(g.to_string());
            codes.push(b.to_string());
        }
        TerminalScreenColor::Indexed { index } => {
            codes.push(if background { "48" } else { "38" }.to_string());
            codes.push("5".to_string());
            codes.push(index.to_string());
        }
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
    fn plain_cells_keep_default_colors_for_app_theme_resolution() {
        let mut screen = HeadlessTerminalScreen::new(20, 4, 100);
        screen.process(b"theme");

        let snapshot = screen.snapshot();
        let cell = snapshot
            .cells
            .iter()
            .find(|cell| cell.text == "t")
            .expect("plain cell");

        assert_eq!(cell.fg, TerminalScreenColor::Default);
        assert_eq!(cell.bg, TerminalScreenColor::Default);
    }

    #[test]
    fn sgr_colors_remain_semantic_until_ui_palette_resolution() {
        let mut screen = HeadlessTerminalScreen::new(20, 4, 100);
        screen.process(b"\x1b[31mred\x1b[0m \x1b[48;5;4mblue-bg");

        let snapshot = screen.snapshot();
        let red = snapshot
            .cells
            .iter()
            .find(|cell| cell.text == "r")
            .expect("red cell");
        let blue_bg = snapshot
            .cells
            .iter()
            .find(|cell| cell.text == "b")
            .expect("blue bg cell");

        assert_eq!(red.fg, TerminalScreenColor::Indexed { index: 1 });
        assert_eq!(red.bg, TerminalScreenColor::Default);
        assert_eq!(blue_bg.bg, TerminalScreenColor::Indexed { index: 4 });
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
        assert!(scrolled.total_lines >= scrolled.rows + scrolled.display_offset);
        assert!(
            scrolled
                .cells
                .iter()
                .any(|cell| cell.row == 0 && !cell.text.trim().is_empty())
        );
        assert!(scrolled.cells.iter().all(|cell| cell.row >= 0));
        assert!(
            scrolled
                .cells
                .iter()
                .all(|cell| (cell.row as usize) < scrolled.rows)
        );

        screen.scroll_to_bottom();
        assert_eq!(screen.snapshot().display_offset, 0);
    }

    #[test]
    fn keyframe_replaces_previous_screen_and_scrollback() {
        let mut screen = HeadlessTerminalScreen::new(20, 4, 100);
        screen.process(b"old one\r\nold two\r\nold three\r\nold four\r\nold five");

        screen.replace_with_keyframe(b"\x1b[2J\x1b[Hnew one\r\n\x1b[3;1Hnew input");

        let current = screen.snapshot();
        assert!(current.data.contains("new one"));
        assert!(current.data.contains("new input"));
        assert!(!current.data.contains("old one"));
        assert_eq!(current.display_offset, 0);

        screen.scroll_lines(8);
        let scrolled = screen.snapshot();
        assert_eq!(scrolled.display_offset, 0);
        assert!(!scrolled.data.contains("old one"));
    }

    #[test]
    fn hides_cursor_when_current_input_row_is_outside_viewport() {
        let mut screen = HeadlessTerminalScreen::new(20, 4, 100);
        screen.process(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix\r\nseven");
        let bottom = screen.snapshot();
        assert!(bottom.cursor.visible);

        screen.scroll_lines(2);
        let scrolled = screen.snapshot();

        assert_eq!(scrolled.display_offset, 2);
        assert!(!scrolled.cursor.visible);
    }

    #[test]
    fn pixel_scroll_keeps_fractional_offset_without_synthetic_rows() {
        let mut screen = HeadlessTerminalScreen::new(20, 4, 100);
        screen.process(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix");

        screen.scroll_pixels(7.0, 10.0);
        let partial = screen.snapshot();
        assert_eq!(partial.display_offset, 0);
        assert_eq!(partial.scroll_pixel_offset, 7.0);

        screen.scroll_pixels(6.0, 10.0);
        let scrolled = screen.snapshot();
        assert!(scrolled.display_offset > 0);
        assert_eq!(scrolled.scroll_pixel_offset, 3.0);
        assert!(
            scrolled
                .cells
                .iter()
                .all(|cell| cell.row >= 0 && (cell.row as usize) < scrolled.rows)
        );

        screen.settle_pixel_scroll();
        assert_eq!(screen.snapshot().scroll_pixel_offset, 3.0);
    }

    #[test]
    fn keeps_requested_small_viewport_size() {
        let mut screen = HeadlessTerminalScreen::new(5, 3, 100);
        screen.process(b"small");

        let snapshot = screen.snapshot();

        assert_eq!(snapshot.cols, 5);
        assert_eq!(snapshot.rows, 3);
    }
}
