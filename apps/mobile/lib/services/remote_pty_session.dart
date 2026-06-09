class RemotePtySnapshot {
  const RemotePtySnapshot({
    required this.sessionId,
    required this.content,
    required this.bufferLength,
    required this.sequence,
  });

  final String sessionId;
  final String content;
  final int bufferLength;
  final int sequence;
}

class RemotePtySession<T> {
  RemotePtySession(this.sessionId, {required this.maxCachedChars});

  final String sessionId;
  final int maxCachedChars;
  String _content = '';
  int _bufferLength = 0;
  int _sequence = 0;
  bool awaitingSnapshot = false;
  _RemotePtyPageBuffer? _pageBuffer;
  final Map<int, T> _heldSequencedLive = {};
  final List<T> _heldUnsequencedLive = [];

  String get content => _content;
  int get bufferLength => _bufferLength;
  int get sequence => _sequence;
  bool get isRestoringSnapshot => awaitingSnapshot || _pageBuffer != null;

  RemotePtySnapshot snapshot() => RemotePtySnapshot(
    sessionId: sessionId,
    content: _content,
    bufferLength: _bufferLength,
    sequence: _sequence,
  );

  void resetTransient({bool resetSequence = false}) {
    awaitingSnapshot = false;
    _pageBuffer = null;
    _heldSequencedLive.clear();
    _heldUnsequencedLive.clear();
    if (resetSequence) _sequence = 0;
  }

  void requireSnapshot() {
    awaitingSnapshot = true;
    _pageBuffer = null;
    _heldSequencedLive.clear();
    _heldUnsequencedLive.clear();
  }

  void setSequence(int sequence) {
    _sequence = sequence;
  }

  bool holdLive({required int? sequence, required T output}) {
    if (!awaitingSnapshot) return false;
    if (sequence == null) {
      _heldUnsequencedLive.add(output);
    } else {
      _heldSequencedLive.putIfAbsent(sequence, () => output);
    }
    return true;
  }

  RemotePtySnapshotPageResult acceptSnapshotPage({
    required String data,
    required int offset,
    required int? bufferLength,
    required bool truncated,
  }) {
    final pageBuffer = offset == 0 || _pageBuffer == null
        ? _RemotePtyPageBuffer(bufferLength, nextOffset: offset)
        : _pageBuffer!;
    if (offset == 0) {
      _pageBuffer = pageBuffer;
    }
    final accepted = pageBuffer.accept(
      data: data,
      offset: offset,
      bufferLength: bufferLength,
      truncated: truncated,
    );
    if (!accepted.accepted) {
      _pageBuffer = null;
      return accepted;
    }
    if (accepted.ready) {
      _pageBuffer = null;
    } else {
      _pageBuffer = pageBuffer;
      _bufferLength = accepted.nextOffset;
    }
    return accepted;
  }

  List<T> replaceFromSnapshot({
    required String content,
    required int? bufferLength,
    required int? sequence,
  }) {
    _content = _trimToCacheLimit(content);
    if (bufferLength != null) _bufferLength = bufferLength;
    final baseSequence = sequence ?? _sequence;
    _sequence = baseSequence;
    awaitingSnapshot = false;
    _pageBuffer = null;
    final keys = _heldSequencedLive.keys.toList()..sort();
    final replay = <T>[];
    for (final key in keys) {
      if (key > baseSequence) {
        final output = _heldSequencedLive[key];
        if (output != null) replay.add(output);
      }
    }
    replay.addAll(_heldUnsequencedLive);
    _heldSequencedLive.clear();
    _heldUnsequencedLive.clear();
    return replay;
  }

  void appendLive({
    required String data,
    required int? bufferLength,
    required int? sequence,
  }) {
    if (data.isNotEmpty) {
      _content = _trimToCacheLimit(_content + data);
    }
    if (bufferLength != null) _bufferLength = bufferLength;
    if (sequence != null) _sequence = sequence;
  }

  void clear() {
    _content = '';
    _bufferLength = 0;
    _sequence = 0;
    awaitingSnapshot = false;
    _pageBuffer = null;
    _heldSequencedLive.clear();
    _heldUnsequencedLive.clear();
  }

  String _trimToCacheLimit(String value) {
    final runes = value.runes.toList(growable: false);
    if (runes.length <= maxCachedChars) return value;
    return String.fromCharCodes(runes.skip(runes.length - maxCachedChars));
  }
}

class RemotePtySnapshotPageResult {
  const RemotePtySnapshotPageResult({
    required this.accepted,
    required this.ready,
    required this.data,
    required this.nextOffset,
    required this.progress,
  });

  final bool accepted;
  final bool ready;
  final String data;
  final int nextOffset;
  final double? progress;
}

class _RemotePtyPageBuffer {
  _RemotePtyPageBuffer(this.bufferLength, {required this.nextOffset});

  final StringBuffer _buffer = StringBuffer();
  int nextOffset;
  int? bufferLength;

  RemotePtySnapshotPageResult accept({
    required String data,
    required int offset,
    required int? bufferLength,
    required bool truncated,
  }) {
    this.bufferLength ??= bufferLength;
    if (offset != nextOffset) {
      return RemotePtySnapshotPageResult(
        accepted: false,
        ready: false,
        data: '',
        nextOffset: nextOffset,
        progress: null,
      );
    }
    _buffer.write(data);
    nextOffset += data.runes.length;
    final expectedLength = bufferLength ?? this.bufferLength;
    final completeByLength =
        expectedLength != null && nextOffset >= expectedLength;
    final ready = !truncated || completeByLength;
    return RemotePtySnapshotPageResult(
      accepted: true,
      ready: ready,
      data: ready ? _buffer.toString() : '',
      nextOffset: nextOffset,
      progress: expectedLength == null || expectedLength <= 0
          ? null
          : (nextOffset / expectedLength).clamp(0.0, 1.0),
    );
  }
}

class RemotePtySessionStore<T> {
  RemotePtySessionStore({required this.maxCachedChars});

  final int maxCachedChars;
  final Map<String, RemotePtySession<T>> _sessions = {};

  RemotePtySession<T> session(String sessionId) => _sessions.putIfAbsent(
    sessionId,
    () => RemotePtySession<T>(sessionId, maxCachedChars: maxCachedChars),
  );

  RemotePtySnapshot? snapshot(String sessionId) =>
      _sessions[sessionId]?.snapshot();

  String? content(String sessionId) {
    final content = _sessions[sessionId]?.content;
    return content == null || content.isEmpty ? null : content;
  }

  int bufferLength(String sessionId) => _sessions[sessionId]?.bufferLength ?? 0;
  int sequence(String sessionId) => _sessions[sessionId]?.sequence ?? 0;

  void remove(String sessionId) {
    _sessions.remove(sessionId);
  }

  void clear() {
    _sessions.clear();
  }
}
