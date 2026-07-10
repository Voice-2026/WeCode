# codux-gateway — Roadmap

`crates/codux-gateway` is a Rust rewrite of the Python `kiro-gateway` (vendored
read-only under `reference/kiro-gateway/` during the port). It exposes
OpenAI- and Anthropic-compatible APIs backed by Kiro / AWS CodeWhisperer's
`generateAssistantResponse` endpoint.

## Done — Phases 1–4 (single-account standalone gateway)

- **Phase 1** — repo scaffold, AGPL relicense (`LICENSE` + `NOTICE`), crate
  skeleton, axum router, `GET /health`, API-key middleware, standalone
  `codux-gateway` binary, graceful shutdown, workspace member.
- **Phase 2** — auth: credential sources (JSON file / kiro-cli SQLite / raw
  refresh token), both refresh flows (Kiro Desktop + AWS SSO OIDC), proactive
  600s refresh, 403 force-refresh, write-back (`src/auth/`).
- **Phase 3** — upstream client with retry (`src/upstream/mod.rs`), heuristic
  AWS event-stream parser (`src/upstream/event_stream.rs`), unified→Kiro
  converter (`src/convert/kiro.rs`), Anthropic `/v1/messages` streaming (11
  events) + non-streaming + `/v1/messages/count_tokens`, model resolver.
- **Phase 4** — OpenAI `/v1/chat/completions` (stream + non-stream) and
  `/v1/models`, OpenAI adapter (tool_calls, images, `reasoning_effort`).

### Not yet verified live
The upstream call path (`request_kiro` → event-stream parse → SSE) is unit-tested
against fixtures but has **not** been run against a real Kiro account or
diffed side-by-side against the Python gateway (no credentials were available
during the port). Before trusting parity, run the standalone binary with real
credentials and compare against `reference/kiro-gateway` on identical prompts
(text, tool_use, images) for both APIs.

## Done — Phase 5 (multi-account + failover)

`crates/codux-gateway/src/accounts.rs`:
- `accounts` array in config → per-account `KiroAuth`; empty → single-account.
- `state.json` persistence (atomic tmp+rename) with `current_account_index` and
  per-account failure/stat state.
- Sticky global index; circuit breaker `60s * 2^(failures-1)` capped at 1 day,
  10% probabilistic retry during cooldown.
- `request_with_failover` loop (`MAX_ATTEMPTS = accounts * 2`), FATAL vs
  RECOVERABLE classification (`classify_error`); single-account bypasses breaker.
- Returns the chosen `Arc<KiroAuth>` so routes can reuse it (e.g. for MCP).

## Done — Phase 6a (thinking parser + truncation recovery)

- `src/thinking.rs` — FSM extracting `<thinking>`/`<think>`/… blocks, wired into
  `kiro_event_stream` (emits `KiroEvent::Thinking`). OpenAI → `reasoning_content`
  (or text per mode); Anthropic → `thinking` content blocks. Gated on
  `fake_reasoning` (default off).
- `src/truncation.rs` — in-memory store keyed by tool_call_id / content hash;
  saved on the response side (streaming + non-streaming) and consumed on the next
  request via `inject_notices`. Simplification vs Python: injects a single trailing
  user notice rather than per-tool `is_error` tool_results.

## Done — Phase 6b (web search Path B + tiktoken)

- `src/mcp.rs` — `POST {api_host}/mcp` JSON-RPC `web_search`, summary generation,
  and tool auto-injection. Path B: inject the `web_search` tool (when
  `web_search_enabled`), intercept its calls in the stream/collect paths, run the
  MCP search, and fold the summary into content. Gated off by default.
- `tiktoken` cargo feature → cl100k_base counts in `src/tokens.rs` (default build
  keeps the chars/4 ×1.15 heuristic).

### Not done in 6b
- **Path A** (native Anthropic `server_tool_use` early-return when the client
  itself sends a `web_search` tool): the SSE emulation exists in the reference
  (`mcp_tools.py:generate_anthropic_web_search_sse`) but is not ported. Path B
  covers the common auto-search case.

## Done — Phase 6c backbone (embedded gateway service)

- `apps/desktop/runtime/Cargo.toml` depends on `codux-gateway`.
- `apps/desktop/runtime/src/gateway_service.rs` — `GatewayService` reads the
  `"gateway"` section of `settings.json` (`{ enabled, config }`), starts the
  server on the shared `async_runtime`, and stops it via a oneshot channel
  (`stop` / `restart_from_support_dir` / `Drop`). `GatewaySettings::{load,save}`
  round-trip through `ConfigStore`.

## Done — Phase 6c desktop integration (settings pane + lifecycle)

- `apps/desktop/src/app/settings/panes/gateway.rs` adds the GPUI Gateway settings
  pane with service status, endpoint fields, API key, region, web search, and
  credential source controls.
- `apps/desktop/src/app/settings/mod.rs` registers the Gateway tab
  (`settings.tab.gateway`) with `ServerStack` icon and render dispatch.
- `CoduxApp` owns `GatewaySettings` and the main-window `GatewayService`; startup
  reads `settings.json.gateway`, setting changes save through `GatewaySettings`,
  and the main window reloads/restarts the service on settings updates.
- `GatewayService` exposes process-wide runtime status so the settings window can
  show listening address or startup errors (for example, port already in use).
- Verified with `cargo check -p codux` and a local dev-app visual smoke: the
  Settings window opens and the Gateway tab renders without crashing.

## Cleanup
- Remove `reference/kiro-gateway/` once live parity is confirmed.
- `justfile` gateway target (`cargo run -p codux-gateway`).
