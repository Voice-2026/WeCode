<h1 align="center">Codux Mobile</h1>

<p align="center">
  <strong>A native mobile controller for the Codux desktop workspace.</strong>
</p>

<p align="center">
  <a href="https://github.com/duxweb/codux-flutter/releases">
    <img src="https://img.shields.io/badge/version-0.1.5-22d3ee?style=flat-square" alt="Version">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-GPLv3-blue?style=flat-square" alt="License">
  </a>
  <img src="https://img.shields.io/badge/platform-Android-3ddc84?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/flutter-native%20terminal-02569b?style=flat-square" alt="Flutter">
  <img src="https://img.shields.io/badge/languages-EN%20%7C%20ZH-lightgrey?style=flat-square" alt="Languages">
</p>

<p align="center">
  <a href="https://github.com/duxweb/codux">Codux for macOS</a> &middot;
  <a href="https://github.com/duxweb/codux-flutter/releases">Download</a> &middot;
  <a href="https://github.com/duxweb/codux-flutter/issues">Feedback</a>
</p>

<p align="center">
  English | <a href="README.zh-CN.md">简体中文</a>
</p>

---

## Preview

<p align="center">
  <img src="docs/images/device.jpg" width="260" alt="Codux Mobile device list">
  <img src="docs/images/main.jpg" width="260" alt="Codux Mobile terminal screen">
</p>

## Why Codux Mobile?

Codux Desktop owns the real projects, terminals, AI tool sessions, Git/worktree state, files, and relay pairing flow. Codux Mobile is the phone-side controller that connects to that workspace and gives you a touch-first remote runtime view without forcing the desktop terminal UI to resize.

The mobile client focuses on three things:

- **Reliable terminal rendering on Android** — uses a native Flutter platform view backed by Termux `TerminalView` / `TerminalEmulator`, not WebView or xterm.js.
- **Mobile-safe input** — quick-key toolbar, IME toggle, text selection, scrollback, paste, image upload, and keyboard avoidance tuned for terminal TUI apps.
- **Codux workspace integration** — QR pairing, device list, project tabs, terminal split list, file browser, and AI usage panels all connect to the Codux desktop host through the v3.1 remote protocol.

## Features

| Area | Status | Description |
|:--|:--|:--|
| Pairing | Ready | Scan the QR code shown by Codux on macOS, submit a pairing request, and wait for host confirmation. |
| Device Management | Ready | Save multiple Mac devices locally, edit relay address / display name, and reconnect in the background. |
| Remote Terminal | Ready | Render remote PTY output through the native Android terminal plugin and send explicit user input back to the Mac host. |
| Keyboard Handling | Ready | Keeps terminal height stable while shifting the surface around the Android IME, avoiding TUI redraw corruption. |
| Quick Keys | Ready | Two-row terminal toolbar with Esc, Tab, Copy, Paste, Upload, arrows, Delete, Enter, Ctrl, Shift, Alt, keyboard toggle, and `^C`. |
| Files | Ready | Browse project files, remember per-project path, open/edit files, rename, copy path, and delete through the Mac host. |
| AI Stats | Ready | Shows current project and recent AI usage data forwarded by the Codux host. |
| Updates | Ready | Checks the latest GitHub Release for `duxweb/codux-flutter`. |

## Architecture

```text
Codux Mobile (Flutter controller)
  ├─ UI shell: renders runtime state and emits user intent
  ├─ Runtime store: selected project, active terminal, sync state
  ├─ Protocol client: v3.1 envelopes, capabilities, chunk assembly, ack/retry
  ├─ Rust transport FFI: WebRTC DataChannel and WebSocket relay fallback
  └─ Native terminal plugin: Flutter PlatformView + Termux TerminalView

Codux Desktop host
  ├─ Owns projects, terminal sessions, PTYs, files, Git/worktree state, and AI usage
  └─ Confirms mobile pairing and serves runtime-domain protocol messages
```

The mobile app is controller-only. It does not try to become the source of truth for terminal sessions, files, Git state, or projects. It renders and interacts with the host-owned workspace through explicit runtime-domain protocol messages. Business payloads are wrapped as end-to-end encrypted `secure.message` envelopes, while terminal history uses bounded v3.1 buffer windows with chunk assembly and progress reporting.

## Requirements

- Flutter stable with Dart `^3.11.5`
- Android SDK 36
- JDK 17
- Android 8.0 / API 26 or later
- A running Codux macOS host and relay pairing code

## Development

```bash
cd /Volumes/Web/codux-gpui/apps/mobile
flutter pub get
flutter run
```

### Validation

```bash
flutter analyze
flutter test
flutter build apk --debug
flutter build apk --release
```

Debug APK:

```text
build/app/outputs/flutter-apk/app-debug.apk
```

Release APK:

```text
build/app/outputs/flutter-apk/app-release.apk
```

## Logging

Flutter and the native terminal plugin share the same build-time log level:

```bash
flutter run --dart-define=CODUX_LOG_LEVEL=debug
flutter build apk --release --dart-define=CODUX_LOG_LEVEL=warn
```

Supported levels:

- `debug`
- `info`
- `warn`
- `error`
- `off`

The default is `warn`. Release workflows build with `warn` unless overridden.

## Release

This repository includes the same release shape as the macOS app:

- `CHANGELOG.md` and `CHANGELOG.zh-CN.md` keep versioned release notes.
- `scripts/release/build-release-notes.sh` extracts bilingual release notes for GitHub Releases.
- `.github/workflows/test-build.yml` creates manual debug / release APK artifacts.
- `.github/workflows/release-build.yml` builds on `v*` tags and publishes GitHub Release assets.

### Android Signing

Published releases require these repository secrets:

- `CODUX_ANDROID_KEYSTORE_BASE64`
- `CODUX_ANDROID_KEYSTORE_PASSWORD`
- `CODUX_ANDROID_KEY_ALIAS`
- `CODUX_ANDROID_KEY_PASSWORD`

Generate the base64 value from a keystore:

```bash
base64 -i codux-release.jks | pbcopy
```

For local development without `android/key.properties`, release builds fall back to debug signing so `flutter run --release` still works. GitHub published releases use the signing secrets when configured and otherwise publish the same debug-signing fallback APK with a workflow warning.

### Publish A Version

1. Add notes under the target version in `CHANGELOG.md` and `CHANGELOG.zh-CN.md`.
2. Update `pubspec.yaml` version if needed.
3. Commit the release changes.
4. Tag and push:

```bash
git tag v0.1.4
git push origin main
git push origin v0.1.4
```

The release workflow builds `Codux-Mobile-<version>-android.apk`, generates `SHA256SUMS.txt`, extracts release notes, and uploads the assets to GitHub Releases.

## Repository Layout

| Path | Description |
|:--|:--|
| `lib/` | Flutter app shell, relay client, screens, widgets, themes, and i18n. |
| `plugin/codux_native_terminal/` | Native Android terminal plugin used by the Flutter app. |
| `android/` | Android application wrapper and release signing config. |
| `.github/workflows/` | Manual test builds and tag-triggered release builds. |
| `scripts/release/` | Release note extraction helpers. |
| `docs/images/` | Reserved screenshot location. |

## License

Codux Mobile is licensed under the GNU General Public License v3.0, the same license used by Codux for macOS. See `LICENSE` for details.

## Related Projects

- [Codux for macOS](https://github.com/duxweb/codux)
- [Codux Mobile Releases](https://github.com/duxweb/codux-flutter/releases)
