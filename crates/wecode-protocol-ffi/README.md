# wecode-protocol-ffi

C ABI bridge for Flutter.

## Owns

- Protocol helper functions exposed to Dart.
- Controller transport opaque handles backed by `wecode-remote-transport`.
- Remote terminal session and headless screen handles backed by `wecode-terminal-core`.
- Terminal output sequencer handles.
- Remote runtime model handles for project selection, terminal list binding, and
  stale acknowledgement handling.
- JSON-based boundary objects for Dart/Rust interop.

## Does Not Own

- Flutter widgets.
- Terminal drawing policy.
- Business domain state beyond opaque shared-core handles.

## Build

```bash
cargo build -p wecode-protocol-ffi
```

Mobile platform scripts live in `apps/mobile/plugin/wecode_protocol_ffi/scripts`.

## Test

```bash
cargo test -p wecode-protocol-ffi
cd apps/mobile && flutter test test/remote_protocol_ffi_test.dart
```
