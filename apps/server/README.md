# Codux Server

Codux Server is the Rust v3 relay service. It handles pairing ticket exchange, WebSocket fallback routing, and WebRTC signaling support for Codux peers.

## Responsibilities

- Issue and prune pairing tickets.
- Route host and controller WebSocket messages.
- Support v3 protocol relay paths without owning business runtime state.
- Keep relay behavior aligned with `crates/codux-protocol` and `crates/codux-remote-transport`.

## Non-Responsibilities

- It does not decrypt or own terminal, file, project, Git, worktree, or AI stats payloads.
- It does not run PTYs.
- It does not replace endpoint authorization or pairing confirmation.

## Commands

```bash
cargo run -p codux-server
cargo test -p codux-server
```

The Go relay in `../relay-server` remains during migration for legacy deployments.
