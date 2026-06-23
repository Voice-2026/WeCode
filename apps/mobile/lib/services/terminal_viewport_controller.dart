import '../models/remote_models.dart';
import 'log_service.dart';

class TerminalViewportResize {
  const TerminalViewportResize({required this.cols, required this.rows});

  final int cols;
  final int rows;
}

class TerminalViewportScrollRequest {
  const TerminalViewportScrollRequest({
    required this.requestId,
    required this.displayOffset,
    required this.maxLines,
    required this.overscanRows,
  });

  final int requestId;
  final int displayOffset;
  final int maxLines;
  final int overscanRows;
}

class _ViewportSize {
  const _ViewportSize(this.cols, this.rows);

  final int cols;
  final int rows;

  bool matches(int cols, int rows) => this.cols == cols && this.rows == rows;
}

class TerminalViewportController {
  int? _lastCols;
  int? _lastRows;
  int? _pendingCols;
  int? _pendingRows;
  final Map<String, _ViewportSize> _sentBySession = {};
  final Map<String, int> _generationBySession = {};
  final Map<String, int> _confirmedOffsetBySession = {};
  final Map<String, int> _desiredOffsetBySession = {};
  final Map<String, int> _latestScrollRequestBySession = {};
  final Map<String, double> _scrollPixelRemainderBySession = {};
  String? _owner;
  int _generation = 0;
  int _scrollRequestCounter = 0;

  String? get owner => _owner;
  int get generation => _generation;
  int? get pendingCols => _pendingCols;
  int? get pendingRows => _pendingRows;

  /// The host's authoritative grid size for [sessionId] as last reported in a
  /// viewport-state message, or null if none seen yet. The host keeps its own
  /// (often taller) row count for remote viewers, so this can differ from the
  /// phone's measured viewport; the local cell screen is sized to this.
  ({int cols, int rows})? reportedSize(String sessionId) {
    final size = _sentBySession[sessionId.trim()];
    if (size == null) return null;
    return (cols: size.cols, rows: size.rows);
  }

  void resetSizes() {
    _lastCols = null;
    _lastRows = null;
    _pendingCols = null;
    _pendingRows = null;
    _sentBySession.clear();
  }

  void resetScroll() {
    _confirmedOffsetBySession.clear();
    _desiredOffsetBySession.clear();
    _latestScrollRequestBySession.clear();
    _scrollPixelRemainderBySession.clear();
  }

  bool applyRemoteState(RelayEnvelope message) {
    final payload = message.payload;
    if (payload is! Map) return false;
    final sessionId = message.sessionId?.trim() ?? '';
    final nextGeneration = _intValue(payload['generation']) ?? 0;
    final currentGeneration = sessionId.isEmpty
        ? _generation
        : (_generationBySession[sessionId] ?? 0);
    if (nextGeneration < currentGeneration) return false;
    _generation = nextGeneration;
    if (sessionId.isNotEmpty) {
      _generationBySession[sessionId] = nextGeneration;
    }
    _owner = payload['owner']?.toString();
    final cols = _intValue(payload['cols']);
    final rows = _intValue(payload['rows']);
    if (sessionId.isNotEmpty &&
        cols != null &&
        rows != null &&
        cols > 0 &&
        rows > 0) {
      _sentBySession[sessionId] = _ViewportSize(cols, rows);
    }
    CoduxLog.debug(
      '[codux-flutter-terminal] viewport owner=${_owner ?? ''} size=${cols ?? 0}x${rows ?? 0} generation=$_generation session=${message.sessionId ?? ''}',
    );
    return true;
  }

  // resize/flushPending only PROPOSE an envelope; the dedup cache is
  // committed via markSent after the caller actually sends it. Committing
  // up front poisoned the cache when a send gate (terminal list not loaded
  // yet) dropped the envelope, permanently suppressing the resize.
  TerminalViewportResize? resize({
    required String sessionId,
    required int cols,
    required int rows,
    required bool keyboardVisible,
  }) {
    final id = sessionId.trim();
    if (id.isEmpty || cols <= 0 || rows <= 0) return null;
    _pendingCols = cols;
    _pendingRows = rows;
    final lastSessionSize = _sentBySession[id];
    final nextRows = keyboardVisible
        ? (lastSessionSize?.rows ?? _lastRows ?? rows)
        : rows;
    if (lastSessionSize?.matches(cols, nextRows) == true) return null;
    return TerminalViewportResize(cols: cols, rows: nextRows);
  }

  TerminalViewportResize? flushPending({
    required String sessionId,
    required bool force,
  }) {
    final id = sessionId.trim();
    if (id.isEmpty) return null;
    final cols = _pendingCols;
    final rows = _pendingRows;
    if (cols == null || rows == null || cols <= 0 || rows <= 0) return null;
    final lastSessionSize = _sentBySession[id];
    if (lastSessionSize?.matches(cols, rows) == true) return null;
    if (!force) {
      if (_lastCols == cols && _lastRows == rows) return null;
    }
    return TerminalViewportResize(cols: cols, rows: rows);
  }

  void markSent(String sessionId, TerminalViewportResize resize) {
    final id = sessionId.trim();
    if (id.isEmpty) return;
    _lastCols = resize.cols;
    _lastRows = resize.rows;
    _sentBySession[id] = _ViewportSize(resize.cols, resize.rows);
  }

  TerminalViewportScrollRequest? requestScrollPixels({
    required String sessionId,
    required double pixels,
    required double cellHeight,
    required int maxLines,
    required int viewportRows,
    required int overscanRows,
  }) {
    final id = sessionId.trim();
    if (id.isEmpty || pixels == 0 || cellHeight <= 0) return null;
    final accumulated =
        (_scrollPixelRemainderBySession[id] ?? 0) + (pixels / cellHeight);
    final delta = accumulated.truncate();
    _scrollPixelRemainderBySession[id] = accumulated - delta;
    if (delta == 0) return null;
    final current =
        _desiredOffsetBySession[id] ?? _confirmedOffsetBySession[id] ?? 0;
    return _requestAbsoluteOffset(
      sessionId: id,
      displayOffset: current + delta,
      maxLines: maxLines,
      viewportRows: viewportRows,
      overscanRows: overscanRows,
      resetPixelRemainder: false,
    );
  }

  TerminalViewportScrollRequest requestAbsoluteScroll({
    required String sessionId,
    required int displayOffset,
    required int maxLines,
    required int viewportRows,
    required int overscanRows,
  }) {
    return _requestAbsoluteOffset(
      sessionId: sessionId.trim(),
      displayOffset: displayOffset,
      maxLines: maxLines,
      viewportRows: viewportRows,
      overscanRows: overscanRows,
      resetPixelRemainder: true,
    );
  }

  bool acceptScrollResponse({
    required String sessionId,
    required int requestId,
    required int displayOffset,
  }) {
    final id = sessionId.trim();
    if (id.isEmpty) return false;
    final latest = _latestScrollRequestBySession[id] ?? 0;
    if (requestId < latest) return false;
    _latestScrollRequestBySession[id] = requestId;
    _confirmedOffsetBySession[id] = displayOffset;
    _desiredOffsetBySession[id] = displayOffset;
    return true;
  }

  TerminalViewportScrollRequest _requestAbsoluteOffset({
    required String sessionId,
    required int displayOffset,
    required int maxLines,
    required int viewportRows,
    required int overscanRows,
    required bool resetPixelRemainder,
  }) {
    final id = sessionId.trim();
    if (id.isEmpty) {
      return const TerminalViewportScrollRequest(
        requestId: 0,
        displayOffset: 0,
        maxLines: 1,
        overscanRows: 0,
      );
    }
    _scrollRequestCounter += 1;
    final maxOffset = _maxDisplayOffset(maxLines, viewportRows);
    final clampedOffset = displayOffset.clamp(0, maxOffset);
    if (resetPixelRemainder) {
      _scrollPixelRemainderBySession.remove(id);
    }
    _desiredOffsetBySession[id] = clampedOffset;
    _latestScrollRequestBySession[id] = _scrollRequestCounter;
    return TerminalViewportScrollRequest(
      requestId: _scrollRequestCounter,
      displayOffset: clampedOffset,
      maxLines: maxLines < 1 ? 1 : maxLines,
      overscanRows: overscanRows < 0 ? 0 : overscanRows,
    );
  }
}

int? _intValue(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  return int.tryParse('${value ?? ''}');
}

int _maxDisplayOffset(int maxLines, int viewportRows) {
  final total = maxLines < 1 ? 1 : maxLines;
  final rows = viewportRows < 1 ? 1 : viewportRows;
  return total > rows ? total - rows : 0;
}
