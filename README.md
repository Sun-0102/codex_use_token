# Codex Reserve

一个基于 Tauri 2 的本地 Codex 用量监控与规划工具。产品采用“状态栏优先”：macOS 常驻菜单栏，Windows 提供置顶悬浮窗和系统托盘入口。桌面壳层、紧凑用量面板、领域模型、IPC 基线、Codex CLI 探测、app-server 子进程生命周期、JSONL 通道、initialize 握手、真实账户/额度数据展示、cc-switch 今日真实 Token 统计和安装版 CLI 路径解析已经建立。

> 本项目不是 OpenAI 官方产品。应用不会复制或直接读取 Codex 登录凭据；通过本机 `codex app-server` 的标准输入输出协议获取当前登录账户可见的限额信息，并可从本机 cc-switch 数据库读取纯数字用量统计。

## 技术栈

- Tauri 2 + Rust 2024
- React 19 + TypeScript
- Vite 7
- Vitest

## 当前桌面行为

| 平台 | 启动后行为 | 主要交互 |
| --- | --- | --- |
| macOS | 不显示 Dock 图标，常驻菜单栏 | 左键状态栏图标在图标下方显示/隐藏原生 NSPanel 浮层，点击外部自动收起；右键打开菜单 |
| Windows | 右上角显示 340×82 置顶用量悬浮条，同时常驻系统托盘 | 点击悬浮条展开 420×510 详细面板；支持拖动、收起和隐藏 |

关闭面板只会隐藏窗口；只有托盘菜单中的“退出 Codex Reserve”会结束应用。Windows 收起态显示两个配额百分比和微型余量轨，展开态显示重置时间、当前任务 Token、今日真实 Token、缓存命中、套餐和连接状态。macOS 状态栏标题会显示已知的 `5h` 与 `W` 剩余额度；某个窗口缺失时显示 `--`。

应用启动后会立即读取一次监控数据，随后由 Tauri 后台每 10 秒发出统一刷新节拍，刷新 CLI、账户、额度、账户 Token、cc-switch 今日 Token 和当前任务 Token；节拍不依赖窗口可见性，因此隐藏后仍保持相同刷新频率。若上一轮尚未结束，则跳过当前轮次以避免请求重叠；刷新失败时保留最后成功数据并明确标记为过期缓存。

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

## 目录结构

```text
src/
├── features/usage/        # 配额领域模型、真实/Demo/stale 适配器和用量面板
├── platform/              # Tauri IPC 边界
├── App.tsx
└── main.tsx
src-tauri/
├── src/app_server_handshake.rs # initialize / initialized 握手
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
- Rust 侧已能启动并清理 `codex app-server --stdio` 子进程，完成 JSONL 请求/响应关联和 `initialize` / `initialized` 握手，并通过 `account/read`、`account/rateLimits/read`、`account/usage/read` 读取真实账户、额度和账户 Token 汇总。
- 今日真实 Token 优先读取 `.cc-switch/cc-switch.db` 的 `proxy_request_logs` 数字统计，按 `input_tokens + output_tokens + cache_creation_tokens` 计算；`input_tokens` 已包含缓存命中，因此不会再次叠加 `cache_read_tokens`。
- `account/usage/read` 当前展示的是账户接口日桶/累计/日峰值，不等同于 cc-switch 使用统计页的“真实消耗 Tokens”口径；该数据只作为 cc-switch 不可用时的兜底。
- `runtime_health` 只验证前后端 IPC 基线，不访问 Codex。
- `set_usage_window_mode` 负责在 Windows 收起态和详细态之间安全调整窗口尺寸。
- `update_tray_usage` 会同步真实或 stale 剩余额度；演示数据不会写入实时托盘标题。
- 当前 10 秒刷新由 Rust 后台线程提供节拍并复用现有 IPC：各 Codex 账户接口会并发启动独立的短生命周期 app-server 会话；上一轮未结束时不会重复启动下一轮。
- 历史记录、规划和阈值通知仍在后续任务中。

当前做到哪里、下一步做什么以及历次验证记录，统一维护在 [开发任务清单](docs/IMPLEMENTATION_PLAN.md)。每次完成需求后必须同步更新该文档。
