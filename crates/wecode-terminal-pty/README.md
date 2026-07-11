# wecode-terminal-pty

Shared local PTY driver.

## Owns

- `LocalPtyDriver` implementation of `wecode-terminal-core::TerminalDriver`.
- Shell/command spawning through `portable_pty`.
- PTY input, resize, viewport state, output capture, and bounded history.
- Headless-friendly local terminal sessions.

## Does Not Own

- Remote terminal protocol.
- Desktop AI runtime policy.
- GPUI terminal rendering.
- Flutter terminal rendering.

## Used By

- `apps/agent` smoke tests and future headless host runtime.
- Future desktop delegation for local PTY execution.

## Test

```bash
cargo test -p wecode-terminal-pty
cargo run -p wecode-agent -- smoke pty
```
