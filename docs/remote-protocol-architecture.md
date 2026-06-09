# Codux Remote Protocol Architecture

Codux remote is organized as a layered runtime protocol, not as UI-specific terminal forwarding.

## Roles

- **Desktop app (macOS / Windows)**: can act as a controller and a controlled host.
- **Mobile app (Android / iOS)**: controller only. It does not own local projects, PTYs, Git state, or file state.
- **Linux controlled agent**: planned headless host that exposes the same host-side runtime domains without a GUI.
- **Relay service**: discovery, pairing-ticket exchange, signaling, and WebSocket fallback transport. It does not own runtime state.

## Layers

```text
UI
  Reads runtime state and emits user intent.

Runtime store / controller
  Owns project, terminal, file, Git, worktree, and AI-stat decisions.
  Converts protocol messages into normalized runtime state.

Protocol
  Defines versioning, capabilities, secure envelopes, message domains,
  sequence handling, terminal buffer windows, chunking, progress, ack,
  and retry semantics.

Transport drivers
  Move protocol envelopes over local memory, WebSocket relay, WebRTC
  DataChannel, or future transports such as QUIC/WebTransport.
```

The UI must not branch on transport type. Git, file, terminal, worktree, and AI-stat features consume the same runtime API whether the active transport is local, WebRTC, WebSocket relay, or a future driver.

## v3.1 Capabilities

`host.info` advertises the protocol version and host capabilities:

- `protocolVersion`: currently `v3.1`.
- `capabilities.domains`: supported runtime domains such as `project`, `terminal`, `worktree`, `file`, and `aiStats`.
- `capabilities.terminalBuffer`: terminal history limits and chunking support.

Terminal history is sent as bounded buffer windows. Large snapshots can be split into `chunked` payloads identified by `snapshotId`, `chunkIndex`, and `chunkCount`. Controllers assemble chunks by session and snapshot before rendering. This keeps large Codex resume histories from becoming one oversized transport message and gives mobile a real progress value.

## Runtime Domains

The protocol is domain-oriented:

- `project.*`: project list, selection, add/edit/remove.
- `terminal.*`: terminal list, create/close, resize, input, output, buffer, upload.
- `worktree.*`: list/select/create/merge/remove.
- `file.*`: list/read/write/rename/delete.
- `ai.stats`: project-scoped AI usage summary.

Future Git-specific controller messages should follow the same pattern instead of binding Git logic to any transport or UI widget.

## Reuse Plan

The current desktop repository keeps the Rust protocol module in `runtime/src/remote/protocol.rs`. Once v3.1 is stable across desktop and mobile, that module can be split into a shared crate or standalone repository. Flutter can then adopt the shared Rust protocol through FFI if the added Android NDK and iOS framework build complexity is justified.
