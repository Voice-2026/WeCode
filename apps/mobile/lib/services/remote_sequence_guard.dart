class RemoteSequenceGuard {
  RemoteSequenceGuard({this.maxEntriesPerChannel = 128});

  final int maxEntriesPerChannel;
  final Map<String, _RemoteSequenceWindow> _seenByChannel = {};

  bool accept({
    required String type,
    required String? sessionId,
    required int? seq,
  }) {
    if (seq == null) return true;
    final channel = _channelFor(type: type, sessionId: sessionId);
    final seen = _seenByChannel.putIfAbsent(
      channel,
      () => _RemoteSequenceWindow(maxEntries: maxEntriesPerChannel),
    );
    return seen.accept(seq);
  }

  void reset() {
    _seenByChannel.clear();
  }
}

class _RemoteSequenceWindow {
  _RemoteSequenceWindow({required this.maxEntries});

  final int maxEntries;
  final List<int> _seen = [];
  int _maxSeq = 0;

  bool accept(int seq) {
    if (_maxSeq > maxEntries && seq <= _maxSeq - maxEntries) return false;
    if (_seen.contains(seq)) return false;
    _seen.add(seq);
    if (seq > _maxSeq) _maxSeq = seq;
    while (_seen.length > maxEntries) {
      _seen.removeAt(0);
    }
    return true;
  }
}

String _channelFor({required String type, required String? sessionId}) {
  final session = sessionId?.trim();
  if (session != null && session.isNotEmpty) {
    return 'session:$session';
  }
  return 'type:$type';
}
