import 'terminal_output_sequencer.dart';

class TerminalOutputResyncResult {
  const TerminalOutputResyncResult({
    required this.render,
    required this.ack,
  });

  final bool render;
  final int? ack;
}

TerminalOutputResyncResult observeTerminalOutputForResync({
  required TerminalOutputSequencer sequencer,
  required String sessionId,
  required bool isBuffer,
  required int? outputSeq,
  required int? offset,
  bool resetsSequence = false,
}) {
  final sequence = sequencer.observe(
    sessionId: sessionId,
    isBuffer: isBuffer,
    outputSeq: outputSeq,
    offset: offset,
    resetsSequence: resetsSequence,
  );
  switch (sequence.action) {
    case TerminalOutputSequenceAction.accept:
    case TerminalOutputSequenceAction.baseline:
      return TerminalOutputResyncResult(
        render: true,
        ack: outputSeq,
      );
    case TerminalOutputSequenceAction.duplicate:
      return TerminalOutputResyncResult(
        render: false,
        ack: outputSeq,
      );
  }
}
