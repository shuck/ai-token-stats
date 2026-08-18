# AI Token 统计

Windows 桌面（系统托盘）工具，汇总本机 Codex、ZCode、Claude Code、OpenCode、DSH（DeepSeek Harness）等 AI 编程助手的 token 消耗，并以卡片和堆叠柱状图的形式展示。

## 功能特性

- 多 Agent 聚合统计：Codex / ZCode / Claude Code / OpenCode / DSH，支持按 Agent 或按模型查看
- 每日堆叠柱状图，悬停可查看当天明细（轮次、输入/缓存/输出/推理 token、上下文窗口等）
- 顶部摘要卡片：区间总用量、今日用量、总命中率、今日命中率、今日上下文峰值
- 时间范围切换：最近 7 / 14 / 30 / 90 天
- 系统托盘常驻：双击托盘图标打开面板，每分钟自动刷新
- 基于 SQLite 的增量缓存：只扫描有变化的源文件，刷新快速
- 单实例运行（全局互斥量），关闭窗口时最小化到托盘

## 统计指标

| 指标 | 说明 |
| --- | --- |
| 输入 / 缓存输入 | input tokens / cached input tokens（命中） |
| 缓存写入 | cache write input tokens |
| 输出 / 推理 | output tokens / reasoning output tokens |
| 总 token | 每次请求的 total tokens |
| 轮次 | 产生用量记录的请求次数 |
| 上下文窗口 | 请求对应的模型上下文上限 |
| 命中率 | 缓存输入 ÷ 输入 |
| 上下文使用率峰值 | 单次请求 input ÷ context window 的最大值 |

## 数据来源

各 Agent 数据路径由程序自动发现：

1. 环境变量：Codex 查 `CODEX_HOME`，ZCode 查 `ZCODE_DATA`，DSH 查 `DSH_HOME`。
2. 默认位置：`~/.codex`、`~/.claude/projects`、`~/.local/share/opencode/opencode.db`、`~/.dsh`。
3. 受限目录扫描（深度 ≤ 4、限时限量）：按特征识别 Codex（`logs_2.sqlite` 或 `sessions`+`archived_sessions`）、ZCode（含 `message` 表的 `db.sqlite`）、Claude（`.claude\projects`）、OpenCode（含 `session` 表的 `opencode.db`）、DSH（含 `storages/session_projcache.json` 的 `.dsh` 目录）。

发现结果保存在 exe 同目录的 `config.json` 中。缓存的路径失效时，程序自动重新发现并更新；仍找不到时可通过托盘菜单「设置 Agent 路径…」手动指定。「重新扫描路径」可强制重扫全部 Agent。

工具自身的增量缓存数据库（`ai-token-stats-cache.db`）与 `config.json` 一起放在 exe 同目录（exe 目录不可写时回退到 `%APPDATA%\ai-token-stats\`）。

## 使用

1. 直接运行 `ai-token-stats.exe`，程序常驻系统托盘。
2. 双击托盘图标（或右键 → 打开面板）显示主窗口。
3. 在主窗口切换时间范围、选择 Agent，点击「刷新」或等待每分钟自动刷新。
4. 点击窗口关闭按钮只隐藏到托盘，通过托盘菜单「退出」结束程序。
5. 右键托盘图标可选择「重新扫描路径」强制重扫，或「设置 Agent 路径…」手动指定各 Agent 数据源。

## 构建

前置要求：

- Windows 10/11
- Go 1.26+（见 `go.mod`）

```powershell
go build -ldflags "-H windowsgui -s -w" -o ai-token-stats.exe
```

`rsrc.syso` 已包含应用图标与清单（Common Controls v6、DPI 感知），`go build` 会自动将其嵌入。

## 命令行参数

| 参数 | 说明 |
| --- | --- |
| `-smoke` | 冒烟测试：收集数据后在控制台输出汇总（天/轮次/Agent/模型），随即退出 |
| `-hold` | 启动后挂起 5 秒，用于测试单实例逻辑 |

## 项目结构

```
ai-token-stats
├── main.go        # 程序入口、窗口与托盘界面
├── collector.go   # 各 Agent 数据采集与汇总
├── cache.go       # SQLite 增量缓存
├── chart.go       # 图表绘制与提示框
├── paths.go       # Agent 路径发现、配置读写
├── settings.go    # Agent 路径设置对话框
├── app.manifest   # Windows 清单（Common Controls v6、DPI）
├── app.ico        # 应用图标
├── rsrc.syso      # 嵌入的图标/清单资源
└── go.mod / go.sum
```

## 说明

- 各 Agent 数据路径自动发现并缓存到 exe 同目录的 `config.json`，无需手动配置；路径失效会自动重扫更新。
- 旧路径仍存在时程序会继续使用旧路径，此时可用托盘菜单「重新扫描路径」强制重扫。
- 模型归属通过会话元数据或日志匹配得出，无法识别时记为 `unknown`。
