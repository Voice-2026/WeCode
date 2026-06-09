enum TerminalBufferRequestMode { historyRestore, liveResume, historyPage }

Map<String, Object> buildTerminalBufferRequestPayload({
  required String requestId,
  required TerminalBufferRequestMode mode,
  required int offset,
  required int maxChars,
  bool chunking = false,
  int? chunkChars,
  int? resumeFromSeq,
}) {
  final payload = <String, Object>{
    'requestId': requestId,
    'tail': false,
    'offset': mode == TerminalBufferRequestMode.historyRestore ? 0 : offset,
    'maxChars': maxChars,
  };
  if (chunking && chunkChars != null) {
    payload['chunkChars'] = chunkChars;
  }
  if (mode == TerminalBufferRequestMode.liveResume &&
      resumeFromSeq != null &&
      resumeFromSeq > 0) {
    payload['resumeFromSeq'] = resumeFromSeq;
  }
  return payload;
}
