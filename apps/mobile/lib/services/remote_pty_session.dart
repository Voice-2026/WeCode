import 'package:codux_protocol_ffi/codux_protocol_ffi.dart'
    as codux_terminal_core;

// The terminal output orchestration + per-session PTY state now live in the
// shared Rust core (RemoteTerminalOutputRouter, reached via the FFI
// RemoteOutputRouter). Only these screen-snapshot type aliases remain here,
// used by the UI layer.
typedef RemoteTerminalScreenSnapshot =
    codux_terminal_core.TerminalScreenSnapshot;
typedef RemoteTerminalScreenCell = codux_terminal_core.TerminalScreenCell;
