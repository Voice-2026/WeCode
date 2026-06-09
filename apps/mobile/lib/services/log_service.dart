import 'package:flutter/foundation.dart';

enum CoduxLogLevel { debug, info, warn, error, off }

CoduxLogLevel coduxLogLevelFromName(String value) => CoduxLog._parse(value);

class CoduxLogEntry {
  const CoduxLogEntry({
    required this.time,
    required this.level,
    required this.message,
  });

  final DateTime time;
  final CoduxLogLevel level;
  final String message;

  String format() {
    final local = time.toLocal().toIso8601String();
    return '[$local] [${level.name}] $message';
  }
}

class CoduxLog {
  CoduxLog._();

  static const int _maxEntries = 800;
  static final List<CoduxLogEntry> _entries = [];

  static const _levelName = String.fromEnvironment(
    'CODUX_LOG_LEVEL',
    defaultValue: 'info',
  );

  static CoduxLogLevel _level = _parse(_levelName);

  static CoduxLogLevel get level => _level;
  static String get nativeLevelName => _level.name;
  static bool get isDebugEnabled => _enabled(CoduxLogLevel.debug);

  static void setLevelName(String value) {
    _level = _parse(value);
  }

  static void debug(String message) => _print(CoduxLogLevel.debug, message);

  static void info(String message) => _print(CoduxLogLevel.info, message);

  static void warn(String message) => _print(CoduxLogLevel.warn, message);

  static void error(String message) => _print(CoduxLogLevel.error, message);

  static List<CoduxLogEntry> snapshot() => List.unmodifiable(_entries);

  static String snapshotText() =>
      snapshot().map((entry) => entry.format()).join('\n');

  static void clear() => _entries.clear();

  static void _print(CoduxLogLevel messageLevel, String message) {
    if (!_enabled(messageLevel)) return;
    _entries.add(
      CoduxLogEntry(
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

  static bool _enabled(CoduxLogLevel messageLevel) {
    if (_level == CoduxLogLevel.off) return false;
    return messageLevel.index >= _level.index;
  }

  static CoduxLogLevel _parse(String value) {
    final normalized = value.trim().toLowerCase();
    for (final item in CoduxLogLevel.values) {
      if (item.name == normalized) return item;
    }
    return CoduxLogLevel.warn;
  }
}
