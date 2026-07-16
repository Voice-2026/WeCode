<p align="center">
  <img src="docs/images/icon.png" width="128" height="128" alt="WeCode">
</p>

<h1 align="center">WeCode</h1>

<p align="center">
  <b>AI 코딩을 위한 고성능 터미널: 데스크톱, 휴대폰, 서버를 하나의 워크스페이스로</b><br/>
  <b>Rust + GPUI</b> 로 만든 WeCode는 Codex, Claude Code, 8개 이상의 AI 코딩 CLI를 통합하고, 실시간 agent 상태, 토큰 분석, 로컬 메모리, 자격 증명이 격리된 SSH / 데이터베이스 접근, 장시간 실행 중인 agent 작업을 어디서든 이어받을 수 있는 암호화된 디바이스 링크를 제공합니다.
</p>

<p align="center">
  <a href="https://github.com/Voice-2026/WeCode/releases/latest"><img src="https://img.shields.io/github/v/release/Voice-2026/WeCode?label=release&color=blue" alt="Latest release"></a>
  <a href="https://github.com/Voice-2026/WeCode/releases"><img src="https://img.shields.io/github/downloads/Voice-2026/WeCode/total?label=downloads&color=brightgreen" alt="Total downloads"></a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-8250df" alt="Platform">
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Voice-2026/WeCode?color=orange" alt="License"></a>
  <a href="https://github.com/Voice-2026/WeCode/stargazers"><img src="https://img.shields.io/github/stars/Voice-2026/WeCode?color=yellow" alt="GitHub stars"></a>
</p>

<p align="center">
  <a href="https://wecode.dux.cn">웹사이트</a> &middot;
  <a href="https://wecode.dux.cn/zh-cn/getting-started/">문서</a> &middot;
  <a href="https://github.com/Voice-2026/WeCode/releases/latest">다운로드</a> &middot;
  <a href="https://github.com/Voice-2026/WeCode/issues">피드백</a>
</p>

<p align="center">
  <a href="README.md">English</a> | <a href="README.zh-CN.md">简体中文</a> | <a href="README.ja.md">日本語</a> | 한국어
</p>

---

![WeCode](docs/images/screenshot.png)

## WeCode가 필요한 이유

AI 코딩 CLI는 매우 강력하지만, 통제하기 어려워지기도 쉽습니다. 실제 작업은 프로젝트, Git worktree, 터미널, 세션, 토큰, 원격 shell, 그리고 어렴풋이 기억나는 컨텍스트 사이에 흩어집니다. **WeCode는 이 혼란을 진지한 AI 코딩을 위한 안정적인 네이티브 워크스페이스로 정리합니다.**

| AI 코딩에서 자주 생기는 문제 | WeCode가 제공하는 것 |
| :--- | :--- |
| AI CLI마다 상태가 따로 있음 | Codex, Claude Code, OpenCode, Kiro CLI, Kimi Code, CodeWhale, MiMo Code, Agy를 아우르는 프로젝트 중심 뷰. |
| 외부 Agent에 안정적인 제어 수단이 없음 | JSON Product CLI로 프로젝트, worktree, 세션, 모델, 터미널, 자동 작업을 제어합니다. |
| 긴 agent 실행을 이어가기 어려움 | 실시간 상태, 로컬 히스토리, 세션 복원, worktree를 따라가는 컨텍스트. |
| 병렬 작업이 서로 충돌함 | worktree-first 모델. 각 작업은 자체 터미널, Git 상태, 파일, AI 세션을 유지합니다. |
| 토큰 사용량이 보이지 않음 | 도구, 모델, 프로젝트, worktree, 날짜별 사용량. 스프레드시트가 필요 없습니다. |
| 세션 사이에 컨텍스트가 사라짐 | 습관, 프로젝트 프로필, 모듈 메모를 로컬에 저장하고 지원되는 CLI에 자동 주입합니다. |
| 자격 증명이 프롬프트에 들어감 | 저장 및 테스트된 SSH / 데이터베이스 프로필과 agent가 사용할 수 있는 `wecode-ssh` / `wecode-db` 명령. **자격 증명은 모델에 보이지 않습니다**. |
| 자리를 비우면 작업을 이어가기 어려움 | 휴대폰을 P2P / relay 링크로 페어링하고 어디서든 같은 세션을 조작합니다. |
| 코드가 다른 머신에 있음 | 서버, 여분의 Mac, Linux 박스 같은 headless host에 연결해 로컬처럼 터미널, Git, AI를 조작합니다. |

WeCode는 또 하나의 에디터가 아닙니다. AI 코딩 CLI를 생활처럼 쓰는 개발자를 위한 제어 평면이며, 여러 프로젝트와 장시간 agent 작업을 안정적으로 다루기 위한 도구입니다.

## 빠른 시작

macOS: [Homebrew](https://brew.sh) 로 설치합니다.

```bash
brew install --cask Voice-2026/tap/wecode
```

1. **프로젝트를 엽니다.** Git worktree, 프로젝트 상태, 프로젝트별 세션이 자동으로 준비됩니다.
2. **내장 터미널에서 AI CLI를 시작합니다.** `codex`, `claude`, `opencode` 등을 실행하세요. non-invasive wrapper가 실시간 상태, 토큰 추적, 메모리 주입을 별도 설정 없이 활성화합니다.
3. **자리를 비워도 계속합니다.** 휴대폰이나 headless host를 한 번 페어링하면 어디서든 같은 실행 중 세션을 이어받을 수 있습니다.

Homebrew를 쓰지 않는 경우 [다운로드](#download)를 참고하세요.

## 자격 증명은 AI에 전달되지 않습니다

agent는 서버와 데이터베이스가 자주 필요합니다. 하지만 비밀번호를 프롬프트에 붙여 넣거나 모델이 설정 파일을 읽게 하는 것은 자격 증명 유출로 이어집니다. WeCode는 연결 프로필을 로컬에 저장하고 agent에는 안전한 두 명령만 제공합니다.

- **`wecode-ssh`**: agent는 `wecode-ssh list` 를 실행해 프로필 이름과 호스트만 봅니다. 비밀번호와 키는 WeCode helper 프로세스 안에서 주입되며, 모델 컨텍스트, 대화 기록, shell 히스토리에 들어가지 않습니다.
- **`wecode-db`**: MySQL / PostgreSQL / SQLite에도 같은 격리를 제공합니다. WeCode에 한 번 저장하고 프로필 이름으로 쿼리합니다. 읽기 전용 프로필은 wrapper 내부의 단일 문장 allowlist로 강제되므로 모델이 스스로 권한을 확장할 수 없습니다.
- **프로젝트별 설정이 필요 없습니다.** 지원되는 모든 CLI는 WeCode 환경 지시문을 통해 이 명령들을 자동으로 알게 됩니다.

<p align="center"><img src="docs/images/credential-isolation.png" alt="wecode-ssh list shows profile names and hosts only — never passwords"></p>

## AI CLI 호환성

WeCode는 non-invasive wrapper와 도구별 adapter를 사용합니다. WeCode 컨텍스트를 주입하기 위해 프로젝트 prompt 파일을 쓰거나 AI CLI의 전역 설정을 바꾸지 않습니다.

| AI CLI | 실시간 상태 | 토큰 사용량 | 모델 설정 | 전체 접근 모드 | 환경 지시문 |
| :--- | :---: | :---: | :---: | :---: | :--- |
| Codex | ✓ | ✓ | ✓ | ✓ | developer instructions 경유 |
| Claude Code / reclaude | ✓ | ✓ | ✓ | ✓ | `--append-system-prompt` 경유 |
| OpenCode | ✓ | ✓ | ✓ | ✓ | 관리형 플러그인 설정 경유 |
| MiMo Code | ✓ | ✓ | ✓ | ✓ | 관리형 플러그인 설정 경유 |
| Kimi Code | ✓ | ✓ | ✓ | — | 관리형 `--agent-file` 경유 |
| Kiro CLI | ✓ | ✓ | ✓ | ✓ | 주입하지 않음. 확인된 non-invasive prompt channel 없음 |
| CodeWhale | ✓ | ✓ | ✓ | ✓ | interactive session에는 주입하지 않음 |
| Agy | ✓ | ✓ | ✓ | ✓ | 주입하지 않음. 확인된 non-invasive prompt channel 없음 |

환경 지시문에는 WeCode 메모리와 `wecode-ssh`, `wecode-db` 같은 runtime 명령이 포함됩니다. 지원되지 않는 도구도 가능한 범위에서 세션을 추적하지만, 프로젝트 파일이나 사용자 전역 설정을 강제로 사용해 prompt를 주입하지 않습니다.

## Product CLI와 자동 작업

내장 `wecode` Product CLI는 안정적인 JSON protocol을 통해 외부 Agent가 실행 중인 Desktop을 제어할 수 있게 합니다. 프로젝트와 모델 조회, 세션 생성·재개, prompt 전송, worktree·terminal·예약 자동 작업 관리를 지원합니다.

```bash
wecode app status --json
wecode session create --project <project-id> --agent <agent-id> --model <model-id> --json
wecode automation list --json
wecode automation run --id <automation-id> --json
```

포함된 `wecode-control` Skill은 Codex 등 외부 Agent를 위한 전체 제어 contract를 설명합니다. 새 자동 작업은 기본적으로 **Claude + Kiro**와 **Opus 4.8**을 사용하며, editor와 CLI 모두에서 모델을 선택할 수 있습니다.

## 하나의 워크스페이스, 모든 디바이스

> **Beta.** headless host 연결은 이번 릴리스에서 먼저 beta로 제공됩니다. 연결, 페어링, host 측 데이터 흐름은 아직 활발히 테스트 중이므로 거친 부분이 있을 수 있습니다. 피드백을 환영합니다.

데스크톱, 휴대폰, headless host는 모두 종단 간 암호화된 **P2P / relay 링크** 위의 peer로 동작합니다. 긴 agent 실행을 어디서든 계속 조작할 수 있습니다.

- **가능하면 직접 연결.** WeCode는 P2P 경로를 우선하고, 네트워크가 필요로 하면 relay로 fallback합니다.
- **SSH 원격 데스크톱이 아닙니다.** 디바이스를 한 번 페어링한 뒤 WeCode 자체에 바로 연결합니다.
- **공인 IP가 필요 없습니다.** 데스크톱, 휴대폰, host는 일반 가정, 사무실, 모바일 네트워크에서도 페어링하고 재연결할 수 있습니다.

```mermaid
flowchart LR
    subgraph drivers["조작하는 디바이스"]
        P["📱 휴대폰"]
        D["💻 데스크톱"]
    end
    subgraph hosts["작업이 실행되는 곳 (host)"]
        D2["💻 다른 데스크톱"]
        H["🖥️ Headless host<br/>server · spare Mac · Linux"]
    end
    P -->|"🔒 P2P / Relay"| D2
    P -->|"🔒 P2P / Relay"| H
    D -->|"🔒 P2P / Relay"| D2
    D -->|"🔒 P2P / Relay"| H
```

controller인 **데스크톱** 또는 **휴대폰**은 host인 **다른 데스크톱** 또는 **headless host**에 연결할 수 있습니다. 데스크톱은 두 역할을 모두 가집니다. 자신의 프로젝트를 host하면서 다른 host도 조작할 수 있습니다. 휴대폰은 조작만 합니다. 작업은 host 머신에 남아 있으므로 디바이스를 바꿔도 세션은 끊기지 않습니다.

- **휴대폰 handoff.** 몇 초 만에 페어링하고 같은 터미널, 히스토리, AI 세션을 휴대폰에서 이어갈 수 있습니다.
- **Headless host.** 서버, 여분의 Mac, Linux 박스에서 `wecode`를 실행하고 로컬처럼 터미널, Git, AI를 조작합니다. [`apps/agent/README.md`](apps/agent/README.md)를 참고하세요.
- **세션 연속성.** 연결이 끊긴 뒤에도 같은 실행 중 shell과 agent 세션에 다시 연결할 수 있습니다.

## 터미널 펫

agent가 소비하는 모든 토큰은 워크스페이스에 사는 픽셀 펫을 성장시킵니다. 부화시키고, 이름을 붙이고, 코딩하면서 레벨업하는 모습을 지켜보세요. 다섯 가지 능력치(Wisdom, Chaos, Night, Stamina, Empathy)는 실제로 언제, 어떻게 작업하는지에서 자랍니다. 커스텀 sprite 펫을 설치하거나 오래된 동료를 명예의 전당에 보낼 수도 있습니다.

쓸모없지만, 꼭 필요합니다.

<p align="center"><img src="docs/images/pet.png" width="320" alt="WeCode terminal pet"></p>

## Local-First 설계

- **데이터는 사용자의 것입니다.** 프로젝트, 터미널, 세션, 메모리, 토큰 통계, 자격 증명은 사용자의 머신에 남습니다. WeCode cloud도 계정 가입도 없습니다.
- **암호화된 디바이스 링크.** 데스크톱 ⇄ 휴대폰 ⇄ host 트래픽은 종단 간 암호화됩니다. 직접 P2P가 불가능할 때도 relay는 암호문만 전달합니다.
- **원칙적으로 프로젝트를 건드리지 않습니다.** WeCode는 리포지토리에 prompt 파일을 쓰지 않고 AI CLI의 전역 설정도 바꾸지 않습니다. 모든 컨텍스트 주입은 검토 가능한 wrapper와 adapter를 통해 이루어집니다.

## Download

**데스크톱 앱**

macOS: [Homebrew](https://brew.sh) 로 설치합니다.

```bash
brew install --cask Voice-2026/tap/wecode
```

또는 직접 다운로드하세요.

| 플랫폼 | 다운로드 |
| :--- | :--- |
| macOS · Apple Silicon | [⬇ `wecode-macos-aarch64.dmg`](https://github.com/Voice-2026/WeCode/releases/latest/download/wecode-macos-aarch64.dmg) |

macOS에서는 `.dmg`를 열고 WeCode를 Applications로 드래그합니다. 그런 다음 프로젝트를 열고 AI CLI를 시작하면 됩니다.

**Headless host (`wecode-agent`)**: Beta, 2.0에 포함

macOS / Linux: 한 줄로 설치합니다(OS / arch 자동 감지, `wecode`로 `PATH`에 설치).

```bash
curl -fsSL https://raw.githubusercontent.com/Voice-2026/WeCode/main/apps/agent/scripts/install.sh | sh
```

Flags: `--beta` · `--version <x.y.z>` · `--dir <path>` · `--setup` · `--mirror <prefix>`(GitHub가 느린 지역용) · `--uninstall`. 또는 바이너리를 직접 다운로드할 수 있습니다.

| 플랫폼 | 다운로드 |
| :--- | :--- |
| macOS · Apple Silicon | [⬇ `wecode-macos-aarch64`](https://github.com/Voice-2026/WeCode/releases/latest/download/wecode-macos-aarch64) |
| macOS · Intel | [⬇ `wecode-macos-x86_64`](https://github.com/Voice-2026/WeCode/releases/latest/download/wecode-macos-x86_64) |
| Linux · arm64 | [⬇ `wecode-linux-aarch64`](https://github.com/Voice-2026/WeCode/releases/latest/download/wecode-linux-aarch64) |
| Linux · x64 | [⬇ `wecode-linux-x86_64`](https://github.com/Voice-2026/WeCode/releases/latest/download/wecode-linux-x86_64) |

바이너리를 `wecode`라는 이름으로 `PATH`에 두고 `wecode config` → `wecode install` → `wecode qrcode`를 실행합니다.

자세한 내용은 `wecode <command> --help` 또는 [`apps/agent/README.md`](apps/agent/README.md)를 참고하세요.

<details>
<summary><b>모든 headless host 명령</b></summary>

| 명령 | 기능 |
| :--- | :--- |
| `wecode config` | 대화식 설정(디바이스 이름, relay). `wecode.toml`을 씁니다. |
| `wecode install` | 시작 서비스로 등록합니다(launchd / `systemd --user` / Task Scheduler). |
| `wecode start` / `stop` | host를 시작(foreground)하거나 중지합니다. |
| `wecode status` | 실행 여부, node id, 페어링된 디바이스 수를 표시합니다. |
| `wecode qrcode` / `link` | 페어링 QR을 보여주거나 데스크톱에 붙여 넣을 pairing ticket을 출력합니다. |
| `wecode device` | 페어링된 디바이스 목록. `device:del <id>` / `device:rename <id>` / `device:clear`로 관리합니다. |
| `wecode update` | 이 바이너리를 다운로드, 검증, 교체한 뒤 host를 재시작합니다. |
| `wecode uninstall` | 서비스를 중지하고 제거합니다. |

</details>

## Web Tunnel Browser

WeCode Desktop에서 페어링된 headless host를 제어할 때, 지구본 **Web Tunnel Browser** 버튼은 host로 브라우징하는 proxy-isolated Chromium을 엽니다. host에서 Vite가 `http://127.0.0.1:5173/` 를 실행 중이면 그 URL을 입력했을 때 암호화된 WeCode link를 통해 열립니다. HTTPS, WebSocket, HMR, LAN 주소, `.local` 이름, VPN 라우트까지 포함됩니다.

<details>
<summary><b>진단 및 참고</b></summary>

- host-local URL은 controller 머신이 아니라 host에서 해석됩니다.
- 모든 `wecode-agent`는 `http://127.0.0.1:8765/` 에 내장 진단 페이지를 제공합니다. Web Tunnel Browser로 열어 tunnel 상태와 live round-trip latency를 확인하세요.
- 한 컴퓨터에서 테스트해도 같은 tunnel 경로를 사용하지만, 실제 cross-machine 도달성은 다른 머신에서 WeCode host를 실행해 확인해야 합니다.

</details>

## 키보드 단축키

| 동작 | 단축키 |
| :--- | :--- |
| 새 분할 | `⌘T` |
| 새 탭 | `⌘D` |
| Git 패널 전환 | `⌘G` |
| AI 패널 전환 | `⌘Y` |
| 프로젝트 전환 | `⌘1` – `⌘9` |

모든 단축키는 **Settings → Shortcuts** 에서 사용자 지정할 수 있습니다.

## 시스템 요구 사항

**데스크톱 앱**

- macOS 14.0 (Sonoma) 이상

**Headless host (`wecode-agent`)**

- macOS, Linux(x86_64 및 arm64)

## 피드백

버그나 기능 요청이 있으면 [GitHub Issues](https://github.com/Voice-2026/WeCode/issues)에 남겨 주세요.

버그 보고 시 **Help → Export Diagnostics** 를 사용해 생성된 `.zip`을 첨부하는 것을 권장합니다. runtime logs, rotated logs, performance summaries, 저장된 app state, invalid-state backups, 일치하는 macOS diagnostic reports가 포함됩니다.

수동 로그 경로:

- `~/Library/Application Support/WeCode/logs/runtime-rust.log`
- `~/Library/Application Support/WeCode/logs/performance-summary.json`

---

## Contributors

WeCode에 코드, issue, 테스트, 피드백으로 기여해 주신 모든 분께 감사드립니다.

<p align="center">
  <a href="https://github.com/Voice-2026/WeCode/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=Voice-2026/WeCode" alt="WeCode contributors">
  </a>
</p>

## GitHub Star Trend

WeCode가 긴 agent 실행을 구해 준 적이 있다면, ⭐ 는 더 많은 사람이 WeCode를 찾는 데 도움이 됩니다.

[![Star History Chart](https://api.star-history.com/svg?repos=Voice-2026/WeCode&type=Date)](https://star-history.com/#Voice-2026/WeCode&Date)

<p align="center">
  원래는 dmux라는 이름을 원했지만 이미 사용 중이었습니다. 그래서 WeCode가 되었습니다. 중국어로는 "Cool Dux"처럼 들립니다.
</p>

<p align="center">
  <a href="https://wecode.dux.cn">wecode.dux.cn</a>
</p>
