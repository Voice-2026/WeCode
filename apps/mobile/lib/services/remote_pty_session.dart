import 'package:wecode_protocol_ffi/wecode_protocol_ffi.dart'
    as wecode_terminal_core;

// The terminal output orchestration + per-session PTY state now live in the
// shared Rust core (RemoteTerminalOutputRouter, reached via the FFI
// RemoteOutputRouter). Only these screen-snapshot type aliases remain here,
// used by the UI layer.
typedef RemoteTerminalScreenSnapshot =
    wecode_terminal_core.TerminalScreenSnapshot;
typedef RemoteTerminalScreenCell = wecode_terminal_core.TerminalScreenCell;
