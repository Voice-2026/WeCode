<p align="center">
  <img src="docs/images/icon.png" width="128" height="128" alt="WeCode">
</p>

<h1 align="center">WeCode</h1>

<p align="center">
  <b>The high-performance AI coding terminal — desktop, phone, and server, one workspace</b><br/>
  Built with <b>Rust + GPUI</b>, WeCode unifies Codex, Claude Code, and 8+ AI coding CLIs with live agent status, token analytics, local memory, credential-isolated SSH &amp; database access, and encrypted device links for taking over long-running agent work from anywhere.
</p>

<p align="center">
  <a href="https://github.com/Voice-2026/WeCode/releases/latest"><img src="https://img.shields.io/github/v/release/Voice-2026/WeCode?label=release&color=blue" alt="Latest release"></a>
  <a href="https://github.com/Voice-2026/WeCode/releases"><img src="https://img.shields.io/github/downloads/Voice-2026/WeCode/total?label=downloads&color=brightgreen" alt="Total downloads"></a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-8250df" alt="Platform">
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Voice-2026/WeCode?color=orange" alt="License"></a>
  <a href="https://github.com/Voice-2026/WeCode/stargazers"><img src="https://img.shields.io/github/stars/Voice-2026/WeCode?color=yellow" alt="GitHub stars"></a>
</p>

<p align="center">
  <a href="https://wecode.dux.cn">Website</a> &middot;
  <a href="https://wecode.dux.cn/zh-cn/getting-started/">Docs</a> &middot;
  <a href="https://github.com/Voice-2026/WeCode/releases/latest">Download</a> &middot;
  <a href="https://github.com/Voice-2026/WeCode/issues">Feedback</a>
</p>

<p align="center">
  English | <a href="README.zh-CN.md">简体中文</a> | <a href="README.ja.md">日本語</a> | <a href="README.ko.md">한국어</a>
</p>

---

![WeCode](docs/images/screenshot.png)

## Why WeCode

AI coding CLIs are incredibly powerful — and incredibly easy to lose control of. Real work sprawls across projects, Git worktrees, terminals, sessions, tokens, remote shells, and context you half-remember. **WeCode turns that chaos into one durable, native workspace built for serious AI coding.**

| When AI coding gets messy | WeCode gives you |
| :------------------------ | :-------------- |
| Every AI CLI has its own state | One project-aware view across Codex, Claude Code, OpenCode, Kiro CLI, Kimi Code, CodeWhale, MiMo Code, and Agy. |
| External agents need reliable control | A JSON product CLI for projects, worktrees, sessions, models, terminals, and scheduled automations. |
| Long agent runs are hard to resume | Live status, local history, session restore, and context that follows each worktree. |
| Parallel tasks collide | A worktree-first model where every task keeps its own terminals, Git state, files, and AI sessions. |
| Token spend is a black box | Usage by tool, model, project, worktree, and day — no spreadsheets. |
| Context evaporates between sessions | Local memory for habits, project profiles, and module notes, injected back into supported CLIs automatically. |
| Credentials end up in prompts | Saved, tested SSH and database profiles, plus `wecode-ssh` / `wecode-db` commands agents can use **without ever seeing your credentials**. |
| You walk away mid-run | Pair your phone over P2P / relay links and keep driving the session from anywhere. |
| The code lives on another machine | Connect a headless host — a server, spare Mac, or Linux box — and drive its terminals, Git, and AI as if they were local. |

WeCode is **not** another editor. It's the control plane for developers who live in AI coding CLIs and need a rock-solid way to run multi-project, long-running agent work.

## Quick Start

macOS — install with [Homebrew](https://brew.sh):

```bash
brew install --cask Voice-2026/tap/wecode
```

1. **Open a project.** Git worktrees, project state, and per-project sessions are picked up automatically.
2. **Start your AI CLI in the built-in terminal** — `codex`, `claude`, `opencode`, and friends. The non-invasive wrapper lights up live status, token tracking, and memory injection with zero configuration.
3. **Leave the desk.** Pair your phone or a headless host once, then take over the same running session from anywhere.

Without Homebrew, see [Download](#download).

## Your Credentials Never Reach the AI

Agents constantly need servers and databases — but pasting a password into a prompt, or letting the model read your config files, is exactly how credentials leak. WeCode stores connection profiles locally and hands agents two safe commands instead:

- **`wecode-ssh`** — the agent runs `wecode-ssh list`, sees profile names and hosts only, and connects through the wrapper. Passwords and keys are injected inside WeCode's helper process; they never enter the model's context, the transcript, or your shell history.
- **`wecode-db`** — the same isolation for MySQL / PostgreSQL / SQLite: saved once in WeCode, queried by profile name. Read-only profiles are enforced inside the wrapper with a single-statement allowlist, so the model can't escalate its own access.
- **Zero per-project setup.** Every supported CLI learns about these commands automatically through WeCode's environment directives.

<p align="center"><img src="docs/images/credential-isolation.png" alt="wecode-ssh list shows profile names and hosts only — never passwords"></p>

## AI CLI Compatibility

WeCode uses non-invasive wrappers and per-tool adapters. It does not write project prompt files or mutate your global AI CLI configuration just to inject WeCode context.

| AI CLI | Live status | Token usage | Model setting | Full-access mode | Environment directives |
| :--- | :---: | :---: | :---: | :---: | :--- |
| Codex | ✓ | ✓ | ✓ | ✓ | ✓ via developer instructions |
| Claude Code / reclaude | ✓ | ✓ | ✓ | ✓ | ✓ via `--append-system-prompt` |
| OpenCode | ✓ | ✓ | ✓ | ✓ | ✓ via managed plugin config |
| MiMo Code | ✓ | ✓ | ✓ | ✓ | ✓ via managed plugin config |
| Kimi Code | ✓ | ✓ | ✓ | — | ✓ via managed `--agent-file` |
| Kiro CLI | ✓ | ✓ | ✓ | ✓ | Not injected; no confirmed non-invasive prompt channel |
| CodeWhale | ✓ | ✓ | ✓ | ✓ | Not injected for interactive sessions |
| Agy | ✓ | ✓ | ✓ | ✓ | Not injected; no confirmed non-invasive prompt channel |

Environment directives include WeCode memory plus runtime commands such as `wecode-ssh` and `wecode-db`. For unsupported tools, WeCode still tracks sessions where possible, but it will not force prompt injection through project files or user-level config.

## Product CLI & Automations

The bundled `wecode` product CLI lets other agents control the running Desktop through a stable JSON protocol. It can discover projects and models, create or resume sessions, send prompts, manage worktrees and terminals, and operate scheduled automations.

```bash
wecode app status --json
wecode session create --project <project-id> --agent <agent-id> --model <model-id> --json
wecode automation list --json
wecode automation run --id <automation-id> --json
```

The included `wecode-control` Skill documents the complete contract for Codex and other external agents. New automation tasks default to **Claude + Kiro** with **Opus 4.8**, while the editor and CLI also support explicit model selection.

## One Workspace, Every Device

> **Beta.** Connecting to a headless host ships first as a beta in this release — the connection, pairing, and host-side data flow are still under active testing, so expect rough edges. Feedback is very welcome.

Desktop, phone, and a headless host all act as **peers** over end-to-end encrypted **P2P / relay links**, so you can keep driving long agent runs from anywhere.

- **Direct when possible.** WeCode prefers P2P paths and falls back to relay when the network requires it.
- **Not SSH remote desktop.** Pair devices once, then connect straight into WeCode itself.
- **No public IP required.** Desktop, phone, and host can pair and reconnect across ordinary home, office, and mobile networks.

```mermaid
flowchart LR
    subgraph drivers["You drive from"]
        P["📱 Phone"]
        D["💻 Desktop"]
    end
    subgraph hosts["Work runs on (the host)"]
        D2["💻 Another desktop"]
        H["🖥️ Headless host<br/>server · spare Mac · Linux"]
    end
    P -->|"🔒 P2P / Relay"| D2
    P -->|"🔒 P2P / Relay"| H
    D -->|"🔒 P2P / Relay"| D2
    D -->|"🔒 P2P / Relay"| H
```

Any controller — a **desktop** or a **phone** — can connect to any host — **another desktop** or a **headless host**. A desktop is both: it hosts its own projects and can drive others; a phone drives only. The work stays on the host machine, so switching devices does not interrupt the session.

- **Phone handoff.** Pair in seconds and continue the same terminals, history, and AI sessions from your phone.
- **Headless host.** Run `wecode` on a server, spare Mac, or Linux box and drive its terminals, Git, and AI as if they were local. See [`apps/agent/README.md`](apps/agent/README.md).
- **Session continuity.** Reconnect to the same running shells and agent sessions after disconnects.

## Your Terminal Pet

Every token your agents burn feeds a pixel pet that lives in your workspace. Hatch it, name it, and watch it level up as you code — its five stats (Wisdom, Chaos, Night, Stamina, Empathy) grow out of how, and when, you actually work. Install custom sprite pets, or retire old companions into your hall of fame.

Completely useless. Absolutely essential.

<p align="center"><img src="docs/images/pet.png" width="320" alt="WeCode terminal pet"></p>

## Local-First by Design

- **Your data stays yours.** Projects, terminals, sessions, memory, token stats, and credentials live on your machines — there is no WeCode cloud and no account to sign up for.
- **Encrypted device links.** Desktop ⇄ phone ⇄ host traffic is end-to-end encrypted; relays only forward ciphertext when a direct P2P path isn't possible.
- **Non-invasive by principle.** WeCode never writes prompt files into your repositories and never mutates your AI CLIs' global configs — all context injection goes through wrappers and per-tool adapters you can inspect.

## Download

**Desktop app**

macOS — install with [Homebrew](https://brew.sh):

```bash
brew install --cask Voice-2026/tap/wecode
```

Or download directly:

| Platform | Download |
| :--- | :--- |
| macOS · Apple Silicon | [⬇ `wecode-macos-aarch64.dmg`](https://github.com/Voice-2026/WeCode/releases/latest/download/wecode-macos-aarch64.dmg) |

Open the macOS `.dmg` and drag WeCode to Applications. Then open a project, start your AI CLI, and go.

**Headless host (`wecode-agent`)** — Beta, ships with 2.0

macOS / Linux — one line (auto-detects OS/arch, installs as `wecode` on your `PATH`):

```bash
curl -fsSL https://raw.githubusercontent.com/Voice-2026/WeCode/main/apps/agent/scripts/install.sh | sh
```

Flags: `--beta` · `--version <x.y.z>` · `--dir <path>` · `--setup` · `--mirror <prefix>` (if GitHub is slow where you are) · `--uninstall`. Or download the binary directly:

| Platform | Download |
| :--- | :--- |
| macOS · Apple Silicon | [⬇ `wecode-macos-aarch64`](https://github.com/Voice-2026/WeCode/releases/latest/download/wecode-macos-aarch64) |
| macOS · Intel | [⬇ `wecode-macos-x86_64`](https://github.com/Voice-2026/WeCode/releases/latest/download/wecode-macos-x86_64) |
| Linux · arm64 | [⬇ `wecode-linux-aarch64`](https://github.com/Voice-2026/WeCode/releases/latest/download/wecode-linux-aarch64) |
| Linux · x64 | [⬇ `wecode-linux-x86_64`](https://github.com/Voice-2026/WeCode/releases/latest/download/wecode-linux-x86_64) |

Put the binary on your `PATH` as `wecode`, then run `wecode config` → `wecode install` → `wecode qrcode`.

Run `wecode <command> --help` for details, or see [`apps/agent/README.md`](apps/agent/README.md).

<details>
<summary><b>All headless host commands</b></summary>

| Command | What it does |
| :--- | :--- |
| `wecode config` | Interactive setup (device name, relay). Writes `wecode.toml`. |
| `wecode install` | Run as a startup service (launchd / `systemd --user` / Task Scheduler). |
| `wecode start` / `stop` | Start (foreground) or stop the host. |
| `wecode status` | Whether it's running, node id, and paired-device count. |
| `wecode qrcode` / `link` | Show the pairing QR / print the pairing ticket to paste on the desktop. |
| `wecode device` | List paired devices; `device:del <id>` / `device:rename <id>` / `device:clear` to manage. |
| `wecode update` | Download, verify, and replace this binary, then restart the host. |
| `wecode uninstall` | Stop and remove the service. |

</details>

## Web Tunnel Browser

When you control a paired headless host from WeCode Desktop, the globe **Web Tunnel Browser** button opens a proxy-isolated Chromium that browses **as the host**: if the host runs Vite at `http://127.0.0.1:5173/`, type that URL and it opens through the encrypted WeCode link — HTTPS, WebSocket, HMR, LAN addresses, `.local` names, and VPN routes included.

<details>
<summary><b>Diagnostics &amp; notes</b></summary>

- Host-local URLs are resolved on the host, not on your controller machine.
- Every `wecode-agent` serves a built-in diagnostic page at `http://127.0.0.1:8765/`. Open it through the Web Tunnel Browser to verify tunnel health and live round-trip latency.
- Testing on one computer still exercises the same tunnel path, but true cross-machine reachability should be verified with the WeCode host running on a different machine.

</details>

## Keyboard Shortcuts

| Action | Shortcut |
| :----- | :------- |
| New Split | `⌘T` |
| New Tab | `⌘D` |
| Toggle Git Panel | `⌘G` |
| Toggle AI Panel | `⌘Y` |
| Switch Project | `⌘1` – `⌘9` |

Customize everything in **Settings → Shortcuts**.

## System Requirements

**Desktop app**

- macOS 14.0 (Sonoma) or later

**Headless host (`wecode-agent`)**

- macOS and Linux (x86_64 and arm64)

## Feedback

Found a bug or have a feature request? Open an [issue on GitHub](https://github.com/Voice-2026/WeCode/issues).

For bug reports, use **Help → Export Diagnostics** and attach the generated `.zip` — it bundles runtime logs, rotated logs, performance summaries, saved app state, invalid-state backups, and matching macOS diagnostic reports when available.

Manual log paths:

- `~/Library/Application Support/WeCode/logs/runtime-rust.log`
- `~/Library/Application Support/WeCode/logs/performance-summary.json`

---

## Contributors

Thanks to everyone who has contributed code, issues, testing, and feedback to WeCode.

<p align="center">
  <a href="https://github.com/Voice-2026/WeCode/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=Voice-2026/WeCode" alt="WeCode contributors">
  </a>
</p>

## GitHub Star Trend

If WeCode ever rescued one of your long agent runs, a ⭐ helps more people find it.

[![Star History Chart](https://api.star-history.com/svg?repos=Voice-2026/WeCode&type=Date)](https://star-history.com/#Voice-2026/WeCode&Date)

<p align="center">
  Wanted to be dmux, but that name was taken. So it's WeCode now — which sounds like "Cool Dux" in Chinese.
</p>

<p align="center">
  <a href="https://wecode.dux.cn">wecode.dux.cn</a>
</p>
