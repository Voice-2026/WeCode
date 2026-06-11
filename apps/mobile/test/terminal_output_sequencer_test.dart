import 'package:codux_flutter/services/terminal_output_sequencer.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('accepts ordered terminal output', () {
    final sequencer = TerminalOutputSequencer();

    final first = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 1,
    );
    final second = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 2,
    );

    expect(first.action, TerminalOutputSequenceAction.accept);
    expect(second.action, TerminalOutputSequenceAction.accept);
    expect(sequencer.sequenceFor('term-1'), 2);
  });

  test('drops duplicate terminal output', () {
    final sequencer = TerminalOutputSequencer()
      ..observe(sessionId: 'term-1', isBuffer: false, outputSeq: 1)
      ..observe(sessionId: 'term-1', isBuffer: false, outputSeq: 2);

    final duplicate = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 2,
    );

    expect(duplicate.action, TerminalOutputSequenceAction.duplicate);
    expect(duplicate.previousSeq, 2);
    expect(sequencer.sequenceFor('term-1'), 2);
  });

  test('rebases live sequence gaps without forcing full buffer', () {
    final sequencer = TerminalOutputSequencer()
      ..observe(sessionId: 'term-1', isBuffer: false, outputSeq: 1);

    final gap = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 3,
    );
    final laterDelta = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 4,
    );

    expect(gap.action, TerminalOutputSequenceAction.accept);
    expect(gap.previousSeq, 1);
    expect(laterDelta.action, TerminalOutputSequenceAction.accept);
    expect(sequencer.sequenceFor('term-1'), 4);
  });

  test('full buffer resets sequence and resumes ordered output', () {
    final sequencer = TerminalOutputSequencer()
      ..observe(sessionId: 'term-1', isBuffer: false, outputSeq: 1)
      ..observe(sessionId: 'term-1', isBuffer: false, outputSeq: 3);

    final baseline = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: true,
      outputSeq: 3,
      offset: 0,
    );
    final next = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 4,
    );

    expect(baseline.action, TerminalOutputSequenceAction.baseline);
    expect(next.action, TerminalOutputSequenceAction.accept);
    expect(sequencer.sequenceFor('term-1'), 4);
  });

  test('full buffer can reset sequence after host restart', () {
    final sequencer = TerminalOutputSequencer()
      ..observe(sessionId: 'term-1', isBuffer: false, outputSeq: 8);

    final baseline = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: true,
      outputSeq: 0,
      offset: 0,
    );
    final next = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 1,
    );

    expect(baseline.action, TerminalOutputSequenceAction.baseline);
    expect(next.action, TerminalOutputSequenceAction.accept);
    expect(sequencer.sequenceFor('term-1'), 1);
  });

  test('truncated tail buffer clears gap state for large histories', () {
    final sequencer = TerminalOutputSequencer()
      ..observe(sessionId: 'term-1', isBuffer: false, outputSeq: 1);

    final gap = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 900,
    );
    final tailBuffer = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: true,
      outputSeq: 900,
      offset: 120000,
      resetsSequence: true,
    );
    final next = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 901,
    );

    expect(gap.action, TerminalOutputSequenceAction.accept);
    expect(tailBuffer.action, TerminalOutputSequenceAction.baseline);
    expect(next.action, TerminalOutputSequenceAction.accept);
    expect(sequencer.sequenceFor('term-1'), 901);
  });

  test('full buffer catches up to live output seen while resyncing', () {
    final sequencer = TerminalOutputSequencer()
      ..observe(sessionId: 'term-1', isBuffer: false, outputSeq: 100);

    final gap = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 900,
    );
    final liveWhileResyncing = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 940,
    );
    final baseline = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: true,
      outputSeq: 900,
      offset: 0,
    );
    final afterSnapshotSeq = sequencer.sequenceFor('term-1');
    final next = sequencer.observe(
      sessionId: 'term-1',
      isBuffer: false,
      outputSeq: 941,
    );

    expect(gap.action, TerminalOutputSequenceAction.accept);
    expect(liveWhileResyncing.action, TerminalOutputSequenceAction.accept);
    expect(baseline.action, TerminalOutputSequenceAction.baseline);
    expect(afterSnapshotSeq, 900);
    expect(next.action, TerminalOutputSequenceAction.accept);
    expect(sequencer.sequenceFor('term-1'), 941);
  });
}
