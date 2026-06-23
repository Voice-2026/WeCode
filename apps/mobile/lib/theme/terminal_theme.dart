import 'package:flutter/material.dart';

import 'app_theme.dart';

class TerminalPaintColors {
  const TerminalPaintColors({
    required this.fg,
    required this.bg,
    required this.drawBackground,
  });

  final Color fg;
  final Color bg;
  final bool drawBackground;
}

class TerminalTheme {
  // Resolved cell colors are a pure function of (fg, bg, inverse) plus the
  // compile-time AppColors. The painter calls this once per visible cell on
  // every repaint, and each call runs several computeLuminance() passes. A
  // terminal screen only has a handful of distinct color combos, so memoizing
  // by a compact value key removes nearly all of that per-cell luminance work.
  static final Map<String, TerminalPaintColors> _cellColorCache = {};

  static String _colorKey(Map<String, dynamic> value) {
    final kind = '${value['kind'] ?? ''}';
    return switch (kind) {
      'rgb' => 'r${value['r']},${value['g']},${value['b']}',
      'indexed' => 'i${value['index']}',
      'named' => 'n${value['name']}',
      _ => kind,
    };
  }

  static TerminalPaintColors resolveCellColors({
    required Map<String, dynamic> fg,
    required Map<String, dynamic> bg,
    required bool inverse,
    bool bold = false,
    bool dim = false,
  }) {
    final cacheKey = '${_colorKey(fg)}|${_colorKey(bg)}|$inverse|$bold|$dim';
    final cached = _cellColorCache[cacheKey];
    if (cached != null) return cached;
    final resolved = _resolveCellColors(
      fg: fg,
      bg: bg,
      inverse: inverse,
      bold: bold,
      dim: dim,
    );
    if (_cellColorCache.length > 512) _cellColorCache.clear();
    _cellColorCache[cacheKey] = resolved;
    return resolved;
  }

  static TerminalPaintColors _resolveCellColors({
    required Map<String, dynamic> fg,
    required Map<String, dynamic> bg,
    required bool inverse,
    required bool bold,
    required bool dim,
  }) {
    final foreground = _resolveScreenColor(fg, AppColors.terminalText);
    var background = _resolveScreenColor(bg, AppColors.terminalBg);
    var fgColor = foreground.color;
    var bgColor = background.color;
    var drawBackground = !background.isDefault && bgColor != AppColors.terminalBg;

    if (background.isHostLightSurface) {
      bgColor = AppColors.terminalElevated;
      drawBackground = true;
      background = background.copyWith(color: bgColor);
    }

    if (inverse) {
      var inverseFg = background.color;
      if (bold) {
        inverseFg = _boldForeground(bg, inverseFg);
      }
      inverseFg = _ensureReadable(inverseFg, foreground.color);
      if (dim) {
        inverseFg = _dimColor(inverseFg, foreground.color);
      }
      return TerminalPaintColors(
        fg: inverseFg,
        bg: foreground.color,
        drawBackground: true,
      );
    }

    if (bold) {
      fgColor = _boldForeground(fg, fgColor);
    }
    if (!foreground.isRgb) {
      fgColor = _ensureReadable(fgColor, bgColor);
    }
    if (dim) {
      fgColor = _dimColor(fgColor, bgColor);
    }

    return TerminalPaintColors(
      fg: fgColor,
      bg: bgColor,
      drawBackground: drawBackground,
    );
  }
}

class _ResolvedScreenColor {
  const _ResolvedScreenColor({
    required this.color,
    required this.isDefault,
    required this.isRgb,
    required this.isHostLightSurface,
  });

  final Color color;
  final bool isDefault;
  final bool isRgb;
  final bool isHostLightSurface;

  _ResolvedScreenColor copyWith({Color? color}) {
    final nextColor = color ?? this.color;
    return _ResolvedScreenColor(
      color: nextColor,
      isDefault: isDefault,
      isRgb: isRgb,
      isHostLightSurface: _isHostLightSurface(nextColor),
    );
  }
}

_ResolvedScreenColor _resolveScreenColor(
  Map<String, dynamic> value,
  Color fallback,
) {
  final kind = '${value['kind'] ?? ''}';
  final color = switch (kind) {
    'rgb' => Color.fromARGB(
      255,
      _channel(value['r']),
      _channel(value['g']),
      _channel(value['b']),
    ),
    'indexed' => _ansiIndexedColor(value['index']),
    'named' => _namedColor('${value['name'] ?? ''}', fallback),
    _ => fallback,
  };
  return _ResolvedScreenColor(
    color: color,
    isDefault: kind.isEmpty || kind == 'default',
    isRgb: kind == 'rgb',
    isHostLightSurface: kind != 'default' && _isHostLightSurface(color),
  );
}

int _channel(Object? value) {
  if (value is num) return value.toInt().clamp(0, 255);
  return int.tryParse('${value ?? ''}')?.clamp(0, 255) ?? 0;
}

Color _ansiIndexedColor(Object? value) {
  final index = value is num ? value.toInt() : int.tryParse('${value ?? ''}');
  if (index == null) return AppColors.terminalText;
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
  return AppColors.terminalText;
}

Color _boldForeground(Map<String, dynamic> value, Color fallback) {
  if ('${value['kind'] ?? ''}' != 'indexed') return fallback;
  final index = value['index'] is num
      ? (value['index'] as num).toInt()
      : int.tryParse('${value['index'] ?? ''}');
  if (index == null || index < 0 || index >= 8) return fallback;
  return _ansiIndexedColor(index + 8);
}

Color _namedColor(String name, Color fallback) {
  return switch (name) {
    'Black' || 'DimBlack' => const Color(0xFF000000),
    'Red' || 'DimRed' || 'BrightRed' => const Color(0xFFFF6B6B),
    'Green' || 'DimGreen' || 'BrightGreen' => const Color(0xFF69DB7C),
    'Yellow' || 'DimYellow' || 'BrightYellow' => const Color(0xFFFFD43B),
    'Blue' || 'DimBlue' || 'BrightBlue' => const Color(0xFF74C0FC),
    'Magenta' || 'DimMagenta' || 'BrightMagenta' => const Color(0xFFE599F7),
    'Cyan' || 'DimCyan' || 'BrightCyan' => const Color(0xFF66D9E8),
    'White' ||
    'DimWhite' ||
    'BrightWhite' ||
    'BrightForeground' => const Color(0xFFF8F9FA),
    'Background' => AppColors.terminalBg,
    _ => fallback,
  };
}

Color _dimColor(Color fg, Color bg) {
  const blend = 0.4;
  int channel(double foreground, double background) {
    final value = foreground * 255 * (1 - blend) + background * 255 * blend;
    return value.round().clamp(0, 255);
  }

  return Color.fromARGB(
    255,
    channel(fg.r, bg.r),
    channel(fg.g, bg.g),
    channel(fg.b, bg.b),
  );
}

Color _ensureReadable(Color fg, Color bg) {
  if (_contrastRatio(fg, bg) >= 3.0 || !_isNeutral(fg)) return fg;
  return bg.computeLuminance() > 0.5 ? AppColors.terminalBg : AppColors.terminalText;
}

double _contrastRatio(Color a, Color b) {
  final l1 = a.computeLuminance();
  final l2 = b.computeLuminance();
  final lighter = l1 > l2 ? l1 : l2;
  final darker = l1 > l2 ? l2 : l1;
  return (lighter + 0.05) / (darker + 0.05);
}

bool _isHostLightSurface(Color color) {
  return color.computeLuminance() > 0.78 && _isNeutral(color);
}

bool _isNeutral(Color color) {
  final r = (color.r * 255).round();
  final g = (color.g * 255).round();
  final b = (color.b * 255).round();
  final maxChannel = [r, g, b].reduce((a, b) => a > b ? a : b);
  final minChannel = [r, g, b].reduce((a, b) => a < b ? a : b);
  return maxChannel - minChannel <= 24;
}
