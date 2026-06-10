# codux-protocol

Shared v3.1 remote protocol definitions.

## Owns

- Protocol version and message type constants.
- Runtime resource names and capability payloads.
- Relay envelope DTOs and relay policy helpers.
- Transport candidate DTOs.
- Resource subscription target parsing and routing helpers.
- Terminal buffer payload chunking rules.

## Does Not Own

- Network sockets or WebRTC connections.
- Encryption key storage or authorization policy.
- Terminal history state.
- UI state.

## Used By

- Desktop runtime host.
- Flutter through `codux-protocol-ffi`.
- Rust relay/server code.
- Shared transport and runtime crates.

## Test

```bash
cargo test -p codux-protocol
```
