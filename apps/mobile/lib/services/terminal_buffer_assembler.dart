class TerminalBufferAssemblyResult {
  const TerminalBufferAssemblyResult({
    required this.ready,
    required this.progress,
    this.payload,
  });

  final bool ready;
  final double? progress;
  final Map<dynamic, dynamic>? payload;
}

class TerminalBufferAssembler {
  TerminalBufferAssembler({this.maxChars = 200000});

  final int maxChars;
  final Map<String, _TerminalBufferAssembly> _assemblies = {};

  TerminalBufferAssemblyResult accept({
    required String sessionId,
    required Map<dynamic, dynamic> payload,
  }) {
    if (payload['buffer'] != true || payload['chunked'] != true) {
      return TerminalBufferAssemblyResult(
        ready: true,
        progress: null,
        payload: payload,
      );
    }
    final snapshotId = payload['snapshotId']?.toString().trim();
    final chunkIndex = _intValue(payload['chunkIndex']);
    final chunkCount = _intValue(payload['chunkCount']);
    if (snapshotId == null ||
        snapshotId.isEmpty ||
        chunkIndex == null ||
        chunkCount == null ||
        chunkCount <= 0 ||
        chunkIndex < 0 ||
        chunkIndex >= chunkCount) {
      return const TerminalBufferAssemblyResult(ready: false, progress: null);
    }

    final key = '$sessionId:$snapshotId';
    _assemblies.removeWhere((otherKey, _) {
      return otherKey.startsWith('$sessionId:') && otherKey != key;
    });
    final assembly = _assemblies.putIfAbsent(
      key,
      () => _TerminalBufferAssembly(
        sessionId: sessionId,
        snapshotId: snapshotId,
        chunkCount: chunkCount,
        basePayload: Map<dynamic, dynamic>.from(payload),
        maxChars: maxChars,
      ),
    );
    if (assembly.chunkCount != chunkCount) {
      _assemblies.remove(key);
      return const TerminalBufferAssemblyResult(ready: false, progress: null);
    }
    assembly.add(chunkIndex, payload['data']?.toString() ?? '');
    if (!assembly.complete) {
      return TerminalBufferAssemblyResult(
        ready: false,
        progress: assembly.progress,
      );
    }
    _assemblies.remove(key);
    return TerminalBufferAssemblyResult(
      ready: true,
      progress: 1,
      payload: assembly.payload(),
    );
  }

  void remove(String sessionId) {
    _assemblies.removeWhere((key, _) => key.startsWith('$sessionId:'));
  }

  void reset() {
    _assemblies.clear();
  }
}

class _TerminalBufferAssembly {
  _TerminalBufferAssembly({
    required this.sessionId,
    required this.snapshotId,
    required this.chunkCount,
    required this.basePayload,
    required this.maxChars,
  });

  final String sessionId;
  final String snapshotId;
  final int chunkCount;
  final Map<dynamic, dynamic> basePayload;
  final int maxChars;
  final Map<int, String> chunks = {};
  int _chars = 0;

  void add(int index, String data) {
    if (chunks.containsKey(index)) return;
    final nextChars = _chars + data.runes.length;
    if (nextChars > maxChars) return;
    chunks[index] = data;
    _chars = nextChars;
  }

  bool get complete => chunks.length == chunkCount;

  double get progress => chunkCount <= 0 ? 0 : chunks.length / chunkCount;

  Map<dynamic, dynamic> payload() {
    final data = List.generate(
      chunkCount,
      (index) => chunks[index] ?? '',
    ).join();
    return Map<dynamic, dynamic>.from(basePayload)
      ..['data'] = data
      ..['offset'] = basePayload['startOffset'] ?? basePayload['offset']
      ..['chunked'] = false
      ..['assembled'] = true
      ..remove('chunkIndex')
      ..remove('chunkCount');
  }
}

int? _intValue(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  return int.tryParse('${value ?? ''}');
}
