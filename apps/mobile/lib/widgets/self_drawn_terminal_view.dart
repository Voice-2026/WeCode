import 'dart:io';
import 'dart:ui' as ui;

import 'package:codux_protocol_ffi/codux_protocol_ffi.dart';
import 'package:flutter/material.dart';
import 'package:flutter/physics.dart';

import '../services/remote_terminal_output_controller.dart';
import '../theme/app_theme.dart';
import '../theme/terminal_theme.dart';
import 'native_terminal_view.dart' show NativeTerminalCursorMetrics;

/// Feature flag: render the terminal with the self-drawn Flutter renderer
/// (single source of truth = the Rust cell grid) instead of the native
/// SwiftTerm / Termux emulator. Off by default until paint performance and the
/// input/scroll/selection layers are validated on device.
const bool kUseSelfDrawnTerminal = true;

/// Self-drawn terminal that renders the shared Rust core's cell grid directly,
/// instead of feeding an ANSI byte stream to a native emulator (SwiftTerm /
/// Termux). The Rust `HeadlessTerminalScreen` is the single source of truth —
/// the same snapshot the GPUI desktop draws from — so there is no second VT
/// parser, no ANSI replay, and no scrollback reconstruction to drift.
class SelfDrawnTerminalView extends StatefulWidget {
  const SelfDrawnTerminalView({
    super.key,
    required this.sessionId,
    required this.controller,
    required this.repaintSignal,
    required this.fontSize,
    this.onResize,
    this.onInput,
    this.onSendKey,
    this.onCursorMetrics,
    this.keyboardRequested = false,
    this.keyboardRequestSerial = 0,
  });

  final String? sessionId;
  final RemoteTerminalOutputController controller;

  /// Fires whenever terminal output for the active session changes; the view
  /// re-reads the snapshot (gated by render generation) and repaints.
  final Listenable repaintSignal;
  final double fontSize;
  final void Function(int cols, int rows)? onResize;

  /// Raw typed text (batched by the host send path, same as the native view).
  final ValueChanged<String>? onInput;

  /// Pre-encoded key bytes (enter, backspace, ...), sent immediately.
  final ValueChanged<String>? onSendKey;
  final ValueChanged<NativeTerminalCursorMetrics?>? onCursorMetrics;
  final bool keyboardRequested;
  final int keyboardRequestSerial;

  @override
  State<SelfDrawnTerminalView> createState() => _SelfDrawnTerminalViewState();
}

class _SelfDrawnTerminalViewState extends State<SelfDrawnTerminalView>
    with SingleTickerProviderStateMixin {
  static const double _lineHeightMultiplier = 1.3;
  // Zero-width space anchor in the hidden input (kept invisible and harmless if
  // ever emitted), used to detect inserts vs a backspace on an empty field.
  static final String _sentinel = String.fromCharCode(0x200b);
  static final String _fontFamily = Platform.isIOS ? 'Menlo' : 'monospace';

  // Per-cell paragraph cache, keyed by (text, color, style). Terminal content
  // is highly repetitive, so this turns per-cell layout into a cache hit after
  // warmup while keeping every glyph grid-aligned at its own column.
  final Map<String, ui.Paragraph> _glyphCache = {};

  // Hidden anchor input that captures the soft keyboard / IME. The sentinel
  // zero-width space lets us detect both inserted text and a backspace that
  // would otherwise leave the field empty.
  final TextEditingController _inputController = TextEditingController();
  final FocusNode _focusNode = FocusNode();
  bool _resetting = false;

  TerminalScreenSnapshot? _snapshot;
  int _appliedGen = -1;
  double _cellWidth = 0;
  double _cellHeight = 0;
  double _glyphTop = 0;
  int _cols = 0;
  int _rows = 0;
  NativeTerminalCursorMetrics? _lastCursorMetrics;

  // Momentum (fling) scrolling: a friction simulation drives scroll-pixel
  // deltas after the finger lifts, decelerating to a stop.
  late final AnimationController _fling = AnimationController.unbounded(
    vsync: this,
  );
  double _flingLast = 0;

  @override
  void initState() {
    super.initState();
    _measureCell();
    _resetInput();
    _inputController.addListener(_handleInputChange);
    _fling.addListener(_onFlingTick);
    widget.repaintSignal.addListener(_onSignal);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _refresh(force: true);
      _applyKeyboard();
    });
  }

  @override
  void didUpdateWidget(covariant SelfDrawnTerminalView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.repaintSignal != oldWidget.repaintSignal) {
      oldWidget.repaintSignal.removeListener(_onSignal);
      widget.repaintSignal.addListener(_onSignal);
    }
    if (widget.fontSize != oldWidget.fontSize) {
      _glyphCache.clear();
      _measureCell();
      _cols = 0;
      _rows = 0;
      _scheduleRefresh();
    }
    if (widget.sessionId != oldWidget.sessionId) {
      _appliedGen = -1;
      _lastCursorMetrics = null;
      // Force a resize for the newly-active session: the viewport pixel size is
      // unchanged across a session switch, so _syncGrid would otherwise not
      // re-fire, and the new session's host PTY would never be told the mobile
      // size (repaint apps then paint at the host's old row count, leaving the
      // bottom blank).
      _cols = 0;
      _rows = 0;
      _scheduleRefresh();
    }
    if (widget.keyboardRequestSerial != oldWidget.keyboardRequestSerial ||
        widget.keyboardRequested != oldWidget.keyboardRequested) {
      _applyKeyboard();
    }
  }

  @override
  void dispose() {
    widget.repaintSignal.removeListener(_onSignal);
    _inputController.removeListener(_handleInputChange);
    _inputController.dispose();
    _focusNode.dispose();
    _fling.dispose();
    super.dispose();
  }

  void _onSignal() => _refresh();

  /// Refresh after the current frame. Used from `didUpdateWidget` so the
  /// snapshot read and its `onCursorMetrics` callback never call `setState`
  /// on an ancestor while the tree is still building.
  void _scheduleRefresh({bool force = true}) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _refresh(force: force);
    });
  }

  void _applyKeyboard() {
    if (!mounted) return;
    if (widget.keyboardRequested) {
      _focusNode.requestFocus();
    } else if (_focusNode.hasFocus) {
      _focusNode.unfocus();
    }
  }

  void _measureCell() {
    final painter = TextPainter(
      text: TextSpan(
        text: 'M',
        style: TextStyle(fontFamily: _fontFamily, fontSize: widget.fontSize),
      ),
      textDirection: TextDirection.ltr,
    )..layout();
    _cellWidth = painter.width;
    _cellHeight = widget.fontSize * _lineHeightMultiplier;
    _glyphTop = ((_cellHeight - painter.height) / 2).clamp(0.0, _cellHeight);
  }

  // ---- input ---------------------------------------------------------------

  void _resetInput() {
    _resetting = true;
    _inputController.value = TextEditingValue(
      text: _sentinel,
      selection: const TextSelection.collapsed(offset: 1),
    );
    _resetting = false;
  }

  void _handleInputChange() {
    if (_resetting) return;
    final value = _inputController.value;
    // Wait for the IME to commit before emitting composing text.
    if (value.composing.isValid && !value.composing.isCollapsed) return;
    final text = value.text;
    if (text == _sentinel) return;
    if (text.isEmpty) {
      _sendKey('backspace');
    } else {
      final inserted = text.startsWith(_sentinel)
          ? text.substring(_sentinel.length)
          : text.replaceFirst(_sentinel, '');
      if (inserted.isNotEmpty) _sendText(inserted);
    }
    _resetInput();
  }

  void _sendText(String text) {
    // Newlines map to the Enter key (CR); other text is sent raw through the
    // same batched path the native view used.
    final parts = text.split('\n');
    for (var i = 0; i < parts.length; i++) {
      if (parts[i].isNotEmpty) widget.onInput?.call(parts[i]);
      if (i < parts.length - 1) _sendKey('enter');
    }
  }

  void _sendKey(String key) {
    final bytes = terminalKeyInput(
      key: key,
      applicationCursor: _snapshot?.applicationCursor ?? false,
    );
    if (bytes.isNotEmpty) widget.onSendKey?.call(bytes);
  }

  // ---- snapshot / grid -----------------------------------------------------

  void _refresh({bool force = false}) {
    final sessionId = widget.sessionId;
    if (sessionId == null) {
      if (_snapshot != null) setState(() => _snapshot = null);
      return;
    }
    final gen = widget.controller.renderGeneration(sessionId);
    if (!force && gen == _appliedGen) return;
    final snapshot = widget.controller.screenSnapshot(sessionId);
    _appliedGen = gen;
    if (mounted) setState(() => _snapshot = snapshot);
    _emitCursorMetrics();
  }

  void _emitCursorMetrics() {
    final callback = widget.onCursorMetrics;
    final snapshot = _snapshot;
    if (callback == null || snapshot == null || _cellHeight <= 0) return;
    final metrics = NativeTerminalCursorMetrics(
      row: snapshot.cursor.row,
      col: snapshot.cursor.col,
      lineHeight: _cellHeight,
    );
    if (metrics == _lastCursorMetrics) return;
    _lastCursorMetrics = metrics;
    callback(metrics);
  }

  void _syncGrid(BoxConstraints constraints) {
    final sessionId = widget.sessionId;
    if (sessionId == null || _cellWidth <= 0 || _cellHeight <= 0) return;
    final cols = (constraints.maxWidth / _cellWidth).floor().clamp(1, 1000);
    final rows = (constraints.maxHeight / _cellHeight).floor().clamp(1, 1000);
    if (cols == _cols && rows == _rows) return;
    _cols = cols;
    _rows = rows;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || widget.sessionId != sessionId) return;
      widget.controller.resizeScreen(sessionId, cols: cols, rows: rows);
      widget.onResize?.call(cols, rows);
      _refresh(force: true);
    });
  }

  // ---- scroll --------------------------------------------------------------

  void _scrollBy(double pixels) {
    final sessionId = widget.sessionId;
    if (sessionId == null || _cellHeight <= 0 || pixels == 0) return;
    widget.controller.scrollScreenPixels(
      sessionId,
      pixels: pixels,
      cellHeight: _cellHeight,
    );
    _refresh(force: true);
  }

  void _onDragStart(DragStartDetails details) => _fling.stop();

  void _onDragUpdate(DragUpdateDetails details) => _scrollBy(details.delta.dy);

  void _onDragEnd(DragEndDetails details) {
    final velocity = details.velocity.pixelsPerSecond.dy;
    if (_cellHeight <= 0 || velocity.abs() < 80) {
      _settleScroll();
      return;
    }
    // Decelerating momentum scroll: the friction sim's position is the running
    // scroll offset in pixels; each tick we feed the delta to the core.
    _flingLast = 0;
    _fling.value = 0;
    _fling
        .animateWith(FrictionSimulation(0.135, 0, velocity))
        .whenCompleteOrCancel(_settleScroll);
  }

  void _onFlingTick() {
    final value = _fling.value;
    _scrollBy(value - _flingLast);
    _flingLast = value;
  }

  void _settleScroll() {
    final sessionId = widget.sessionId;
    if (sessionId == null) return;
    widget.controller.settleScreenPixelScroll(sessionId);
    _refresh(force: true);
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      fit: StackFit.expand,
      children: [
        // Hidden input anchor for the soft keyboard / IME. It fills the area
        // but is fully transparent and focused programmatically; the gesture
        // layer above is opaque, so it intercepts all pointers and the anchor
        // never shows a caret or selection of its own. EditableText (vs
        // TextField) needs no Material ancestor.
        EditableText(
          controller: _inputController,
          focusNode: _focusNode,
          style: const TextStyle(
            fontSize: 1,
            height: 1,
            color: Color(0x00000000),
          ),
          cursorColor: const Color(0x00000000),
          backgroundCursorColor: const Color(0x00000000),
          // A plain text field with suggestions enabled summons the user's full
          // IME (incl. CJK composition / candidate bar) rather than MIUI's
          // basic/secure keyboard. Autocorrect stays off for terminal input.
          keyboardType: TextInputType.text,
          maxLines: null,
          autocorrect: false,
          enableSuggestions: true,
          showCursor: false,
          rendererIgnoresPointer: true,
        ),
        GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTap: _focusNode.requestFocus,
          onVerticalDragStart: _onDragStart,
          onVerticalDragUpdate: _onDragUpdate,
          onVerticalDragEnd: _onDragEnd,
          child: LayoutBuilder(
            builder: (context, constraints) {
              _syncGrid(constraints);
              return ColoredBox(
                color: AppColors.bgBase,
                child: CustomPaint(
                  size: Size(constraints.maxWidth, constraints.maxHeight),
                  painter: _TerminalGridPainter(
                    snapshot: _snapshot,
                    cellWidth: _cellWidth,
                    cellHeight: _cellHeight,
                    glyphTop: _glyphTop,
                    fontSize: widget.fontSize,
                    fontFamily: _fontFamily,
                    glyphCache: _glyphCache,
                  ),
                ),
              );
            },
          ),
        ),
      ],
    );
  }
}

class _TerminalGridPainter extends CustomPainter {
  _TerminalGridPainter({
    required this.snapshot,
    required this.cellWidth,
    required this.cellHeight,
    required this.glyphTop,
    required this.fontSize,
    required this.fontFamily,
    required this.glyphCache,
  });

  final TerminalScreenSnapshot? snapshot;
  final double cellWidth;
  final double cellHeight;
  final double glyphTop;
  final double fontSize;
  final String fontFamily;
  final Map<String, ui.Paragraph> glyphCache;

  @override
  void paint(Canvas canvas, Size size) {
    final snapshot = this.snapshot;
    if (snapshot == null || cellWidth <= 0 || cellHeight <= 0) return;

    canvas.save();
    canvas.clipRect(Offset.zero & size);
    // Smooth scrolling: shift the grid by the sub-row pixel offset, and lift
    // any pre-rendered overscan rows (host-served scroll) above the viewport.
    canvas.translate(
      0,
      snapshot.scrollPixelOffset - snapshot.marginRows * cellHeight,
    );

    final bgPaint = Paint();
    for (final cell in snapshot.cells) {
      if (cell.row < 0) continue;
      final colors = TerminalTheme.resolveCellColors(
        fg: cell.fg,
        bg: cell.bg,
        inverse: cell.inverse,
        bold: cell.bold,
        dim: cell.dim,
      );
      final span = cell.width < 1 ? 1 : cell.width;
      final x = cell.col * cellWidth;
      final y = cell.row * cellHeight;

      if (colors.drawBackground) {
        bgPaint.color = colors.bg;
        canvas.drawRect(
          Rect.fromLTWH(x, y, cellWidth * span, cellHeight),
          bgPaint,
        );
      }

      if (cell.hidden || cell.text.trim().isEmpty) continue;
      final paragraph = _glyph(
        cell.text,
        colors.fg,
        bold: cell.bold,
        italic: cell.italic,
        underline: cell.underline,
        strikeout: cell.strikeout,
      );
      canvas.drawParagraph(paragraph, Offset(x, y + glyphTop));
    }

    _paintCursor(canvas, snapshot);
    canvas.restore();
  }

  void _paintCursor(Canvas canvas, TerminalScreenSnapshot snapshot) {
    final cursor = snapshot.cursor;
    if (!cursor.visible) return;
    if (cursor.row < 0 || cursor.row >= snapshot.rows) return;
    final x = cursor.col * cellWidth;
    final y = cursor.row * cellHeight;
    final paint = Paint()..color = AppColors.textPrimary;

    switch (cursor.shape) {
      case TerminalScreenCursorShape.beam:
        canvas.drawRect(Rect.fromLTWH(x, y, 2, cellHeight), paint);
      case TerminalScreenCursorShape.underline:
        canvas.drawRect(
          Rect.fromLTWH(x, y + cellHeight - 2, cellWidth, 2),
          paint,
        );
      case TerminalScreenCursorShape.hollowBlock:
        paint
          ..style = PaintingStyle.stroke
          ..strokeWidth = 1;
        canvas.drawRect(Rect.fromLTWH(x, y, cellWidth, cellHeight), paint);
      case TerminalScreenCursorShape.block:
        canvas.drawRect(Rect.fromLTWH(x, y, cellWidth, cellHeight), paint);
        final cell = _cursorCell(snapshot, cursor.row, cursor.col);
        if (cell != null && !cell.hidden && cell.text.trim().isNotEmpty) {
          final glyph = _glyph(
            cell.text,
            AppColors.bgBase,
            bold: cell.bold,
            italic: cell.italic,
            underline: false,
            strikeout: false,
          );
          canvas.drawParagraph(glyph, Offset(x, y + glyphTop));
        }
    }
  }

  TerminalScreenCell? _cursorCell(
    TerminalScreenSnapshot snapshot,
    int row,
    int col,
  ) {
    for (final cell in snapshot.cells) {
      if (cell.row == row && cell.col == col) return cell;
    }
    return null;
  }

  ui.Paragraph _glyph(
    String text,
    Color color, {
    required bool bold,
    required bool italic,
    required bool underline,
    required bool strikeout,
  }) {
    final key =
        '$text|${color.toARGB32()}|$bold|$italic|$underline|$strikeout';
    final cached = glyphCache[key];
    if (cached != null) return cached;

    final decorations = <TextDecoration>[
      if (underline) TextDecoration.underline,
      if (strikeout) TextDecoration.lineThrough,
    ];
    final builder =
        ui.ParagraphBuilder(ui.ParagraphStyle(
            fontFamily: fontFamily,
            fontSize: fontSize,
            height: 1.0,
          ))
          ..pushStyle(
            ui.TextStyle(
              color: color,
              fontWeight: bold ? FontWeight.w600 : FontWeight.normal,
              fontStyle: italic ? FontStyle.italic : FontStyle.normal,
              fontFamily: fontFamily,
              fontSize: fontSize,
              decoration: decorations.isEmpty
                  ? null
                  : TextDecoration.combine(decorations),
              decorationColor: color,
            ),
          )
          ..addText(text);
    final paragraph = builder.build()
      ..layout(const ui.ParagraphConstraints(width: double.infinity));

    if (glyphCache.length > 4096) glyphCache.clear();
    glyphCache[key] = paragraph;
    return paragraph;
  }

  @override
  bool shouldRepaint(covariant _TerminalGridPainter old) {
    return !identical(old.snapshot, snapshot) ||
        old.cellWidth != cellWidth ||
        old.cellHeight != cellHeight;
  }
}
