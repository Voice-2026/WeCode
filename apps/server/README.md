# Codux Server

Codux Server is the Rust relay service for Codux desktop, mobile, and headless peers. It handles persisted pairing, device authorization, v3 ticket exchange, WebSocket fallback routing, and WebRTC signaling support.

## Responsibilities

- Persist host registrations, paired devices, and pairing state in SQLite.
- Issue and prune v3 pairing tickets, including short six-digit pairing codes.
- Route host and controller WebSocket messages for legacy `/api` clients and v3 stateless relay clients.
- Record optional JSONL relay statistics for deployment diagnostics.
- Support v3 protocol relay paths without owning business runtime state.
- Keep relay behavior aligned with `crates/codux-protocol` and `crates/codux-remote-transport`.

## Non-Responsibilities

- It does not decrypt or own terminal, file, project, Git, worktree, or AI stats payloads.
- It does not run PTYs.
- It does not decrypt endpoint-to-endpoint secure messages.

## Commands

```bash
cargo run -p codux-server
cargo test -p codux-server
```

## Configuration

```bash
cargo run -p codux-server -- --config apps/server/config.example.toml
```

The server accepts TOML, environment variables, and CLI flags:

- `CODEX_SERVICE_CONFIG` / `--config`
- `CODEX_SERVER_ADDR` / `--addr`
- `CODEX_SERVER_DB` / `--db`
- `CODEX_STATS_ENABLED` / `--stats`
- `CODEX_STATS_PATH` / `--stats-path`
- `CODEX_STATS_FLUSH_INTERVAL` / `--stats-flush-interval`
- `CODEX_PAIRING_TTL` / `--pairing-ttl`
- `CODEX_SHUTDOWN_TIMEOUT` / `--shutdown-timeout`
- `CODEX_READ_HEADER_TIMEOUT` / `--read-header-timeout`

## Docker

```bash
docker build -f apps/server/Dockerfile -t codux-service .
docker compose -f apps/server/docker-compose.yml up -d
```
