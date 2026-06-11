import 'package:codux_protocol_ffi/codux_protocol_ffi.dart'
    as codux_protocol_core;

class RemoteSequenceGuard {
  RemoteSequenceGuard({int maxEntriesPerChannel = 128})
    : _core = codux_protocol_core.RemoteSequenceGuardCore(
        maxEntriesPerChannel: maxEntriesPerChannel,
      );

  final codux_protocol_core.RemoteSequenceGuardCore _core;

  bool accept({
    required String type,
    required String? sessionId,
    required int? seq,
  }) {
    return _core.accept(type: type, sessionId: sessionId, seq: seq);
  }

  void reset() {
    _core.reset();
  }

  void dispose() {
    _core.dispose();
  }
}
