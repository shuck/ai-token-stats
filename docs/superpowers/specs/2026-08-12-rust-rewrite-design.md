# Rust 重写设计

日期：2026-08-12
状态：已确认（待实现）

## 背景与目标

把 AI Token 统计工具（Windows 托盘应用）从 Go（lxn/walk）完整重写为 Rust。当前 Go 版约 2700 行，包含 4 个 Agent 的 token 采集、SQLite 增量缓存、路径自动发现、自定义图表面板。

目标：

1. 完整重写：托盘 + 面板 + 图表全部使用 Rust，功能与现版 1:1 对齐。
2. 数据与缓存格式沿用现有实现，现有缓存数据直接复用，可随时与旧版本对比。
3. 仓库根目录替换为 Rust 工程，Go 源码从工作树移除（git 历史保留）。
4. 构建产物 exe 不入库，提供一键构建脚本。

## 技术选型（已确认）

| 决策点 | 选择 |
| --- | --- |
| 重写范围 | 完整重写（GUI + 采集 + 缓存 + 图表） |
| GUI 框架 | eframe/egui + tray-icon |
| 功能范围 | 1:1 复刻现有 Go 版 |
| 数据/缓存格式 | 沿用现有（config.json、records_v2、source_files、source_watermarks） |
| 仓库布局 | 根目录替换为 Rust 工程，Go 从工作树移除 |
| 构建产物 | exe 不入库，提供 build.ps1 |
| 工程结构 | Cargo workspace：core 库 + app 二进制 |

## 1. 工程结构

```
ai-token-stats/
├── Cargo.toml                # workspace
├── crates/
│   ├── core/                 # 纯逻辑库（不依赖 GUI）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs     # config.json 读写（原子、损坏备份）
│   │       ├── discovery.rs  # Agent 路径发现与校验
│   │       ├── cache.rs      # SQLite 增量缓存（同 schema）
│   │       ├── codex.rs      # Codex JSONL + 日志库采集
│   │       ├── zcode.rs      # ZCode 采集（按 time_updated 增量）
│   │       ├── claude.rs     # Claude JSONL 采集
│   │       ├── opencode.rs   # OpenCode 采集（按 time_updated 增量）
│   │       └── report.rs     # 汇总统计
│   └── app/                  # 桌面二进制
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs       # 入口、单实例、-smoke
│           ├── ui.rs         # eframe 面板
│           ├── chart.rs      # 堆叠柱状图 + 悬停 tooltip
│           ├── tray.rs       # 托盘集成
│           └── settings.rs   # 设置 Agent 路径对话框
├── build.ps1                 # 一键构建（cargo build --release 并复制 exe 到根目录）
├── README.md                 # 重写
└── .gitignore                # 忽略 target/、ai-token-stats.exe、config.json、缓存 db
```

Go 源文件（main.go、collector.go、cache.go、chart.go、paths.go、settings.go 及 *_test.go、app.ico、app.manifest、rsrc.syso）从工作树移除。

## 2. 依赖

### core

- `rusqlite`（bundled 特性，内置 C SQLite）
- `serde` / `serde_json`（配置与 JSONL 解析）
- `regex`（Codex 日志正则）
- `walkdir`（JSONL 目录遍历、受限扫描）
- `chrono`（Asia/Shanghai 固定时区日期分桶）
- `windows-sys`（逻辑盘枚举 GetLogicalDrives）

### app

- `eframe` / `egui`
- `tray-icon`
- `windows-sys`（命名互斥量、IsIconic/ShowWindow(SW_RESTORE)/SetForegroundWindow）

## 3. 数据流与语义（照搬现有实现）

### 配置

- 目录解析：`current_exe` 所在目录；不可写回退 `%APPDATA%\ai-token-stats\`（用临时文件写测判定）。
- `config.json` 结构沿用：`agents.{Codex|ZCode|Claude|OpenCode}.{path,detected_at}`。
- 读写：原子写入（临时文件 + rename）；损坏时备份为 `config.json.corrupt-<ts>` 并重扫重写。

### 发现

- 顺序：环境变量（CODEX_HOME、ZCODE_DATA）→ 默认位置（~/.codex、~/.claude/projects、~/.local/share/opencode/opencode.db、%APPDATA%\ZCode）→ 受限扫描。
- 扫描限制：深度 ≤ 4、每根 ≤ 20000 目录、每根 20 秒、跳过系统目录（$Recycle.Bin 等）。
- 特征识别：Codex=logs_2.sqlite 或 sessions+archived_sessions；ZCode=含 message 表+data 列的 db.sqlite；Claude=.claude\projects；OpenCode=含 session 表+tokens_input 列的 opencode.db。
- 多命中取 ModTime 最新；全部未命中进入"未配置"状态（该 Agent 无数据，不影响其他）。

### 缓存

- SQLite 同 schema：`records_v2`（PK source+record_key）、`source_files`、`source_watermarks`。
- Codex/Claude：按 JSONL 文件 mtime+size 增量，变更文件重读并删除旧记录重插。
- ZCode：按 `time_updated` 水位线（键 `zcode-updated-ts`），记录 Ts/Date 仍取 `time_created`——保持刚修复的语义。
- OpenCode：按 `time_updated` 水位线（键 `opencode-ts`）。
- Codex 日志：按日志行 ts 水位线（键 `codex-logs`）。

### 统计

- `report` 结构对齐 Go 版：GeneratedAt、Timezone、Days、RangeStart/End、Totals、Today、Daily、Models、Agents，以及 ByModel/ByAgent 分桶。
- 指标：input、cached、cache_write、output、reasoning、total、turns、MaxContextWindow、MaxUsagePercent、HitRate。
- 日期归属：Asia/Shanghai 固定 +8 时区；范围 = 今天往前 (days-1) 天。

## 4. GUI（egui 1:1 复刻）

- 900×600 主窗口、垂直渐变背景、顶部控件（天数下拉 7/14/30/90、Agent 下拉 全部/Codex/ZCode/Claude/OpenCode、刷新按钮）。
- 5 张汇总卡片：最近 N 天、今日、总命中率、今日命中率、今日上下文峰值。
- 按天堆叠柱状图（egui painter）：按 Agent 或按模型堆叠；悬停显示当日明细 tooltip（日期、总/输入/缓存/输出/推理、轮次、上下文、命中率、各 Agent/模型拆分）；日期标签间隔自适应。
- 每分钟自动刷新；关闭窗口隐藏到托盘（不退出）。
- 托盘菜单：打开面板、刷新、重新扫描路径、设置 Agent 路径…、退出。
- 双击托盘打开面板：最小化则 SW_RESTORE，随后 SetForegroundWindow 置前（保留已修复语义）。
- 托盘集成方案：`tray-icon` 的 `MenuEvent` 全局 receiver 每帧轮询，触发 `ctx.request_repaint()`。
- 中文字体：启动时加载 `C:\Windows\Fonts\msyh.ttc`（Microsoft YaHei）注入 egui `FontDefinitions`；数值格式化沿用 亿/万/无数据 文案。
- 单实例：`Global\AITokenStatsTray` 命名互斥量，已有实例则直接退出。
- `-smoke`：同步发现并输出汇总（天/轮次/Agent/模型）后退出，等价于 Go 版。
- 设置对话框：4 行（Codex home / ZCode db / Claude projects / OpenCode db）+ 浏览按钮 + 确定（validateAgentPath 校验后写回 config.json）/ 取消。
- 窗口/托盘图标：启动时程序化生成 32×32 蓝色柱状图（等价 Go 版 makeIcon），同时用于窗口与托盘；`app.ico`/`rsrc.syso` 不再需要。

## 5. 迁移顺序（里程碑）

1. **脚手架**：workspace + core 骨架 + app 最小窗口（egui 显示"无数据"）。
2. **core：config + discovery**：移植配置读写、目录解析、发现与校验；移植对应单测。
3. **core：cache + collectors + report**：移植增量缓存、4 个采集器、汇总；单测 + 用现有缓存库等价性对拍。
4. **app：完整界面**：面板、卡片、图表、悬停、托盘、设置对话框、单实例、字体、自动刷新。
5. **收尾**：build.ps1、README、移除 Go 文件、等价性验收。

## 6. 测试与验收

### 单元测试（core）

- 配置：读写 round-trip、缺失、损坏、原子写。
- 校验：4 个 Agent 的 validateAgentPath 正反例。
- 发现：临时目录构造特征结构、多命中取最新、环境变量候选、USERPROFILE 隔离（防真实用户目录干扰）。
- ZCode 增量：占位 total=0 → 原地更新 → 按 time_updated 增量可读到新值、水位线正确、日期不变。
- 缓存：insert/replace、水位线语义。
- 汇总：按天/Agent/模型分桶、命中率、上下文峰值。

### 等价性验收

- Rust core 直接读取现有 `ai-token-stats-cache.db`，输出与 Go 版 `report` 按天/Agent/模型逐项对拍一致。
- `-smoke` 输出格式与 Go 版一致。

### 手动验收

- 托盘双击打开/置前、关闭隐藏、菜单各项、设置路径保存、重新扫描。
- 每分钟自动刷新；路径失效自动重扫。
- 首次扫描耗时不慢于 Go 版；热缓存刷新秒级。

## 7. 明确不做

- Pi Agent 接入（待用户确认数据源后另开计划）。
- 缓存 schema 重设计、预聚合。
- 跨平台支持（仅 Windows）。
- 自动更新、多语言、主题切换。

## 8. 风险

- eframe 与 tray-icon 的事件循环集成是主要风险点；采用 MenuEvent receiver 每帧轮询方案规避。
- 中文渲染依赖 msyh.ttc 加载；缺失时回退到 egui 默认字体并提示。
- rusqlite bundled 首次编译较慢；属一次性成本。
