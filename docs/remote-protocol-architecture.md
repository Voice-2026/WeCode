# Codux Remote Protocol Architecture

Codux remote is organized as a layered runtime protocol, not as UI-specific terminal forwarding.

## Roles

- **Desktop app (macOS / Windows)**: can act as a controller and a controlled host.
- **Mobile app (Android / iOS)**: controller only. It does not own local projects, PTYs, Git state, or file state.
- **Linux controlled agent**: planned headless host that exposes the same host-side runtime domains without a GUI.
- **Service**: host registration, device records, and short-lived pairing-ticket exchange. It does not forward runtime messages or own runtime state.

## Target Layers

```text
UI renderer
  Reads runtime models and emits user intent. It never consumes transport
  messages directly and never owns terminal history, sequence, or resync logic.

Runtime models / buffer pools
  Own project, terminal session, file, Git, worktree, and AI-stat state.
  Every baseline and live delta enters these models before UI rendering.
  Terminal data enters a local or remote PTY session model; UI attaches to
  that model the same way for local and remote terminals.

Bidirectional subscription layer
  Owns resource.subscribe, resource.unsubscribe, baseline, delta, ack, and
  resync semantics. Any peer may publish resources and subscribe to resources
  exposed by the other peer.

Protocol router
  Defines versioning, capabilities, secure envelopes, message domains,
  sequence handling, requestId, error, and schema compatibility.

Transport drivers
  Move protocol envelopes over local memory or the Iroh QUIC transport.
```

The UI must not branch on transport type. Git, file, terminal, worktree, and AI-stat features consume the same runtime API whether the active transport is local memory or Iroh.

## Bidirectional Resource Model

Codux remote should converge on a peer-to-peer resource model instead of a controller-only request model. Each peer can:

- publish resources it owns, such as terminal sessions, file trees, Git state, projects, or worktrees;
- subscribe to resources exposed by the other peer;
- receive a baseline for the subscribed resource;
- receive ordered deltas after the baseline;
- acknowledge processed sequence ranges;
- request resync when sequence gaps or incompatible state are detected.

Terminal sessions are one resource type in this model. A controller subscribes to a terminal session or project terminal scope. The host sends a baseline buffer window, then streams `terminal.output` deltas. While the controller is hydrating the baseline, newer deltas are held in the remote PTY session buffer and replayed after the baseline sequence. UI code only attaches to the resulting runtime model.

## v3.1 Capabilities

`host.info` advertises the protocol version and host capabilities:

- `protocolVersion`: currently `v3.1`.
- `capabilities.domains`: supported runtime domains such as `project`, `terminal`, `worktree`, `file`, and `aiStats`.
- `capabilities.terminalBuffer`: terminal history limits and chunking support.

Terminal history is sent as bounded buffer windows. Large restore windows can be split into `chunked` payloads identified by `snapshotId`, `chunkIndex`, and `chunkCount`; the `snapshotId` field is a wire-level restore transaction id kept for v3.1 compatibility, not a separate screen snapshot source. Controllers assemble chunks by session and transaction before writing the bytes into `RemotePtySession`. This keeps large Codex resume histories from becoming one oversized transport message and gives mobile a real progress value.

## Runtime Domains

The protocol is domain-oriented:

- `project.*`: project list, selection, add/edit/remove.
- `terminal.*`: terminal list, create/close, resize, input, output, buffer, upload.
- `worktree.*`: list/select/create/merge/remove.
- `file.*`: list/read/write/rename/delete.
- `ai.stats`: project-scoped AI usage summary.

Future Git-specific controller messages should follow the same pattern instead of binding Git logic to any transport or UI widget.

## Current Terminal Alignment

The current Mac host and Flutter controller are being aligned to the target model:

- Mac host owns the real local PTY session.
- Flutter owns a `RemotePtySession` model for each subscribed remote session.
- `resource.subscribe` with `resource=terminals` is the standard subscription entry point.
- `terminal.subscribe` remains host-side compatibility for older controller builds only.
- `terminal.buffer` remains the baseline/hydration payload while the protocol migrates toward generic `resource.baseline`.
- Live `terminal.output` deltas are written into `RemotePtySession`, not directly into UI.
- UI/native terminal rendering only replays the model for the active session.

## Shared Crate Boundary

The desktop repository now starts the shared Rust boundary inside the workspace:

- `crates/codux-protocol`: protocol version, capabilities, shared secure/relay envelope DTOs, transport candidate DTOs, subscription helpers, a shared resource subscription registry, chunking, and baseline payload construction.
- `crates/codux-remote-transport`: shared remote transport boundary. It owns the Iroh host/controller transport, local memory transport for tests/headless paths, relay URL normalization for pairing tickets, transport path state callbacks, and transport factory rules. It does not know about terminal, Git, file, or UI state.
- `crates/codux-protocol-ffi`: C ABI for Flutter protocol and terminal-core bindings.
- `crates/codux-runtime-core`: shared runtime domain boundary. It owns common host.info, project, file, Git, worktree, upload, and terminal payload rules, shared terminal domain interfaces, and `RuntimeSubscriptionRouter`; desktop host already delegates those protocol shapes and subscription routing to this crate.
- `crates/codux-terminal-core`: platform-neutral terminal session semantics such as sequence, buffer-window restore, retained live output, and cache limits.
- `crates/codux-terminal-pty`: shared local PTY driver for host/headless targets.

Protocol, runtime domain, terminal core, and PTY driver crates are intentionally separate. Protocol describes what is sent between peers, including relay envelopes and transport candidates. Runtime core owns domain-level models and payloads such as `host.info` and project/file/Git/worktree shapes. Terminal core owns terminal state semantics. Terminal PTY owns local process execution. This keeps future local PTY, remote PTY, Linux headless, and mobile rendering paths aligned without coupling transport schemas to terminal storage internals.

Flutter now uses the Rust-backed `codux_protocol_ffi` plugin for protocol envelope construction, relay policy, controller-side transport URL/STUN/selection rules, controller transport handles, and remote terminal session state. Dart still keeps compile-time constants and UI/runtime model wiring where Flutter requires const switch cases or native widget state. It does not keep duplicate terminal restore/sequence/replay, transport normalization, or controller transport lifecycle logic; Dart only stores token-to-object references for Dart-owned envelopes and UI state that cannot cross the FFI boundary.

The desktop host now uses `codux-runtime-core::RuntimeSubscriptionRouter` for terminal viewers and project-scoped runtime resources such as Git status, worktrees, and AI stats. Project and terminal list refreshes also use the shared subscription route, with project list payloads rebuilt per device so each controller keeps its own selected-project scope.

Desktop, headless, and Flutter controller targets consume the shared Iroh transport crate instead of keeping transport driver code inside app runtimes. Flutter controller code uses a Rust-backed opaque transport handle with event polling for state/message delivery. The desktop runtime still owns pairing settings, encryption, authorization, and runtime domain routing; the transport crate only moves protocol envelopes.

## Terminal Driver Boundary

`codux-terminal-core` is not a process launcher. It owns platform-neutral terminal state after bytes or restore windows have been decoded, and defines the shared `TerminalDriver` / `TerminalSessionHandle` interface used by local, remote, desktop, and headless terminal sessions.

`codux-terminal-pty` is the shared local PTY driver crate. It wraps `portable_pty`, spawns shells or commands, captures output history, emits terminal events, and implements the shared terminal driver interface. This is the crate a Linux headless controlled agent should start from.

`apps/agent` is the headless controlled-agent app target. It is intentionally separate from the desktop app: it links protocol, terminal core, and local PTY driver crates, but it does not link GPUI, desktop windows, tray code, or desktop runtime UI policy.

Desktop `TerminalManager` remains a desktop adapter for now because it also binds AI runtime state, memory/tool environment, desktop session metadata, and existing app-specific behavior. It should progressively delegate local PTY execution to `codux-terminal-pty` while keeping desktop-specific policy outside the shared driver.

Current boundary:

```text
codux-terminal-core
  TerminalDriver / TerminalSessionHandle trait
  RemotePtySession / sequence / restore / cache semantics

codux-terminal-pty
  LocalPtyDriver(portable_pty)
  suitable for Linux headless and future desktop delegation

apps/agent
  headless controlled-agent binary
  no GPUI / no desktop runtime UI

controller side
  RemotePtySession(protocol model)

UI
  attaches to runtime models only
```

This lets desktop local terminals, future Linux headless terminals, and remote controller terminals share the same runtime-facing API without forcing mobile to link local process PTY code.
