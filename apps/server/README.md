# Codux Server

Codux Server is an optional Rust service for Codux host registration, device records, and deployment health checks. Pairing and runtime traffic are carried directly by Iroh transport tickets and streams, not by this service.

## Responsibilities

- Persist host registrations and paired device records in SQLite.
- Expose health checks for deployments that still run a service process.

## Non-Responsibilities

- It does not issue pairing tickets or short pairing codes.
- It does not decrypt or own terminal, file, project, Git, worktree, or AI stats payloads.
- It does not proxy runtime messages.
- It does not run PTYs.
- It does not participate in endpoint-to-endpoint secure messages.

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
- `CODEX_SHUTDOWN_TIMEOUT` / `--shutdown-timeout`
- `CODEX_READ_HEADER_TIMEOUT` / `--read-header-timeout`

## Docker

```bash
docker build -f apps/server/Dockerfile -t codux-service .
docker compose -f apps/server/docker-compose.yml up -d
```
