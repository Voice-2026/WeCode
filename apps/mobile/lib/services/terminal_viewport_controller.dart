import '../models/remote_models.dart';
import 'log_service.dart';

class TerminalViewportResize {
  const TerminalViewportResize({required this.cols, required this.rows});

  final int cols;
  final int rows;
}

class TerminalViewportController {
  int? _lastCols;
  int? _lastRows;
  int? _pendingCols;
  int? _pendingRows;
  String? _owner;
  int _generation = 0;

  String? get owner => _owner;
  int get generation => _generation;
  int? get pendingCols => _pendingCols;
  int? get pendingRows => _pendingRows;

  void resetSizes() {
    _lastCols = null;
    _lastRows = null;
    _pendingCols = null;
    _pendingRows = null;
  }

  bool applyRemoteState(RelayEnvelope message) {
    final payload = message.payload;
    if (payload is! Map) return false;
    final nextGeneration = _intValue(payload['generation']) ?? 0;
    if (nextGeneration < _generation) return false;
    _generation = nextGeneration;
    _owner = payload['owner']?.toString();
    final cols = _intValue(payload['cols']);
    final rows = _intValue(payload['rows']);
    CoduxLog.debug(
      '[codux-flutter-terminal] viewport owner=${_owner ?? ''} size=${cols ?? 0}x${rows ?? 0} generation=$_generation session=${message.sessionId ?? ''}',
    );
    return true;
  }

  TerminalViewportResize? resize({
    required int cols,
    required int rows,
    required bool keyboardVisible,
  }) {
    if (cols <= 0 || rows <= 0) return null;
    _pendingCols = cols;
    _pendingRows = rows;
    final nextRows = keyboardVisible ? (_lastRows ?? rows) : rows;
    if (_lastCols == cols && _lastRows == nextRows) return null;
    _lastCols = cols;
    _lastRows = nextRows;
    return TerminalViewportResize(cols: cols, rows: nextRows);
  }

  TerminalViewportResize? flushPending({required bool force}) {
    final cols = _pendingCols;
    final rows = _pendingRows;
    if (cols == null || rows == null || cols <= 0 || rows <= 0) return null;
    if (!force && _lastCols == cols && _lastRows == rows) return null;
    _lastCols = cols;
    _lastRows = rows;
    return TerminalViewportResize(cols: cols, rows: rows);
  }
}

int? _intValue(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  return int.tryParse('${value ?? ''}');
}
