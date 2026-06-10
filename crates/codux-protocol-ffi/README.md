# codux-protocol-ffi

C ABI bridge for Flutter.

## Owns

- Protocol helper functions exposed to Dart.
- Controller transport opaque handles backed by `codux-remote-transport`.
- Remote terminal session handles backed by `codux-terminal-core`.
- Terminal output sequencer handles.
- JSON-based boundary objects for Dart/Rust interop.

## Does Not Own

- Flutter widgets or app runtime state.
- Native terminal rendering.
- Business domain state beyond opaque shared-core handles.

## Build

```bash
cargo build -p codux-protocol-ffi
```

Mobile platform scripts live in `apps/mobile/plugin/codux_protocol_ffi/scripts`.

## Test

```bash
cargo test -p codux-protocol-ffi
cd apps/mobile && flutter test test/remote_protocol_ffi_test.dart
```
