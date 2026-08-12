# AI Token 统计

Windows 桌面（系统托盘）工具，汇总本机 Codex、ZCode、Claude Code、OpenCode 等 AI 编程助手的 token 消耗，并以卡片和堆叠柱状图的形式展示。

## 功能特性

- 多 Agent 聚合统计：Codex / ZCode / Claude Code / OpenCode，支持按 Agent 或按模型查看
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

数据路径为源码中硬编码的本机路径，按 Agent 汇总：

| Agent | 来源 |
| --- | --- |
| Codex | `D:\ai-data\codex\sessions` 和 `D:\ai-data\codex\archived_sessions` 下的 JSONL 会话文件，以及 `logs_2.sqlite`、`state_5.sqlite` |
| ZCode | `D:\ai-data\zcode-data\cli\db\db.sqlite` |
| Claude Code | `C:\Users\zc\.claude\projects` 下的 JSONL 会话文件 |
| OpenCode | `C:\Users\zc\.local\share\opencode\opencode.db` |

首次运行时会在 `D:\ai-data\codex\codex-usage-tool\ai-token-stats-cache.db` 自动创建增量缓存数据库，之后只读取发生变化的文件。

## 使用

1. 直接运行 `ai-token-stats.exe`，程序常驻系统托盘。
2. 双击托盘图标（或右键 → 打开面板）显示主窗口。
3. 在主窗口切换时间范围、选择 Agent，点击「刷新」或等待每分钟自动刷新。
4. 点击窗口关闭按钮只隐藏到托盘，通过托盘菜单「退出」结束程序。

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
├── app.manifest   # Windows 清单（Common Controls v6、DPI）
├── app.ico        # 应用图标
├── rsrc.syso      # 嵌入的图标/清单资源
└── go.mod / go.sum
```

## 说明

- 各 Agent 的数据路径均为本机硬编码路径，换机器使用需修改 `collector.go` 顶部的常量。
- 模型归属通过会话元数据或日志匹配得出，无法识别时记为 `unknown`。
