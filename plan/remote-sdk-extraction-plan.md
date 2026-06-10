# Remote Protocol and Terminal SDK Extraction Plan

## 目标

当前 v3.1 先作为稳定基线发布。跨端互联开始前，再把远程协议、传输驱动、远程 runtime、终端 session/pty 能力整理成可抽离的内部 SDK。先在 monorepo/workspace 内形成清晰边界，跑通 Mac/Windows/Linux/Flutter 场景后，再决定是否拆成独立仓库。

## 建议模块

1. `codux-protocol`
   - envelope schema
   - protocol version and capabilities
   - pairing payloads
   - bidirectional resource subscription messages
   - baseline/delta/resync/ack payloads
   - terminal buffer chunk/restore payloads during v3.1 migration
   - project/file/git/worktree/terminal domain message types

2. `codux-remote-transport`
   - transport trait/factory
   - websocket relay driver
   - webrtc datachannel driver
   - future quic driver
   - path/latency/health state normalization

3. `codux-terminal-core`
   - terminal output sequence guard
   - restore-window assembler
   - remote terminal session/cache
   - viewport ownership model
   - input ack/retry model

4. `codux-remote-runtime`
   - project list and selected project state
   - bidirectional subscription model
   - terminal session map backed by local/remote pty models
   - resource model stores for file/git/worktree/project state
   - file/git/worktree runtime domain controllers
   - host runtime instance reset handling

## 最终多端互通目标

```text
Transport driver factory
  WebSocket relay / WebRTC DataChannel / future QUIC

Protocol router
  version / capabilities / envelope / seq / ack / requestId / errors

Bidirectional subscription layer
  resource.subscribe / unsubscribe / baseline / delta / resync

Runtime models and buffer pools
  TerminalSession / RemotePtySession / FileTree / GitState / ProjectState

UI renderer
  only attaches to runtime models and emits user intent
```

Mac、Windows、Linux headless、Flutter 都按 peer 处理。任意 peer 可以发布自己拥有的资源，也可以订阅对端资源。移动端当前只发布控制意图，不发布本地项目资源；桌面端和 Linux agent 发布项目、终端、文件、Git、worktree 等资源。传输驱动只负责连通性和消息收发，上层不依赖 WebSocket、WebRTC 或未来 QUIC 的具体差异。

## 当前收口任务

1. Mac host 支持标准 `resource.subscribe(resource=terminals)`，并保留 `terminal.subscribe` 兼容入口。
2. Flutter 订阅项目或 session 时通过 Rust FFI 构造 `resource.subscribe`，携带 baseline/resume 选项。
3. Flutter `RemotePtySession` 作为唯一远程终端数据池：baseline、分页、live delta、held buffer、seq、resync 都进入模型。
4. UI 进入项目、前台恢复、resize 只挂载或 replay 模型；不主动全量拉历史。
5. 只有无缓存、host runtime 重启、seq gap、显式 resync 时才触发 full hydrate。
6. `terminal.buffer` 仍作为 v3.1 终端 baseline/hydration 载荷；后续再升级为通用 `resource.baseline`。

## Monorepo 迁移状态

当前目录：

```text
codux/
  apps/
    desktop/
      runtime/
      runtime-assets/
      scripts/
    agent/
    mobile/
    server/
    relay-server/
  crates/
    codux-protocol/
    codux-protocol-ffi/
    codux-remote-transport/
    codux-runtime-core/
    codux-terminal-core/
    codux-terminal-pty/
  docs/
  plan/
```

已完成：

- `apps/desktop`：Rust + GPUI desktop app，包含桌面端 runtime、assets 和 release scripts。
- `apps/agent`：Headless controlled agent 薄入口，依赖公共 protocol/terminal core/local PTY driver，不依赖 GPUI 或桌面 runtime。
- `apps/mobile`：Flutter mobile controller。
- `apps/server`：Rust v3 relay service，承接 ticket、signaling 和 WebSocket fallback。
- `apps/relay-server`：Go relay service，迁移期继续保留老协议兼容。
- `crates/codux-protocol`：共享协议边界，包含 v3.1 消息名、资源名、secure/relay envelope DTO、transport candidate DTO、订阅消息、通用资源订阅注册表、relay 策略和 terminal buffer payload。
- `crates/codux-remote-transport`：共享 transport 边界，包含 host/controller WebSocket relay driver、WebRTC DataChannel direct/fallback driver、local memory transport、URL/STUN 规范化、transport factory 和 path state 回调；不承载 terminal/Git/file/UI 业务状态。
- `crates/codux-protocol-ffi`：Flutter 协议和终端 core 绑定的 C ABI，直接复用 `codux-protocol` 和 `codux-terminal-core`。
- `crates/codux-runtime-core`：共享 runtime domain 边界，已承载 host.info、project/file/git/worktree/upload/terminal payload 规则、RuntimeSubscriptionRouter 和 terminal domain 接口；桌面 host 已委托这些协议 shape 与通用资源订阅路由到公共 crate。
- `crates/codux-terminal-core`：共享终端模型边界，负责远程 session 缓存、buffer-window restore、terminal output sequence guard 和 held-live replay 判定。
- `crates/codux-terminal-pty`：共享 host/headless local PTY driver，基于 `portable_pty`。
- Mac host 已接入通用资源订阅注册表：terminal viewer、project/terminal list、Git status、worktree、AI stats 更新都从订阅表计算目标设备；project list 按设备重建 payload，避免不同控制端 selected project 串线。

原则：

- 顶层仓库统一版本、文档、计划和 CI；各 app 的构建/发布脚本跟随各自目录。
- Cargo workspace 只包含 Rust app/crates，包括 `apps/server`；不把 Flutter、web、Go 兼容服务加入 Cargo workspace。
- Flutter、web、Go 兼容服务作为 `apps/*` 子项目保留各自原生构建系统。

待迁移：

- `apps/web`：官网/文档站点，等当前桌面、移动端、服务端迁移提交稳定后再导入。
- `codux-remote-transport` 已提供 host-side WebSocket/WebRTC driver、controller-side relay/WebRTC direct/fallback factory 和 local memory transport，也通过 FFI 供 Flutter controller 复用 URL/STUN/transport 选择规则与 opaque transport handle；后续增强项是更细的 async event-stream bridge、连接错误诊断和 QUIC/WebTransport driver。
- `codux-runtime-core` 后续继续迁移 project/file/git/worktree/terminal domain controller 的剩余状态机；当前已先接入 host.info、纯 payload/排序/命名/上传规则、通用订阅路由和公共接口。

## Platform bindings

- Desktop Rust API uses workspace crates directly.
- Flutter protocol and transport bindings use `apps/mobile/plugin/codux_protocol_ffi`, backed by `crates/codux-protocol-ffi`.
- Android builds `libcodux_protocol_ffi.so` with NDK/cargo-ndk during Gradle preBuild.
- iOS/macOS build the Rust static library from the Flutter plugin podspec script phase.
- Dart keeps compile-time message constants for switch/case matching and Flutter UI/runtime wiring only. Protocol envelope construction, relay policy, transport URL/STUN/selection rules, controller transport lifecycle, and terminal session state now route through Rust FFI without a duplicate Dart implementation.
- Flutter `RemotePtySession` and terminal output sequencer use `codux-terminal-core` through FFI for content, buffer length, sequence, buffer-window restore, cache trimming, duplicate/gap handling, and held-live replay selection. Dart only keeps token-to-object references for Dart-owned objects that cannot cross the FFI boundary.

## Terminal / PTY 边界

| Layer | Current owner | Shared? | 作用 |
| --- | --- | --- | --- |
| `codux-terminal-core` | Rust workspace crate | Yes | 平台无关的远程终端数据模型：缓存、分页恢复、sequence guard、restore window、held-live replay 判定。 |
| `codux-terminal-pty` | Rust workspace crate | Yes for host/headless | 基于 `portable_pty` 的标准 local PTY driver，实现公共 `TerminalDriver`/`TerminalSessionHandle` 接口，可供 Linux headless 和后续桌面适配复用。 |
| `codux-protocol` | Rust workspace crate | Yes | v3.1 消息名、资源名、secure/relay envelope DTO、transport candidate DTO、订阅模型、relay 策略、terminal buffer payload 标准。 |
| `codux-remote-transport` | Rust workspace crate | Yes | 多驱动远程传输层：WebSocket relay、WebRTC host/controller direct/fallback driver、URL/STUN 规则和 transport factory；与 runtime domain 和 PTY 解耦。 |
| `codux-runtime-core` | Rust workspace crate | Yes | host.info、project/file/Git/worktree/upload/terminal 的 domain payload 规则和公共 runtime 接口。 |
| `codux-protocol-ffi` | Rust workspace crate + Flutter plugin | Yes for Flutter binding | 把 protocol 和 terminal-core 暴露给 Flutter；不承载 UI 状态。 |
| `apps/agent` | Headless app | Shared host entry | 无 UI 被控端入口，直接复用 `codux-protocol`、`codux-terminal-core`、`codux-terminal-pty`，后续接入 transport/runtime domains。 |
| `TerminalManager` | Desktop runtime | Desktop adapter | 桌面端专用适配层：连接 AI runtime、记忆/工具环境和现有桌面 session 生命周期；已对齐公共 `TerminalDriver` trait，后续可逐步委托给 `codux-terminal-pty`。 |
| Local PTY driver | `codux-terminal-pty` / desktop adapter | Shared driver interface | 真正执行命令并产生终端字节流。 |
| Remote PTY session | Controller side, backed by `codux-terminal-core` | Yes | 保存从远端协议来的终端状态，让 UI 像挂载本地模型一样读取。 |
| UI terminal renderer | Desktop GPUI / Flutter native terminal | No | 只渲染 runtime model，并发送输入/resize 等用户意图。 |

结论：Headless 应该是独立 `apps/agent`，不是桌面端的 headless 编译变体。`portable_pty` 不直接塞进 `codux-terminal-core`，而是放在独立 `codux-terminal-pty` host/headless driver crate。`codux-terminal-core` 提供公共 `TerminalDriver`/`TerminalSessionHandle` trait 和平台无关模型；`codux-terminal-pty` 实现 local PTY driver；桌面端 `TerminalManager` 继续承接 AI runtime 和桌面专用环境，并作为公共 trait 的桌面适配器。

## 不马上拆独立仓库的原因

- 当前版本需要先发布稳定基线。
- 跨端互联场景还没有完全验证，过早拆仓库会固化不成熟接口。
- 独立仓库会带来版本、发布、FFI、移动端绑定、CI 成本。
- 先在 monorepo 内抽清边界，后续拆仓库更稳。

## 抽离顺序

1. 固化 v3.1 文档和测试。
2. 先把 Mac host 和 Flutter terminal 链路对齐到订阅驱动的 RemotePtySession 模型。
3. 把 protocol payload 从 host/ui 调用点继续下沉到 `codux-protocol` 和 `codux-runtime-core`。
4. 将 project/file/git/worktree/terminal host-side domain controller 状态机继续迁入 `codux-runtime-core`。
5. 将 transport driver 接口和状态机稳定为可替换工厂。
6. 将桌面端 `TerminalManager` 逐步委托给 `codux-terminal-pty`，保留 AI runtime/记忆/工具环境作为桌面适配层。
7. 跨端互联接入 Linux headless host。
8. 验证 Mac/Windows/Linux/Flutter 多端互联后再评估独立仓库。

## 发布策略

- 当前版本发布小正式版。
- 后续 SDK 抽离作为 1.8.x 或 2.x 的内部架构任务。
- 独立仓库只在跨端互联稳定后执行。
