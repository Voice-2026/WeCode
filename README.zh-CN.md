<p align="center">
  <img src="docs/images/icon.png" width="128" height="128" alt="Codux">
</p>

<h1 align="center">Codux AI</h1>

<p align="center">
  <b>为 AI 编程 Agent 打造的原生终端。</b><br/>
  把 Codex、Claude Code 等 8+ AI 编程 CLI 收进一个按项目组织的终端——实时状态、Token 统计、本地记忆、安全 SSH、手机接力。
</p>

<p align="center">
  <a href="https://codux.dux.cn">官网</a> &middot;
  <a href="https://github.com/duxweb/codux/releases">下载</a> &middot;
  <a href="https://github.com/duxweb/codux-flutter/releases">移动端</a> &middot;
  <a href="#作者微信">作者微信</a> &middot;
  <a href="https://github.com/duxweb/codux/issues">反馈</a>
</p>

<p align="center">
  <a href="README.md">English</a> | 简体中文
</p>

---

![Codux AI](docs/images/screenshot.png)

## 为什么用 Codux AI

AI 编程 CLI 很强——也极其容易失控。真正干活时，工作会散落到项目、Git worktree、终端、历史会话、Token、远程 shell，和你只记得一半的上下文里。**Codux AI 把这片混乱收进一个稳定的原生工作台，专为认真做 AI 编程的人打造。**

| AI 编程哪里容易乱 | Codux AI 给你什么 |
| :---------------- | :---------------- |
| 每个 AI CLI 各管各的状态 | 一个按项目组织的视图，统一 Codex、Claude Code、Gemini CLI、OpenCode、Kiro CLI、Kimi Code、CodeWhale、Agy。 |
| 长 agent 任务难恢复 | 实时 runtime 状态、本地历史索引、会话恢复，以及按 worktree 关联的上下文。 |
| 并行任务互相打架 | 以 worktree 为核心：每个任务保留自己的终端、Git 状态、文件和 AI 会话。 |
| Token 花销是个黑盒 | 按工具、模型、项目、worktree、日期统计用量——不用再记账。 |
| 会话之间上下文蒸发 | 本地记忆保存习惯、项目画像、模块笔记，并自动注入回支持的 CLI。 |
| 服务器连接又脆又危险 | 已保存、已测试的 SSH 配置，加一个 **凭证永不外泄** 的 `codux-ssh` 命令给 agent 用。 |
| 任务跑一半离开电脑 | 用手机经 Iroh 配对，随时随地接着控制会话。 |

Codux AI **不是** 又一个编辑器。它是给重度泡在 AI 编程 CLI 里的开发者的控制台，让多项目、长会话的 agent 工作稳得住。

## 一个 runtime，所有 AI CLI

Codux 从托管终端识别支持的 CLI，读取它们的本地会话历史，并在工具允许时安装应用托管的 hook 或记忆文件。

| 工具 | Runtime 状态 | 历史索引 | 会话恢复 | 记忆注入 |
| :--- | :----------- | :------- | :------- | :------- |
| Codex | 完整 | 完整 | 完整 | 支持 |
| Claude Code | 完整 | 完整 | 完整 | 支持 |
| Gemini CLI | 完整 | 完整 | 取决于工具 | 支持 |
| OpenCode | 完整 | 完整 | 取决于工具 | 支持 |
| Kiro CLI | 完整 | 完整 | 取决于工具 | 支持 |
| Kimi Code | 完整 | 完整 | 取决于工具 | 取决于工具 |
| CodeWhale | 完整 | 完整 | 取决于工具 | 支持 |
| Agy | 完整 | 完整 | 取决于工具 | 支持 |

`完整` 表示 Codux 能在正常终端工作流里追踪该能力；`取决于工具` 表示 Codux 会保留工作区和历史，具体恢复行为仍由 CLI 自身决定。

底层每个工具都是一个 **runtime driver**，集成链路统一，因此会话之间不串状态，接入新工具也很容易：

- **Hooks** 捕获开始、完成、中断、权限等待、模型/会话元数据。
- **Probes** 探测运行中的会话、工具、模型和累计用量。
- **History sources** 把本地 CLI transcript 归一成一条时间线。
- **记忆注入** 给支持的 CLI 喂项目上下文，不用在各 wrapper 里重复拼逻辑。

## 为长 agent 运行打造

Codux 不是带标签页的终端——它是一层感知 AI 的控制层，让长跑的 agent 工作 **可见、可恢复、可安全续接**。

- **实时 agent 状态，而不只是 scrollback。** 运行中、已完成、中断、等待授权、计划更新中的会话，每个都绑回正确的项目和 worktree；CLI 暴露任务计划时也一并显示。
- **为 AI 调过的终端。** scrollback、选择复制、ANSI/alt-screen、组合键、鼠标上报、滚动条，全在托管终端层处理好。
- **Token 看得清。** 按工具、模型、项目、worktree、日期统计用量——不用记账。
- **跟随任务的记忆，且全本地。** Codux 从本地 transcript 提炼长期偏好、项目画像、模块笔记，过滤噪声，只注入相关部分。历史和记忆从不离开你的机器。
- **终端旁就是项目面。** 文件浏览、Markdown/图片预览、独立 Git 评审与 diff 窗口，让评审就近发生。
- **对 prompt 安全的剪贴板和路径。** 粘贴的图片变成带本地路径的临时文件，而不是 base64；拖入的文件插入可直接用的 shell-quoted 路径。
- **agent 偷不走的 SSH。** `codux-ssh <profile>` 通过已保存、已测试的 profile 执行远程命令——密码、口令、私钥路径绝不进提示词。

## 手机接力

用手机经共享的 **Iroh** 传输配对，随时随地接着控制会话。

- 用短期二维码 ticket 秒级配对；Iroh 自动选最优直连路径，不通时回落到中继。
- 项目、终端、文件、AI 会话始终跑在桌面端，手机只负责控制；大段历史靠有界 baseline 和 sequence guard 安全恢复。

## 桌面宠物

可选的桌面伙伴，会随你的 AI 编程习惯成长——会对用量、提醒和 agent 活动做反应。可以从 Petdex 导入 Codex 风格的自定义宠物包，格式是扁平的 `pet.json` + `spritesheet.png`。

## Worktree 优先的工作流

Codux 按真实工作发生的方式建模：**项目 → Worktree / 任务 → 终端、文件、Git、AI 会话。**

- 为并行任务开 Git worktree，不让分支状态互相缠绕。
- 切换任务时把一切都带着走——终端标签、分屏、面板高度、当前 AI 会话、文件上下文、Git 状态。
- 评审 worktree 变更、对比 base 分支、合并回主线、清理完成的 worktree。
- AI 历史和运行状态跟随 worktree，项目级记忆保持共享。

这正是 Codux 和普通终端复用工具的根本区别：它 *知道* 每个终端属于哪个项目和 worktree，并围绕这层关系重建整个工作区。

## 原生，不是 Electron

Codux 是基于 **Rust** + **GPUI**（和 [Zed](https://zed.dev) 同源的栈）的真原生应用——终端渲染、项目切换、长时间嘈杂的 agent 输出都顺滑。桌面端和移动端共享同一套纯 Rust `alacritty_terminal` 引擎（viewport、scrollback、光标、远程 PTY 语义完全一致），runtime 也为未来无 GUI 的 Linux 被控端暴露同一套 domain 做好了准备。

## 快速开始

1. 从 [GitHub Releases](https://github.com/duxweb/codux/releases) 或 [codux.dux.cn](https://codux.dux.cn) 下载 Codux。
2. 安装：
   - **macOS** —— 打开 `.dmg`，把 Codux 拖进应用程序文件夹。
   - **Windows** —— 运行 `setup.exe` 安装包。
3. 打开一个项目目录。
4. 在集成终端里启动你的 AI CLI。
5. 可选 —— 创建 worktree 任务、连接 SSH 配置，或配对 Codux Mobile。

| 平台 | 推荐下载 |
| :--- | :------- |
| macOS | `codux-*-macos-*.dmg` |
| Windows | `codux-*-windows-x86_64-setup.exe` |

updater 包和 `latest.json` 用于自动更新和自动化——大多数人下载上面两个安装包之一即可。

## 快捷键

| 操作 | 快捷键 |
| :--- | :----- |
| 新建分屏 | `⌘T` |
| 新建标签页 | `⌘D` |
| 切换 Git 面板 | `⌘G` |
| 切换 AI 面板 | `⌘Y` |
| 切换项目 | `⌘1` – `⌘9` |

所有快捷键都能在 **设置 → 快捷键** 里自定义。

## 演示视频

GitHub README 不渲染第三方播放器，可前往 [Bilibili](https://www.bilibili.com/video/BV1mK9vBCEYD/) 观看演示。

## 作者微信

扫码添加作者微信，备注 Codux，邀你加入 DUXAI 交流社群。

<p align="center">
  <img src="docs/images/wechat-author.png" width="320" alt="作者微信二维码">
</p>

## 仓库结构

本仓库是 Codux monorepo：

- `apps/desktop` —— Rust + GPUI 桌面应用、runtime、资源和发布脚本。
- `apps/agent` —— 不含 GPUI 的无头被控 agent，链接协议、终端核心和共享本地 PTY 驱动。
- `apps/mobile` —— Flutter 移动控制端。
- `crates/codux-protocol` —— 共享远程协议：能力、envelope DTO、传输候选、中继规则。
- `crates/codux-protocol-ffi` —— 面向 Flutter 的协议与终端核心 C ABI 绑定。
- `crates/codux-runtime-core` —— host、项目、文件、Git、worktree、上传、终端的共享 runtime domain 规则。
- `crates/codux-terminal-core` —— 共享终端会话、序列、baseline 恢复和远程 PTY 模型（纯 Rust `alacritty_terminal` 引擎）。
- `crates/codux-terminal-pty` —— 面向 host/无头目标的共享 `portable_pty` 本地 PTY 驱动。

Flutter 保留自己的原生构建系统。远程连接完全跑在共享的 Iroh 传输上。

## 开发

```bash
cargo run
```

提交变更前建议运行：

```bash
cargo check
cargo test -p codux-runtime ssh::tests
node apps/desktop/scripts/release/test-package-gpui.mjs
```

桌面端通过推送版本标签（如 `v1.6.2`）触发发布。发布工作流会构建原生 macOS 和 Windows 产物、发布 GitHub Release，并更新对应的自动更新通道。

## 系统要求

- macOS 14.0 (Sonoma) 或更高
- Windows 11

## 反馈

发现 Bug 或有功能建议？欢迎在 [GitHub Issues](https://github.com/duxweb/codux/issues) 提出。

提交 Bug 时，推荐用 **帮助 → 导出诊断包**，把生成的 `.zip` 附上——里面有运行日志、轮转日志、性能摘要、应用状态、无效状态备份，以及可匹配到的 macOS 诊断报告。

手动日志路径：

- `~/Library/Application Support/Codux/logs/runtime-rust.log`
- `~/Library/Application Support/Codux/logs/performance-summary.json`
- `%APPDATA%\Codux\logs\runtime-rust.log`

---

## 贡献者

感谢所有为 Codux 贡献代码、Issue、测试和反馈的朋友。

<p align="center">
  <a href="https://github.com/duxweb/codux/graphs/contributors">
    <img src="https://contrib.rocks/image?repo=duxweb/codux" alt="Codux 贡献者">
  </a>
</p>

## GitHub Star 趋势

[![Star History Chart](https://api.star-history.com/svg?repos=duxweb/codux&type=Date)](https://star-history.com/#duxweb/codux&Date)

<p align="center">
  本来想叫 dmux，可惜名字被占了，那就叫 Codux 吧——中文谐音刚好是「酷 Dux」。
</p>

<p align="center">
  <a href="https://codux.dux.cn">codux.dux.cn</a>
</p>
