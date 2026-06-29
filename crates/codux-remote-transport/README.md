# codux-remote-transport

Shared remote transport drivers and transport selection rules.

## Owns

- Iroh host and controller transports.
- Local memory transport for tests and headless paths.
- Codux service URL normalization for pairing tickets and transport preference rules.
- Transport state/message callbacks.
- Web Tunnel protocol (`/codux/web-tunnel/1`) for browser preview traffic.

## Does Not Own

- Secure envelope encryption.
- Project, terminal, file, Git, worktree, or AI stats state.
- UI rendering or runtime model mutation.

## FFI

Flutter consumes this crate through `codux-protocol-ffi`, which exposes opaque controller transport handles:

- connect from JSON config;
- send JSON envelope;
- poll state/message events;
- close transport.

## Web Tunnel

The browser preview path is a tunnel protocol, not a localhost-only port map.
Controllers open an independent Iroh custom ALPN, `/codux/web-tunnel/1`, so page
traffic cannot block the control or terminal streams.

- `tcpConnect` is the only browser tunnel operation. The controller accepts
  local browser HTTP/CONNECT traffic and opens a host-side TCP connection to the
  requested host and port.
- DNS, `/etc/hosts`, VPN, loopback, bound local domains, and LAN reachability are
  resolved on the host side, so the browser can access what the host can access:
  `localhost`, LAN addresses, `.local`, and host-bound development domains.
- HTTPS, WebSocket, HMR, and other upgraded browser traffic work through the same
  byte stream once the CONNECT tunnel is established.
- Preview links and toolbar-launched browser sessions use an external
  Chromium-based browser instance with `--proxy-server` pointed at the
  controller-side listener. Blank toolbar sessions let the user type any
  host-reachable `http`/`https` URL in the browser address bar. Safari/system
  browser fallback is intentionally avoided because it cannot reliably launch a
  per-window proxy-isolated browser session.
- The controller-side `127.0.0.1` listener is only the browser entry point into
  the tunnel; it is not the security boundary and does not mean the target is
  restricted to controller loopback.

## Test

```bash
cargo test -p codux-remote-transport
```
