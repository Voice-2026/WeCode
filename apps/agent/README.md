# Codux Agent

Codux Agent is the headless controlled-agent target. It is intended for Linux servers and other non-GPUI hosts that need to expose Codux runtime domains over the shared remote protocol.

## Responsibilities

- Start without GPUI, tray, or desktop window dependencies.
- Reuse shared protocol, transport, runtime, terminal core, and local PTY crates.
- Provide smoke tests for headless PTY and transport wiring.
- Grow into the standard Linux controlled host for cross-device Codux.

## Current Status

The agent is a thin entry point today. It validates the shared crate boundary and provides smoke commands, but it does not yet expose the full project/file/Git/worktree host runtime.

## Commands

```bash
cargo run -p codux-agent -- --version
cargo run -p codux-agent -- --pty-smoke
cargo run -p codux-agent -- --transport-smoke
```

## Boundary

Agent code should stay small. Shared host runtime behavior belongs in `crates/codux-runtime-core`; transport behavior belongs in `crates/codux-remote-transport`; PTY behavior belongs in `crates/codux-terminal-pty`.
