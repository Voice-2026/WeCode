<p align="center">
  <img src="docs/images/icon.png" width="128" height="128" alt="Codux">
</p>

<h1 align="center">Codux AI</h1>

<p align="center">
  <b>The native terminal built for AI coding agents.</b><br/>
  Run Codex, Claude Code, and 6 more AI coding CLIs in one project-aware terminal — live agent state, token analytics, durable memory, agent-safe SSH, and phone handoff.
</p>

<p align="center">
  <a href="https://codux.dux.cn">Website</a> &middot;
  <a href="https://github.com/duxweb/codux/releases">Download</a> &middot;
  <a href="https://github.com/duxweb/codux-flutter/releases">Mobile</a> &middot;
  <a href="#wechat">WeChat</a> &middot;
  <a href="https://github.com/duxweb/codux/issues">Feedback</a>
</p>

<p align="center">
  English | <a href="README.zh-CN.md">简体中文</a>
</p>

---

![Codux AI](docs/images/screenshot.png)

## Why Codux AI

AI coding CLIs are incredibly powerful — and incredibly easy to lose control of. Real work sprawls across projects, Git worktrees, terminals, sessions, tokens, remote shells, and context you half-remember. **Codux AI turns that chaos into one durable, native workspace built for serious AI coding.**

| When AI coding gets messy | Codux AI gives you |
| :------------------------ | :----------------- |
| Every AI CLI has its own state | One project-aware view across Codex, Claude Code, Gemini CLI, OpenCode, Kiro CLI, Kimi Code, CodeWhale, and Agy. |
| Long agent runs are hard to resume | Live runtime status, local history indexing, session restore, and per-worktree context. |
| Parallel tasks collide | A worktree-first model where every task keeps its own terminals, Git state, files, and AI sessions. |
| Token spend is a black box | Usage by tool, model, project, worktree, and day — no spreadsheets. |
| Context evaporates between sessions | Local memory for habits, project profiles, and module notes, injected back into supported CLIs automatically. |
| Server access is fragile | Saved, tested SSH profiles and a `codux-ssh` command agents can use **without ever seeing your credentials**. |
| You walk away mid-run | Pair your phone over Iroh and keep driving the session from anywhere. |

Codux AI is **not** another editor. It's the control plane for developers who live in AI coding CLIs and need a rock-solid way to run multi-project, long-running agent work.

## One runtime, every AI CLI

Codux detects supported CLIs from managed terminals, reads their local session history, and installs app-managed hooks or memory files where the tool allows it.

| Tool | Runtime Status | History Index | Resume | Memory Injection |
| :--- | :------------- | :------------ | :----- | :--------------- |
| Codex | Full | Full | Full | Yes |
| Claude Code | Full | Full | Full | Yes |
| Gemini CLI | Full | Full | Tool-dependent | Yes |
| OpenCode | Full | Full | Tool-dependent | Yes |
| Kiro CLI | Full | Full | Tool-dependent | Yes |
| Kimi Code | Full | Full | Tool-dependent | Tool-dependent |
| CodeWhale | Full | Full | Tool-dependent | Yes |
| Agy | Full | Full | Tool-dependent | Yes |

`Full` means Codux tracks that capability from the normal terminal workflow. `Tool-dependent` means Codux preserves the workspace and history while exact resume behavior is up to the CLI.

Under the hood, each tool is a **runtime driver** with a consistent integration path, so sessions never cross state and new tools are easy to add:

- **Hooks** capture starts, completions, interruptions, permission waits, and model/session metadata.
- **Probes** detect running sessions, tools, models, and accumulated usage.
- **History sources** normalize local CLI transcripts into one timeline.
- **Memory injection** feeds project context to supported CLIs without wrapper hacks.

## Built for long agent runs

Codux isn't a terminal with tabs — it's an AI-aware control layer that keeps long-running agent work **visible, recoverable, and safe to continue**.

- **Live agent state, not just scrollback.** Running, completed, interrupted, permission-waiting, and plan-updating sessions — each tied to the right project and worktree, with the task plan surfaced when the CLI exposes it.
- **A terminal tuned for AI.** Scrollback, selection, ANSI/alt-screen apps, modified keys, mouse reporting, and scrollbars, all handled in the managed terminal layer.
- **Token spend, made visible.** Usage by tool, model, project, worktree, and day — no spreadsheets.
- **Memory that follows the work — and stays local.** Codux mines durable preferences, project profiles, and module notes from local transcripts and injects only what's relevant. History and memory never leave your machine.
- **Project surfaces beside the terminal.** File browsing, Markdown/image preview, and focused Git review/diff windows keep review work next to the running CLI.
- **Prompt-safe clipboard & paths.** Pasted images become temp files with local paths instead of base64 blobs; dragged files insert shell-quoted paths agents can use instantly.
- **SSH that agents can't leak.** `codux-ssh <profile>` runs remote commands through saved, tested profiles — passwords, passphrases, and key paths never reach the prompt.

## Phone-to-desktop handoff

Pair your phone over the shared **Iroh** transport and keep driving the session from anywhere.

- Pair in seconds with a short-lived QR ticket; Iroh picks the best direct path and falls back to a relay when needed.
- Projects, terminals, files, and AI sessions keep running on the desktop — your phone just controls them, with large histories recovered safely over bounded baselines and sequence guards.

## Desktop pets

Optional companions that grow with your AI coding habits — they react to usage, reminders, and agent activity. Import Codex-style custom pet packs from Petdex with a flat `pet.json` + `spritesheet.png` format.

## Worktree-first workflow

Codux models real AI work the way it actually happens: **Project → Worktree / Task → Terminals, Files, Git, AI Sessions.**

- Spin up Git worktrees for parallel tasks without tangling branch state.
- Switch tasks and keep everything — terminal tabs, splits, panel sizes, active AI sessions, file context, and Git state.
- Review worktree changes against the base branch, merge back, and clean up finished worktrees.
- Keep AI history and runtime activity scoped to the worktree, while project memory stays shared.

This is what sets Codux apart from a plain terminal multiplexer: it *knows* which project and worktree each terminal belongs to, and rebuilds the whole workspace around that relationship.

## Native, not Electron

Codux is a true native app in **Rust** on **GPUI** — the same stack that powers [Zed](https://zed.dev) — so terminal rendering, project switching, and long, noisy agent runs stay fast and smooth. Desktop and mobile share one pure-Rust `alacritty_terminal` engine (identical viewport, scrollback, cursor, and remote-PTY semantics), and the runtime is shaped so future headless Linux hosts can expose the same domains without a GUI.

## Getting Started

1. Download Codux from [GitHub Releases](https://github.com/duxweb/codux/releases) or [codux.dux.cn](https://codux.dux.cn).
2. Install:
   - **macOS** — open the `.dmg` and drag Codux to Applications.
   - **Windows** — run the `setup.exe` installer.
3. Open a project folder.
4. Start an AI CLI in the integrated terminal.
5. Optional — create a worktree task, connect an SSH profile, or pair Codux Mobile.

| Platform | Recommended download |
| :------- | :------------------- |
| macOS | `codux-*-macos-*.dmg` |
| Windows | `codux-*-windows-x86_64-setup.exe` |

Updater archives and `latest.json` are published for auto-updates and automation — most users just want one of the two installers above.

## Keyboard Shortcuts

| Action | Shortcut |
| :----- | :------- |
| New Split | `⌘T` |
| New Tab | `⌘D` |
| Toggle Git Panel | `⌘G` |
| Toggle AI Panel | `⌘Y` |
| Switch Project | `⌘1` – `⌘9` |

Customize everything in **Settings → Shortcuts**.

## Demo Video

GitHub READMEs can't embed third-party players — watch the demo on [Bilibili](https://www.bilibili.com/video/BV1mK9vBCEYD/).

## WeChat

Scan to add the author on WeChat and ask to join the DUXAI community group.

<p align="center">
  <img src="docs/images/wechat-author.png" width="320" alt="Author WeChat QR code">
</p>

## Repository Layout

This repo is the Codux monorepo:

- `apps/desktop` — Rust + GPUI desktop app, runtime, assets, and release scripts.
- `apps/agent` — headless controlled-agent app linking protocol, terminal core, and the shared local PTY driver without GPUI.
- `apps/mobile` — Flutter mobile controller.
- `crates/codux-protocol` — shared remote protocol: capabilities, envelope DTOs, transport candidates, and relay rules.
- `crates/codux-protocol-ffi` — Flutter-facing C ABI for the protocol and terminal-core bindings.
- `crates/codux-runtime-core` — shared runtime domain rules for host, project, file, Git, worktree, upload, and terminal shapes.
- `crates/codux-terminal-core` — shared terminal session, sequencing, baseline restore, and remote-PTY model (pure-Rust `alacritty_terminal` engine).
- `crates/codux-terminal-pty` — shared `portable_pty` local PTY driver for host/headless targets.

Flutter keeps its own native build system. Remote connectivity runs entirely on the shared Iroh transport.

## Development

```bash
cargo run
```

Useful checks before submitting changes:

```bash
cargo check
cargo test -p codux-runtime ssh::tests
node apps/desktop/scripts/release/test-package-gpui.mjs
```

Desktop releases are cut by pushing a version tag such as `v1.6.2`. The release workflow builds native macOS and Windows artifacts, publishes the GitHub Release, and updates the configured updater channel.

## System Requirements

- macOS 14.0 (Sonoma) or later
- Windows 11

## Feedback

Found a bug or have a feature request? Open an [issue on GitHub](https://github.com/duxweb/codux/issues).

For bug reports, use **Help → Export Diagnostics** and attach the generated `.zip` — it bundles runtime logs, rotated logs, performance summaries, saved app state, invalid-state backups, and matching macOS diagnostic reports when available.

Manual log paths:

- `~/Library/Application Support/Codux/logs/runtime-rust.log`
- `~/Library/Application Support/Codux/logs/performance-summary.json`
- `%APPDATA%\Codux\logs\runtime-rust.log`

---

## Contributors

Thanks to everyone who has contributed code, issues, testing, and feedback to Codux.

<p align="center">
  <a href="https://github.com/duxweb/codux/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=duxweb/codux" alt="Codux contributors">
  </a>
</p>

## GitHub Star Trend

[![Star History Chart](https://api.star-history.com/svg?repos=duxweb/codux&type=Date)](https://star-history.com/#duxweb/codux&Date)

<p align="center">
  Wanted to be dmux, but that name was taken. So it's Codux now — which sounds like "Cool Dux" in Chinese.
</p>

<p align="center">
  <a href="https://codux.dux.cn">codux.dux.cn</a>
</p>
