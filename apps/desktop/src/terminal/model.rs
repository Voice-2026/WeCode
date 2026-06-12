struct TerminalModel {
    handle: TerminalStateHandle,
    stdin_writer: Arc<Mutex<Box<dyn Write + Send>>>,
    session_event_rx: mpsc::Receiver<TerminalUiEvent>,
    events: VecDeque<TerminalInternalEvent>,
    pending_output_bytes: Vec<u8>,
    output_flush_pending: bool,
    snapshot_dirty: bool,
    sync_output_depth: usize,
    sync_output_pending_notify: bool,
    sync_output_scan_tail: Vec<u8>,
    color_scheme_state: TerminalColorSchemeState,
    title: Option<String>,
    exited: bool,
    focused: bool,
    colors: ColorPalette,
    paste_images_as_paths: bool,
    window_size: TerminalWindowSize,
    selection: SelectionState,
    #[cfg(test)]
    written_bytes: Option<Arc<Mutex<Vec<u8>>>>,
    _reader_task: Task<()>,
}

#[derive(Clone)]
struct TerminalStateHandle {
    screen: Arc<Mutex<HeadlessTerminalScreen>>,
    snapshot: Arc<Mutex<TerminalContent>>,
}

#[derive(Clone, Copy, Debug)]
enum TerminalInternalEvent {
    Resize { cols: usize, rows: usize },
    Scroll { lines: i32 },
}

impl TerminalModel {
    fn new<W>(
        stdin_writer: W,
        bytes_rx: flume::Receiver<Vec<u8>>,
        session_event_rx: mpsc::Receiver<TerminalUiEvent>,
        config: &TerminalConfig,
        cx: &mut Context<Self>,
    ) -> Self
    where
        W: Write + Send + 'static,
    {
        let screen = Arc::new(Mutex::new(HeadlessTerminalScreen::new(
            config.cols,
            config.rows,
            config.scrollback,
        )));
        let snapshot = TerminalContent::from_screen_snapshot(screen.lock().snapshot());
        let reader_task = cx.spawn(async move |this: WeakEntity<Self>, cx| {
            while let Ok(bytes) = bytes_rx.recv_async().await {
                if this
                    .update(cx, |model, cx| model.receive_output(bytes, cx))
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            handle: TerminalStateHandle {
                screen,
                snapshot: Arc::new(Mutex::new(snapshot)),
            },
            stdin_writer: Arc::new(Mutex::new(Box::new(stdin_writer) as Box<dyn Write + Send>)),
            session_event_rx,
            events: VecDeque::new(),
            pending_output_bytes: Vec::new(),
            output_flush_pending: false,
            snapshot_dirty: false,
            sync_output_depth: 0,
            sync_output_pending_notify: false,
            sync_output_scan_tail: Vec::new(),
            color_scheme_state: TerminalColorSchemeState::default(),
            title: None,
            exited: false,
            focused: false,
            colors: config.colors.clone(),
            paste_images_as_paths: config.paste_images_as_paths,
            window_size: TerminalWindowSize {
                num_lines: config.rows as u16,
                num_cols: config.cols as u16,
                cell_width: 1,
                cell_height: 1,
            },
            selection: SelectionState::default(),
            #[cfg(test)]
            written_bytes: None,
            _reader_task: reader_task,
        }
    }

    fn receive_output(&mut self, bytes: Vec<u8>, cx: &mut Context<Self>) {
        if self.output_flush_pending {
            self.pending_output_bytes.extend(bytes);
            return;
        }

        self.output_flush_pending = true;
        self.process_output_bytes(&bytes, cx);
        self.schedule_pending_output_flush(cx);
    }

    fn schedule_pending_output_flush(&mut self, cx: &mut Context<Self>) {
        let timer = cx.background_executor().clone();
        cx.spawn(async move |model: WeakEntity<Self>, cx| {
            timer.timer(TERMINAL_OUTPUT_FRAME_INTERVAL).await;
            let _ = model.update(cx, |model, cx| {
                model.output_flush_pending = false;
                model.flush_output(cx);
            });
        })
        .detach();
    }

    fn flush_output(&mut self, cx: &mut Context<Self>) {
        let bytes = std::mem::take(&mut self.pending_output_bytes);
        if bytes.is_empty() {
            if self.process_pending_events(cx) {
                cx.notify();
            }
            return;
        }

        self.process_output_bytes(&bytes, cx);
    }

    fn process_output_bytes(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        let before_display_offset = self.handle.display_offset();
        let sync_update = self.update_synchronized_output_state(bytes);
        let color_scheme_update =
            update_terminal_color_scheme_state(bytes, &mut self.color_scheme_state);
        self.respond_to_color_scheme_queries(color_scheme_update.query_count);
        self.process_bytes(bytes);
        trace_terminal_protocol_bytes(
            bytes,
            sync_update,
            self.sync_output_depth,
            color_scheme_update,
            self.color_scheme_state.updates_enabled,
        );
        self.trace_terminal_state_after_output(bytes.len());
        let event_should_notify = self.process_pending_events(cx);

        if self.sync_output_depth > 0 {
            self.sync_output_pending_notify = true;
            return;
        }

        if sync_update.should_notify || event_should_notify || self.sync_output_pending_notify {
            self.sync_output_pending_notify = false;
        }
        let after_display_offset = self.handle.display_offset();
        if after_display_offset != before_display_offset {
            self.snapshot_dirty = true;
        }
        cx.notify();
    }

    fn update_synchronized_output_state(&mut self, bytes: &[u8]) -> SyncOutputUpdate {
        update_synchronized_output_state(
            bytes,
            &mut self.sync_output_depth,
            &mut self.sync_output_scan_tail,
        )
    }

    fn trace_terminal_state_after_output(&self, bytes_len: usize) {
        if !terminal_trace_enabled() {
            return;
        }
        let content = self.live_snapshot();
        terminal_trace(&format!(
            "state bytes={} sync_depth={} cursor_visible={} cursor_row={} cursor_col={} cursor_shape={:?} display_offset={}",
            bytes_len,
            self.sync_output_depth,
            content.cursor.visible,
            content.cursor.row,
            content.cursor.col,
            content.cursor.shape,
            content.display_offset,
        ));
    }

    fn process_pending_events(&mut self, _cx: &mut Context<Self>) -> bool {
        let mut should_notify = false;
        while let Ok(event) = self.session_event_rx.try_recv() {
            match event {
                TerminalUiEvent::Wakeup => {
                    if !self.output_flush_pending {
                        should_notify = true;
                    }
                }
                TerminalUiEvent::Viewport { cols, rows } => {
                    self.resize(
                        cols as usize,
                        rows as usize,
                        TerminalWindowSize {
                            num_lines: rows,
                            num_cols: cols,
                            cell_width: self.window_size.cell_width,
                            cell_height: self.window_size.cell_height,
                        },
                    );
                    should_notify = true;
                }
                TerminalUiEvent::Exit => {
                    self.exited = true;
                    should_notify = true;
                }
                TerminalUiEvent::Error(message) => {
                    self.title = Some(format!("Terminal error: {message}"));
                    should_notify = true;
                }
            }
        }
        should_notify
    }

    fn process_bytes(&mut self, bytes: &[u8]) {
        let before_selection = self.selection_range().and_then(|range| {
            let text = self.handle.selected_text_for_range(range);
            (!text.is_empty()).then_some((range, text))
        });
        self.handle.screen.lock().process(bytes);
        if let Some((range, text)) = before_selection {
            let content = self.live_snapshot();
            if selected_text_from_content(&content, range) != text
                && let Some(next_range) = find_selection_text_range(&content, &text)
            {
                self.selection.set_range(next_range);
            }
        }
        self.snapshot_dirty = true;
    }

    fn update_colors(&mut self, colors: ColorPalette) {
        let was_dark = self.colors.is_dark();
        let is_dark = colors.is_dark();
        self.colors = colors;
        if self.color_scheme_state.updates_enabled && was_dark != is_dark {
            self.write_color_scheme_report();
        }
    }

    fn update_config(&mut self, colors: ColorPalette, paste_images_as_paths: bool) {
        self.paste_images_as_paths = paste_images_as_paths;
        self.update_colors(colors);
    }

    fn respond_to_color_scheme_queries(&self, query_count: usize) {
        for _ in 0..query_count {
            self.write_color_scheme_report();
        }
    }

    fn write_color_scheme_report(&self) {
        self.write_bytes(terminal_color_scheme_report(&self.colors));
    }

    fn sync(&mut self, cx: &mut Context<Self>) -> TerminalContent {
        self.process_pending_events(cx);
        self.sync_model_events()
    }

    fn sync_model_events(&mut self) -> TerminalContent {
        let mut snapshot_dirty = self.snapshot_dirty;
        while let Some(event) = self.events.pop_front() {
            match event {
                TerminalInternalEvent::Resize { cols, rows } => {
                    snapshot_dirty |= self.handle.resize(cols, rows);
                }
                TerminalInternalEvent::Scroll { lines } => {
                    snapshot_dirty |= self.handle.scroll_display(lines);
                }
            }
        }
        if snapshot_dirty {
            self.handle.publish_snapshot();
            self.snapshot_dirty = false;
        }
        self.handle.snapshot()
    }

    fn prepare_input_viewport(&mut self, cx: &mut Context<Self>) {
        if self.prepare_input_viewport_snapshot() {
            self.handle.publish_snapshot();
            self.snapshot_dirty = false;
            cx.notify();
        }
    }

    #[cfg(test)]
    fn prepare_input_viewport_for_test(&mut self) {
        if self.prepare_input_viewport_snapshot() {
            self.handle.publish_snapshot();
            self.snapshot_dirty = false;
        }
    }

    fn prepare_input_viewport_snapshot(&mut self) -> bool {
        let mut snapshot_dirty = self.snapshot_dirty;
        let events = std::mem::take(&mut self.events);
        for event in events {
            match event {
                TerminalInternalEvent::Resize { cols, rows } => {
                    snapshot_dirty |= self.handle.resize(cols, rows);
                }
                TerminalInternalEvent::Scroll { .. } => {}
            }
        }
        snapshot_dirty | self.handle.scroll_to_bottom()
    }

    #[cfg(test)]
    fn sync_for_test(&mut self) -> TerminalContent {
        self.sync_model_events()
    }

    fn live_snapshot(&self) -> TerminalContent {
        TerminalContent::from_screen_snapshot(self.handle.screen.lock().snapshot())
    }

    fn mode(&self) -> TerminalInputMode {
        self.handle.mode()
    }

    fn display_offset(&self) -> usize {
        self.handle.display_offset()
    }

    fn snapshot(&self) -> TerminalContent {
        self.handle.snapshot()
    }

    fn current_ime_cursor_bounds(&self, layout: &TerminalLayoutMetrics) -> Option<Bounds<Pixels>> {
        let content = self.handle.snapshot();
        ime_cursor_bounds_from_content(&content, layout)
    }

    fn dimensions(&self) -> (usize, usize) {
        self.handle.dimensions()
    }

    fn scroll_display(&mut self, lines: i32) -> bool {
        self.events
            .push_back(TerminalInternalEvent::Scroll { lines });
        true
    }

    fn start_selection(&mut self, point: TerminalSelectionPoint) {
        self.selection.start(point);
    }

    fn update_selection(&mut self, point: TerminalSelectionPoint) {
        if self.selection.dragging || self.selection.anchor.is_some() {
            self.selection.update(point);
        }
    }

    fn clear_selection(&mut self) {
        self.selection.clear();
    }

    fn selected_text(&self) -> Option<String> {
        self.selection_range()
            .map(|range| self.handle.selected_text_for_range(range))
    }

    fn selection_range(&self) -> Option<SelectionRange> {
        self.selection.range()
    }

    fn resize(&mut self, cols: usize, rows: usize, window_size: TerminalWindowSize) {
        self.window_size = window_size;
        if self.dimensions() == (cols, rows) {
            return;
        }
        match self.events.back_mut() {
            Some(TerminalInternalEvent::Resize { cols: c, rows: r }) => {
                *c = cols;
                *r = rows;
            }
            _ => self
                .events
                .push_back(TerminalInternalEvent::Resize { cols, rows }),
        }
    }

    fn write_bytes(&self, bytes: &[u8]) {
        let mut writer = self.stdin_writer.lock();
        let _ = writer.write_all(bytes);
        let _ = writer.flush();
    }

    #[cfg(test)]
    fn written_bytes_for_test(&self) -> Vec<u8> {
        self.written_bytes
            .as_ref()
            .map(|bytes| bytes.lock().clone())
            .unwrap_or_default()
    }

    fn paste_text(&self, text: &str) {
        self.write_bytes(&codux_terminal_core::terminal_paste_input_bytes(
            text,
            self.mode().bracketed_paste,
        ));
    }

    fn report_focus_change(&self, focused: bool) {
        if !self.mode().focus_in_out {
            return;
        }
        self.write_bytes(if focused { b"\x1b[I" } else { b"\x1b[O" });
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    #[cfg(test)]
    fn new_for_test(cols: usize, rows: usize, scrollback: usize) -> Self {
        let (_session_event_tx, session_event_rx) = mpsc::channel();
        let written_bytes = Arc::new(Mutex::new(Vec::new()));
        let screen = Arc::new(Mutex::new(HeadlessTerminalScreen::new(
            cols, rows, scrollback,
        )));
        let snapshot = TerminalContent::from_screen_snapshot(screen.lock().snapshot());
        Self {
            handle: TerminalStateHandle {
                screen,
                snapshot: Arc::new(Mutex::new(snapshot)),
            },
            stdin_writer: Arc::new(Mutex::new(Box::new(TestTerminalWriter {
                bytes: written_bytes.clone(),
            }) as Box<dyn Write + Send>)),
            session_event_rx,
            events: VecDeque::new(),
            pending_output_bytes: Vec::new(),
            output_flush_pending: false,
            snapshot_dirty: false,
            sync_output_depth: 0,
            sync_output_pending_notify: false,
            sync_output_scan_tail: Vec::new(),
            color_scheme_state: TerminalColorSchemeState::default(),
            title: None,
            exited: false,
            focused: false,
            colors: ColorPalette::default(),
            paste_images_as_paths: true,
            window_size: TerminalWindowSize {
                num_lines: rows as u16,
                num_cols: cols as u16,
                cell_width: 1,
                cell_height: 1,
            },
            selection: SelectionState::default(),
            written_bytes: Some(written_bytes),
            _reader_task: Task::ready(()),
        }
    }
}

#[cfg(test)]
struct TestTerminalWriter {
    bytes: Arc<Mutex<Vec<u8>>>,
}

#[cfg(test)]
impl Write for TestTerminalWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.bytes.lock().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl TerminalStateHandle {
    fn mode(&self) -> TerminalInputMode {
        self.screen.lock().snapshot().input_mode
    }

    fn display_offset(&self) -> usize {
        self.screen.lock().display_offset()
    }

    fn dimensions(&self) -> (usize, usize) {
        let snapshot = self.screen.lock().snapshot();
        (snapshot.cols, snapshot.rows)
    }

    fn snapshot(&self) -> TerminalContent {
        self.snapshot.lock().clone()
    }

    fn publish_snapshot(&self) {
        let snapshot = self.screen.lock().snapshot();
        *self.snapshot.lock() = TerminalContent::from_screen_snapshot(snapshot);
    }

    fn resize(&self, cols: usize, rows: usize) -> bool {
        let (current_cols, current_rows) = self.dimensions();
        if cols == current_cols && rows == current_rows {
            return false;
        }
        self.screen.lock().resize(cols, rows);
        true
    }

    fn scroll_display(&self, lines: i32) -> bool {
        let before = self.display_offset();
        self.screen.lock().scroll_lines(lines);
        self.display_offset() != before
    }

    fn scroll_to_bottom(&self) -> bool {
        let before = self.display_offset();
        self.screen.lock().scroll_to_bottom();
        self.display_offset() != before
    }

    fn selected_text_for_range(&self, range: SelectionRange) -> String {
        let content = self.snapshot();
        selected_text_from_content(&content, range)
    }
}

fn selected_text_from_content(content: &TerminalContent, range: SelectionRange) -> String {
    let mut lines = Vec::new();
    for line in range.start.line..=range.end.line {
        let start_col = if line == range.start.line {
            range.start.col
        } else {
            0
        };
        let end_col = if line == range.end.line {
            range.end.col
        } else {
            content.columns
        };
        lines.push(selected_line_text(content, line, start_col, end_col));
    }
    lines.join("\n")
}

fn selected_line_text(
    content: &TerminalContent,
    line: i32,
    start_col: usize,
    end_col: usize,
) -> String {
    let row_cells = content
        .cells
        .iter()
        .filter(|cell| cell.line() == line)
        .collect::<Vec<_>>();
    let row_text = terminal_row_text(&row_cells);
    row_text
        .into_iter()
        .filter(|(col, _)| *col >= start_col && *col < end_col)
        .map(|(_, ch)| ch)
        .collect()
}

fn find_selection_text_range(content: &TerminalContent, selected_text: &str) -> Option<SelectionRange> {
    let mut lines = selected_text.split('\n');
    let first_line = lines.next()?;
    if lines.next().is_some() || first_line.is_empty() {
        return None;
    }

    for line in content.line_for_display_row(0)
        ..=content.line_for_display_row(content.visible_rows().saturating_sub(1))
    {
        let row_cells = content
            .cells
            .iter()
            .filter(|cell| cell.line() == line)
            .collect::<Vec<_>>();
        let row_text = terminal_row_text(&row_cells);
        let chars = row_text.iter().map(|(_, ch)| *ch).collect::<String>();
        if let Some(byte_start) = chars.find(first_line) {
            let start_char = chars[..byte_start].chars().count();
            let end_char = start_char + first_line.chars().count();
            let start_col = row_text.get(start_char).map(|(col, _)| *col)?;
            let end_col = row_text
                .get(end_char.saturating_sub(1))
                .map(|(col, ch)| col.saturating_add(terminal_char_width(*ch)))?;
            return Some(SelectionRange {
                start: TerminalSelectionPoint {
                    line,
                    col: start_col,
                },
                end: TerminalSelectionPoint { line, col: end_col },
            });
        }
    }
    None
}
