import 'package:flutter/foundation.dart';

enum WeCodeLogLevel { debug, info, warn, error, off }

WeCodeLogLevel wecodeLogLevelFromName(String value) => WeCodeLog._parse(value);

class WeCodeLogEntry {
  const WeCodeLogEntry({
    required this.time,
    required this.level,
    required this.message,
  });

  final DateTime time;
  final WeCodeLogLevel level;
  final String message;

  String format() {
    final local = time.toLocal().toIso8601String();
    return '[$local] [${level.name}] $message';
  }
}

class WeCodeLog {
  WeCodeLog._();

  static const int _maxEntries = 800;
  static final List<WeCodeLogEntry> _entries = [];

  static const _levelName = String.fromEnvironment(
    'WECODE_LOG_LEVEL',
    defaultValue: 'info',
  );

  static WeCodeLogLevel _level = _parse(_levelName);

  static WeCodeLogLevel get level => _level;
  static String get nativeLevelName => _level.name;
  static bool get isDebugEnabled => _enabled(WeCodeLogLevel.debug);

  static void setLevelName(String value) {
    _level = _parse(value);
  }

  static void debug(String message) => _print(WeCodeLogLevel.debug, message);

  static void info(String message) => _print(WeCodeLogLevel.info, message);

  static void warn(String message) => _print(WeCodeLogLevel.warn, message);

  static void error(String message) => _print(WeCodeLogLevel.error, message);

  static List<WeCodeLogEntry> snapshot() => List.unmodifiable(_entries);

  static String snapshotText() =>
      snapshot().map((entry) => entry.format()).join('\n');

  static void clear() => _entries.clear();

  static void _print(WeCodeLogLevel messageLevel, String message) {
    if (!_enabled(messageLevel)) return;
    _entries.add(
      WeCodeLogEntry(
        time: DateTime.now(),
        level: messageLevel,
        message: message,
      ),
    );
    if (_entries.length > _maxEntries) {
      _entries.removeRange(0, _entries.length - _maxEntries);
    }
    debugPrint(message);
  }

  static bool _enabled(WeCodeLogLevel messageLevel) {
    if (_level == WeCodeLogLevel.off) return false;
    return messageLevel.index >= _level.index;
  }

  static WeCodeLogLevel _parse(String value) {
    final normalized = value.trim().toLowerCase();
    for (final item in WeCodeLogLevel.values) {
      if (item.name == normalized) return item;
    }
    return WeCodeLogLevel.warn;
  }
}
