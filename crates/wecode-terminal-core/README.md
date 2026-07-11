# wecode-terminal-core

Platform-neutral terminal model and traits.

## Owns

- `TerminalDriver` and `TerminalSessionHandle` traits.
- Terminal launch and state DTOs.
- Remote PTY session cache semantics.
- Remote runtime project/terminal selection model and action planning.
- Baseline restore, retained live output, sequence handling, and cache trimming.
- `libghostty-vt` backed headless screen read model for controller-side rendering.
- Viewport ownership DTOs.

## Does Not Own

- Process spawning.
- `portable_pty`.
- GPUI or Flutter rendering.
- Network transport.

## Used By

- `wecode-terminal-pty` for the local PTY driver interface.
- `wecode-protocol-ffi` for Flutter remote terminal state.
- Desktop runtime tests and adapters.

## Layering Rule

Controller UIs do not decide when to resend project selection or terminal list
requests. They submit project, terminal, and host acknowledgement events to
`RemoteRuntimeModel` and execute the returned plan. This keeps project switching,
stale host lists, delayed terminal lists, and terminal binding consistent across
Flutter, desktop, and future headless controllers.

## Test

```bash
cargo test -p wecode-terminal-core
```
