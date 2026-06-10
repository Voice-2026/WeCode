import 'package:codux_flutter/services/terminal_output_resync.dart';
import 'package:codux_flutter/services/terminal_output_sequencer.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('renders delayed burst output in order without resync', () {
    final sequencer = TerminalOutputSequencer();
    final rendered = <int>[];
    final acks = <int>[];

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
    }

    expect(rendered, List<int>.generate(200, (index) => index + 1));
    expect(acks, List<int>.generate(200, (index) => index + 1));
  });

  test('rebases skipped live output without ui buffer recovery', () {
    final sequencer = TerminalOutputSequencer();
    final rendered = <int>[];

    for (final seq in [1, 2, 5, 6, 7, 8, 9]) {
      final result = observeTerminalOutputForResync(
        sequencer: sequencer,
        sessionId: 'term-1',
        isBuffer: false,
        outputSeq: seq,
        offset: null,
      );
      if (result.render) rendered.add(seq);
    }

    expect(rendered, [1, 2, 5, 6, 7, 8, 9]);

    final baseline = observeTerminalOutputForResync(
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

    expect(baseline.render, isTrue);
    expect(next.render, isTrue);
  });

  test('truncated tail buffer can replace output after skipped live data', () {
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
    final tailBuffer = observeTerminalOutputForResync(
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
    expect(nextSkipped.render, isTrue);
    expect(tailBuffer.render, isTrue);
    expect(next.render, isTrue);
  });

  test('baseline resets sequence after skipped live output', () {
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
    final baseline = observeTerminalOutputForResync(
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

    expect(skipped.render, isTrue);
    expect(nextSkipped.render, isTrue);
    expect(baseline.render, isTrue);
    expect(next.render, isTrue);
  });
}
