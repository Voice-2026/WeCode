enum TerminalOutputSequenceAction { accept, duplicate, snapshot }

class TerminalOutputSequenceResult {
  const TerminalOutputSequenceResult({
    required this.action,
    required this.previousSeq,
  });

  final TerminalOutputSequenceAction action;
  final int previousSeq;
  bool get shouldRender =>
      action == TerminalOutputSequenceAction.accept ||
      action == TerminalOutputSequenceAction.snapshot;
}

class TerminalOutputSequencer {
  final Map<String, int> _seqBySession = {};
  final Set<String> _allowNextLiveRebaseSessions = {};

  int sequenceFor(String sessionId) => _seqBySession[sessionId] ?? 0;

  bool isResyncing(String sessionId) => false;

  TerminalOutputSequenceResult observe({
    required String sessionId,
    required bool isBuffer,
    int? outputSeq,
    int? offset,
    bool resetsSequence = false,
  }) {
    final previousSeq = sequenceFor(sessionId);
    if (isBuffer) {
      final shouldReset = (offset ?? 0) <= 0 || resetsSequence;
      if (shouldReset) {
        _allowNextLiveRebaseSessions.add(sessionId);
        if (outputSeq != null) {
          _seqBySession[sessionId] = outputSeq;
        }
      } else if (outputSeq != null && outputSeq >= previousSeq) {
        _seqBySession[sessionId] = outputSeq;
      }
      return TerminalOutputSequenceResult(
        action: TerminalOutputSequenceAction.snapshot,
        previousSeq: previousSeq,
      );
    }
    if (outputSeq == null) {
      return TerminalOutputSequenceResult(
        action: TerminalOutputSequenceAction.accept,
        previousSeq: previousSeq,
      );
    }
    if (outputSeq <= previousSeq) {
      return TerminalOutputSequenceResult(
        action: TerminalOutputSequenceAction.duplicate,
        previousSeq: previousSeq,
      );
    }
    final allowRebase = _allowNextLiveRebaseSessions.remove(sessionId);
    if ((allowRebase || previousSeq > 0) && outputSeq > previousSeq) {
      _seqBySession[sessionId] = outputSeq;
      return TerminalOutputSequenceResult(
        action: TerminalOutputSequenceAction.accept,
        previousSeq: previousSeq,
      );
    }
    _seqBySession[sessionId] = outputSeq;
    _allowNextLiveRebaseSessions.remove(sessionId);
    return TerminalOutputSequenceResult(
      action: TerminalOutputSequenceAction.accept,
      previousSeq: previousSeq,
    );
  }

  void remove(String sessionId) {
    _seqBySession.remove(sessionId);
    _allowNextLiveRebaseSessions.remove(sessionId);
  }

  void reset() {
    _seqBySession.clear();
    _allowNextLiveRebaseSessions.clear();
  }
}
