# Iroh-only Remote Link Design

## Goal

Replace the existing WebRTC/WebSocket remote transport with a single Iroh-only remote link. Codux business messages stay unchanged; only the transport layer changes.

## Non-goals

- Do not replace `RemoteEnvelope` message types such as `project.list`, `terminal.input`, or `terminal.viewport.scroll`.
- Do not keep WebRTC/WebSocket relay as a fallback in the main runtime path.
- Do not move terminal scrollback or rendering responsibility into Flutter.

## Architecture

The new transport boundary is `RemoteTransport` as used by desktop runtime and mobile FFI. Its implementation becomes Iroh-only.

```text
desktop / mobile / headless
  -> codux-remote-transport RemoteTransport
  -> Iroh endpoint + ALPN /codux/remote/1
  -> Iroh direct path or Iroh relay fallback
```

Old modules `webrtc.rs`, `websocket.rs`, and controller-side manual health probing are removed from the main link path. Pairing HTTP APIs may remain for ticket/code exchange, but the pairing payload must advertise only an Iroh candidate.

## Transport Candidate

Add a single transport kind:

- `iroh`

The candidate payload carries:

- `kind`: `iroh`
- `role`: `host`
- `url`: pairing server / relay preset URL for compatibility with existing UI fields
- `nodeId`: host Iroh node id
- `relayUrl`: selected Iroh relay URL

The mobile parser must reject payloads that do not contain a usable `iroh` candidate.

## Connection Model

Host:

- Starts one Iroh endpoint.
- Publishes its node id in pairing payload.
- Accepts connections for ALPN `/codux/remote/1`.
- For each accepted connection, reads length-delimited frames and forwards bytes to existing `handle_transport_message`.

Controller:

- Starts one Iroh endpoint per active remote connection.
- Dials the host node id with ALPN `/codux/remote/1`.
- Sends and receives length-delimited frames.
- Publishes state events:
  - `connecting`
  - `connected:path=direct`
  - `connected:path=relay`
  - `closed`
  - `failed:<reason>`
  - `latency:rtt=<ms>;path=<direct|relay>`

Path state should come from Iroh connection info when available. If the API cannot provide a reliable path synchronously, state may start as `connected:path=unknown` and update after the first path sample. It must not use the old WebRTC probe API.

## Framing

Each Codux envelope is one frame:

- 4-byte big-endian length
- UTF-8 JSON bytes

This keeps the existing FFI event shape unchanged:

- `message`: JSON string
- `state`: state string
- `log`: debug-only diagnostic

## Pairing Server

The existing server remains for:

- ticket/code lookup
- pairing request approval
- device authorization metadata

It should advertise only Iroh candidates in new payloads. It does not forward runtime business messages in the Iroh-only path.

## Compatibility

The target is a breaking transport migration for the next version. Existing paired devices must pair again if their cached candidate list has no `iroh`.

During implementation, old source files may remain temporarily if tests still reference them, but desktop/mobile runtime factories must not select them.

## Tests

Rust:

- Protocol candidate serialization prefers only `iroh`.
- `RemoteTransportFactory::connect_controller` rejects configs without an `iroh` candidate.
- Length-delimited frame codec round-trips multiple envelopes and rejects oversized frames.
- Local in-process Iroh host/controller can exchange a `host.info` envelope.

Dart:

- Pairing parser accepts `iroh` payloads.
- Pairing parser rejects payloads without an Iroh candidate.
- Stored device config sent to FFI contains `nodeId` and `relayUrl`.
- UI transport label maps `iroh` path states to direct/relay display.

Manual:

- Desktop dev host starts Iroh endpoint and pairing QR contains `iroh`.
- Android dev app pairs, connects, shows latency, enters terminal.
- Kill/restart desktop host; mobile transitions to closed/connecting and reconnects after host returns.
- Switch network; path recovers without WebRTC probe calls.
