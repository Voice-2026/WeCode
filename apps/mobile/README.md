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
  <img src="https://img.shields.io/badge/flutter-Rust%20terminal%20core-02569b?style=flat-square" alt="Flutter">
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

- **Reliable terminal rendering on Android** — remote PTY bytes are parsed by the shared Rust `codux-terminal-core` headless screen model and rendered by Flutter, not WebView or a platform-specific terminal plugin.
- **Mobile-safe input** — quick-key toolbar, IME toggle, paste, image upload, and keyboard avoidance tuned for terminal TUI apps.
- **Codux workspace integration** — QR pairing, device list, project tabs, terminal split list, file browser, and AI usage panels all connect to the Codux desktop host through the v3.1 remote protocol.

## Features

| Area | Status | Description |
|:--|:--|:--|
| Pairing | Ready | Scan the QR code shown by Codux on macOS, submit a pairing request, and wait for host confirmation. |
| Device Management | Ready | Save multiple Mac devices locally, edit relay address / display name, and reconnect in the background. |
| Remote Terminal | Ready | Render the Rust-backed remote PTY screen model and send explicit user input back to the Mac host. |
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
  └─ Rust terminal-core FFI: RemotePtySession + libghostty-vt headless screen model

Codux Desktop host
  ├─ Owns projects, terminal sessions, PTYs, files, Git/worktree state, and AI usage
  └─ Confirms mobile pairing and serves runtime-domain protocol messages
```

The mobile app is controller-only. It does not try to become the source of truth for terminal sessions, files, Git state, or projects. It renders and interacts with the host-owned workspace through explicit runtime-domain protocol messages. Business payloads are wrapped as end-to-end encrypted `secure.message` envelopes, while terminal history uses bounded v3.1 buffer windows with chunk assembly and progress reporting. Terminal bytes always enter `RemotePtySession` before rendering; Flutter reads the resulting screen cells and only owns drawing and input intent.

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

Flutter uses the same build-time log level across app code and terminal rendering:

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

The mobile source lives in this monorepo under `apps/mobile`. Mobile signing and public releases stay in the legacy `duxweb/codux-flutter` repository so the existing Android and iOS signing secrets do not need to be moved:

- `CHANGELOG.md` and `CHANGELOG.zh-CN.md` keep versioned release notes.
- `scripts/release/build-release-notes.sh` extracts bilingual release notes for GitHub Releases.
- `duxweb/codux` is the source repository for desktop, shared crates, and mobile app code.
- `duxweb/codux-flutter` is the mobile release repository. Its workflows build from the monorepo source and publish mobile GitHub Release / TestFlight artifacts with the existing mobile secrets.

### Android Signing

Published Android releases require these secrets in `duxweb/codux-flutter`:

- `CODUX_ANDROID_KEYSTORE_BASE64`
- `CODUX_ANDROID_KEYSTORE_PASSWORD`
- `CODUX_ANDROID_KEY_ALIAS`
- `CODUX_ANDROID_KEY_PASSWORD`

Generate the base64 value from a keystore:

```bash
base64 -i codux-release.jks | pbcopy
```

For local development without `android/key.properties`, release builds fall back to debug signing so `flutter run --release` still works. GitHub published releases use the signing secrets when configured and otherwise publish the same debug-signing fallback APK with a workflow warning.

### iOS Signing

TestFlight releases require these secrets in `duxweb/codux-flutter`:

- `IOS_DISTRIBUTION_CERT_BASE64`
- `IOS_DISTRIBUTION_CERT_PASSWORD`
- `IOS_PROVISIONING_PROFILE_BASE64`
- `APP_STORE_CONNECT_API_KEY_ID`
- `APP_STORE_CONNECT_API_ISSUER_ID`
- `APP_STORE_CONNECT_API_KEY_P8_BASE64`

### Publish A Version

1. Add mobile notes under the target version in `apps/mobile/CHANGELOG.md` and `apps/mobile/CHANGELOG.zh-CN.md`.
2. Update `pubspec.yaml` version if needed.
3. Commit the release changes in `duxweb/codux`, then push `main` and the source tag:

```bash
cd /Volumes/Web/codux-gpui
git tag v0.1.4
git push origin main
git push origin v0.1.4
```

4. Push the same tag in `duxweb/codux-flutter` to trigger the mobile release workflows:

```bash
cd /Volumes/Web/codux-flutter
git tag v0.1.4
git push origin v0.1.4
```

The mobile release workflows in `duxweb/codux-flutter` build `apps/mobile` from the matching `duxweb/codux` tag, publish `Codux-Mobile-<version>-arm64-v8a-android.apk` and `SHA256SUMS.txt` to the mobile GitHub Release, and optionally upload the iOS IPA to TestFlight.

## Repository Layout

| Path | Description |
|:--|:--|
| `lib/` | Flutter app shell, relay client, screens, widgets, themes, and i18n. |
| `plugin/codux_protocol_ffi/` | Rust protocol and terminal-core FFI used by Android, iOS, and desktop builds. |
| `android/` | Android application wrapper and release signing config. |
| `.github/workflows/` | Desktop release and formal macOS signing workflows for this monorepo. |
| `scripts/release/` | Release note extraction helpers. |
| `docs/images/` | Reserved screenshot location. |

## License

Codux Mobile is licensed under the GNU General Public License v3.0, the same license used by Codux for macOS. See `LICENSE` for details.

## Related Projects

- [Codux for macOS](https://github.com/duxweb/codux)
- [Codux Mobile Releases](https://github.com/duxweb/codux-flutter/releases)
