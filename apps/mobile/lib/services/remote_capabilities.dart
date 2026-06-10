class TerminalBufferCapability {
  const TerminalBufferCapability({
    this.chunking = false,
    this.maxChars = mobileMaxChars,
    this.chunkChars = 16384,
    this.requestId = false,
  });

  static const int mobileMaxChars = 65536;

  final bool chunking;
  final int maxChars;
  final int chunkChars;
  final bool requestId;

  static const fallback = TerminalBufferCapability();

  factory TerminalBufferCapability.fromHostInfo(
    Object? payload, {
    int clientMaxChars = mobileMaxChars,
  }) {
    if (payload is! Map) return fallback;
    final capabilities = payload['capabilities'];
    if (capabilities is! Map) return fallback;
    final terminalBuffer = capabilities['terminalBuffer'];
    if (terminalBuffer is! Map) return fallback;
    final effectiveClientMax = clientMaxChars < 1
        ? mobileMaxChars
        : clientMaxChars;
    return TerminalBufferCapability(
      chunking: terminalBuffer['chunking'] == true,
      maxChars: _clampInt(
        _intValue(terminalBuffer['maxChars']) ?? fallback.maxChars,
        1,
        effectiveClientMax,
      ),
      chunkChars: _clampInt(
        _intValue(terminalBuffer['chunkChars']) ?? fallback.chunkChars,
        4096,
        65536,
      ),
      requestId: terminalBuffer['requestId'] == true,
    );
  }
}

int? _intValue(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  return int.tryParse('${value ?? ''}');
}

int _clampInt(int value, int min, int max) {
  if (value < min) return min;
  if (value > max) return max;
  return value;
}
