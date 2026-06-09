import 'package:codux_flutter/services/remote_sequence_guard.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('accepts out of order messages from different channels', () {
    final guard = RemoteSequenceGuard();

    expect(guard.accept(type: 'terminal.list', sessionId: null, seq: 34), true);
    expect(guard.accept(type: 'project.list', sessionId: null, seq: 33), true);
  });

  test('drops duplicate sequence in the same channel', () {
    final guard = RemoteSequenceGuard();

    expect(guard.accept(type: 'project.list', sessionId: null, seq: 33), true);
    expect(guard.accept(type: 'project.list', sessionId: null, seq: 33), false);
  });

  test('keeps terminal sessions independent', () {
    final guard = RemoteSequenceGuard();

    expect(
      guard.accept(type: 'terminal.output', sessionId: 'a', seq: 10),
      true,
    );
    expect(
      guard.accept(type: 'terminal.output', sessionId: 'b', seq: 10),
      true,
    );
    expect(
      guard.accept(type: 'terminal.output', sessionId: 'a', seq: 10),
      false,
    );
  });

  test('rejects sequences older than the sliding window', () {
    final guard = RemoteSequenceGuard(maxEntriesPerChannel: 3);

    expect(guard.accept(type: 'project.list', sessionId: null, seq: 4), true);
    expect(guard.accept(type: 'project.list', sessionId: null, seq: 1), false);
  });
}
