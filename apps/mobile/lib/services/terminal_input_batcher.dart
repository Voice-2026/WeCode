import 'dart:async';

typedef TerminalInputTimerFactory =
    Timer Function(Duration delay, void Function() callback);

class TerminalInputBatcher {
  TerminalInputBatcher({
    required this.send,
    this.flushDelay = const Duration(milliseconds: 2),
    this.maxBatchCharacters = 128,
    TerminalInputTimerFactory? timerFactory,
  }) : _timerFactory = timerFactory ?? Timer.new;

  final void Function(String data) send;
  final Duration flushDelay;
  final int maxBatchCharacters;
  final TerminalInputTimerFactory _timerFactory;

  final StringBuffer _buffer = StringBuffer();
  Timer? _flushTimer;

  String get pendingData => _buffer.toString();
  bool get hasPendingData => _buffer.isNotEmpty;

  void add(String data) {
    if (data.isEmpty) return;
    if (_shouldSendImmediately(data)) {
      flush();
      send(data);
      return;
    }
    _buffer.write(data);
    if (_buffer.length >= maxBatchCharacters) {
      flush();
      return;
    }
    _scheduleFlush();
  }

  void flush() {
    _flushTimer?.cancel();
    _flushTimer = null;
    if (_buffer.isEmpty) return;
    final data = _buffer.toString();
    _buffer.clear();
    send(data);
  }

  void reset() {
    _flushTimer?.cancel();
    _flushTimer = null;
    _buffer.clear();
  }

  void dispose() {
    reset();
  }

  void _scheduleFlush() {
    _flushTimer?.cancel();
    _flushTimer = _timerFactory(flushDelay, flush);
  }

  bool _shouldSendImmediately(String data) {
    if (data.runes.length == 1) return true;
    for (final codeUnit in data.codeUnits) {
      if (codeUnit < 0x20 || codeUnit == 0x7f) return true;
    }
    return false;
  }
}
