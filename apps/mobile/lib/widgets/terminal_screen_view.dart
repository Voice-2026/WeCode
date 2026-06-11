import 'dart:math' as math;

import 'package:codux_protocol_ffi/codux_protocol_ffi.dart';
import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';

import '../theme/app_theme.dart';

class TerminalScreenView extends StatefulWidget {
  const TerminalScreenView({
    super.key,
    required this.snapshot,
    required this.keyboardVisible,
    required this.onInput,
    required this.onResize,
    required this.onScrollLines,
    required this.onScrollToBottom,
    required this.onCursorBottom,
  });

  final TerminalScreenSnapshot? snapshot;
  final bool keyboardVisible;
  final ValueChanged<String> onInput;
  final void Function(int cols, int rows) onResize;
  final ValueChanged<int> onScrollLines;
  final VoidCallback onScrollToBottom;
  final ValueChanged<double> onCursorBottom;

  @override
  State<TerminalScreenView> createState() => _TerminalScreenViewState();
}

class _TerminalScreenViewState extends State<TerminalScreenView> {
  final TextEditingController _textController = TextEditingController();
  final FocusNode _inputFocusNode = FocusNode(debugLabel: 'terminal-input');
  bool _followTail = true;

  @override
  void initState() {
    super.initState();
    _textController.addListener(_handleTextInput);
    _syncKeyboardFocus();
  }

  @override
  void didUpdateWidget(covariant TerminalScreenView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.keyboardVisible != oldWidget.keyboardVisible) {
      _syncKeyboardFocus();
    }
    if (widget.snapshot?.data != oldWidget.snapshot?.data &&
        _followTail &&
        widget.snapshot?.displayOffset != 0) {
      widget.onScrollToBottom();
    }
  }

  @override
  void dispose() {
    _textController.removeListener(_handleTextInput);
    _textController.dispose();
    _inputFocusNode.dispose();
    super.dispose();
  }

  void _syncKeyboardFocus() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      if (widget.keyboardVisible) {
        _inputFocusNode.requestFocus();
      } else if (_inputFocusNode.hasFocus) {
        _inputFocusNode.unfocus();
      }
    });
  }

  void _handleTextInput() {
    final value = _textController.value;
    if (value.text.isEmpty) return;
    if (value.composing.isValid && !value.composing.isCollapsed) {
      return;
    }
    widget.onInput(value.text);
    _textController.value = TextEditingValue.empty;
  }

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () => _inputFocusNode.requestFocus(),
      onVerticalDragUpdate: (details) {
        final lines = (details.delta.dy / 12).round();
        if (lines == 0) return;
        _followTail = false;
        widget.onScrollLines(lines);
      },
      onVerticalDragEnd: (_) {
        if (widget.snapshot?.displayOffset == 0 && !_followTail) {
          setState(() => _followTail = true);
        }
      },
      child: LayoutBuilder(
        builder: (context, constraints) {
          const fontSize = 12.0;
          const lineHeight = 1.22;
          final cellWidth = _terminalCellWidth(context, fontSize);
          final cellHeight = fontSize * lineHeight;
          final cols = math.max(20, constraints.maxWidth ~/ cellWidth);
          final rows = math.max(8, constraints.maxHeight ~/ cellHeight);
          WidgetsBinding.instance.addPostFrameCallback((_) {
            widget.onResize(cols, rows);
            final screen = widget.snapshot;
            if (screen != null) {
              widget.onCursorBottom((screen.cursor.row + 1) * cellHeight);
            }
          });
          final snapshot = widget.snapshot;

          return ClipRect(
            child: Stack(
              children: [
                Positioned.fill(
                  child: Listener(
                    onPointerSignal: (event) {
                      if (event is PointerScrollEvent) {
                        final lines = (event.scrollDelta.dy / cellHeight)
                            .round();
                        if (lines == 0) return;
                        _followTail = lines < 0 ? false : _followTail;
                        widget.onScrollLines(lines);
                      }
                    },
                    child: CustomPaint(
                      size: Size.infinite,
                      painter: _TerminalScreenPainter(
                        snapshot: snapshot,
                        cellWidth: cellWidth,
                        cellHeight: cellHeight,
                        fontSize: fontSize,
                      ),
                    ),
                  ),
                ),
                Positioned.fill(
                  child: IgnorePointer(
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        border: Border(
                          right: BorderSide(
                            color: (snapshot?.displayOffset ?? 0) == 0
                                ? Colors.transparent
                                : AppColors.accent.withValues(alpha: 0.35),
                            width: 2,
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  left: 0,
                  top: 0,
                  width: 1,
                  height: 1,
                  child: Opacity(
                    opacity: 0,
                    child: EditableText(
                      controller: _textController,
                      focusNode: _inputFocusNode,
                      style: const TextStyle(
                        color: Colors.transparent,
                        fontSize: 1,
                      ),
                      cursorColor: Colors.transparent,
                      backgroundCursorColor: Colors.transparent,
                      keyboardType: TextInputType.text,
                      autocorrect: false,
                      enableSuggestions: false,
                    ),
                  ),
                ),
              ],
            ),
          );
        },
      ),
    );
  }
}

double _terminalCellWidth(BuildContext context, double fontSize) {
  final painter = TextPainter(
    text: TextSpan(
      text: 'M',
      style: TextStyle(fontFamily: 'SF Mono', fontSize: fontSize, height: 1),
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
  });

  final TerminalScreenSnapshot? snapshot;
  final double cellWidth;
  final double cellHeight;
  final double fontSize;

  @override
  void paint(Canvas canvas, Size size) {
    final screen = snapshot;
    canvas.drawRect(Offset.zero & size, Paint()..color = AppColors.bgBase);
    if (screen == null) return;

    final defaultFg = AppColors.textPrimary;
    final defaultBg = AppColors.bgBase;
    final textPainter = TextPainter(textDirection: TextDirection.ltr);

    for (final cell in screen.cells) {
      if (cell.hidden || cell.text.isEmpty) continue;
      final left = cell.col * cellWidth;
      final top = cell.row * cellHeight;
      if (left >= size.width || top >= size.height) continue;
      final fg = _screenColor(cell.fg, defaultFg);
      final bg = _screenColor(cell.bg, defaultBg);
      if (bg != defaultBg || cell.inverse) {
        canvas.drawRect(
          Rect.fromLTWH(left, top, cellWidth * cell.width, cellHeight),
          Paint()..color = cell.inverse ? fg : bg,
        );
      }
      textPainter.text = TextSpan(
        text: cell.text,
        style: TextStyle(
          color: cell.inverse ? bg : fg,
          fontFamily: 'SF Mono',
          fontSize: fontSize,
          height: 1,
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

    if (screen.cursor.visible) {
      final cursorLeft = screen.cursor.col * cellWidth;
      final cursorTop = screen.cursor.row * cellHeight;
      canvas.drawRect(
        Rect.fromLTWH(cursorLeft, cursorTop, cellWidth, cellHeight),
        Paint()..color = AppColors.accent.withValues(alpha: 0.56),
      );
    }
  }

  @override
  bool shouldRepaint(covariant _TerminalScreenPainter oldDelegate) {
    return snapshot != oldDelegate.snapshot ||
        cellWidth != oldDelegate.cellWidth ||
        cellHeight != oldDelegate.cellHeight ||
        fontSize != oldDelegate.fontSize;
  }
}

Color _screenColor(Map<String, dynamic> value, Color fallback) {
  switch ('${value['kind'] ?? ''}') {
    case 'rgb':
      return Color.fromARGB(
        255,
        _channel(value['r']),
        _channel(value['g']),
        _channel(value['b']),
      );
    case 'indexed':
      return _indexedColor(value['index']);
    case 'named':
      return _namedColor('${value['name'] ?? ''}', fallback);
    default:
      return fallback;
  }
}

int _channel(Object? value) {
  if (value is num) return value.toInt().clamp(0, 255);
  return int.tryParse('${value ?? ''}')?.clamp(0, 255) ?? 0;
}

Color _indexedColor(Object? value) {
  final index = value is num ? value.toInt() : int.tryParse('${value ?? ''}');
  if (index == null) return AppColors.textPrimary;
  return _ansiIndexedColor(index);
}

Color _ansiIndexedColor(int index) {
  const basic = [
    Color(0xFF0D1117),
    Color(0xFFFF6B6B),
    Color(0xFF69DB7C),
    Color(0xFFFFD43B),
    Color(0xFF74C0FC),
    Color(0xFFE599F7),
    Color(0xFF66D9E8),
    Color(0xFFE6EDF3),
    Color(0xFF6E7681),
    Color(0xFFFF8787),
    Color(0xFF8CE99A),
    Color(0xFFFFE066),
    Color(0xFFA5D8FF),
    Color(0xFFF3B4FF),
    Color(0xFF99E9F2),
    Color(0xFFF8F9FA),
  ];
  if (index < basic.length) return basic[index.clamp(0, basic.length - 1)];
  if (index >= 16 && index <= 231) {
    final cube = index - 16;
    final r = cube ~/ 36;
    final g = (cube % 36) ~/ 6;
    final b = cube % 6;
    int channel(int value) => value == 0 ? 0 : 55 + value * 40;
    return Color.fromARGB(255, channel(r), channel(g), channel(b));
  }
  if (index >= 232 && index <= 255) {
    final value = 8 + (index - 232) * 10;
    return Color.fromARGB(255, value, value, value);
  }
  return AppColors.textPrimary;
}

Color _namedColor(String name, Color fallback) {
  switch (name) {
    case 'Black':
    case 'DimBlack':
      return const Color(0xFF000000);
    case 'Red':
    case 'DimRed':
    case 'BrightRed':
      return const Color(0xFFFF6B6B);
    case 'Green':
    case 'DimGreen':
    case 'BrightGreen':
      return const Color(0xFF69DB7C);
    case 'Yellow':
    case 'DimYellow':
    case 'BrightYellow':
      return const Color(0xFFFFD43B);
    case 'Blue':
    case 'DimBlue':
    case 'BrightBlue':
      return const Color(0xFF74C0FC);
    case 'Magenta':
    case 'DimMagenta':
    case 'BrightMagenta':
      return const Color(0xFFE599F7);
    case 'Cyan':
    case 'DimCyan':
    case 'BrightCyan':
      return const Color(0xFF66D9E8);
    case 'White':
    case 'DimWhite':
    case 'BrightWhite':
    case 'BrightForeground':
      return const Color(0xFFF8F9FA);
    default:
      return name == 'Background' ? AppColors.bgBase : fallback;
  }
}
