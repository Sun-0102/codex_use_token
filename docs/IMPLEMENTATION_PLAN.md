# Codex Reserve 开发任务清单

最后更新：2026-07-24

> 本文档是项目进度的唯一事实来源。每次新增、修改或完成需求后，都必须同步更新“当前状态”“任务清单”“验证记录”和“更新日志”。

## 当前状态

| 项目 | 状态 |
| --- | --- |
| 当前版本 | `0.1.0` |
| 当前阶段 | T322 统一监控刷新间隔调整为 30 秒已完成；等待用户重新打包安装验证刷新稳定性 |
| 当前数据 | 账户状态/套餐来自真实 `account/read`；额度窗口来自真实 `account/rateLimits/read`；今日真实 Token 直接来自 `$CODEX_HOME/sessions`（默认 `~/.codex/sessions`）和 `archived_sessions` 的 `event_msg/token_count` 数字事件，按累计快照差值还原请求并以 `input_tokens + output_tokens` 统计，不依赖 cc-switch；账户 Token 日桶/累计仍来自真实 `account/usage/read` 作为兜底；当前任务 Token 来自同一个应用级 app-server 长连接接收的 `thread/tokenUsage/updated`（仅当前连接可见）；全部监控项启动时立即读取并由 Tauri 后台每 30 秒发出刷新节拍，窗口隐藏后继续刷新，上一轮未结束时跳过本轮；安装版会解析 PATH、nvm、fnm、asdf、Homebrew 和登录 shell 中的 Codex CLI 路径；账户、额度、账户 Token 和当前任务 Token 复用一个 `codex app-server --stdio` 长连接；连接失败时显示 Demo 或 stale，不标记为实时 |
| macOS | 菜单栏常驻；原生 NSPanel 浮层；点击外部自动隐藏；托盘定位拿不到 monitor 时降级到右上角显示 |
| Windows | 托盘入口与 340×82 / 420×510 两种窗口形态已编码，尚未在 Windows 真机验收 |
| Codex 连接 | 已能探测 CLI 状态、解析所需协议数据、启动/清理应用级 app-server 长连接、进行 JSONL 编解码与请求/响应关联、完成一次性握手，分流通知、处理请求超时、脱敏 stderr，管理异常退出后的重建与 shutdown 清理，用模拟 app-server 覆盖关键场景，并通过 `account/read` 读取真实登录状态和套餐、通过 `account/rateLimits/read` 读取所有限额桶并显示真实剩余比例/重置时间，能合并 `account/rateLimits/updated` 稀疏通知，通过 `account/usage/read` 显示真实账户 Token 日桶/累计，直接解析 Codex 本地会话 JSONL 显示今日真实 Token、缓存命中和请求数，能解析/展示 `thread/tokenUsage/updated` 当前任务 Token 通知，将同一份真实快照同步到菜单栏、收起态和详细面板，并在断开后保留最后快照且标记 stale；真实数据可用后不再显示 Demo 数值 |
| 下一项任务 | T401：设计 SQLite schema、迁移与数据保留策略 |
| Git | 已初始化；`master` 跟踪 `origin/master`，远端为用户指定的 Gitee 仓库 |

当前结论：软件框架、悬浮层、CLI 探测、协议类型边界、app-server 子进程生命周期、应用级 app-server 长连接、JSONL 请求/响应通道、初始化握手、通知分流、请求超时、stderr 日志边界、异常退出检测、重建与应用退出清理、模拟 app-server 场景覆盖、真实账户状态读取、真实限额桶读取、真实限额显示、稀疏限额通知合并、真实账户 Token 日桶/累计、Codex 本地会话今日 Token 统计、线程级 Token 通知展示、同源快照同步、stale 标记、Demo/实时互斥决策、统一 30 秒自动刷新与防重叠、安装版 CLI 路径解析、真实数据面板 UI 验收修复、指标卡裁切修复、可读性优化、菜单栏单周窗口额度同步修复、macOS 面板定位失败兜底、Token 中文单位显示和 Token 口径文案修正已经完成；现在等待用户重新打包安装验证数据与刷新稳定性。

## 状态约定

- `[x]` 已完成：代码、验收条件和必要测试均已通过。
- `[ ]` 待开发：尚未开始或不满足验收条件。
- `进行中`：当前唯一优先开发项。
- `阻塞`：必须写明阻塞原因和解除条件。

## 已完成

### F0 — 桌面应用框架

- [x] T001：安装并验证 Node.js、Rust stable 和 Cargo 开发环境。
- [x] T002：创建 Tauri 2 + React 19 + TypeScript + Vite 项目，不创建 Git 仓库。
- [x] T003：建立 `usedPercent → remainingPercent` 用量领域模型及前端单元测试。
- [x] T004：建立明确标记为“演示”的用量界面。
- [x] T005：建立 Rust `runtime_health` IPC 基线。
- [x] T006：建立系统托盘、窗口显示/隐藏、后台常驻和明确退出行为。
- [x] T007：建立 Windows 340×82 收起态和 420×510 详细态切换逻辑。
- [x] T008：建立 macOS 菜单栏入口，不显示 Dock 图标。
- [x] T009：将 macOS 主窗口转换为非激活式原生 NSPanel。
- [x] T010：面板显示前先取得状态栏位置；取消屏幕中央/右上角错误兜底。
- [x] T011：macOS 面板失焦自动收起，并支持跨桌面空间显示。
- [x] T012：预留经过范围校验的 `update_tray_usage` 命令。
- [x] T013：完成当前视觉重构：双层额度环、建议区、指标卡片和演示状态。
- [x] T014：锁定 macOS `tauri-nspanel` 依赖到明确提交版本。
- [x] T015：建立长期开发任务清单和“每次完成需求必须更新文档”的项目级规则。
- [x] T016：初始化 Git 仓库，将代码提交并发布到用户指定的 Gitee `master` 分支。

### R0 — 协议可行性验证

- [x] R001：确认本机 `codex-cli 0.144.5` 可生成 app-server JSON Schema 和 TypeScript 类型。
- [x] R002：确认 `account/rateLimits/read` 可提供短周期/长周期使用比例、重置时间、套餐和 Credits。
- [x] R003：确认 `account/usage/read` 可提供每日 Token、累计 Token 和峰值数据。
- [x] R004：确认 `thread/tokenUsage/updated` 可提供输入、缓存输入、输出、推理及总 Token。
- [x] R005：确认 `account/rateLimits/updated` 为稀疏更新，客户端必须合并快照或重新读取。

说明：R0 只代表协议已经在本机验证，不代表这些接口已经接入当前软件。

## 下一步：真实数据最短链路

以下任务按顺序执行。F1、F2 与 F3 已完成；真实数据最短链路已经可测试。后续从 T401 开始做历史记录与规划。

### F1 — Codex CLI 与协议边界

- [x] T101 — Codex CLI 探测与兼容状态
  - 发现 `codex` 可执行文件和版本。
  - 区分：可用、未安装、未登录、版本不兼容、启动失败。
  - 不读取、复制或输出 `auth.json` 中的凭据。
  - 验收：使用临时假 CLI 覆盖所有状态；界面能显示可操作的错误提示。

- [x] T102 — 固化当前 app-server 协议夹具与 Rust 类型
  - 生成并保存最小必要 Schema/夹具。
  - 覆盖 `initialize`、`account/read`、`account/rateLimits/read`、`account/usage/read`。
  - 兼容缺失字段、未知套餐、多 `limitId` 和可空 Credits。
  - 验收：不连接真实账户即可完成反序列化测试。

### F2 — App-server 会话模块

- [x] T201 — 管理 `codex app-server --stdio` 子进程生命周期。
- [x] T202 — 实现 JSONL 编解码、请求 ID 和响应关联。
- [x] T203 — 实现 `initialize` / `initialized` 握手。
- [x] T204 — 实现通知分流、请求超时和 stderr 日志边界。
- [x] T205 — 实现异常退出检测、退避重连和应用退出清理。
- [x] T206 — 使用模拟 app-server 覆盖成功、超时、畸形 JSON、退出和重连。

F2 验收：模块可独立测试；应用退出后不产生孤儿进程；日志不包含认证材料。

### F3 — 真实限额与 Token 使用量

- [x] T301 — 调用 `account/read`，显示登录状态和套餐。
- [x] T302 — 调用 `account/rateLimits/read`，读取所有限额桶。
- [x] T303 — 将 `usedPercent` 转成真实剩余比例并显示重置时间。
- [x] T304 — 订阅并正确合并 `account/rateLimits/updated` 稀疏通知。
- [x] T305 — 调用 `account/usage/read`，显示账户每日 Token 桶、累计和峰值 Token。
- [x] T306 — 接收 `thread/tokenUsage/updated`，显示当前连接可见任务的输入、缓存输入、输出、推理及总 Token。
- [x] T307 — 将同一份真实快照同步到菜单栏、Windows 收起态和详细面板。
- [x] T308 — 记录采样时间；断开后保留最后快照并明确标记 `stale`。
- [x] T309 — 真实数据可用后移除 Demo 数值；连接失败时绝不伪装成实时数据。
- [x] T310 — 修复真实数据面板 UI 验收问题：窗口按钮焦点框、实时文案、Credits 文案、最低余量说明和 Token 大数格式。
- [x] T311 — 修复真实数据面板指标卡显示不全：压缩面板布局、补足指标区高度并明确文本行高。
- [x] T312 — 优化真实数据面板可读性：提高小字字号和对比度，缩短技术文案，中文指标不使用等宽字体。
- [x] T313 — 修复菜单栏周剩余额度不显示：支持单窗口快照按时长映射到 `5h` 或 `W`，缺失窗口显示 `--`。
- [x] T314 — 修复 macOS 面板托盘定位失败后不显示：`No monitor found` 时降级定位但继续显示 NSPanel。
- [x] T315 — 将 Token 大数改为中文单位显示：万、亿，避免用户手动换算 M。
- [x] T316 — 修正 Token 指标口径文案：`account/usage/read` 的日桶/累计不再标成“今日 Token”，并明确不是 Codex 使用统计页真实消耗口径。
- [x] T317 — 接入 cc-switch 今日真实 Token 口径：读取 `.cc-switch/cc-switch.db` 的 `proxy_request_logs` 数字统计，显示今日真实 Token、缓存命中、新增输入、输出和请求数。（数据源已由 T321 替换）
- [x] T318 — 将 CLI、账户、额度、账户 Token、今日 Token 和当前任务 Token 统一为启动时立即读取、由 Tauri 后台每 10 秒触发自动刷新；隐藏窗口后继续刷新，整轮请求防重叠，失败后保留最后成功值并标记过期缓存。（刷新间隔已由 T322 调整为 30 秒）
- [x] T319 — 修复安装版 macOS App 找不到 nvm/fnm/asdf 下 Codex CLI 的问题：CLI 探测和 app-server 启动共用解析到的绝对路径，失败时显示更明确的 CLI 诊断文案。
- [x] T320 — 将账户、额度、账户 Token 和当前任务 Token 改为复用同一个应用级 `codex app-server --stdio` 长连接，启动后只握手一次，连接不可用时重建，降低 10 秒刷新中的短进程启动抖动。
- [x] T321 — 移除今日 Token 对 cc-switch SQLite 的依赖，直接解析 Codex `sessions` 与 `archived_sessions` JSONL 中的 `event_msg/token_count` 数字事件，以累计差值还原今日请求；不解析、保存或展示消息正文，不读取登录凭据。
- [x] T322 — 将统一监控刷新间隔从 10 秒调整为 30 秒；应用启动时仍立即读取，窗口隐藏后继续刷新，上一轮未结束时仍跳过本轮。

F3 验收：界面能显示当前账户真实的限额、重置时间、Codex 本地会话今日真实 Token 和账户 Token 日桶/累计兜底；数据来源与口径清晰；断开连接不会继续显示“实时”。（已完成，等待用户手工测试）

## 后续任务

### F4 — 历史记录与规划

- [ ] T401：设计 SQLite schema、迁移与数据保留策略。
- [ ] T402：对相同快照去重，保存限额、Token、重置时间和采样来源。
- [ ] T403：计算近期消耗速度、预计耗尽时间和每日安全预算。
- [ ] T404：正确处理滚动窗口、额度重置和百分比跳变。
- [ ] T405：增加近 24 小时和 7 天趋势视图。

### F5 — 通知与设置

- [ ] T501：增加 30%、15%、5% 余量通知及窗口级去重。
- [ ] T502：增加额度重置和数据长期失联提醒。
- [ ] T503：增加刷新频率、通知阈值和历史保留期设置。
- [ ] T504：增加开机启动开关。
- [ ] T505：增加诊断信息复制，确保内容不包含凭据。

### F6 — 跨平台与发布

- [ ] T601：在 Windows x64 真机验收托盘、悬浮窗、拖动和展开/收起。
- [ ] T602：设计正式应用图标和托盘模板图标。
- [ ] T603：完成 macOS Apple Silicon 签名、公证和安装验证。
- [ ] T604：完成 Windows x64 安装与卸载验证。
- [ ] T605：建立升级前协议兼容检查和发布清单。

## 已知边界

- 账户限额和账户 Token 汇总可以通过账户接口刷新；`account/usage/read` 当前只按协议暴露账户日桶/累计/日峰值，不等同于本地会话逐请求统计口径。
- 今日真实 Token 读取 `$CODEX_HOME/sessions`（默认 `~/.codex/sessions`）和 `archived_sessions` 中与今天有关的 JSONL；只解析时间戳和 `event_msg/token_count` 数字字段，不读取 Codex 登录凭据，不解析、保存或展示消息正文。
- Codex 的 `input_tokens` 已包含 `cached_input_tokens`，因此今日总量按 `input_tokens + output_tokens` 计算；缓存命中只用于拆分新增输入，不重复叠加。
- `thread/tokenUsage/updated` 只保证当前 app-server 连接可见任务的细粒度实时数据；其他 Codex 客户端的活动以账户汇总和限额刷新为准。
- 当前 30 秒刷新由 Rust 后台线程发出事件并复用既有 IPC；Codex 账户、额度、账户 Token 和当前任务 Token 复用同一个应用级 app-server 长连接并串行发送请求，上一轮尚未结束时跳过下一轮，避免重叠启动。
- 安装版 macOS App 不继承终端的 nvm PATH；CLI 解析会检查显式 `CODEX_CLI_PATH`、进程 PATH、`~/.local/bin`、Volta、Cargo、asdf、nvm、fnm、Homebrew，以及登录 shell 返回的 `codex` 路径；app-server 会使用同一个绝对路径启动。
- app-server 的部分接口可能随 Codex CLI 版本变化，因此类型必须从实际安装版本生成并做兼容检查。
- 软件只能调用 Codex 提供的账户数据，不应直接读取、解析或展示登录凭据。
- Windows 行为目前只有代码与编译层验证，不能标记为真机完成。

## 每次需求完成后的更新规则

每次完成开发需求时，必须执行以下动作：

1. 更新本文档顶部的“最后更新”“当前阶段”和“下一项任务”。
2. 给新需求分配稳定任务编号；不要只写一段没有编号的描述。
3. 只有满足验收条件并通过相关检查后，才能把任务改为 `[x]`。
4. 在“验证记录”中写明实际运行的检查及结果。
5. 在“更新日志”中记录日期、任务编号和用户可感知的变化。
6. 若实现改变数据真实性、权限、安全或平台行为，必须同步更新“已知边界”和 `README.md`。
7. 未完成或仅做可行性验证的内容必须明确标注，不能写成已经上线。

## 验证记录

| 日期 | 范围 | 结果 |
| --- | --- | --- |
| 2026-07-20 | 前端测试 | Vitest：28 个测试通过 |
| 2026-07-20 | Rust 测试 | Cargo：69 个测试通过（30 个库单元测试，含 3 个 T301 账户读取测试、3 个 T302 限额读取测试、3 个 T304 稀疏通知合并测试、3 个 T305 Token 用量测试、3 个 T306 线程 Token 通知测试、2 个 T317 cc-switch 统计解析测试和 1 个 T313 托盘部分窗口测试 + 11 个 T102 协议夹具测试 + 5 个 T201 子进程生命周期测试 + 5 个 T202 JSONL 测试 + 2 个 T203 握手测试 + 5 个 T204 连接边界测试 + 6 个 T205 supervisor 测试 + 5 个 T206 模拟 app-server 测试） |
| 2026-07-20 | T102 协议边界 | 使用本机 `codex-cli 0.144.5` 生成并保存 6 份最小 Schema；初始化、账户、限额和 Token 用量夹具测试全部通过 |
| 2026-07-20 | T201 子进程生命周期 | 使用模拟 app-server 覆盖默认 `codex app-server --stdio` 命令、启动参数、piped stdio、异常退出状态记录、显式停止和 Drop 清理 |
| 2026-07-20 | T202 JSONL 通道 | 使用内存 reader/writer 覆盖 JSONL 请求写入、递增请求 ID、通知跳过、乱序响应缓存和类型化响应反序列化 |
| 2026-07-20 | T203 initialize 握手 | 使用内存 JSONL 通道覆盖 `initialize` 成功后发送 `initialized`，以及服务端错误时不发送 `initialized` |
| 2026-07-20 | T204 连接边界 | 使用内存流和 Unix stream 覆盖通知分流、请求超时、stderr 脱敏函数、stderr reader 脱敏输出和日志长度上限 |
| 2026-07-20 | T205 Supervisor | 使用假会话和假 `codex` 子进程覆盖异常退出检测、退避重连、shutdown/Drop 后禁止重启和真实子进程清理 |
| 2026-07-20 | T206 模拟 app-server | 使用假 `codex app-server --stdio` 覆盖成功响应、请求超时、畸形 JSON 错误传播、子进程退出检测和 supervisor 重连 |
| 2026-07-20 | T301 account/read | 使用模拟连接覆盖 initialize/initialized/account-read 顺序、signed-in 套餐保留、signed-out 状态，以及不把邮箱或 credential source 写入展示消息 |
| 2026-07-20 | T302 rateLimits/read | 使用协议夹具和模拟连接覆盖顶层限额桶、`rateLimitsByLimitId` 多桶、稀疏桶、Credits 可空值和真实限额读取请求顺序 |
| 2026-07-20 | T303 真实限额显示 | 使用前端单元测试覆盖真实 `usedPercent` 到 `remainingPercent` 的转换、重置时间保留、套餐/Credits 展示和无真实窗口时回退 Demo |
| 2026-07-20 | T304 稀疏限额通知 | 使用单元测试覆盖默认桶稀疏更新、按 limitId 分组更新、保留旧重置时间/长周期窗口，以及空通知触发完整刷新 |
| 2026-07-20 | T305 account/usage/read | 使用协议夹具和模拟连接覆盖 Token 汇总、每日用量桶、稀疏汇总和真实 Token 用量读取请求顺序；前端测试覆盖账户每日桶/累计/峰值/趋势展示 |
| 2026-07-20 | T306 thread/tokenUsage/updated | 使用通知夹具覆盖输入、缓存输入、输出、推理、总 Token 解析，缺失 total 时本地求和，以及忽略无关通知；前端测试覆盖当前任务 Token 展示和等待状态 |
| 2026-07-20 | T307 同源快照同步 | 使用前端单元测试覆盖真实 snapshot 到菜单栏剩余比例的映射，以及 Demo snapshot 不写入实时托盘标题 |
| 2026-07-20 | T308 stale 标记 | 使用前端单元测试覆盖最后真实快照保留为 stale；UI 区分实时、过期缓存和演示数据 |
| 2026-07-20 | T309 Demo/实时互斥 | 使用前端单元测试覆盖真实 snapshot 优先于 Demo、断开后 stale、首次连接失败不伪装实时 |
| 2026-07-20 | T310 面板 UI 验收修复 | 使用前端单元测试覆盖大 Token 数 M 缩写、缺失窗口不当作 0%、Credits 文案不再显示 T302 待接入；实际样式修复窗口动作按钮 focus 框 |
| 2026-07-20 | T311 面板布局裁切修复 | 使用前端构建和类型检查验证压缩后的面板布局；指标卡文字设置明确行高，避免数字和说明被裁切 |
| 2026-07-20 | T312 面板可读性优化 | 使用前端测试和构建验证短文案、Token 趋势文案和当前任务 Token 文案；CSS 提高小字字号、对比度并使用系统中文字体 |
| 2026-07-20 | T313 菜单栏单周窗口同步 | 使用前端测试覆盖仅有 1 周窗口时映射到 `W`，使用 Rust 测试覆盖 `5h -- · W 75%` 标题格式 |
| 2026-07-20 | T314 macOS 定位兜底 | Rust 全量测试、格式检查和 Clippy 通过；`move_window_constrained(Position::TrayBottomCenter)` 失败时不再阻断 `show_and_make_key` |
| 2026-07-20 | T315 Token 中文单位 | 使用前端测试覆盖万/亿格式：`23,456 → 2.3万`、`363,885,618 → 3.6亿` |
| 2026-07-20 | T316 Token 口径文案 | 使用前端测试覆盖账户日桶、最近日桶、账户累计、日峰值和“非统计页”提示；`npm run check` 通过，9 个测试文件、27 个测试通过，TypeScript 与 Vite 构建通过 |
| 2026-07-20 | T317 cc-switch 今日真实 Token | 使用本机 `.cc-switch/cc-switch.db` 验证 `proxy_request_logs` 口径：`input_tokens` 已包含缓存命中，今日真实 Token 按 `input_tokens + output_tokens + cache_creation_tokens` 计算；前端测试覆盖截图示例 `151,779,617 → 1.5亿`，Rust 测试覆盖不重复计算缓存命中 |
| 2026-07-20 | T318 统一 10 秒刷新 | `npm run check` 通过：10 个前端测试文件、44 个测试通过，TypeScript 类型检查与 Vite 生产构建通过；`cargo test` 的 70 个 Rust 测试、`cargo fmt --check` 和 Clippy `-D warnings` 通过。前端测试覆盖立即刷新、后台事件触发、防重叠、StrictMode 跨生命周期共享在途请求、异常后继续刷新、成功值保留、过期缓存标记和 CLI stale 在连接卡片可见；Rust 测试锁定后台节拍为 10 秒 |
| 2026-07-21 | T319 安装版 CLI 路径解析 | `npm run check` 通过：10 个前端测试文件、46 个测试通过，TypeScript 类型检查与 Vite 生产构建通过；`cargo test` 通过：71 个 Rust 测试通过；`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`npm run tauri -- build --debug --no-bundle` 和 `git diff --check` 通过。测试覆盖 nvm/fnm/asdf 候选路径排序和 CLI 诊断文案 |
| 2026-07-21 | T320 app-server 长连接 | `cargo test` 通过：72 个 Rust 测试通过，新增测试覆盖账户读取和限额读取复用同一个 fake app-server 进程；`npm run check` 通过：10 个前端测试文件、46 个测试通过；`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`npm run tauri -- build --debug --no-bundle` 和 `git diff --check` 通过 |
| 2026-07-24 | T321 Codex 本地会话 Token | 新增 4 个 Rust 行为测试，覆盖累计快照差值、跨午夜基线、消息/损坏行过滤、活动与归档副本去重，并在 `TZ=UTC` 下复验；本机实时解析得到 287 请求、30,795,319 Token，与 CC Switch 3.16.5 已导入的 `codex_session` 数字逐项一致。`npm run check` 通过：10 个前端测试文件、46 个测试，TypeScript 与 Vite 构建通过；Rust 串行全量检查通过：35 个库测试和 39 个集成测试；`cargo fmt --check`、Clippy `-D warnings`、Tauri debug 无 bundle 构建和 `git diff --check` 通过 |
| 2026-07-24 | T322 统一 30 秒刷新 | TDD 红灯确认原行为为 10 秒，修改后定向测试锁定 30 秒；`npm run check` 通过：10 个前端测试文件、46 个测试，TypeScript 与 Vite 构建通过；Rust 串行全量检查通过：35 个库测试和 39 个集成测试；`cargo fmt --check`、Clippy `-D warnings`、Tauri debug 无 bundle 构建和 `git diff --check` 通过。首次全量运行触发既有 app-server 复用测试偶发失败，单独复验及随后全量重跑均通过 |
| 2026-07-20 | Rust 静态检查 | `cargo fmt --check` 和 `cargo clippy --all-targets -- -D warnings` 通过 |
| 2026-07-20 | T206 主代理审查 | 按无 Git 项目约束审查 T206 变更；未发现规范或规格遗留问题 |
| 2026-07-20 | T301 主代理审查 | 按无 Git 项目约束审查 T301 变更；确认只展示真实账户状态/套餐，额度仍标记为 Demo |
| 2026-07-20 | T302 主代理审查 | 按无 Git 项目约束审查 T302 变更；确认保留原始限额桶结构，尚未把界面额度标记为实时 |
| 2026-07-20 | T303 主代理审查 | 按无 Git 项目约束审查 T303 变更；确认只有真实限额窗口可用时才将 snapshot 标记为实时 |
| 2026-07-20 | T304 主代理审查 | 按无 Git 项目约束审查 T304 变更；确认稀疏通知不会覆盖缺失字段，缺少快照时要求重新读取 |
| 2026-07-20 | T305 主代理审查 | 按无 Git 项目约束审查 T305 变更；确认 Token 汇总来自账户接口，失败时不伪装为实时数据 |
| 2026-07-20 | T306 主代理审查 | 按无 Git 项目约束审查 T306 变更；确认线程级 Token 明确标记为当前连接可见任务，不替代账户汇总 |
| 2026-07-20 | T307 主代理审查 | 按无 Git 项目约束审查 T307 变更；确认菜单栏、收起态和详细面板使用同一份真实 snapshot，Demo 不同步为实时托盘数据 |
| 2026-07-20 | T308 主代理审查 | 按无 Git 项目约束审查 T308 变更；确认 stale snapshot 保留采样时间且不会被标记为实时 |
| 2026-07-20 | T309 主代理审查 | 按无 Git 项目约束审查 T309 变更；确认真实、stale、Demo 三种状态互斥且文案清晰 |
| 2026-07-20 | T310 主代理审查 | 按无 Git 项目约束审查 T310 变更；确认面板文案、状态标记和数字格式与真实数据阶段一致 |
| 2026-07-20 | T311 主代理审查 | 按无 Git 项目约束审查 T311 变更；确认底部四张指标卡在当前窗口高度内保留足够行高和垂直空间 |
| 2026-07-20 | T312 主代理审查 | 按无 Git 项目约束审查 T312 变更；确认面板不再直接展示长协议名，小字对比度和字号满足当前窗口可读性 |
| 2026-07-20 | T313 主代理审查 | 按无 Git 项目约束审查 T313 变更；确认菜单栏可显示部分真实窗口，缺失窗口不会阻止已知周额度同步 |
| 2026-07-20 | T314 主代理审查 | 按无 Git 项目约束审查 T314 变更；确认 macOS 托盘定位失败会降级到 TopRight 并继续显示面板 |
| 2026-07-20 | T315 主代理审查 | 按无 Git 项目约束审查 T315 变更；确认账户汇总和当前任务 Token 共用中文单位格式，小数字仍保持普通数字 |
| 2026-07-20 | T316 主代理审查 | 按无 Git 项目约束审查 T316 变更；确认面板不再把 `account/usage/read` 日桶误标为使用统计页今日真实消耗 |
| 2026-07-20 | T317 主代理审查 | 按无 Git 项目约束审查 T317 变更；确认只读取 cc-switch 数字统计字段，今日 Token 不重复叠加 `cache_read_tokens`，账户日桶保留为兜底 |
| 2026-07-20 | T205 主代理审查 | 按无 Git 项目约束审查 T205 变更；未发现规范或规格遗留问题 |
| 2026-07-20 | T204 主代理审查 | 按无 Git 项目约束审查 T204 变更；未发现规范或规格遗留问题 |
| 2026-07-20 | T203 主代理审查 | 按无 Git 项目约束审查 T203 变更；未发现规范或规格遗留问题 |
| 2026-07-20 | T202 主代理审查 | 按无 Git 项目约束审查 T202 变更；未发现规范或规格遗留问题 |
| 2026-07-20 | T201 主代理审查 | 按无 Git 项目约束审查 T201 变更；未发现规范或规格遗留问题 |
| 2026-07-20 | T102 双轴审查 | 修正 `usedPercent` 与生成 Schema 的整数类型偏差后，规范与规格复核均无遗留发现 |
| 2026-07-20 | 类型与构建 | TypeScript、Vite、Clippy、Tauri debug 构建通过 |
| 2026-07-20 | macOS 运行 | 状态栏应用可启动；NSPanel 初始化未崩溃；实际点击位置由用户截图确认 |
| 2026-07-20 | 仓库约束 | 已确认项目根目录不存在 `.git` |
| 2026-07-20 | T016 Git 规则 | 已按用户明确授权移除 `AGENTS.md` 中禁止 Git 操作的规则 |
| 2026-07-20 | T016 提交前验证 | `npm run check` 通过：9 个前端测试文件、28 个测试、TypeScript 与 Vite 构建均成功；`cargo test` 通过：69 个 Rust 测试；`cargo fmt --check` 通过；敏感文件名与常见密钥模式扫描无发现 |
| 2026-07-20 | T016 远端检查 | Gitee `origin` 可访问且 `git ls-remote --heads origin` 未返回分支，适合执行首次推送 |
| 2026-07-20 | T016 首次发布 | 根提交 `010d21a` 已成功推送到 Gitee `master`；本地 `master` 已设置为跟踪 `origin/master` |

## 更新日志

| 日期 | 任务 | 变化 |
| --- | --- | --- |
| 2026-07-20 | T001–T008 | 建立 Tauri/React 项目、Demo 用量面板、托盘和跨平台窗口框架 |
| 2026-07-20 | T009–T014 | macOS 改为原生 NSPanel，修复首次点击定位并完成视觉重构 |
| 2026-07-20 | R001–R005 | 使用本机 Codex Schema 确认真实限额与 Token 数据接口 |
| 2026-07-20 | T015 | 将实施计划升级为长期任务清单，并固定每次需求完成后的更新规则 |
| 2026-07-20 | T016 | 按用户最新授权移除项目的 Git 禁用规则，初始化仓库并将完整项目发布到指定 Gitee `master` 分支 |
| 2026-07-20 | T101 | 增加 CLI 路径发现、版本兼容、登录状态和启动失败分类，并在界面显示可操作状态 |
| 2026-07-20 | T102 | 固化 app-server 0.144.5 最小 Schema、合成夹具和 Rust 类型，兼容缺失字段、未知套餐、多限额桶与空 Credits |
| 2026-07-20 | T201 | 增加 app-server 会话模块，可启动 `codex app-server --stdio`、接管标准流、记录退出状态，并在显式停止或 Drop 时清理子进程 |
| 2026-07-20 | T202 | 增加 app-server JSONL 通信模块，支持写入带递增 ID 的请求、读取并关联响应、缓存乱序响应，并跳过尚未分流的通知 |
| 2026-07-20 | T203 | 增加 `initialize` / `initialized` 握手模块，成功初始化后发送 initialized 通知；初始化失败时停止握手 |
| 2026-07-20 | T204 | 增加 app-server 连接层，将 stdout 响应和通知分流，支持请求级超时，并对 stderr 日志做凭据脱敏后再暴露 |
| 2026-07-20 | T205 | 增加 app-server supervisor，检测异常退出后按退避策略重连，并在 shutdown 时停止当前会话且禁止后续重启 |
| 2026-07-20 | T206 | 增加模拟 app-server 集成测试，并让连接层把 stdout 中的畸形 JSON 明确传播为协议错误 |
| 2026-07-20 | T301 | 增加 `codex_account_status` IPC 命令，启动 app-server 完成握手后调用 `account/read`，前端显示真实账户登录状态和套餐但不把 Demo 额度标为实时 |
| 2026-07-20 | T302 | 增加 `codex_rate_limits_status` IPC 命令，启动 app-server 完成握手后调用 `account/rateLimits/read`，返回顶层和按 limitId 分组的真实限额桶 |
| 2026-07-20 | T303 | 前端接入真实限额状态，将 `usedPercent` 转为剩余比例并显示真实重置时间；限额不可用时继续显示 Demo |
| 2026-07-20 | T304 | 增加 `account/rateLimits/updated` 通知合并逻辑，支持默认桶和按 limitId 分组的稀疏更新；不可合并通知触发完整刷新 |
| 2026-07-20 | T305 | 增加 `codex_usage_status` IPC 命令，调用 `account/usage/read` 并在界面展示账户每日 Token 桶、累计 Token、峰值和每日趋势桶数量 |
| 2026-07-20 | T306 | 增加 `codex_thread_token_usage_status` IPC 命令，解析 `thread/tokenUsage/updated` 并在界面展示当前任务的输入、缓存输入、输出、推理和总 Token |
| 2026-07-20 | T307 | 增加真实 snapshot 到菜单栏标题的同步逻辑；Windows 收起态和详细面板继续共用同一份 snapshot 渲染 |
| 2026-07-20 | T308 | 增加 stale 数据源，保留最后真实快照并在读取失败后显示“过期缓存”标记 |
| 2026-07-20 | T309 | 抽出 snapshot 显示决策，确保真实数据可用时移除 Demo 数值，首次连接失败不伪装实时，断开后只显示 stale |
| 2026-07-20 | T310 | 修复面板视觉和文案问题：去掉窗口按钮大焦点框、把状态改为“额度实时”、修正 Credits 文案、显示具体最低额度窗口并将大 Token 数缩写为 M |
| 2026-07-20 | T311 | 压缩主面板垂直布局，降低 hero/insight 高度、增加指标区最小高度，并给指标文字设置明确 line-height，修复底部卡片显示不全 |
| 2026-07-20 | T312 | 提高辅助文字字号和对比度，缩短“当前任务”和 Token 趋势文案，中文指标卡改用系统字体和 tabular 数字，减少发虚感 |
| 2026-07-20 | T313 | 菜单栏同步逻辑按窗口时长识别 5h/1 周额度；Rust IPC 支持 `null` 百分比，周额度单独可用时显示 `5h -- · W 75%` |
| 2026-07-20 | T314 | 修复 macOS `No monitor found for the window` 后面板不显示；托盘底部定位失败时尝试右上角定位并继续显示 NSPanel |
| 2026-07-20 | T315 | 将 Token 数量格式从 M 改为中文单位：万、亿；例如 `363.9M` 改为 `3.6亿`，`23,456` 改为 `2.3万` |
| 2026-07-20 | T316 | 将面板“今日 Token / 累计 Token”改为“账户日桶 / 账户累计”，并在趋势小字标出“非统计页”，避免与 Codex 设置页 1.52 亿“真实消耗 Tokens”混淆 |
| 2026-07-20 | T317 | 接入 cc-switch 今日真实 Token：面板优先显示“今日 Token”约亿级真实消耗，并展示缓存命中、新增输入、输出和请求数；`account/usage/read` 日桶只作为兜底 |
| 2026-07-20 | T318 | 所有监控项改为启动时立即读取并由 Tauri 后台每 10 秒触发刷新；窗口隐藏后继续刷新，慢请求和 StrictMode 首轮不重叠，失败后保留最后成功数据并标记“过期缓存”，连接卡片会保留 CLI 状态过期提示 |
| 2026-07-21 | T319 | 修复安装版 App 从 Finder 启动时不继承 nvm PATH 导致只显示演示数据的问题；真实账户、额度、账户 Token 和当前任务 Token 读取都会使用解析到的 Codex CLI 绝对路径 |
| 2026-07-21 | T320 | 将真实账户、额度、账户 Token 和当前任务 Token 读取切换为应用级 app-server 长连接；每轮 10 秒刷新不再为这些接口重复启动多个短生命周期 app-server |
| 2026-07-24 | T321 | 今日 Token 改为直接解析 Codex 自己的 `sessions` / `archived_sessions` JSONL；移除运行时对 cc-switch 数据库和 `sqlite3` 的依赖，界面数据源文案改为“本地会话”，账户日桶继续作为失败兜底 |
| 2026-07-24 | T322 | 统一监控刷新间隔由 10 秒调整为 30 秒；启动时仍立即读取，隐藏窗口后继续刷新，慢请求防重叠行为保持不变 |
