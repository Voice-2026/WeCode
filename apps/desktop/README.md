# Codux Desktop

Codux Desktop is the Rust + GPUI application. It is the primary controller UI and can also act as a remote controlled host for mobile, desktop, and future headless peers.

## Responsibilities

- GPUI windows, panels, menus, shortcuts, terminal renderer, and desktop-specific UX.
- Workspace orchestration for projects, worktrees, Git, files, memory, updates, SSH, and AI CLI sessions.
- Desktop host runtime for v3.1 remote protocol domains.
- Desktop adapters around shared terminal and runtime crates when extra AI/runtime policy is required.
- Packaging and release scripts for the native desktop app.

## Shared Boundaries

Desktop should reuse:

- `crates/codux-protocol` for protocol constants and wire payload helpers.
- `crates/codux-remote-transport` for WebSocket/WebRTC transport drivers.
- `crates/codux-runtime-core` for reusable runtime-domain payload and subscription logic.
- `crates/codux-terminal-core` for terminal model traits and remote terminal semantics.
- `crates/codux-terminal-pty` as the shared local PTY driver target for future delegation.

`runtime/` still contains desktop-specific runtime policy, but new cross-platform logic should move into shared crates first.

## Commands

From the repository root:

```bash
just desktop
cargo run -p codux
cargo test -p codux
cargo test -p codux-runtime
```

Release packaging scripts live in `scripts/release/`.
