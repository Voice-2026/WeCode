import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import '../models/remote_models.dart';

typedef TerminalUploadSend = Future<bool> Function(RelayEnvelope message);
typedef TerminalUploadYield = Future<void> Function();
typedef TerminalUploadProgressCallback =
    void Function(TerminalUploadProgress progress);

class TerminalUploadSender {
  TerminalUploadSender({
    required TerminalUploadSend send,
    this.chunkSize = 12 * 1024,
    this.ackTimeout = const Duration(seconds: 8),
    this.maxRetries = 4,
    TerminalUploadYield? afterChunkAck,
  }) : _send = send,
       _afterChunkAck = afterChunkAck;

  final TerminalUploadSend _send;
  final int chunkSize;
  final Duration ackTimeout;
  final int maxRetries;
  final TerminalUploadYield? _afterChunkAck;
  final Map<String, Completer<TerminalUploadAck>> _pendingAcks = {};

  Future<void> uploadFile({
    required String sessionId,
    required String name,
    required String mime,
    required Uint8List bytes,
    String kind = 'file',
    TerminalUploadProgressCallback? onProgress,
  }) async {
    if (bytes.isEmpty) return;
    final uploadId = '${DateTime.now().microsecondsSinceEpoch}-${bytes.length}';
    final totalChunks = (bytes.length + chunkSize - 1) ~/ chunkSize;
    await _sendWithAck(
      RelayEnvelope(
        type: 'terminal.upload.start',
        sessionId: sessionId,
        payload: {
          'uploadId': uploadId,
          'name': name,
          'mime': mime,
          'kind': _normalizeKind(kind),
          'totalBytes': bytes.length,
          'totalChunks': totalChunks,
          'chunkSize': chunkSize,
        },
      ),
      uploadId: uploadId,
      stage: 'start',
    );

    var uploadedBytes = 0;
    for (var index = 0; index < totalChunks; index += 1) {
      final start = index * chunkSize;
      final end = (start + chunkSize).clamp(0, bytes.length);
      final chunk = Uint8List.sublistView(bytes, start, end);
      await _sendWithAck(
        RelayEnvelope(
          type: 'terminal.upload.chunk',
          sessionId: sessionId,
          payload: {
            'uploadId': uploadId,
            'chunkIndex': index,
            'totalChunks': totalChunks,
            'offset': start,
            'data': base64Encode(chunk),
          },
        ),
        uploadId: uploadId,
        stage: 'chunk',
        chunkIndex: index,
      );
      uploadedBytes = end;
      onProgress?.call(
        TerminalUploadProgress(
          uploadId: uploadId,
          uploadedBytes: uploadedBytes,
          totalBytes: bytes.length,
          chunkIndex: index + 1,
          totalChunks: totalChunks,
        ),
      );
      await _afterChunkAck?.call();
    }

    await _sendWithAck(
      RelayEnvelope(
        type: 'terminal.upload.finish',
        sessionId: sessionId,
        payload: {
          'uploadId': uploadId,
          'totalBytes': bytes.length,
          'totalChunks': totalChunks,
        },
      ),
      uploadId: uploadId,
      stage: 'finish',
    );
  }

  Future<void> uploadImage({
    required String sessionId,
    required String name,
    required String mime,
    required Uint8List bytes,
    TerminalUploadProgressCallback? onProgress,
  }) {
    return uploadFile(
      sessionId: sessionId,
      name: name,
      mime: mime,
      bytes: bytes,
      kind: 'image',
      onProgress: onProgress,
    );
  }

  void handleAck(RelayEnvelope message) {
    final payload = message.payload;
    if (payload is! Map) return;
    final uploadId = payload['uploadId']?.toString();
    final stage = payload['stage']?.toString();
    if (uploadId == null || stage == null) return;
    final chunkIndex = _asInt(payload['chunkIndex']);
    final key = _ackKey(uploadId, stage, chunkIndex);
    final completer = _pendingAcks.remove(key);
    if (completer == null || completer.isCompleted) return;
    completer.complete(
      TerminalUploadAck(
        uploadId: uploadId,
        stage: stage,
        ok: payload['ok'] != false,
        chunkIndex: chunkIndex,
        message: payload['message']?.toString(),
      ),
    );
  }

  void dispose() {
    for (final completer in _pendingAcks.values) {
      if (!completer.isCompleted) {
        completer.completeError(TimeoutException('Upload cancelled'));
      }
    }
    _pendingAcks.clear();
  }

  Future<void> _sendWithAck(
    RelayEnvelope message, {
    required String uploadId,
    required String stage,
    int? chunkIndex,
  }) async {
    final key = _ackKey(uploadId, stage, chunkIndex);
    Object? lastError;
    for (var attempt = 0; attempt <= maxRetries; attempt += 1) {
      final completer = Completer<TerminalUploadAck>();
      _pendingAcks[key] = completer;
      final sent = await _send(message);
      if (!sent) {
        _pendingAcks.remove(key);
        lastError = StateError('Upload transport is not connected');
      } else {
        try {
          final ack = await completer.future.timeout(ackTimeout);
          if (ack.ok) return;
          throw StateError(ack.message ?? 'Upload was rejected by Mac');
        } on TimeoutException catch (error) {
          if (_pendingAcks[key] == completer) {
            _pendingAcks.remove(key);
          }
          lastError = error;
        }
      }
      if (attempt < maxRetries) {
        await Future<void>.delayed(Duration(milliseconds: 200 * (attempt + 1)));
      }
    }
    throw StateError('Upload failed waiting for $stage ack: $lastError');
  }

  String _ackKey(String uploadId, String stage, int? chunkIndex) {
    return '$uploadId:$stage:${chunkIndex ?? -1}';
  }

  int? _asInt(Object? value) {
    if (value is int) return value;
    if (value is num) return value.toInt();
    return int.tryParse('${value ?? ''}');
  }

  String _normalizeKind(String value) =>
      value.trim().toLowerCase() == 'image' ? 'image' : 'file';
}

class TerminalUploadProgress {
  const TerminalUploadProgress({
    required this.uploadId,
    required this.uploadedBytes,
    required this.totalBytes,
    required this.chunkIndex,
    required this.totalChunks,
  });

  final String uploadId;
  final int uploadedBytes;
  final int totalBytes;
  final int chunkIndex;
  final int totalChunks;

  int get percent => totalBytes <= 0
      ? 100
      : ((uploadedBytes / totalBytes) * 100).clamp(0, 100).round();
}

class TerminalUploadAck {
  const TerminalUploadAck({
    required this.uploadId,
    required this.stage,
    required this.ok,
    this.chunkIndex,
    this.message,
  });

  final String uploadId;
  final String stage;
  final bool ok;
  final int? chunkIndex;
  final String? message;
}
