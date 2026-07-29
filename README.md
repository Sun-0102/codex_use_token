# Codex Reserve

一个基于 Tauri 2 的本地 Codex 用量监控与规划工具。产品采用“状态栏优先”：macOS 常驻菜单栏，Windows 提供置顶悬浮窗和系统托盘入口。桌面壳层、紧凑用量面板、领域模型、IPC 基线、Codex CLI 探测、app-server 长连接生命周期、JSONL 通道、initialize 握手、真实账户/额度数据展示、Codex 本地会话今日 Token 统计和安装版 CLI 路径解析已经建立。

> 本项目不是 OpenAI 官方产品。应用不会复制或直接读取 Codex 登录凭据；通过本机 `codex app-server` 的标准输入输出协议获取当前登录账户可见的限额信息，并从 Codex 自己的本地会话 JSONL 中只解析时间戳和 `token_count` 数字事件，不解析、保存或展示消息正文。

## 技术栈

- Tauri 2 + Rust 2024
- React 19 + TypeScript
- Vite 7
- Vitest

## 当前桌面行为

| 平台 | 启动后行为 | 主要交互 |
| --- | --- | --- |
| macOS | 不显示 Dock 图标，常驻菜单栏 | 左键状态栏图标在图标下方显示/隐藏原生 NSPanel 浮层，点击外部自动收起；右键打开菜单 |
| Windows | 右上角显示 82×82 置顶周额度悬浮球，同时常驻系统托盘 | 按住圆球拖动可移动位置，单击展开 420×510 详细面板；展开、收起和托盘隐藏/显示后保留用户位置 |

关闭面板只会隐藏窗口；只有托盘菜单中的“退出 Codex Reserve”会结束应用。Windows 收起态只显示周额度圆环、百分比和“实时 / 缓存 / 演示”状态；按住移动超过 5px 后进入原生窗口拖动，普通单击才展开详细面板，查看重置时间、当前任务 Token、今日真实 Token、缓存命中和连接诊断。macOS 菜单栏使用动态周额度环形模板图标，旁边只显示周额度百分比。

应用启动后会立即读取一次监控数据，随后由 Tauri 后台每 30 秒发出统一刷新节拍，刷新 CLI、账户、额度、账户 Token、本地会话今日 Token 和当前任务 Token；节拍不依赖窗口可见性，因此隐藏后仍保持相同刷新频率。Codex 账户、额度、账户 Token 和当前任务 Token 共用同一个应用级 `codex app-server --stdio` 长连接，避免每轮重复启动多个进程。若上一轮尚未结束，则跳过当前轮次以避免请求重叠；刷新失败时保留最后成功数据并明确标记为过期缓存。

## 本地开发

前置条件：Node.js、Rust stable、Cargo、macOS Xcode Command Line Tools，以及已登录的 Codex CLI。

```bash
npm install
npm run check
npm run tauri dev
```

常用命令：

```bash
npm run dev        # 仅启动 Web 前端
npm run test       # 运行 TypeScript 单元测试
npm run typecheck  # TypeScript 类型检查
npm run build      # 构建前端静态资源
npm run tauri dev  # 启动桌面开发应用
npm run tauri build
```

Rust 检查：

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## 在 GitHub 构建 Windows 安装包

仓库的 `Build Windows` GitHub Actions 会在每次推送到 `main` 后自动使用 `windows-latest` 构建 Windows x64 版本，也可以在 GitHub 的 **Actions → Build Windows → Run workflow** 手动运行。

构建成功后，打开对应的 Actions 运行记录，在页面底部下载 `codex-reserve-windows-x64`。GitHub 下载的是 ZIP，解压后包含 NSIS 的 `*-setup.exe` 和 WiX 的 `.msi` 安装包。当前安装包未做 Windows 代码签名，分发给其他人时可能出现 Microsoft Defender SmartScreen 提示。

Windows 安装版按以下位置读取本机 Codex：

- 今日 Token：读取 Windows 的 `CODEX_HOME` 或 `%USERPROFILE%\.codex`；若 WSL 中也安装了 Codex，同时通过 `wslpath` 定位各发行版的 `$CODEX_HOME`（默认 `~/.codex`），合并 `sessions` 和 `archived_sessions` 的数字用量。
- Codex CLI：优先使用已登录的 Windows 原生 CLI；原生不可用时，自动枚举普通 WSL 发行版，并通过 `wsl.exe --distribution ... --exec codex app-server --stdio` 复用 WSL 内的登录状态。Windows 原生候选包括 `CODEX_CLI_PATH`、`PATH`、`CODEX_INSTALL_DIR`、官方独立安装目录 `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin`、`%APPDATA%\npm`、WindowsApps 和 `%USERPROFILE%\.volta\bin`。
- WSL 探测会跳过 Docker Desktop 的内部发行版；界面检测到 WSL CLI 时显示 `WSL (发行版名称)`。探测只读取 CLI 路径、Codex 状态目录路径和 Token 数字事件，不读取或展示登录凭据与消息正文。

## 目录结构

```text
src/
├── features/usage/        # 配额领域模型、真实/Demo/stale 适配器和用量面板
├── platform/              # Tauri IPC 边界
├── App.tsx
└── main.tsx
src-tauri/
├── src/app_server_handshake.rs # initialize / initialized 握手
├── src/app_server_client.rs    # 应用级 app-server 长连接客户端
├── src/app_server_jsonl.rs     # JSONL 请求、响应和通知基础通道
├── src/app_server_protocol.rs  # Codex app-server 最小协议类型
├── src/app_server_session.rs   # codex app-server --stdio 子进程生命周期
├── src/cli_probe.rs       # Codex CLI 路径、版本和登录状态探测
├── src/commands.rs        # 可调用的 Rust 命令
├── src/desktop.rs         # 托盘、窗口定位和常驻生命周期
├── src/lib.rs             # Tauri 应用装配
└── tauri.conf.json
docs/
└── IMPLEMENTATION_PLAN.md # 当前进度、下一步任务、验收记录和更新日志
```

## 当前边界

- UI 会明确区分实时、过期缓存和演示数据。
- macOS 主窗口已转换为非激活式原生 NSPanel；优先显示在状态栏图标下方，取不到 monitor 时降级到右上角但仍会显示。
- 应用会执行 `codex --version` 和 `codex login status` 检测 CLI，但不会读取或展示登录凭据；安装版会额外检查 `PATH`、nvm、fnm、asdf、Homebrew 和登录 shell 解析到的 `codex` 路径。
- Windows 安装版会同时兼容 Windows 原生 Codex 与 WSL Codex：原生可用时优先使用原生 app-server，否则自动使用已登录的 WSL CLI；今日 Token 会合并可读取的 Windows 与 WSL 会话目录。详细面板页脚会直接显示连接失败原因，鼠标悬停可查看完整路径。
- Rust 侧已能启动并清理应用级 `codex app-server --stdio` 长连接，完成 JSONL 请求/响应关联和 `initialize` / `initialized` 握手，并通过 `account/read`、`account/rateLimits/read`、`account/usage/read` 读取真实账户、额度和账户 Token 汇总。
- 今日真实 Token 直接扫描 `$CODEX_HOME/sessions`（默认 `~/.codex/sessions`）和 `archived_sessions` 中与今天有关的 JSONL，只处理 `event_msg/token_count`，根据累计快照差值还原每次请求；`input_tokens` 已包含缓存命中，因此总量按 `input_tokens + output_tokens` 计算，不再次叠加 `cached_input_tokens`。
- `account/usage/read` 当前展示的是账户接口日桶/累计/日峰值，不等同于本地会话逐请求统计口径；本地会话日志不可用时，该数据作为兜底。
- `runtime_health` 只验证前后端 IPC 基线，不访问 Codex。
- `set_usage_window_mode` 负责在 Windows 82×82 周额度悬浮球和 420×510 详细面板之间安全调整窗口尺寸。
- `update_tray_usage` 只同步真实周额度；演示数据不会写入实时托盘标题。
- 当前 30 秒刷新由 Rust 后台线程提供节拍并复用现有 IPC：Codex 账户接口复用同一个应用级 app-server 长连接并串行发送请求；上一轮未结束时不会重复启动下一轮。
- 历史记录、规划和阈值通知仍在后续任务中。

当前做到哪里、下一步做什么以及历次验证记录，统一维护在 [开发任务清单](docs/IMPLEMENTATION_PLAN.md)。每次完成需求后必须同步更新该文档。
