<p align="center">
  <img src="docs/images/icon.png" width="128" height="128" alt="Codux">
</p>

<h1 align="center">Codux AI</h1>

<p align="center">
  <b>A high-performance Rust-native command center for AI coding CLIs.</b><br/>
  Run Claude Code, Codex, Gemini CLI, OpenCode, and Kiro CLI across projects without losing sessions, context, tokens, or momentum.
</p>

<p align="center">
  <a href="https://codux.dux.cn">Website</a> &middot;
  <a href="https://github.com/duxweb/codux/releases">Download</a> &middot;
  <a href="https://github.com/duxweb/codux-flutter/releases">Mobile</a> &middot;
  <a href="https://github.com/duxweb/codux-service/releases">Relay Service</a> &middot;
  <a href="#wechat">WeChat</a> &middot;
  <a href="https://github.com/duxweb/codux/issues">Feedback</a>
</p>

<p align="center">
  English | <a href="README.zh-CN.md">简体中文</a>
</p>

---

![Codux AI](docs/images/screenshot.png)

## Why Codux AI

AI coding tools are powerful, but real work quickly spreads across terminals, projects, logs, tokens, and half-remembered context. Codux AI turns that chaos into one durable workspace.

| What you need | What Codux AI gives you |
| :------------ | :---------------------- |
| One place for every AI CLI | Launch and monitor Claude Code, Codex, Gemini CLI, OpenCode, and Kiro CLI from project-aware terminals. |
| Long-running sessions that stay useful | Restore past AI sessions, see live activity, and jump back into the original tool when work continues. |
| Token visibility without spreadsheets | Track usage by tool, model, project, and day so AI coding cost becomes visible instead of vague. |
| Memory that follows the project | Keep user habits, project profile, and module notes in local SQLite memory, then inject app-managed context into supported CLIs. |
| Git and files next to the terminal | Review changes, stage diffs, browse files, preview assets, and drag paths into the terminal without leaving the workspace. |
| A way to keep moving away from the desk | Pair Codux Mobile with the desktop host and continue AI CLI sessions through an encrypted relay. |

Codux AI is not trying to replace your editor. It is built for developers who already use AI CLIs heavily and need a better control plane for multi-project, long-running AI work.

## Rust Native UI

Codux AI is now built as a Rust-native GPUI desktop app instead of a WebView shell. The UI, terminal workspace, sidebars, dialogs, state handling, and update flow run in the native process, keeping project switching, terminal rendering, Git review, file browsing, and AI activity tracking responsive under long-running workloads.

## AI Tool Support

Codux detects supported CLIs from the integrated terminal, reads local session history where available, and installs app-managed hooks or memory files for tools that support them.

| Tool | Activity + history | Resume | Memory |
| :--- | :----------------- | :----- | :----- |
| Claude Code | Full | Full | Yes |
| Codex | Full | Full | Yes |
| Gemini CLI | Full | Full | Yes |
| OpenCode | Full | Full | Yes |
| Kiro CLI | Full | Partial | Yes |

`Full` means Codux can drive the feature from the normal terminal workflow. `Partial` means the tool exposes enough data for tracking, but restore still depends on the CLI's own behavior.

## Demo Video

GitHub README does not render third-party iframe players. Watch the demo on [Bilibili](https://www.bilibili.com/video/BV1mK9vBCEYD/).

## WeChat

Scan the QR code to add the author on WeChat and ask to join the DUXAI community group.

<p align="center">
  <img src="docs/images/wechat-author.png" width="320" alt="Author WeChat QR code">
</p>

## Mobile Handoff

Codux Mobile and Codux Service form a separate remote-control stack. Your projects and terminals stay on the desktop host; the relay only forwards encrypted traffic.

- **Codux Desktop**: the main app for projects, terminals, Git, stats, memory, and remote hosting.
- **Codux Mobile**: Android client for pairing with desktop, running AI CLI sessions remotely, browsing files, and uploading images.
- **Codux Service**: lightweight Go relay for pairing and encrypted WebSocket forwarding.

Quick trial relay nodes:

| Node | URL |
| :--- | :-- |
| China relay | `https://codux-service.dux.plus` |
| Global transit | `https://codux-node.dux.plus` |

Terminal input, output, file payloads, project lists, and AI stats are end-to-end encrypted between Codux Desktop and Codux Mobile. For long-term use, self-hosting [codux-service](https://github.com/duxweb/codux-service/releases) is recommended.

## Custom Pets

Codux includes optional desktop companions that grow with your AI coding habits. You can also import Codex-style pet packages from Petdex using a flat `pet.json` + `spritesheet.png` format.

## Getting Started

1. Download Codux from [GitHub Releases](https://github.com/duxweb/codux/releases) or [codux.dux.cn](https://codux.dux.cn).
2. Install it:
   - macOS: open the `.dmg` and drag Codux to Applications.
   - Windows: run the `setup.exe` installer.
3. Open a project folder.
4. Start an AI CLI in the integrated terminal.

Recommended downloads:

| Platform | File |
| :------- | :--- |
| macOS | `codux-*-macos-*.dmg` |
| Windows | `codux-*-windows-x86_64-setup.exe` |

Updater archives, unsigned builds, and `latest.json` are published for update channels, fallback testing, or automation. Most users should download one of the two installers above.

## Development

```bash
cargo run
```

Useful checks before submitting changes:

```bash
cargo fmt
cargo test -q -p codux-runtime update
cargo check
```

Desktop releases are created by pushing a release tag. The workflow builds the Rust-native macOS and Windows artifacts, publishes a GitHub Release, and updates the configured updater channel.

## Keyboard Shortcuts

| Action | Shortcut |
| :----- | :------- |
| New Split | `⌘T` |
| New Tab | `⌘D` |
| Toggle Git Panel | `⌘G` |
| Toggle AI Panel | `⌘Y` |
| Switch Project | `⌘1` - `⌘9` |

All shortcuts can be customized in **Settings > Shortcuts**.

## System Requirements

- macOS 14.0 (Sonoma) or later
- Windows 11

## Feedback

Found a bug or have a feature request? Open an [issue on GitHub](https://github.com/duxweb/codux/issues).

For bug reports, use **Help -> Export Diagnostics** and attach the generated `.zip`. It includes runtime logs, rotated logs, performance summaries, saved app state, invalid state backups, and matching macOS diagnostic reports when available.

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
  Wanted to be dmux, but that name was taken. So it's Codux now, which sounds like "Cool Dux" in Chinese.
</p>

<p align="center">
  <a href="https://codux.dux.cn">codux.dux.cn</a>
</p>
