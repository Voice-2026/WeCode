# codux-remote-transport

Shared remote transport drivers and transport selection rules.

## Owns

- WebSocket relay host and controller transports.
- WebRTC host and controller direct DataChannel paths with relay fallback.
- Local memory transport for tests and headless paths.
- Relay URL normalization, WebSocket URL building, STUN defaults, and transport preference rules.
- Transport state/message callbacks.

## Does Not Own

- Secure envelope encryption.
- Project, terminal, file, Git, worktree, or AI stats state.
- UI rendering or runtime model mutation.

## FFI

Flutter consumes this crate through `codux-protocol-ffi`, which exposes opaque controller transport handles:

- connect from JSON config;
- send JSON envelope;
- poll state/message events;
- close transport.

## Test

```bash
cargo test -p codux-remote-transport
```
