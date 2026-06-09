import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:codux_flutter/models/remote_models.dart';
import 'package:codux_flutter/services/terminal_upload_sender.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('uploads image in acked chunks', () async {
    final sent = <RelayEnvelope>[];
    late final TerminalUploadSender sender;
    sender = TerminalUploadSender(
      chunkSize: 3,
      ackTimeout: const Duration(milliseconds: 100),
      send: (message) async {
        sent.add(message);
        scheduleMicrotask(() => sender.handleAck(_ackFor(message)));
        return true;
      },
    );

    final progress = <int>[];
    await sender.uploadImage(
      sessionId: 'session-1',
      name: 'image.png',
      mime: 'image/png',
      bytes: Uint8List.fromList([1, 2, 3, 4, 5, 6, 7]),
      onProgress: (value) => progress.add(value.percent),
    );

    expect(sent.map((message) => message.type), [
      'terminal.upload.start',
      'terminal.upload.chunk',
      'terminal.upload.chunk',
      'terminal.upload.chunk',
      'terminal.upload.finish',
    ]);
    expect((sent.first.payload as Map)['kind'], 'image');
    final chunkPayloads = sent
        .where((message) => message.type == 'terminal.upload.chunk')
        .map((message) => message.payload as Map)
        .toList();
    expect(chunkPayloads.map((payload) => payload['chunkIndex']), [0, 1, 2]);
    expect(base64Decode(chunkPayloads[0]['data'] as String), [1, 2, 3]);
    expect(base64Decode(chunkPayloads[2]['data'] as String), [7]);
    expect(progress, [43, 86, 100]);
  });

  test('retries when a chunk ack times out', () async {
    final sent = <RelayEnvelope>[];
    var firstChunkAttempt = true;
    late final TerminalUploadSender sender;
    sender = TerminalUploadSender(
      chunkSize: 4,
      ackTimeout: const Duration(milliseconds: 10),
      maxRetries: 1,
      send: (message) async {
        sent.add(message);
        final payload = message.payload as Map?;
        if (message.type == 'terminal.upload.chunk' &&
            payload?['chunkIndex'] == 0 &&
            firstChunkAttempt) {
          firstChunkAttempt = false;
          return true;
        }
        scheduleMicrotask(() => sender.handleAck(_ackFor(message)));
        return true;
      },
    );

    await sender.uploadImage(
      sessionId: 'session-1',
      name: 'image.png',
      mime: 'image/png',
      bytes: Uint8List.fromList([1, 2, 3, 4]),
    );

    expect(
      sent.where((message) => message.type == 'terminal.upload.chunk').length,
      2,
    );
  });

  test('uploads file with file kind', () async {
    final sent = <RelayEnvelope>[];
    late final TerminalUploadSender sender;
    sender = TerminalUploadSender(
      chunkSize: 8,
      ackTimeout: const Duration(milliseconds: 100),
      send: (message) async {
        sent.add(message);
        scheduleMicrotask(() => sender.handleAck(_ackFor(message)));
        return true;
      },
    );

    await sender.uploadFile(
      sessionId: 'session-1',
      name: 'notes.txt',
      mime: 'text/plain',
      bytes: Uint8List.fromList([1, 2, 3]),
    );

    expect((sent.first.payload as Map)['kind'], 'file');
  });

  test('stops upload when transport refuses the start message', () async {
    final sent = <RelayEnvelope>[];
    final sender = TerminalUploadSender(
      chunkSize: 8,
      ackTimeout: const Duration(milliseconds: 10),
      maxRetries: 0,
      send: (message) async {
        sent.add(message);
        return false;
      },
    );

    await expectLater(
      sender.uploadFile(
        sessionId: 'session-1',
        name: 'notes.txt',
        mime: 'text/plain',
        bytes: Uint8List.fromList([1, 2, 3]),
      ),
      throwsStateError,
    );

    expect(sent.map((message) => message.type), ['terminal.upload.start']);
  });
}

RelayEnvelope _ackFor(RelayEnvelope message) {
  final payload = message.payload as Map;
  return RelayEnvelope(
    type: 'terminal.upload.ack',
    sessionId: message.sessionId,
    payload: {
      'uploadId': payload['uploadId'],
      'stage': switch (message.type) {
        'terminal.upload.start' => 'start',
        'terminal.upload.chunk' => 'chunk',
        'terminal.upload.finish' => 'finish',
        _ => '',
      },
      if (payload['chunkIndex'] != null) 'chunkIndex': payload['chunkIndex'],
      'ok': true,
    },
  );
}
