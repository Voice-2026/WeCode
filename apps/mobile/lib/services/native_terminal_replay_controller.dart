class NativeTerminalReplayController {
  final Map<String, NativeTerminalReplay> _replayBySession = {};
  final Map<String, int> _contentLengthBySession = {};
  int _revision = 0;

  NativeTerminalReplay replay(String sessionId) {
    return _replayBySession[sessionId] ??
        NativeTerminalReplay.empty(sessionId: sessionId);
  }

  bool replaceSession(String sessionId, String content) {
    final current = _replayBySession[sessionId];
    if (current != null && current.content == content) return false;
    _revision += 1;
    _replayBySession[sessionId] = NativeTerminalReplay(
      sessionId: sessionId,
      content: content,
      append: '',
      reset: true,
      revision: _revision,
    );
    _contentLengthBySession[sessionId] = content.length;
    return true;
  }

  bool syncSession(String sessionId, String content) {
    final currentLength = _contentLengthBySession[sessionId];
    if (currentLength == null ||
        currentLength > content.length ||
        !content.startsWith(_replayBySession[sessionId]?.content ?? '')) {
      return replaceSession(sessionId, content);
    }
    if (currentLength == content.length) return false;
    final append = content.substring(currentLength);
    _revision += 1;
    _replayBySession[sessionId] = NativeTerminalReplay(
      sessionId: sessionId,
      content: content,
      append: append,
      reset: false,
      revision: _revision,
    );
    _contentLengthBySession[sessionId] = content.length;
    return true;
  }

  void removeSession(String sessionId) {
    if (_replayBySession.remove(sessionId) != null) {
      _contentLengthBySession.remove(sessionId);
      _revision += 1;
    }
  }

  void resetAll() {
    if (_replayBySession.isNotEmpty) {
      _replayBySession.clear();
      _contentLengthBySession.clear();
      _revision += 1;
    }
  }
}

class NativeTerminalReplay {
  const NativeTerminalReplay({
    required this.sessionId,
    required this.content,
    required this.append,
    required this.reset,
    required this.revision,
  });

  factory NativeTerminalReplay.empty({required String sessionId}) {
    return NativeTerminalReplay(
      sessionId: sessionId,
      content: '',
      append: '',
      reset: true,
      revision: 0,
    );
  }

  final String sessionId;
  final String content;
  final String append;
  final bool reset;
  final int revision;
}
