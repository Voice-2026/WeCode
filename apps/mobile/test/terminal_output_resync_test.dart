import 'package:codux_flutter/services/terminal_output_resync.dart';
import 'package:codux_flutter/services/terminal_output_sequencer.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('renders delayed burst output in order without resync', () {
    final sequencer = TerminalOutputSequencer();
    final rendered = <int>[];
    final acks = <int>[];
    var requestedFullBuffer = false;

    for (var seq = 1; seq <= 200; seq += 1) {
      final result = observeTerminalOutputForResync(
        sequencer: sequencer,
        sessionId: 'term-1',
        isBuffer: false,
        outputSeq: seq,
        offset: null,
      );
      if (result.render) rendered.add(seq);
      if (result.ack != null) acks.add(result.ack!);
      requestedFullBuffer = requestedFullBuffer || result.requestFullBuffer;
    }

    expect(rendered, List<int>.generate(200, (index) => index + 1));
    expect(acks, List<int>.generate(200, (index) => index + 1));
    expect(requestedFullBuffer, isFalse);
  });

  test('rebases skipped live output without full buffer recovery', () {
    final sequencer = TerminalOutputSequencer();
    final rendered = <int>[];
    var fullBufferRequests = 0;

    for (final seq in [1, 2, 5, 6, 7, 8, 9]) {
      final result = observeTerminalOutputForResync(
        sequencer: sequencer,
        sessionId: 'term-1',
        isBuffer: false,
        outputSeq: seq,
        offset: null,
      );
      if (result.render) rendered.add(seq);
      if (result.requestFullBuffer) fullBufferRequests += 1;
    }

    expect(rendered, [1, 2, 5, 6, 7, 8, 9]);
    expect(fullBufferRequests, 0);

    final snapshot = observeTerminalOutputForResync(
      sequencer: sequencer,
      sessionId: 'term-1',
      isBuffer: true,
      outputSeq: 9,
      offset: 0,
    );
    final next = observeTerminalOutputForResync(
      sequencer: sequencer,
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 10,
      offset: null,
    );

    expect(snapshot.render, isTrue);
    expect(snapshot.requestFullBuffer, isFalse);
    expect(next.render, isTrue);
    expect(next.requestFullBuffer, isFalse);
  });

  test(
    'truncated tail snapshot can replace output after skipped live data',
    () {
      final sequencer = TerminalOutputSequencer();
      final first = observeTerminalOutputForResync(
        sequencer: sequencer,
        sessionId: 'term-1',
        isBuffer: false,
        outputSeq: 1,
        offset: null,
      );
      final skipped = observeTerminalOutputForResync(
        sequencer: sequencer,
        sessionId: 'term-1',
        isBuffer: false,
        outputSeq: 900,
        offset: null,
      );
      final nextSkipped = observeTerminalOutputForResync(
        sequencer: sequencer,
        sessionId: 'term-1',
        isBuffer: false,
        outputSeq: 901,
        offset: null,
      );
      final tailSnapshot = observeTerminalOutputForResync(
        sequencer: sequencer,
        sessionId: 'term-1',
        isBuffer: true,
        outputSeq: 901,
        offset: 120000,
        resetsSequence: true,
      );
      final next = observeTerminalOutputForResync(
        sequencer: sequencer,
        sessionId: 'term-1',
        isBuffer: false,
        outputSeq: 902,
        offset: null,
      );

      expect(first.render, isTrue);
      expect(skipped.render, isTrue);
      expect(skipped.requestFullBuffer, isFalse);
      expect(nextSkipped.render, isTrue);
      expect(nextSkipped.requestFullBuffer, isFalse);
      expect(tailSnapshot.render, isTrue);
      expect(tailSnapshot.requestFullBuffer, isFalse);
      expect(next.render, isTrue);
      expect(next.requestFullBuffer, isFalse);
    },
  );

  test('snapshot resets sequence after skipped live output', () {
    final sequencer = TerminalOutputSequencer();
    expect(
      observeTerminalOutputForResync(
        sequencer: sequencer,
        sessionId: 'term-1',
        isBuffer: false,
        outputSeq: 100,
        offset: null,
      ).render,
      isTrue,
    );
    final skipped = observeTerminalOutputForResync(
      sequencer: sequencer,
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 900,
      offset: null,
    );
    final nextSkipped = observeTerminalOutputForResync(
      sequencer: sequencer,
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 940,
      offset: null,
    );
    final snapshot = observeTerminalOutputForResync(
      sequencer: sequencer,
      sessionId: 'term-1',
      isBuffer: true,
      outputSeq: 900,
      offset: 0,
    );
    final next = observeTerminalOutputForResync(
      sequencer: sequencer,
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 941,
      offset: null,
    );

    expect(skipped.requestFullBuffer, isFalse);
    expect(nextSkipped.requestFullBuffer, isFalse);
    expect(snapshot.render, isTrue);
    expect(snapshot.requestFullBuffer, isFalse);
    expect(next.render, isTrue);
    expect(next.requestFullBuffer, isFalse);
  });
}
