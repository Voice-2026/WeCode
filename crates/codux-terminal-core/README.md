# codux-terminal-core

Platform-neutral terminal model and traits.

## Owns

- `TerminalDriver` and `TerminalSessionHandle` traits.
- Terminal launch and snapshot DTOs.
- Remote PTY session cache semantics.
- Baseline restore, retained live output, sequence handling, and cache trimming.
- Viewport ownership DTOs.

## Does Not Own

- Process spawning.
- `portable_pty`.
- GPUI or Flutter rendering.
- Network transport.

## Used By

- `codux-terminal-pty` for the local PTY driver interface.
- `codux-protocol-ffi` for Flutter remote terminal state.
- Desktop runtime tests and adapters.

## Test

```bash
cargo test -p codux-terminal-core
```
