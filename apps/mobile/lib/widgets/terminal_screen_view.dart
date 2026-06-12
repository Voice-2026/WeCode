import 'dart:async';
import 'dart:math' as math;

import 'package:codux_protocol_ffi/codux_protocol_ffi.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
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
  });

  final TerminalScreenSnapshot? snapshot;
  final bool keyboardRequested;
  final bool scrollEnabled;
  final ValueChanged<String> onInput;
  final void Function(int cols, int rows) onResize;
  final void Function(double pixels, double cellHeight) onScrollPixels;
  final VoidCallback onSettleScroll;
  final VoidCallback onScrollToBottom;
  final ValueChanged<double> onCursorBottom;

  @override
  State<TerminalScreenView> createState() => _TerminalScreenViewState();
}

class _TerminalScreenViewState extends State<TerminalScreenView>
    with SingleTickerProviderStateMixin
    implements TextInputClient {
  late final AnimationController _inertiaController;
  final FocusNode _keyboardFocusNode = FocusNode(
    debugLabel: 'terminal-screen-input',
  );
  TextInputConnection? _inputConnection;
  TextEditingValue _editingValue = _terminalInputSentinelValue;
  bool _followTail = true;
  bool _coreScrollFlushScheduled = false;
  double _pendingCoreScrollPixels = 0;
  double _visualScrollOffset = 0;
  double _unconfirmedCoreScrollPixels = 0;
  double? _lastSnapshotScrollPosition;
  double _lastInertiaPosition = 0;
  double _dragPixels = 0;
  double _fallbackEventTimeMs = 0;
  bool _cursorBlinkVisible = true;
  Timer? _cursorBlinkTimer;
  final List<_TerminalDragSample> _dragSamples = [];

  @override
  void initState() {
    super.initState();
    _inertiaController = AnimationController.unbounded(vsync: this)
      ..addListener(_handleInertiaTick);
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
      widget.onScrollToBottom();
    }
    if (_cursorSignature(widget.snapshot) !=
        _cursorSignature(oldWidget.snapshot)) {
      _resetCursorBlink();
    }
    _syncVisualScrollFromSnapshot(oldWidget.snapshot, widget.snapshot);
    _syncFollowTailFromSnapshot();
  }

  @override
  void dispose() {
    _cursorBlinkTimer?.cancel();
    _closeKeyboardConnection();
    _keyboardFocusNode.dispose();
    _inertiaController.dispose();
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
    if (snapshot == null) {
      return Rect.fromLTWH(0, 0, 1, _terminalCellHeight);
    }
    final fontSize = _terminalFontSize;
    final cellWidth = _terminalCellWidth(context, fontSize);
    final left = (snapshot.cursor.col * cellWidth).clamp(0.0, size.width);
    final top = (snapshot.cursor.row * _terminalCellHeight).clamp(
      0.0,
      size.height,
    );
    return Rect.fromLTWH(left, top, 1, _terminalCellHeight);
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        const fontSize = _terminalFontSize;
        final cellWidth = _terminalCellWidth(context, fontSize);
        const cellHeight = _terminalCellHeight;
        final cols = math.max(20, constraints.maxWidth ~/ cellWidth);
        final rows = math.max(8, constraints.maxHeight ~/ cellHeight);
        WidgetsBinding.instance.addPostFrameCallback((_) {
          widget.onResize(cols, rows);
          final connection = _inputConnection;
          if (connection != null && connection.attached) {
            _syncKeyboardGeometry(connection);
          }
          final screen = widget.snapshot;
          if (screen != null) {
            final cursorBottom =
                (screen.cursor.row + 1) * cellHeight +
                screen.scrollPixelOffset +
                _visualScrollOffset;
            widget.onCursorBottom(cursorBottom);
          }
        });
        final snapshot = widget.snapshot;

        return KeyboardListener(
          focusNode: _keyboardFocusNode,
          autofocus: widget.keyboardRequested,
          onKeyEvent: _handleKeyEvent,
          child: ClipRect(
            child: Stack(
              children: [
                Positioned.fill(
                  child: CustomPaint(
                    size: Size.infinite,
                    painter: _TerminalScreenPainter(
                      snapshot: snapshot,
                      cellWidth: cellWidth,
                      cellHeight: cellHeight,
                      fontSize: fontSize,
                      scrollOffsetY:
                          (snapshot?.scrollPixelOffset ?? 0) +
                          _visualScrollOffset,
                      cursorBlinkVisible: _cursorBlinkVisible,
                    ),
                  ),
                ),
                Positioned.fill(
                  child: _TerminalScrollGestureLayer(
                    enabled: widget.scrollEnabled,
                    onScrollStart: _handleScrollStart,
                    onScrollPixels: _handleScrollPixels,
                    onPointerScrollPixels: _handlePointerScrollPixels,
                    onScrollEnd: _handleScrollEnd,
                    onScrollCancel: _handleScrollCancel,
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  bool _scrollByPixels(double pixels, double cellHeight) {
    if (!pixels.isFinite || pixels == 0 || cellHeight <= 0) return false;
    final scrollPosition = _currentScrollPosition(cellHeight);
    if (scrollPosition != null && pixels < 0) {
      if (scrollPosition <= _terminalScrollEpsilon) {
        return false;
      }
      pixels = math.max(pixels, -scrollPosition);
      if (pixels.abs() < _terminalScrollEpsilon) {
        return false;
      }
    }
    if (pixels > 0) _followTail = false;
    setState(() {
      _visualScrollOffset += pixels;
      _pendingCoreScrollPixels += pixels;
    });
    _scheduleCoreScrollFlush();
    return true;
  }

  void _handleScrollPixels(double pixels, Duration? sourceTimeStamp) {
    _dragPixels += pixels;
    _recordDragSample(sourceTimeStamp);
    _scrollByPixels(pixels, _terminalCellHeight);
  }

  void _handlePointerScrollPixels(double pixels) {
    _stopInertia();
    _scrollByPixels(pixels, _terminalCellHeight);
  }

  void _handleScrollStart(Duration? sourceTimeStamp) {
    _stopInertia();
    _dragPixels = 0;
    _dragSamples
      ..clear()
      ..add(_TerminalDragSample(_eventTimeMs(sourceTimeStamp), _dragPixels));
  }

  void _handleScrollEnd(double velocity) {
    _flushCoreScroll();
    final inertiaVelocity = _resolveInertiaVelocity(velocity);
    if (inertiaVelocity != 0) {
      _startInertia(inertiaVelocity);
    }
    _dragPixels = 0;
    _dragSamples.clear();
    if (_inertiaController.isAnimating) return;
    _syncFollowTailFromSnapshot();
    widget.onSettleScroll();
  }

  void _handleScrollCancel() {
    _stopInertia();
    _dragPixels = 0;
    _dragSamples.clear();
    _flushCoreScroll();
    _syncFollowTailFromSnapshot();
    widget.onSettleScroll();
  }

  double _resolveInertiaVelocity(double primaryVelocity) {
    final sampledVelocity = _sampledReleaseVelocity();
    final velocity = _resolveReleaseVelocity(primaryVelocity, sampledVelocity);
    if (!velocity.isFinite) return 0;

    final speed = velocity.abs();
    final speedT = _smoothStep(speed / _terminalFullInertiaVelocity);
    final distanceT = _smoothStep(
      _dragPixels.abs() / _terminalFullInertiaDistance,
    );
    final distanceScale =
        _terminalMinInertiaDistanceScale +
        (1 - _terminalMinInertiaDistanceScale) * distanceT;
    final resolvedVelocity =
        velocity.sign *
        math.min(speed, _terminalMaxInertiaVelocity) *
        speedT *
        distanceScale;

    if (resolvedVelocity.abs() < _terminalMinResolvedInertiaVelocity) {
      return 0;
    }
    return resolvedVelocity;
  }

  double _resolveReleaseVelocity(
    double primaryVelocity,
    double? sampledVelocity,
  ) {
    if (sampledVelocity == null || !sampledVelocity.isFinite) {
      return primaryVelocity;
    }
    if (!primaryVelocity.isFinite || primaryVelocity == 0) {
      return sampledVelocity;
    }
    if (sampledVelocity.sign == primaryVelocity.sign) {
      return sampledVelocity.abs() >= primaryVelocity.abs()
          ? sampledVelocity
          : primaryVelocity;
    }
    return sampledVelocity;
  }

  double? _sampledReleaseVelocity() {
    if (_dragSamples.length < 2) return null;
    final last = _dragSamples.last;
    var first = _dragSamples.first;
    for (final sample in _dragSamples.reversed) {
      final elapsedMs = last.timeMs - sample.timeMs;
      if (elapsedMs >= _terminalMinVelocitySampleMs) {
        first = sample;
      }
      if (elapsedMs >= _terminalVelocitySampleWindowMs) break;
    }

    final elapsedMs = last.timeMs - first.timeMs;
    if (elapsedMs < _terminalMinVelocitySampleMs) return null;
    return (last.pixels - first.pixels) / elapsedMs * 1000;
  }

  void _recordDragSample(Duration? sourceTimeStamp) {
    final timeMs = _eventTimeMs(sourceTimeStamp);
    if (_dragSamples.isNotEmpty && timeMs < _dragSamples.last.timeMs) {
      _dragSamples.clear();
    }
    _dragSamples.add(_TerminalDragSample(timeMs, _dragPixels));
    while (_dragSamples.length > 2 &&
        timeMs - _dragSamples.first.timeMs > _terminalVelocitySampleWindowMs) {
      _dragSamples.removeAt(0);
    }
  }

  double _eventTimeMs(Duration? sourceTimeStamp) {
    final frameTimeStamp =
        SchedulerBinding.instance.currentSystemFrameTimeStamp;
    final timestamp = sourceTimeStamp ?? frameTimeStamp;
    final eventTimeMs = timestamp.inMicroseconds / 1000;
    if (eventTimeMs > _fallbackEventTimeMs) {
      _fallbackEventTimeMs = eventTimeMs;
      return eventTimeMs;
    }
    _fallbackEventTimeMs += _terminalFallbackEventStepMs;
    return _fallbackEventTimeMs;
  }

  void _handleInertiaTick() {
    final delta = _inertiaController.value - _lastInertiaPosition;
    _lastInertiaPosition = _inertiaController.value;
    if (delta.abs() < _terminalScrollEpsilon) return;
    if (!_scrollByPixels(delta, _terminalCellHeight)) {
      _stopInertia();
    }
  }

  void _startInertia(double velocity) {
    if (!velocity.isFinite ||
        velocity.abs() < _terminalMinResolvedInertiaVelocity) {
      return;
    }
    _lastInertiaPosition = 0;
    _inertiaController.value = 0;
    _inertiaController
        .animateWith(
          ClampingScrollSimulation(
            position: 0,
            velocity: velocity.clamp(
              -_terminalMaxInertiaVelocity,
              _terminalMaxInertiaVelocity,
            ),
            friction: _terminalInertiaFriction,
          ),
        )
        .whenCompleteOrCancel(() {
          if (!mounted) return;
          _lastInertiaPosition = 0;
          _flushCoreScroll();
          _syncFollowTailFromSnapshot();
          widget.onSettleScroll();
        });
  }

  void _stopInertia() {
    if (_inertiaController.isAnimating) {
      _inertiaController.stop();
    }
    _lastInertiaPosition = 0;
  }

  void _scheduleCoreScrollFlush() {
    if (_coreScrollFlushScheduled) return;
    _coreScrollFlushScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _coreScrollFlushScheduled = false;
      if (!mounted) return;
      _flushCoreScroll();
    });
  }

  void _flushCoreScroll() {
    final pixels = _pendingCoreScrollPixels;
    if (pixels == 0) return;
    _pendingCoreScrollPixels = 0;
    _unconfirmedCoreScrollPixels += pixels;
    widget.onScrollPixels(pixels, _terminalCellHeight);
  }

  void _syncVisualScrollFromSnapshot(
    TerminalScreenSnapshot? oldSnapshot,
    TerminalScreenSnapshot? newSnapshot,
  ) {
    if (newSnapshot == null) {
      _lastSnapshotScrollPosition = null;
      _unconfirmedCoreScrollPixels = 0;
      return;
    }
    final newPosition = _snapshotScrollPosition(newSnapshot);
    final previousPosition =
        _lastSnapshotScrollPosition ??
        (oldSnapshot == null
            ? newPosition
            : _snapshotScrollPosition(oldSnapshot));
    _lastSnapshotScrollPosition = newPosition;

    final consumedPixels = newPosition - previousPosition;
    final snapshotChanged =
        !identical(oldSnapshot, newSnapshot) && oldSnapshot != newSnapshot;
    if (_unconfirmedCoreScrollPixels == 0) return;
    if (consumedPixels == 0) {
      if (!snapshotChanged) return;
      _stopInertia();
      _visualScrollOffset -= _unconfirmedCoreScrollPixels;
      _unconfirmedCoreScrollPixels = 0;
      if (_visualScrollOffset.abs() < _terminalScrollEpsilon) {
        _visualScrollOffset = 0;
      }
      return;
    }

    final consumedSign = consumedPixels.sign;
    final pendingSign = _unconfirmedCoreScrollPixels.sign;
    if (consumedSign != pendingSign) {
      _stopInertia();
      _unconfirmedCoreScrollPixels = 0;
      return;
    }

    final appliedPixels = math.min(
      consumedPixels.abs(),
      _unconfirmedCoreScrollPixels.abs(),
    );
    final adjustment = consumedSign * appliedPixels;
    _unconfirmedCoreScrollPixels -= adjustment;
    _visualScrollOffset -= adjustment;

    if (_unconfirmedCoreScrollPixels.abs() < _terminalScrollEpsilon) {
      _unconfirmedCoreScrollPixels = 0;
    }
    if (_visualScrollOffset.abs() < _terminalScrollEpsilon) {
      _visualScrollOffset = 0;
    }
  }

  void _syncFollowTailFromSnapshot() {
    final screen = widget.snapshot;
    if (screen == null) return;
    if (screen.displayOffset == 0 && screen.scrollPixelOffset <= 0) {
      _followTail = true;
    }
  }

  double? _currentScrollPosition(double cellHeight) {
    final screen = widget.snapshot;
    if (screen == null) return null;
    return screen.displayOffset * cellHeight +
        screen.scrollPixelOffset +
        _visualScrollOffset;
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

class _TerminalScrollGestureLayer extends StatelessWidget {
  const _TerminalScrollGestureLayer({
    required this.enabled,
    required this.onScrollStart,
    required this.onScrollPixels,
    required this.onPointerScrollPixels,
    required this.onScrollEnd,
    required this.onScrollCancel,
  });

  final bool enabled;
  final ValueChanged<Duration?> onScrollStart;
  final void Function(double pixels, Duration? sourceTimeStamp) onScrollPixels;
  final ValueChanged<double> onPointerScrollPixels;
  final ValueChanged<double> onScrollEnd;
  final VoidCallback onScrollCancel;

  @override
  Widget build(BuildContext context) {
    return Listener(
      behavior: HitTestBehavior.opaque,
      onPointerSignal: (event) {
        if (!enabled) return;
        if (event is PointerScrollEvent) {
          onPointerScrollPixels(-event.scrollDelta.dy);
        }
      },
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        dragStartBehavior: DragStartBehavior.down,
        onVerticalDragStart: enabled
            ? (details) => onScrollStart(details.sourceTimeStamp)
            : null,
        onVerticalDragUpdate: enabled
            ? (details) =>
                  onScrollPixels(details.delta.dy, details.sourceTimeStamp)
            : null,
        onVerticalDragEnd: enabled
            ? (details) => onScrollEnd(details.primaryVelocity ?? 0)
            : null,
        onVerticalDragCancel: enabled ? onScrollCancel : null,
      ),
    );
  }
}

const _terminalFontSize = 11.5;
const _terminalLineHeight = 1.25;
const _terminalCellHeight = _terminalFontSize * _terminalLineHeight;
const _terminalLetterSpacing = 0.0;
const _terminalFontFamily = 'Maple Mono NF CN';
const _terminalMaxInertiaVelocity = 3200.0;
const _terminalInertiaFriction = 0.035;
const _terminalFullInertiaVelocity = 1800.0;
const _terminalMinResolvedInertiaVelocity = 4.0;
const _terminalFullInertiaDistance = _terminalCellHeight * 6;
const _terminalMinInertiaDistanceScale = 0.25;
const _terminalVelocitySampleWindowMs = 120.0;
const _terminalMinVelocitySampleMs = 16.0;
const _terminalFallbackEventStepMs = 16.0;
const _terminalScrollEpsilon = 0.01;
const _terminalCursorBlinkInterval = Duration(milliseconds: 530);
const _terminalInputSentinel = '  ';
const _terminalBackspaceInput = '\u0008';
const _terminalInputSentinelValue = TextEditingValue(
  text: _terminalInputSentinel,
  selection: TextSelection.collapsed(offset: _terminalInputSentinel.length),
);
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

double _snapshotScrollPosition(TerminalScreenSnapshot snapshot) {
  return snapshot.displayOffset * _terminalCellHeight +
      snapshot.scrollPixelOffset;
}

String _cursorSignature(TerminalScreenSnapshot? snapshot) {
  final cursor = snapshot?.cursor;
  if (cursor == null) return '';
  return '${cursor.row}:${cursor.col}:${cursor.visible}:${cursor.shape}';
}

double _smoothStep(double value) {
  final x = value.clamp(0.0, 1.0);
  return x * x * (3 - 2 * x);
}

class _TerminalDragSample {
  const _TerminalDragSample(this.timeMs, this.pixels);

  final double timeMs;
  final double pixels;
}

double _terminalCellWidth(BuildContext context, double fontSize) {
  final painter = TextPainter(
    text: TextSpan(
      text: 'm',
      style: TextStyle(
        fontFamily: _terminalFontFamily,
        fontSize: fontSize,
        height: 1,
        letterSpacing: _terminalLetterSpacing,
      ),
    ),
    textDirection: TextDirection.ltr,
  )..layout();
  return painter.width.clamp(6.0, 16.0);
}

class _TerminalScreenPainter extends CustomPainter {
  _TerminalScreenPainter({
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
    canvas.drawRect(Offset.zero & size, Paint()..color = AppColors.bgBase);
    if (screen == null) return;

    final textPainter = TextPainter(textDirection: TextDirection.ltr);
    final cursorCell = _cursorCell(screen);

    for (final cell in screen.cells) {
      if (cell.hidden || cell.text.isEmpty) continue;
      final left = cell.col * cellWidth;
      final top = cell.row * cellHeight + scrollOffsetY;
      if (left >= size.width || top >= size.height || top + cellHeight <= 0) {
        continue;
      }
      final colors = TerminalTheme.resolveCellColors(
        fg: cell.fg,
        bg: cell.bg,
        inverse: cell.inverse,
      );
      if (colors.drawBackground) {
        canvas.drawRect(
          Rect.fromLTWH(left, top, cellWidth * cell.width, cellHeight),
          Paint()..color = colors.bg,
        );
      }
      textPainter.text = TextSpan(
        text: cell.text,
        style: TextStyle(
          color: colors.fg,
          fontFamily: _terminalFontFamily,
          fontSize: fontSize,
          height: 1,
          letterSpacing: _terminalLetterSpacing,
          fontWeight: cell.bold ? FontWeight.w700 : FontWeight.w400,
          fontStyle: cell.italic ? FontStyle.italic : FontStyle.normal,
          decoration: TextDecoration.combine([
            if (cell.underline) TextDecoration.underline,
            if (cell.strikeout) TextDecoration.lineThrough,
          ]),
        ),
      );
      textPainter.layout(maxWidth: cellWidth * cell.width);
      textPainter.paint(
        canvas,
        Offset(left, top + (cellHeight - fontSize) / 2),
      );
    }

    if (screen.cursor.visible && cursorBlinkVisible) {
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
        (cellWidth * cursorCellWidth).roundToDouble().clamp(
          1.0,
          double.infinity,
        ),
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
          textPainter: textPainter,
          canvas: canvas,
          cell: cursorCell,
          left: cursorLeft,
          top: cursorTop,
          color: AppColors.bgBase,
        );
      }
    }
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
  }) {
    textPainter.text = TextSpan(
      text: cell.text,
      style: TextStyle(
        color: color,
        fontFamily: _terminalFontFamily,
        fontSize: fontSize,
        height: 1,
        letterSpacing: _terminalLetterSpacing,
        fontWeight: cell.bold ? FontWeight.w700 : FontWeight.w400,
        fontStyle: cell.italic ? FontStyle.italic : FontStyle.normal,
        decoration: TextDecoration.combine([
          if (cell.underline) TextDecoration.underline,
          if (cell.strikeout) TextDecoration.lineThrough,
        ]),
      ),
    );
    textPainter.layout(maxWidth: cellWidth * cell.width);
    textPainter.paint(canvas, Offset(left, top + (cellHeight - fontSize) / 2));
  }

  @override
  bool shouldRepaint(covariant _TerminalScreenPainter oldDelegate) {
    return snapshot != oldDelegate.snapshot ||
        cellWidth != oldDelegate.cellWidth ||
        cellHeight != oldDelegate.cellHeight ||
        fontSize != oldDelegate.fontSize ||
        scrollOffsetY != oldDelegate.scrollOffsetY ||
        cursorBlinkVisible != oldDelegate.cursorBlinkVisible;
  }
}
