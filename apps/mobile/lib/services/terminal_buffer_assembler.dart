import 'package:codux_protocol_ffi/codux_protocol_ffi.dart'
    as codux_terminal_core;

typedef TerminalBufferAssemblyResult =
    codux_terminal_core.TerminalBufferAssemblyResult;

class TerminalBufferAssembler {
  TerminalBufferAssembler({int maxChars = 200000})
    : _core = codux_terminal_core.TerminalBufferAssemblerCore(
        maxChars: maxChars,
      );

  final codux_terminal_core.TerminalBufferAssemblerCore _core;

  TerminalBufferAssemblyResult accept({
    required String sessionId,
    required Map<dynamic, dynamic> payload,
  }) {
    return _core.accept(sessionId: sessionId, payload: payload);
  }

  void remove(String sessionId) {
    _core.remove(sessionId);
  }

  void reset() {
    _core.reset();
  }

  void dispose() {
    _core.dispose();
  }
}
