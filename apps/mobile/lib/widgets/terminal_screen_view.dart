import 'dart:async';
import 'dart:math' as math;

import 'package:codux_protocol_ffi/codux_protocol_ffi.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../theme/app_theme.dart';
import '../theme/terminal_theme.dart';

class TerminalScreenView extends StatefulWidget {
  const TerminalScreenView({
    super.key,
    required this.snapshot,
    required this.keyboardRequested,
    required this.scrollEnabled,
    required this.onInput,
    required this.onResize,
    required this.onScrollPixels,
    required this.onSettleScroll,
    required this.onScrollToBottom,
    required this.onCursorBottom,
    this.remoteScroll = false,
    this.fontSize = _terminalDefaultFontSize,
    this.onSelectionChanged,
  });

  final TerminalScreenSnapshot? snapshot;
  final bool keyboardRequested;
  final bool scrollEnabled;

  /// Whether scrollback is served by the host (with network latency).
  /// The scroll position is owned by Flutter, so delayed host
  /// confirmations only affect which snapshot rows are available to draw.
  final bool remoteScroll;
  final ValueChanged<String> onInput;
  final void Function(int cols, int rows) onResize;
  final void Function(double pixels, double cellHeight) onScrollPixels;
  final VoidCallback onSettleScroll;
  final VoidCallback onScrollToBottom;
  final ValueChanged<double> onCursorBottom;
  final double fontSize;
  final ValueChanged<String?>? onSelectionChanged;

  @override
  State<TerminalScreenView> createState() => _TerminalScreenViewState();
}

class _TerminalScreenViewState extends State<TerminalScreenView>
    implements TextInputClient {
  final ScrollController _scrollController = ScrollController();
  final FocusNode _keyboardFocusNode = FocusNode(
    debugLabel: 'terminal-screen-input',
  );
  TextInputConnection? _inputConnection;
  TextEditingValue _editingValue = _terminalInputSentinelValue;
  bool _followTail = true;
  bool _scrollIdle = true;
  bool _scrollFlushScheduled = false;
  bool _scrollToBottomScheduled = false;
  bool _suppressScrollEmit = false;
  double _pendingScrollPixels = 0;
  double _unrequestedRemoteScrollPixels = 0;
  double? _lastScrollOffset;
  _ScrollOffsetBounds? _lastRemoteScrollBounds;
  bool _cursorBlinkVisible = true;
  Timer? _cursorBlinkTimer;
  int? _lastEmittedCols;
  int? _lastEmittedRows;
  _TerminalSelectionRange? _selection;
  _TerminalCellPosition? _selectionAnchor;
  Timer? _longPressTimer;
  Offset? _longPressStartPosition;
  bool _selectionDragging = false;

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_handleScrollOffsetChanged);
    _startCursorBlink();
    _syncKeyboardFocus();
  }

  @override
  void didUpdateWidget(covariant TerminalScreenView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.keyboardRequested != oldWidget.keyboardRequested) {
      _syncKeyboardFocus();
    }
    if (widget.snapshot?.data != oldWidget.snapshot?.data &&
        _followTail &&
        widget.snapshot?.displayOffset != 0) {
      _scheduleScrollToBottom();
    }
    if (_remoteScrollWindowSignature(widget.snapshot) !=
        _remoteScrollWindowSignature(oldWidget.snapshot)) {
      _unrequestedRemoteScrollPixels = 0;
    }
    if (_cursorSignature(widget.snapshot) !=
        _cursorSignature(oldWidget.snapshot)) {
      _resetCursorBlink();
    }
    if (widget.snapshot != oldWidget.snapshot && _selection != null) {
      _emitSelection();
    }
  }

  @override
  void dispose() {
    _cursorBlinkTimer?.cancel();
    _longPressTimer?.cancel();
    _closeKeyboardConnection();
    _keyboardFocusNode.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  void _startCursorBlink() {
    _cursorBlinkTimer = Timer.periodic(_terminalCursorBlinkInterval, (_) {
      if (!mounted) return;
      final cursor = widget.snapshot?.cursor;
      if (cursor == null || !cursor.visible) {
        if (!_cursorBlinkVisible) {
          setState(() => _cursorBlinkVisible = true);
        }
        return;
      }
      setState(() => _cursorBlinkVisible = !_cursorBlinkVisible);
    });
  }

  void _resetCursorBlink() {
    if (_cursorBlinkVisible) return;
    setState(() => _cursorBlinkVisible = true);
  }

  void _syncKeyboardFocus() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      if (widget.keyboardRequested) {
        _keyboardFocusNode.requestFocus();
        _openKeyboardConnection();
      } else {
        _keyboardFocusNode.unfocus();
        _closeKeyboardConnection();
      }
    });
  }

  void _scheduleScrollToBottom() {
    if (_scrollToBottomScheduled) return;
    _scrollToBottomScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scrollToBottomScheduled = false;
      if (!mounted || !_followTail || widget.snapshot?.displayOffset == 0) {
        return;
      }
      _jumpToBottom();
      widget.onScrollToBottom();
    });
  }

  void _openKeyboardConnection() {
    final connection = _inputConnection;
    if (connection != null && connection.attached) {
      _syncKeyboardGeometry(connection);
      connection.show();
      return;
    }
    final nextConnection = TextInput.attach(this, _terminalInputConfig);
    _inputConnection = nextConnection;
    _editingValue = _terminalInputSentinelValue;
    nextConnection.setEditingState(_editingValue);
    _syncKeyboardGeometry(nextConnection);
    nextConnection.show();
  }

  void _closeKeyboardConnection() {
    final connection = _inputConnection;
    _inputConnection = null;
    _editingValue = _terminalInputSentinelValue;
    if (connection != null && connection.attached) {
      connection.close();
    }
  }

  void _syncKeyboardGeometry(TextInputConnection connection) {
    final renderObject = context.findRenderObject();
    if (renderObject is! RenderBox || !renderObject.hasSize) return;
    final transform = renderObject.getTransformTo(null);
    connection.setEditableSizeAndTransform(renderObject.size, transform);
    connection.setCaretRect(_caretRect(renderObject.size));
    connection.setComposingRect(_caretRect(renderObject.size));
  }

  Rect _caretRect(Size size) {
    final snapshot = widget.snapshot;
    final fontSize = _normalizeTerminalFontSize(widget.fontSize);
    final cellHeight = _terminalCellHeight(fontSize);
    if (snapshot == null) {
      return Rect.fromLTWH(0, 0, 1, cellHeight);
    }
    final cellWidth = _terminalCellWidth(context, fontSize);
    final left = (snapshot.cursor.col * cellWidth).clamp(0.0, size.width);
    final top = (snapshot.cursor.row * cellHeight).clamp(0.0, size.height);
    return Rect.fromLTWH(left, top, 1, cellHeight);
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final fontSize = _normalizeTerminalFontSize(widget.fontSize);
        final cellWidth = _terminalCellWidth(context, fontSize);
        final cellHeight = _terminalCellHeight(fontSize);
        final cols = math.max(20, constraints.maxWidth ~/ cellWidth);
        final rows = math.max(8, constraints.maxHeight ~/ cellHeight);
        final snapshot = widget.snapshot;
        final remoteScrollBounds = snapshot == null
            ? null
            : _confirmedSnapshotScrollBounds(
                snapshot,
                constraints.maxHeight,
                cellHeight,
                remoteScroll: widget.remoteScroll,
              );
        final remoteVisibleOffset = snapshot == null
            ? null
            : _confirmedSnapshotVisibleOffset(
                snapshot,
                constraints.maxHeight,
                cellHeight,
                remoteScroll: widget.remoteScroll,
              );
        _lastRemoteScrollBounds = remoteScrollBounds;
        // The virtual content covers the full scrollback; the scroll offset
        // is measured from the top of history, bottom = maxScrollExtent. A
        // host-confirmed remote window near the live tail can extend beyond
        // `totalLines * cellHeight - viewportHeight`, so include the confirmed
        // window in the transparent scroll extent instead of forcing the
        // painter to compensate with blank space.
        final contentHeight = _scrollContentHeight(
          snapshot,
          constraints.maxHeight,
          cellHeight,
          remoteScrollBounds,
        );
        final rawScrollOffset = _scrollController.hasClients
            ? _scrollController.position.pixels
            : remoteVisibleOffset ??
                  math.max(0.0, contentHeight - constraints.maxHeight);
        final scrollOffset = snapshot == null
            ? rawScrollOffset
            : _confirmedSnapshotScrollOffset(
                snapshot,
                rawScrollOffset,
                constraints.maxHeight,
                cellHeight,
                remoteScroll: widget.remoteScroll,
              );
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!mounted) return;
          // Only emit a resize when the measured grid actually changed. The
          // build runs on every cursor-blink/setState; emitting unconditionally
          // ran an FFI screen resize per frame.
          if (cols != _lastEmittedCols || rows != _lastEmittedRows) {
            _lastEmittedCols = cols;
            _lastEmittedRows = rows;
            widget.onResize(cols, rows);
          }
          final connection = _inputConnection;
          if (connection != null && connection.attached) {
            _syncKeyboardGeometry(connection);
          }
          _maintainScrollAnchor(
            remoteScrollBounds,
            remoteVisibleOffset: remoteVisibleOffset,
          );
          final screen = widget.snapshot;
          if (screen != null) {
            final offsetNow = _clampRemoteScrollPosition(
              screen,
              constraints.maxHeight,
              cellHeight,
              fallbackOffset: rawScrollOffset,
            );
            final paintOffsetNow = _confirmedSnapshotScrollOffset(
              screen,
              offsetNow,
              constraints.maxHeight,
              cellHeight,
              remoteScroll: widget.remoteScroll,
            );
            final cursorBottom =
                (screen.cursor.row + 1) * cellHeight +
                _painterScrollOffsetY(
                  screen,
                  paintOffsetNow,
                  constraints.maxHeight,
                  cellHeight,
                );
            widget.onCursorBottom(cursorBottom);
          }
        });

        return KeyboardListener(
          focusNode: _keyboardFocusNode,
          autofocus: widget.keyboardRequested,
          onKeyEvent: _handleKeyEvent,
          child: Listener(
            behavior: HitTestBehavior.translucent,
            onPointerDown: (event) => _scheduleSelectionStart(
              event.localPosition,
              scrollOffset,
              constraints.maxHeight,
              cellWidth,
              cellHeight,
            ),
            onPointerMove: (event) => _handleSelectionPointerMove(
              event.localPosition,
              scrollOffset,
              constraints.maxHeight,
              cellWidth,
              cellHeight,
            ),
            onPointerUp: (_) => _endSelectionPointer(),
            onPointerCancel: (_) => _endSelectionPointer(),
            child: ClipRect(
              child: Stack(
                children: [
                  Positioned.fill(
                    child: RepaintBoundary(
                      child: CustomPaint(
                        size: Size.infinite,
                        painter: _TerminalScreenPainter(
                          snapshot: snapshot,
                          cellWidth: cellWidth,
                          cellHeight: cellHeight,
                          fontSize: fontSize,
                          scrollOffsetY: snapshot == null
                              ? 0
                              : _painterScrollOffsetY(
                                  snapshot,
                                  scrollOffset,
                                  constraints.maxHeight,
                                  cellHeight,
                                ),
                          selection: _selection,
                        ),
                      ),
                    ),
                  ),
                  // Cursor on its own layer so the blink doesn't repaint the grid.
                  Positioned.fill(
                    child: RepaintBoundary(
                      child: CustomPaint(
                        size: Size.infinite,
                        painter: _TerminalCursorPainter(
                          snapshot: snapshot,
                          cellWidth: cellWidth,
                          cellHeight: cellHeight,
                          fontSize: fontSize,
                          scrollOffsetY: snapshot == null
                              ? 0
                              : _painterScrollOffsetY(
                                  snapshot,
                                  scrollOffset,
                                  constraints.maxHeight,
                                  cellHeight,
                                ),
                          cursorBlinkVisible: _cursorBlinkVisible,
                        ),
                      ),
                    ),
                  ),
                  // Transparent scroll surface: Flutter physics owns the
                  // position; the painter above translates the snapshot to it.
                  Positioned.fill(
                    child: NotificationListener<ScrollNotification>(
                      onNotification: _handleScrollNotification,
                      child: SingleChildScrollView(
                        controller: _scrollController,
                        physics: _scrollPhysics(remoteScrollBounds),
                        child: SizedBox(
                          height: contentHeight,
                          width: constraints.maxWidth,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        );
      },
    );
  }

  void _handleScrollOffsetChanged() {
    if (!_scrollController.hasClients) return;
    final position = _scrollController.position;
    final previous = _lastScrollOffset;
    // Follow the tail while the position stays within half a row of the
    // bottom; scrolling away releases the pin.
    final cellHeight = _terminalCellHeight(
      _normalizeTerminalFontSize(widget.fontSize),
    );
    final rawOffset = position.pixels;
    _lastScrollOffset = rawOffset;
    _followTail = position.maxScrollExtent - rawOffset <= cellHeight / 2;
    setState(() {});
    if (_suppressScrollEmit || previous == null) return;
    // Offset grows downward from the top of history; the contract wants
    // positive pixels for scrolling up into history.
    final delta = previous - rawOffset;
    if (delta == 0) return;
    if (widget.remoteScroll) {
      _unrequestedRemoteScrollPixels += delta;
      if (!_remoteScrollNeedsHostSnapshot(position, rawOffset)) return;
      if (_unrequestedRemoteScrollPixels == 0) return;
      _pendingScrollPixels += _unrequestedRemoteScrollPixels;
      _unrequestedRemoteScrollPixels = 0;
      _scheduleScrollFlush();
      return;
    }
    _pendingScrollPixels += delta;
    _scheduleScrollFlush();
  }

  bool _handleScrollNotification(ScrollNotification notification) {
    if (notification is ScrollStartNotification) {
      _scrollIdle = false;
    } else if (notification is OverscrollNotification) {
      // Remote scrollback already exposes the complete host-advertised range
      // as the ScrollView extent. Overscroll means the user hit the real edge,
      // not that a new host window is needed.
    } else if (notification is ScrollEndNotification) {
      _scrollIdle = true;
      if (!_suppressScrollEmit) {
        _flushScrollPixels();
        widget.onSettleScroll();
      }
    }
    return false;
  }

  void _scheduleScrollFlush() {
    if (_scrollFlushScheduled) return;
    _scrollFlushScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scrollFlushScheduled = false;
      if (!mounted) return;
      _flushScrollPixels();
    });
  }

  void _flushScrollPixels() {
    final pixels = _pendingScrollPixels;
    if (pixels == 0) return;
    _pendingScrollPixels = 0;
    widget.onScrollPixels(
      pixels,
      _terminalCellHeight(_normalizeTerminalFontSize(widget.fontSize)),
    );
  }

  ScrollPhysics _scrollPhysics(_ScrollOffsetBounds? remoteScrollBounds) {
    if (!widget.scrollEnabled) return const NeverScrollableScrollPhysics();
    return const ClampingScrollPhysics();
  }

  double _scrollContentHeight(
    TerminalScreenSnapshot? screen,
    double viewportHeight,
    double cellHeight,
    _ScrollOffsetBounds? remoteScrollBounds,
  ) {
    final linesHeight = screen == null
        ? 0.0
        : _scrollbackContentHeight(screen, viewportHeight, cellHeight);
    final confirmedHeight = remoteScrollBounds == null
        ? 0.0
        : remoteScrollBounds.max + viewportHeight;
    return math.max(viewportHeight, math.max(linesHeight, confirmedHeight));
  }

  double _scrollbackContentHeight(
    TerminalScreenSnapshot screen,
    double viewportHeight,
    double cellHeight,
  ) {
    if (!widget.remoteScroll) {
      return screen.totalLines * cellHeight;
    }
    final viewportRows = math.max(
      1,
      screen.rows - screen.marginRows - screen.marginRowsBelow,
    );
    final historyRows = math.max(0, screen.totalLines - viewportRows);
    return viewportHeight + historyRows * cellHeight;
  }

  void _startSelection(
    Offset position,
    double scrollOffset,
    double viewportHeight,
    double cellWidth,
    double cellHeight,
  ) {
    final point = _cellPositionAt(
      position,
      scrollOffset,
      viewportHeight,
      cellWidth,
      cellHeight,
    );
    if (point == null) return;
    setState(() {
      _selectionAnchor = point;
      _selection = _TerminalSelectionRange(point, point);
    });
    _emitSelection();
  }

  void _scheduleSelectionStart(
    Offset position,
    double scrollOffset,
    double viewportHeight,
    double cellWidth,
    double cellHeight,
  ) {
    _longPressTimer?.cancel();
    _selectionDragging = false;
    _longPressStartPosition = position;
    _longPressTimer = Timer(_terminalSelectionLongPressDelay, () {
      if (!mounted) return;
      _selectionDragging = true;
      _startSelection(
        position,
        scrollOffset,
        viewportHeight,
        cellWidth,
        cellHeight,
      );
    });
  }

  void _handleSelectionPointerMove(
    Offset position,
    double scrollOffset,
    double viewportHeight,
    double cellWidth,
    double cellHeight,
  ) {
    final start = _longPressStartPosition;
    if (!_selectionDragging) {
      if (start != null &&
          (position - start).distance > _terminalSelectionMoveTolerance) {
        _longPressTimer?.cancel();
      }
      return;
    }
    _extendSelection(
      position,
      scrollOffset,
      viewportHeight,
      cellWidth,
      cellHeight,
    );
  }

  void _endSelectionPointer() {
    _longPressTimer?.cancel();
    _longPressStartPosition = null;
    _selectionDragging = false;
  }

  void _extendSelection(
    Offset position,
    double scrollOffset,
    double viewportHeight,
    double cellWidth,
    double cellHeight,
  ) {
    final anchor = _selectionAnchor;
    if (anchor == null) return;
    final point = _cellPositionAt(
      position,
      scrollOffset,
      viewportHeight,
      cellWidth,
      cellHeight,
    );
    if (point == null) return;
    setState(() {
      _selection = _TerminalSelectionRange(anchor, point);
    });
    _emitSelection();
  }

  _TerminalCellPosition? _cellPositionAt(
    Offset position,
    double scrollOffset,
    double viewportHeight,
    double cellWidth,
    double cellHeight,
  ) {
    final screen = widget.snapshot;
    if (screen == null || cellWidth <= 0 || cellHeight <= 0) return null;
    final offsetY = _painterScrollOffsetY(
      screen,
      scrollOffset,
      viewportHeight,
      cellHeight,
    );
    final row = ((position.dy - offsetY) / cellHeight).floor();
    final col = (position.dx / cellWidth).floor();
    if (row < 0 || row >= screen.rows || col < 0 || col >= screen.cols) {
      return null;
    }
    return _TerminalCellPosition(row, col);
  }

  void _emitSelection() {
    final callback = widget.onSelectionChanged;
    if (callback == null) return;
    callback(_selectedText(widget.snapshot, _selection));
  }

  void _maintainScrollAnchor(
    _ScrollOffsetBounds? remoteScrollBounds, {
    double? remoteVisibleOffset,
  }) {
    if (!_scrollController.hasClients) return;
    final position = _scrollController.position;
    // Pin to the (possibly grown) bottom while following the tail; never
    // fight an in-flight user drag or fling.
    final target = widget.remoteScroll && remoteScrollBounds != null
        ? remoteVisibleOffset ?? remoteScrollBounds.max
        : position.maxScrollExtent;
    if (_followTail &&
        _scrollIdle &&
        (target - position.pixels).abs() > _terminalScrollEpsilon) {
      _suppressedJumpTo(target);
    }
    // Content-extent shrink corrections move pixels without notifying;
    // realign so the next user delta is measured from the real offset.
    _lastScrollOffset = position.pixels;
  }

  double _clampRemoteScrollPosition(
    TerminalScreenSnapshot screen,
    double viewportHeight,
    double cellHeight, {
    required double fallbackOffset,
  }) {
    if (!_scrollController.hasClients) return fallbackOffset;
    final position = _scrollController.position;
    final requested = position.pixels;
    final target = requested
        .clamp(position.minScrollExtent, position.maxScrollExtent)
        .toDouble();
    if ((target - requested).abs() > _terminalScrollEpsilon) {
      _suppressedJumpTo(target);
      _lastScrollOffset = target;
      return target;
    }
    return requested;
  }

  void _jumpToBottom() {
    if (!_scrollController.hasClients) return;
    _suppressedJumpTo(_scrollController.position.maxScrollExtent);
  }

  void _suppressedJumpTo(double target) {
    _suppressScrollEmit = true;
    try {
      _scrollController.jumpTo(target);
    } finally {
      _suppressScrollEmit = false;
    }
  }

  // The snapshot grid is drawn at absolute content coordinates: the
  // viewport portion sits with its bottom at line totalLines -
  // displayOffset, marginRows of above-context render above it and
  // marginRowsBelow of below-context render below it, all translated by
  // the Flutter scroll offset. The sub-line scrollPixelOffset is already
  // folded into the offset the host was asked to show, so it does not
  // reappear here.
  double _painterScrollOffsetY(
    TerminalScreenSnapshot screen,
    double scrollOffset,
    double viewportHeight,
    double cellHeight,
  ) {
    final viewportRows = math.max(
      1,
      screen.rows - screen.marginRows - screen.marginRowsBelow,
    );
    final absoluteTopY =
        (screen.totalLines -
            screen.displayOffset -
            viewportRows -
            screen.marginRows) *
        cellHeight;
    return absoluteTopY -
        scrollOffset +
        _bottomAnchorOffset(screen, viewportHeight, cellHeight);
  }

  // When the host grid is at least one full row shorter than this screen
  // (the desktop owns the viewport from a smaller window) and all content
  // fits in the viewport, anchor content to the bottom so the TUI composer
  // sits by the keyboard. Taller content is already bottom-aligned at
  // maxScrollExtent by the absolute coordinate math.
  double _bottomAnchorOffset(
    TerminalScreenSnapshot screen,
    double viewportHeight,
    double cellHeight,
  ) {
    if (screen.marginRows > 0 ||
        screen.marginRowsBelow > 0 ||
        screen.displayOffset > 0) {
      return 0;
    }
    final contentRows =
        screen.rows - screen.marginRows - screen.marginRowsBelow;
    final deficit = viewportHeight - contentRows * cellHeight;
    if (deficit < cellHeight) return 0;
    return math.max(0.0, viewportHeight - screen.totalLines * cellHeight);
  }

  double _confirmedSnapshotScrollOffset(
    TerminalScreenSnapshot screen,
    double requestedOffset,
    double viewportHeight,
    double cellHeight, {
    required bool remoteScroll,
  }) {
    final bounds = _confirmedSnapshotScrollBounds(
      screen,
      viewportHeight,
      cellHeight,
      remoteScroll: remoteScroll,
    );
    if (bounds == null) {
      return requestedOffset;
    }
    return requestedOffset.clamp(bounds.min, bounds.max).toDouble();
  }

  double? _confirmedSnapshotVisibleOffset(
    TerminalScreenSnapshot screen,
    double viewportHeight,
    double cellHeight, {
    required bool remoteScroll,
  }) {
    final bounds = _confirmedSnapshotScrollBounds(
      screen,
      viewportHeight,
      cellHeight,
      remoteScroll: remoteScroll,
    );
    if (bounds == null) return null;
    final viewportRows = math.max(
      1,
      screen.rows - screen.marginRows - screen.marginRowsBelow,
    );
    final visibleTop =
        (screen.totalLines - screen.displayOffset - viewportRows) *
        cellHeight;
    return visibleTop.clamp(bounds.min, bounds.max).toDouble();
  }

  bool _remoteScrollNeedsHostSnapshot(
    ScrollPosition position,
    double offset,
  ) {
    if (!widget.remoteScroll) return true;
    final bounds = _lastRemoteScrollBounds;
    if (bounds == null) return true;
    final screen = widget.snapshot;
    final cellHeight = _terminalCellHeight(
      _normalizeTerminalFontSize(widget.fontSize),
    );
    final prefetchDistance = _terminalRemotePrefetchDistance(
      _remoteVisibleRows(screen) * cellHeight,
    );
    final scrollingIntoHistory = _unrequestedRemoteScrollPixels > 0;
    final scrollingTowardTail = _unrequestedRemoteScrollPixels < 0;
    final hasHistoryPrefetchHeadroom =
        screen != null && screen.marginRows * cellHeight >= prefetchDistance;
    final hasTailPrefetchHeadroom =
        screen != null &&
        screen.marginRowsBelow * cellHeight >= prefetchDistance;
    if (offset < bounds.min - _terminalScrollEpsilon ||
        offset > bounds.max + _terminalScrollEpsilon) {
      return true;
    }
    if (scrollingIntoHistory &&
        hasHistoryPrefetchHeadroom &&
        offset <= bounds.min + prefetchDistance + _terminalScrollEpsilon &&
        bounds.min > position.minScrollExtent + _terminalScrollEpsilon) {
      return true;
    }
    if (scrollingTowardTail &&
        hasTailPrefetchHeadroom &&
        offset >= bounds.max - prefetchDistance - _terminalScrollEpsilon &&
        bounds.max < position.maxScrollExtent - _terminalScrollEpsilon) {
      return true;
    }
    return false;
  }

  _ScrollOffsetBounds? _confirmedSnapshotScrollBounds(
    TerminalScreenSnapshot screen,
    double viewportHeight,
    double cellHeight, {
    required bool remoteScroll,
  }) {
    if (!remoteScroll || cellHeight <= 0 || viewportHeight <= 0) {
      return null;
    }
    final viewportRows = math.max(
      1,
      screen.rows - screen.marginRows - screen.marginRowsBelow,
    );
    if (screen.displayOffset == 0 && screen.totalLines <= viewportRows) {
      return const _ScrollOffsetBounds(0, 0);
    }
    final visibleTop =
        (screen.totalLines - screen.displayOffset - viewportRows) *
        cellHeight;
    final minOffset = (visibleTop - screen.marginRows * cellHeight).clamp(
      0.0,
      double.infinity,
    );
    final maxOffset = (visibleTop + screen.marginRowsBelow * cellHeight).clamp(
      minOffset,
      double.infinity,
    );
    return _ScrollOffsetBounds(minOffset.toDouble(), maxOffset.toDouble());
  }

  @override
  TextEditingValue? get currentTextEditingValue => _editingValue;

  @override
  AutofillScope? get currentAutofillScope => null;

  void _handleKeyEvent(KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) return;
    final key = switch (event.logicalKey) {
      LogicalKeyboardKey.backspace => 'backspace',
      LogicalKeyboardKey.delete => 'delete',
      LogicalKeyboardKey.enter => 'enter',
      LogicalKeyboardKey.arrowLeft => 'left',
      LogicalKeyboardKey.arrowRight => 'right',
      LogicalKeyboardKey.arrowUp => 'up',
      LogicalKeyboardKey.arrowDown => 'down',
      _ => null,
    };
    if (key == null) return;
    final input = terminalKeyInput(
      key: key,
      applicationCursor: widget.snapshot?.applicationCursor ?? false,
    );
    if (input.isNotEmpty) {
      _resetCursorBlink();
      widget.onInput(input);
    }
  }

  @override
  void updateEditingValue(TextEditingValue value) {
    if (value.composing.isValid && !value.composing.isCollapsed) {
      _editingValue = value;
      return;
    }
    final terminalInput = _terminalInputFromEditingValue(value);
    final normalizedInput = terminalTextInput(terminalInput);
    if (normalizedInput.isNotEmpty) {
      _resetCursorBlink();
      widget.onInput(normalizedInput);
    }
    _resetImeEditingState();
  }

  void _resetImeEditingState() {
    _editingValue = _terminalInputSentinelValue;
    final connection = _inputConnection;
    if (connection != null && connection.attached) {
      connection.setEditingState(_editingValue);
    }
  }

  @override
  void performAction(TextInputAction action) {
    switch (action) {
      case TextInputAction.newline:
      case TextInputAction.done:
      case TextInputAction.go:
      case TextInputAction.send:
      case TextInputAction.unspecified:
      case TextInputAction.none:
        _resetCursorBlink();
        widget.onInput(terminalKeyInput(key: 'enter'));
      case TextInputAction.next:
      case TextInputAction.previous:
      case TextInputAction.search:
      case TextInputAction.join:
      case TextInputAction.route:
      case TextInputAction.emergencyCall:
      case TextInputAction.continueAction:
        break;
    }
  }

  @override
  void connectionClosed() {
    _inputConnection = null;
    _editingValue = _terminalInputSentinelValue;
  }

  @override
  void didChangeInputControl(
    TextInputControl? oldControl,
    TextInputControl? newControl,
  ) {}

  @override
  void insertContent(KeyboardInsertedContent content) {}

  @override
  void insertTextPlaceholder(Size size) {}

  // Flutter 3.44 adds this to TextInputClient; keep it unannotated so older
  // local SDKs still accept the implementation.
  // ignore: annotate_overrides
  bool onFocusReceived() => true;

  @override
  void performPrivateCommand(String action, Map<String, dynamic> data) {}

  @override
  void performSelector(String selectorName) {
    final input = terminalSelectorInput(
      selector: selectorName,
      applicationCursor: widget.snapshot?.applicationCursor ?? false,
    );
    if (input.isNotEmpty) {
      _resetCursorBlink();
      widget.onInput(input);
    }
  }

  @override
  void removeTextPlaceholder() {}

  @override
  void showToolbar() {}

  @override
  void updateFloatingCursor(RawFloatingCursorPoint point) {}

  @override
  void showAutocorrectionPromptRect(int start, int end) {}
}

const _terminalDefaultFontSize = 14.0;
const _terminalLetterSpacing = 0.0;
const _terminalCellWidthProbe =
    '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';
const _terminalFontFeatures = [
  FontFeature.disable('liga'),
  FontFeature.disable('calt'),
];
const _terminalScrollEpsilon = 0.01;
const _terminalCursorBlinkInterval = Duration(milliseconds: 530);
const _terminalSelectionLongPressDelay = Duration(milliseconds: 520);
const _terminalSelectionMoveTolerance = 10.0;
const _terminalInputSentinel = '  ';
const _terminalBackspaceInput = '\u0008';
final _terminalCellWidthCache = <String, double>{};
const _terminalInputSentinelValue = TextEditingValue(
  text: _terminalInputSentinel,
  selection: TextSelection.collapsed(offset: _terminalInputSentinel.length),
);

class _ScrollOffsetBounds {
  const _ScrollOffsetBounds(this.min, this.max);

  final double min;
  final double max;
}

double _terminalRemotePrefetchDistance(double viewportHeight) {
  if (!viewportHeight.isFinite || viewportHeight <= 0) return 0;
  return viewportHeight;
}

int _remoteVisibleRows(TerminalScreenSnapshot? snapshot) {
  if (snapshot == null) return 1;
  final visible = snapshot.rows - snapshot.marginRows - snapshot.marginRowsBelow;
  return visible > 0 ? visible : math.max(1, snapshot.rows);
}

double _normalizeTerminalFontSize(double value) {
  return value.clamp(10.0, 18.0).roundToDouble();
}

double _terminalCellHeight(double fontSize) {
  return (fontSize + 5.0).roundToDouble();
}

const _terminalInputConfig = TextInputConfiguration(
  inputType: TextInputType.emailAddress,
  inputAction: TextInputAction.newline,
  autocorrect: false,
  enableSuggestions: false,
  enableIMEPersonalizedLearning: false,
  enableInteractiveSelection: false,
  enableDeltaModel: false,
  keyboardAppearance: Brightness.dark,
  autofillConfiguration: AutofillConfiguration.disabled,
);

String _terminalInputFromEditingValue(TextEditingValue next) {
  final text = next.text;
  if (text.length < _terminalInputSentinel.length) {
    return _terminalBackspaceInput;
  }
  if (text == _terminalInputSentinel) return '';
  if (text.startsWith(_terminalInputSentinel)) {
    return text.substring(_terminalInputSentinel.length);
  }
  if (text.length > _terminalInputSentinel.length) {
    return text.substring(_terminalInputSentinel.length);
  }
  return '';
}

String _cursorSignature(TerminalScreenSnapshot? snapshot) {
  final cursor = snapshot?.cursor;
  if (cursor == null) return '';
  return '${cursor.row}:${cursor.col}:${cursor.visible}:${cursor.shape}';
}

String _remoteScrollWindowSignature(TerminalScreenSnapshot? snapshot) {
  if (snapshot == null) return '';
  return '${snapshot.totalLines}:${snapshot.displayOffset}:${snapshot.marginRows}:${snapshot.marginRowsBelow}:${snapshot.rows}:${snapshot.cols}';
}

double _terminalCellWidth(BuildContext context, double fontSize) {
  final platform = defaultTargetPlatform;
  final key = '${platform.name}:$fontSize';
  final cached = _terminalCellWidthCache[key];
  if (cached != null) return cached;

  final textPainter = TextPainter(
    text: TextSpan(
      text: _terminalCellWidthProbe,
      style: _terminalTextStyle(
        color: AppColors.textPrimary,
        fontSize: fontSize,
      ),
    ),
    textDirection: TextDirection.ltr,
    maxLines: 1,
  )..layout();
  final measured = textPainter.width / _terminalCellWidthProbe.length;
  final width = measured.clamp(fontSize * 0.5, fontSize * 0.82);
  _terminalCellWidthCache[key] = width;
  return width;
}

String _terminalFontFamily() {
  return switch (defaultTargetPlatform) {
    TargetPlatform.iOS || TargetPlatform.macOS => 'Menlo',
    TargetPlatform.android => 'monospace',
    TargetPlatform.fuchsia ||
    TargetPlatform.linux ||
    TargetPlatform.windows => 'monospace',
  };
}

List<String> _terminalFontFamilyFallback() {
  return switch (defaultTargetPlatform) {
    TargetPlatform.iOS ||
    TargetPlatform.macOS => const ['SF Mono', 'Courier', 'monospace'],
    TargetPlatform.android => const ['monospace'],
    TargetPlatform.fuchsia ||
    TargetPlatform.linux ||
    TargetPlatform.windows => const ['monospace'],
  };
}

// TextStyle is rebuilt once per visible cell per repaint, each call allocating
// a fontFamilyFallback list. The painter only uses a few distinct styles, so
// memoizing by value key removes that per-cell allocation.
final Map<(int, double, bool, bool, TextDecoration?), TextStyle>
_terminalTextStyleCache = {};

TextStyle _terminalTextStyle({
  required Color color,
  required double fontSize,
  bool bold = false,
  bool italic = false,
  TextDecoration? decoration,
}) {
  final key = (color.toARGB32(), fontSize, bold, italic, decoration);
  final cached = _terminalTextStyleCache[key];
  if (cached != null) return cached;
  final style = TextStyle(
    color: color,
    fontFamily: _terminalFontFamily(),
    fontFamilyFallback: _terminalFontFamilyFallback(),
    fontSize: fontSize,
    height: 1,
    letterSpacing: _terminalLetterSpacing,
    fontFeatures: _terminalFontFeatures,
    fontWeight: bold ? FontWeight.w700 : FontWeight.w400,
    fontStyle: italic ? FontStyle.italic : FontStyle.normal,
    decoration: decoration,
  );
  if (_terminalTextStyleCache.length > 1024) _terminalTextStyleCache.clear();
  _terminalTextStyleCache[key] = style;
  return style;
}

// The grid is painted on its own layer (cells only, no cursor). Its
// shouldRepaint ignores the cursor blink, so the 530ms blink toggle no longer
// re-shapes and repaints the entire grid — only the small cursor layer below
// repaints on a blink.
class _TerminalScreenPainter extends CustomPainter {
  _TerminalScreenPainter({
    required this.snapshot,
    required this.cellWidth,
    required this.cellHeight,
    required this.fontSize,
    required this.scrollOffsetY,
    required this.selection,
  });

  final TerminalScreenSnapshot? snapshot;
  final double cellWidth;
  final double cellHeight;
  final double fontSize;
  final double scrollOffsetY;
  final _TerminalSelectionRange? selection;

  @override
  void paint(Canvas canvas, Size size) {
    final screen = snapshot;
    canvas.drawRect(Offset.zero & size, Paint()..color = AppColors.bgBase);
    if (screen == null) return;

    final textPainter = TextPainter(textDirection: TextDirection.ltr);
    _paintSelection(
      canvas,
      size,
      screen,
      selection,
      cellWidth,
      cellHeight,
      scrollOffsetY,
    );

    final cells = screen.cells;
    for (final cell in cells) {
      if (cell.hidden) continue;
      final left = cell.col * cellWidth;
      final top = cell.row * cellHeight + scrollOffsetY;
      if (left >= size.width || top >= size.height || top + cellHeight <= 0) {
        continue;
      }
      final colors = TerminalTheme.resolveCellColors(
        fg: cell.fg,
        bg: cell.bg,
        inverse: cell.inverse,
        bold: cell.bold,
        dim: cell.dim,
      );
      if (colors.drawBackground) {
        canvas.drawRect(
          Rect.fromLTWH(left, top, cellWidth * cell.width, cellHeight),
          Paint()..color = colors.bg,
        );
      }
      // Background-only cells (TUI panel bands erased with a background
      // color) carry no glyph; they still need the rect above.
      if (cell.text.isEmpty) continue;
    }

    for (var i = 0; i < cells.length; i++) {
      final cell = cells[i];
      if (cell.hidden || cell.text.isEmpty) continue;
      final left = cell.col * cellWidth;
      final top = cell.row * cellHeight + scrollOffsetY;
      if (left >= size.width || top >= size.height || top + cellHeight <= 0) {
        continue;
      }
      final run = _terminalTextRun(cells, i);
      if (run == null) continue;
      final colors = TerminalTheme.resolveCellColors(
        fg: cell.fg,
        bg: cell.bg,
        inverse: cell.inverse,
        bold: cell.bold,
        dim: cell.dim,
      );
      textPainter.text = TextSpan(
        text: run.text,
        style: _terminalTextStyle(
          color: colors.fg,
          fontSize: fontSize,
          bold: cell.bold,
          italic: cell.italic,
          decoration: TextDecoration.combine([
            if (cell.underline) TextDecoration.underline,
            if (cell.strikeout) TextDecoration.lineThrough,
          ]),
        ),
      );
      _paintTerminalText(
        canvas: canvas,
        textPainter: textPainter,
        left: left,
        top: top,
        width: cellWidth * run.width,
        height: cellHeight,
        fontSize: fontSize,
        clipRight: size.width,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _TerminalScreenPainter oldDelegate) {
    return snapshot != oldDelegate.snapshot ||
        cellWidth != oldDelegate.cellWidth ||
        cellHeight != oldDelegate.cellHeight ||
        fontSize != oldDelegate.fontSize ||
        scrollOffsetY != oldDelegate.scrollOffsetY ||
        selection != oldDelegate.selection;
  }
}

class _TerminalTextRun {
  const _TerminalTextRun({required this.text, required this.width});

  final String text;
  final int width;
}

_TerminalTextRun? _terminalTextRun(
  List<TerminalScreenCell> cells,
  int firstIndex,
) {
  if (firstIndex < 0 || firstIndex >= cells.length) return null;
  final first = cells[firstIndex];
  if (first.text.isEmpty) return null;
  if (firstIndex > 0) {
    final previous = cells[firstIndex - 1];
    if (_canMergeTerminalText(previous, first) &&
        previous.col + math.max(1, previous.width) == first.col) {
      return null;
    }
  }
  final text = StringBuffer(first.text);
  var width = math.max(1, first.width);
  var col = first.col + math.max(1, first.width);
  for (var i = firstIndex + 1; i < cells.length; i++) {
    final next = cells[i];
    if (!_canMergeTerminalText(first, next)) break;
    if (next.col != col) break;
    text.write(next.text);
    final nextWidth = math.max(1, next.width);
    width += nextWidth;
    col += nextWidth;
  }
  return _TerminalTextRun(text: text.toString(), width: width);
}

bool _canMergeTerminalText(TerminalScreenCell a, TerminalScreenCell b) {
  return !a.hidden &&
      !b.hidden &&
      a.text.isNotEmpty &&
      b.text.isNotEmpty &&
      a.width == 1 &&
      b.width == 1 &&
      a.row == b.row &&
      mapEquals(a.fg, b.fg) &&
      mapEquals(a.bg, b.bg) &&
      a.bold == b.bold &&
      a.dim == b.dim &&
      a.italic == b.italic &&
      a.underline == b.underline &&
      a.inverse == b.inverse &&
      a.strikeout == b.strikeout;
}

@immutable
class _TerminalCellPosition implements Comparable<_TerminalCellPosition> {
  const _TerminalCellPosition(this.row, this.col);

  final int row;
  final int col;

  @override
  int compareTo(_TerminalCellPosition other) {
    final rowCompare = row.compareTo(other.row);
    if (rowCompare != 0) return rowCompare;
    return col.compareTo(other.col);
  }

  @override
  bool operator ==(Object other) {
    return other is _TerminalCellPosition &&
        other.row == row &&
        other.col == col;
  }

  @override
  int get hashCode => Object.hash(row, col);
}

@immutable
class _TerminalSelectionRange {
  const _TerminalSelectionRange(this.anchor, this.head);

  final _TerminalCellPosition anchor;
  final _TerminalCellPosition head;

  _TerminalCellPosition get start =>
      anchor.compareTo(head) <= 0 ? anchor : head;
  _TerminalCellPosition get end => anchor.compareTo(head) <= 0 ? head : anchor;

  bool containsCell(TerminalScreenCell cell) {
    final startPoint = start;
    final endPoint = end;
    if (cell.row < startPoint.row || cell.row > endPoint.row) return false;
    final cellStart = cell.col;
    final cellEnd = cell.col + math.max(1, cell.width) - 1;
    if (startPoint.row == endPoint.row) {
      return cellEnd >= startPoint.col && cellStart <= endPoint.col;
    }
    if (cell.row == startPoint.row) return cellEnd >= startPoint.col;
    if (cell.row == endPoint.row) return cellStart <= endPoint.col;
    return true;
  }

  @override
  bool operator ==(Object other) {
    return other is _TerminalSelectionRange &&
        other.anchor == anchor &&
        other.head == head;
  }

  @override
  int get hashCode => Object.hash(anchor, head);
}

void _paintSelection(
  Canvas canvas,
  Size size,
  TerminalScreenSnapshot screen,
  _TerminalSelectionRange? selection,
  double cellWidth,
  double cellHeight,
  double scrollOffsetY,
) {
  if (selection == null) return;
  final paint = Paint()..color = AppColors.accent.withValues(alpha: 0.28);
  for (final cell in screen.cells) {
    if (cell.hidden || !selection.containsCell(cell)) continue;
    final left = cell.col * cellWidth;
    final top = cell.row * cellHeight + scrollOffsetY;
    if (left >= size.width || top >= size.height || top + cellHeight <= 0) {
      continue;
    }
    canvas.drawRect(
      Rect.fromLTWH(left, top, cellWidth * math.max(1, cell.width), cellHeight),
      paint,
    );
  }
}

String? _selectedText(
  TerminalScreenSnapshot? screen,
  _TerminalSelectionRange? selection,
) {
  if (screen == null || selection == null) return null;
  final rows = <int, List<TerminalScreenCell>>{};
  for (final cell in screen.cells) {
    if (cell.hidden || !selection.containsCell(cell)) continue;
    rows.putIfAbsent(cell.row, () => <TerminalScreenCell>[]).add(cell);
  }
  if (rows.isEmpty) return null;
  final output = StringBuffer();
  final rowKeys = rows.keys.toList()..sort();
  for (final row in rowKeys) {
    final cells = rows[row]!..sort((a, b) => a.col.compareTo(b.col));
    var col = cells.first.col;
    final line = StringBuffer();
    for (final cell in cells) {
      while (col < cell.col) {
        line.write(' ');
        col += 1;
      }
      line.write(cell.text);
      col += math.max(1, cell.width);
    }
    if (output.isNotEmpty) output.writeln();
    output.write(line.toString().trimRight());
  }
  final text = output.toString();
  return text.isEmpty ? null : text;
}

// Cursor-only overlay layer. Repaints on blink toggle / cursor move / scroll
// without touching the grid layer above.
class _TerminalCursorPainter extends CustomPainter {
  _TerminalCursorPainter({
    required this.snapshot,
    required this.cellWidth,
    required this.cellHeight,
    required this.fontSize,
    required this.scrollOffsetY,
    required this.cursorBlinkVisible,
  });

  final TerminalScreenSnapshot? snapshot;
  final double cellWidth;
  final double cellHeight;
  final double fontSize;
  final double scrollOffsetY;
  final bool cursorBlinkVisible;

  @override
  void paint(Canvas canvas, Size size) {
    final screen = snapshot;
    if (screen == null) return;
    if (!screen.cursor.visible || !cursorBlinkVisible) return;

    final cursorCell = _cursorCell(screen);
    final cursorTop = (screen.cursor.row * cellHeight + scrollOffsetY)
        .floorToDouble();
    final cursorIsBlock =
        screen.cursor.shape == TerminalScreenCursorShape.block;
    final cursorCellCol = cursorIsBlock && cursorCell != null
        ? cursorCell.col
        : screen.cursor.col;
    final cursorCellWidth = cursorIsBlock && cursorCell != null
        ? cursorCell.width
        : 1;
    final cursorLeft = (cursorCellCol * cellWidth).floorToDouble();
    final cursorRect = Rect.fromLTWH(
      cursorLeft,
      cursorTop,
      (cellWidth * cursorCellWidth).roundToDouble().clamp(1.0, double.infinity),
      cellHeight.roundToDouble().clamp(1.0, double.infinity),
    );
    if (cursorRect.right <= 0 ||
        cursorRect.left >= size.width ||
        cursorRect.bottom <= 0 ||
        cursorRect.top >= size.height) {
      return;
    }
    _paintCursor(canvas, cursorRect, screen.cursor.shape);
    if (cursorIsBlock && cursorCell != null && cursorCell.text.isNotEmpty) {
      _paintCellText(
        textPainter: TextPainter(textDirection: TextDirection.ltr),
        canvas: canvas,
        cell: cursorCell,
        left: cursorLeft,
        top: cursorTop,
        color: AppColors.bgBase,
        clipRight: size.width,
      );
    }
  }

  @override
  bool shouldRepaint(covariant _TerminalCursorPainter oldDelegate) {
    return cursorBlinkVisible != oldDelegate.cursorBlinkVisible ||
        snapshot != oldDelegate.snapshot ||
        scrollOffsetY != oldDelegate.scrollOffsetY ||
        cellWidth != oldDelegate.cellWidth ||
        cellHeight != oldDelegate.cellHeight ||
        fontSize != oldDelegate.fontSize;
  }

  void _paintCursor(
    Canvas canvas,
    Rect bounds,
    TerminalScreenCursorShape shape,
  ) {
    final paint = Paint()..color = AppColors.accent.withValues(alpha: 0.56);
    switch (shape) {
      case TerminalScreenCursorShape.beam:
        canvas.drawRect(
          Rect.fromLTWH(bounds.left, bounds.top, 2, bounds.height),
          paint,
        );
      case TerminalScreenCursorShape.underline:
        canvas.drawRect(
          Rect.fromLTWH(bounds.left, bounds.bottom - 2, bounds.width, 2),
          paint,
        );
      case TerminalScreenCursorShape.hollowBlock:
        canvas.drawRect(
          bounds.deflate(0.5),
          Paint()
            ..color = AppColors.accent.withValues(alpha: 0.72)
            ..style = PaintingStyle.stroke
            ..strokeWidth = 1,
        );
      case TerminalScreenCursorShape.block:
        canvas.drawRect(
          bounds,
          Paint()..color = AppColors.accent.withValues(alpha: 0.88),
        );
    }
  }

  TerminalScreenCell? _cursorCell(TerminalScreenSnapshot screen) {
    for (final cell in screen.cells) {
      if (cell.hidden || cell.text.isEmpty) continue;
      if (cell.row != screen.cursor.row) continue;
      if (screen.cursor.col >= cell.col &&
          screen.cursor.col < cell.col + cell.width) {
        return cell;
      }
    }
    return null;
  }

  void _paintCellText({
    required TextPainter textPainter,
    required Canvas canvas,
    required TerminalScreenCell cell,
    required double left,
    required double top,
    required Color color,
    required double clipRight,
  }) {
    textPainter.text = TextSpan(
      text: cell.text,
      style: _terminalTextStyle(
        color: color,
        fontSize: fontSize,
        bold: cell.bold,
        italic: cell.italic,
        decoration: TextDecoration.combine([
          if (cell.underline) TextDecoration.underline,
          if (cell.strikeout) TextDecoration.lineThrough,
        ]),
      ),
    );
    _paintTerminalText(
      canvas: canvas,
      textPainter: textPainter,
      left: left,
      top: top,
      width: cellWidth * cell.width,
      height: cellHeight,
      fontSize: fontSize,
      clipRight: clipRight,
    );
  }
}

void _paintTerminalText({
  required Canvas canvas,
  required TextPainter textPainter,
  required double left,
  required double top,
  required double width,
  required double height,
  required double fontSize,
  required double clipRight,
}) {
  textPainter.layout(maxWidth: double.infinity);
  final dx = textPainter.width <= width
      ? math.max(0.0, (width - textPainter.width) / 2)
      : 0.0;
  canvas.save();
  canvas.clipRect(Rect.fromLTRB(left, top, clipRight, top + height));
  textPainter.paint(canvas, Offset(left + dx, top + (height - fontSize) / 2));
  canvas.restore();
}
