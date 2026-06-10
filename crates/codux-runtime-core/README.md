# codux-runtime-core

Shared runtime-domain payload and subscription logic.

## Owns

- `host.info` payload shape and capability mapping.
- Project, file, Git, worktree, upload, and terminal payload helpers.
- Runtime subscription router for resource-oriented updates.
- Cross-platform rules that should be identical for desktop, mobile controller views, and headless hosts.

## Does Not Own

- GPUI state.
- Flutter state.
- Actual PTY process management.
- Transport sockets or WebRTC state.

## Direction

Desktop runtime still owns some host-side controller state. New reusable domain state should move here before it is reused by headless or cross-desktop peers.

## Test

```bash
cargo test -p codux-runtime-core
```
