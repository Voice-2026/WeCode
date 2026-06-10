import 'dart:convert';
import 'dart:io';

class TerminalOutputPayload {
  const TerminalOutputPayload({
    required this.data,
    required this.isBuffer,
    this.offset,
    this.bufferLength,
    this.truncated = false,
    this.requestId,
    this.tail = false,
    this.hasPrevious = false,
  });

  final String data;
  final bool isBuffer;
  final int? offset;
  final int? bufferLength;
  final bool truncated;
  final String? requestId;
  final bool tail;
  final bool hasPrevious;
}

TerminalOutputPayload decodeTerminalOutputPayload(
  Map<dynamic, dynamic> payload,
) {
  final data = _decodeData(payload);
  return TerminalOutputPayload(
    data: data,
    isBuffer: payload['buffer'] == true,
    offset: _intValue(payload['offset']),
    bufferLength: _intValue(payload['bufferLength']),
    truncated: payload['truncated'] == true,
    requestId: _stringValue(payload['requestId']),
    tail: payload['tail'] == true,
    hasPrevious: payload['hasPrevious'] == true,
  );
}

String _decodeData(Map<dynamic, dynamic> payload) {
  final value = '${payload['data'] ?? ''}';
  if (payload['compressed'] != true) return value;
  if (payload['encoding'] != 'base64+deflate+utf8') return value;
  final compressed = base64Url.decode(base64Url.normalize(value));
  return utf8.decode(ZLibCodec(raw: true).decode(compressed));
}

int? _intValue(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  return int.tryParse('${value ?? ''}');
}

String? _stringValue(Object? value) {
  final text = value?.toString().trim();
  return text == null || text.isEmpty ? null : text;
}
