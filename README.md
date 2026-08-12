# AI Token 统计

Windows 桌面（系统托盘）工具，汇总本机 Codex、ZCode、Claude Code、OpenCode 等 AI 编程助手的 token 消耗，并以卡片和堆叠柱状图的形式展示。

使用 Rust（eframe/egui + tray-icon）编写。

## 功能特性

- 多 Agent 聚合统计：Codex / ZCode / Claude Code / OpenCode，支持按 Agent 或按模型查看
- 每日堆叠柱状图，悬停可查看当天明细（轮次、输入/缓存/输出/推理 token、上下文窗口等）
- 顶部摘要卡片：区间总用量、今日用量、总命中率、今日命中率、今日上下文峰值
- 时间范围切换：最近 7 / 14 / 30 / 90 天
- 系统托盘常驻：双击托盘图标打开面板（最小化时恢复并置前），每分钟自动刷新，关闭窗口隐藏到托盘
- 基于 SQLite 的增量缓存：只扫描有变化的源文件，刷新快速
- 单实例运行，路径自动发现并缓存到 `config.json`

## 数据来源

各 Agent 数据路径由程序自动发现：

1. 环境变量：Codex 查 `CODEX_HOME`，ZCode 查 `ZCODE_DATA`。
2. 默认位置：`~/.codex`、`~/.claude/projects`、`~/.local/share/opencode/opencode.db`。
3. 受限目录扫描（深度 ≤ 4、限时限量）：按特征识别 Codex（`logs_2.sqlite` 或 `sessions`+`archived_sessions`）、ZCode（含 `message` 表的 `db.sqlite`）、Claude（`.claude\projects`）、OpenCode（含 `session` 表的 `opencode.db`）。

发现结果保存在 exe 同目录的 `config.json` 中。缓存的路径失效时，程序自动重新发现并更新；仍找不到时可通过托盘菜单「设置 Agent 路径…」手动指定。「重新扫描路径」可强制重扫全部 Agent。

工具自身的增量缓存数据库（`ai-token-stats-cache.db`）与 `config.json` 一起放在 exe 同目录（exe 目录不可写时回退到 `%APPDATA%\ai-token-stats\`）。

## 使用

1. 运行 `ai-token-stats.exe`（或 `.\build.ps1` 构建），程序常驻系统托盘。
2. 双击托盘图标（或右键 → 打开面板）显示主窗口。
3. 在主窗口切换时间范围、选择 Agent，点击「刷新」或等待每分钟自动刷新。
4. 点击窗口关闭按钮只隐藏到托盘，通过托盘菜单「退出」结束程序。

## 构建

前置要求：

- Windows 10/11
- Rust stable（GNU 工具链）与 mingw-w64（含 gcc，用于 rusqlite bundled）

```powershell
.\build.ps1
```

构建产物为根目录的 `ai-token-stats.exe`。

## 命令行参数

| 参数 | 说明 |
| --- | --- |
| `-smoke` | 冒烟测试：收集数据后在控制台输出汇总（天/轮次/Agent/模型），随即退出 |

## 工程结构

```
ai-token-stats
├── Cargo.toml            # workspace
├── crates/
│   ├── core/             # 采集/缓存/发现/统计（纯逻辑，可单测）
│   └── app/              # eframe 桌面界面、托盘、图表
├── build.ps1
└── docs/superpowers/     # 设计文档与实施计划
```

## 说明

- 各 Agent 数据路径自动发现并缓存到 exe 同目录的 `config.json`，无需手动配置；路径失效会自动重扫更新。
- 旧路径仍存在时程序会继续使用旧路径，此时可用托盘菜单「重新扫描路径」强制重扫。
- 模型归属通过会话元数据或日志匹配得出，无法识别时记为 `unknown`。
