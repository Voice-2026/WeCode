import 'package:codux_flutter/services/terminal_buffer_assembler.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('passes through non-chunked terminal buffer payloads', () {
    final assembler = TerminalBufferAssembler();
    final payload = {'buffer': true, 'data': 'ready'};

    final result = assembler.accept(sessionId: 'term-1', payload: payload);

    expect(result.ready, isTrue);
    expect(result.progress, isNull);
    expect(result.payload, same(payload));
  });

  test('assembles chunked terminal buffer payloads in arrival order', () {
    final assembler = TerminalBufferAssembler();

    final first = assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 0, data: 'ab'),
    );
    final second = assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 1, data: '你好'),
    );
    final third = assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 2, data: 'cd'),
    );

    expect(first.ready, isFalse);
    expect(first.progress, closeTo(1 / 3, 0.001));
    expect(second.ready, isFalse);
    expect(second.progress, closeTo(2 / 3, 0.001));
    expect(third.ready, isTrue);
    expect(third.progress, 1);
    expect(third.payload?['data'], 'ab你好cd');
    expect(third.payload?['offset'], 10);
    expect(third.payload?['chunked'], isFalse);
    expect(third.payload?['assembled'], isTrue);
    expect(third.payload?.containsKey('chunkIndex'), isFalse);
    expect(third.payload?.containsKey('chunkCount'), isFalse);
  });

  test('assembles chunked terminal buffer payloads out of order', () {
    final assembler = TerminalBufferAssembler();

    assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 2, data: 'cd'),
    );
    assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 0, data: 'ab'),
    );
    final result = assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 1, data: '你好'),
    );

    expect(result.ready, isTrue);
    expect(result.payload?['data'], 'ab你好cd');
  });

  test('ignores duplicate chunks without advancing progress twice', () {
    final assembler = TerminalBufferAssembler();

    final first = assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 0, data: 'ab'),
    );
    final duplicate = assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 0, data: 'ab'),
    );

    expect(first.progress, closeTo(1 / 3, 0.001));
    expect(duplicate.ready, isFalse);
    expect(duplicate.progress, closeTo(1 / 3, 0.001));
  });

  test('waits when a chunk is missing', () {
    final assembler = TerminalBufferAssembler();

    assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 0, data: 'ab'),
    );
    final result = assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 2, data: 'cd'),
    );

    expect(result.ready, isFalse);
    expect(result.progress, closeTo(2 / 3, 0.001));
  });

  test('does not assemble buffers that exceed the mobile character limit', () {
    final assembler = TerminalBufferAssembler(maxChars: 4);

    assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 0, data: 'abcd', chunkCount: 2),
    );
    final result = assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(index: 1, data: 'ef', chunkCount: 2),
    );

    expect(result.ready, isFalse);
    expect(result.progress, closeTo(1 / 2, 0.001));
  });

  test('replaces stale snapshot assembly for the same session', () {
    final assembler = TerminalBufferAssembler();

    assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(snapshotId: 'old', index: 0, data: 'old'),
    );
    assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(snapshotId: 'new', index: 0, data: 'new-', chunkCount: 2),
    );
    final result = assembler.accept(
      sessionId: 'term-1',
      payload: _chunk(snapshotId: 'new', index: 1, data: 'data', chunkCount: 2),
    );

    expect(result.ready, isTrue);
    expect(result.payload?['data'], 'new-data');
  });
}

Map<String, Object?> _chunk({
  String snapshotId = 'snapshot-1',
  required int index,
  required String data,
  int chunkCount = 3,
}) {
  return {
    'buffer': true,
    'chunked': true,
    'snapshotId': snapshotId,
    'chunkIndex': index,
    'chunkCount': chunkCount,
    'data': data,
    'offset': 10 + index * 2,
    'startOffset': 10,
    'bufferLength': 16,
    'truncated': true,
    'outputSeq': 7,
  };
}
