# codux_protocol_ffi

Flutter FFI bindings for Codux Rust protocol and terminal core crates.

The source of truth is Rust:

- `crates/codux-protocol`: v3 message names, resource names, relay policy, payload helpers.
- `crates/codux-terminal-core`: remote terminal session cache, sequence, buffer-window restore, and held-live replay selection.
- `crates/codux-protocol-ffi`: C ABI exported for Flutter.
- `apps/mobile/plugin/codux_protocol_ffi`: Dart FFI loader and thin API.

Dart keeps compile-time constants only where Flutter requires them, such as
`switch` cases. Runtime helpers, protocol envelope builders, and remote terminal
session state should call this plugin so mobile, desktop, and server stay
aligned to one Rust source. Do not add duplicate Dart fallback implementations
for protocol routing, relay policy, terminal restore handling, sequence, or
held-live replay.

Build the Rust library manually when testing platform packaging:

```sh
apps/mobile/plugin/codux_protocol_ffi/scripts/build-rust.sh aarch64-apple-darwin
apps/mobile/plugin/codux_protocol_ffi/scripts/build-rust.sh aarch64-apple-ios
apps/mobile/plugin/codux_protocol_ffi/scripts/build-rust.sh aarch64-linux-android
```
