# Agent 数据路径动态发现设计

日期：2026-08-12
状态：已确认（待实现）

## 背景与目标

当前 `collector.go` 顶部和 `cache.go` 中的 Agent 数据路径、缓存数据库路径均为硬编码本机绝对路径：

- Codex：`D:\ai-data\codex`（含 sessions、archived_sessions、logs_2.sqlite、state_5.sqlite）
- ZCode：`D:\ai-data\zcode-data\cli\db\db.sqlite`
- Claude Code：`C:\Users\zc\.claude\projects`
- OpenCode：`C:\Users\zc\.local\share\opencode\opencode.db`
- 缓存：`D:\ai-data\codex\codex-usage-tool\ai-token-stats-cache.db`

换机器或 Agent 数据目录迁移后，工具无法工作或需要改源码重编译。

目标：

1. 工具自身的缓存数据库与 `config.json` 放在 exe 同目录，不再依赖 `D:\ai-data\codex`。
2. 各 Agent 数据路径在首次运行时自动发现并缓存到 `config.json`。
3. 缓存路径失效（文件/目录被移动或删除）时自动重新发现并更新配置，无需用户手动修改。
4. 自动发现全部失败时，提供界面手动指定作为兜底。

## 方案总览

采用"配置缓存 + 惰性重探测"：

- 发现只在首次运行或缓存路径失效时执行，日常刷新零扫描开销。
- `config.json` 可编辑，作为自动发现的备用手段。
- 每次刷新（每分钟）前仅做一次 `os.Stat` 校验，成本极低。

## 1. 配置与缓存位置

### 目录解析

工具启动时通过 `os.Executable()` 获取 exe 所在目录，记为 `appDir`。

- `config.json`：`<appDir>\config.json`
- 缓存数据库：`<appDir>\ai-token-stats-cache.db`

若 `appDir` 不可写（例如安装在 Program Files），两者统一回退到
`%APPDATA%\ai-token-stats\`。回退目录由启动时的一次可写性探测决定，运行期间不变。

### config.json 结构

```json
{
  "agents": {
    "Codex":    { "path": "D:\\ai-data\\codex",                              "detected_at": "2026-08-12T12:00:00+08:00" },
    "ZCode":    { "path": "D:\\ai-data\\zcode-data\\cli\\db\\db.sqlite",      "detected_at": "2026-08-12T12:00:00+08:00" },
    "Claude":   { "path": "C:\\Users\\zc\\.claude\\projects",                 "detected_at": "2026-08-12T12:00:00+08:00" },
    "OpenCode": { "path": "C:\\Users\\zc\\.local\\share\\opencode\\opencode.db", "detected_at": "2026-08-12T12:00:00+08:00" }
  }
}
```

路径语义：

- Codex：home 目录，sessions、archived_sessions、logs_2.sqlite、state_5.sqlite 由其推导。
- ZCode / OpenCode：数据库文件的完整路径。
- Claude：projects 目录。

`detected_at` 为 RFC3339 时间，仅作记录，不参与逻辑。

### 配置读写

- 读取失败（文件不存在）：视为首次运行，触发发现。
- 读取失败（JSON 损坏）：备份为 `config.json.corrupt-<时间戳>`，触发发现并重写。
- 写入采用临时文件 + 原子重命名，避免写一半损坏。

## 2. 发现规则

发现仅针对"当前无有效路径"的 Agent 执行，按以下顺序，命中即停：

### 2.1 环境变量

| Agent | 环境变量 | 期望内容 |
| --- | --- | --- |
| Codex | `CODEX_HOME` | home 目录 |
| ZCode | `ZCODE_DATA` | 数据根目录（clidb 由 `<root>\cli\db\db.sqlite` 推导） |

### 2.2 默认位置

| Agent | 候选路径 |
| --- | --- |
| Codex | `%USERPROFILE%\.codex` |
| ZCode | `%APPDATA%\ZCode` 下递归一层查找 `db.sqlite` |
| Claude | `%USERPROFILE%\.claude\projects` |
| OpenCode | `%USERPROFILE%\.local\share\opencode\opencode.db` |

### 2.3 受限扫描

扫描根：

- 系统所有固定逻辑盘（如 C:\、D:\）根
- `%USERPROFILE%`、`%APPDATA%`、`%LOCALAPPDATA%`

限制：

- 深度 ≤ 4（相对扫描根）
- 每根最多访问 20000 个目录
- 每根扫描时限 20 秒
- 跳过 `$Recycle.Bin`、`System Volume Information`、`Windows` 等系统目录
- 扫描在后台 goroutine 执行，不阻塞 UI；`-smoke` 模式改为同步执行

特征识别：

| Agent | 特征 |
| --- | --- |
| Codex | 目录下存在 `logs_2.sqlite`，或同时存在 `sessions` 与 `archived_sessions` 子目录 |
| ZCode | 文件名为 `db.sqlite`，且 SQLite 中存在 `message` 表并含 `data` 列（用 `sqlite_master` 查询，仅一次） |
| Claude | 目录名为 `projects`、父目录名为 `.claude`，且目录内含 `.jsonl` 文件 |
| OpenCode | 文件名为 `opencode.db`，且 SQLite 中存在 `session` 表并含 `tokens_input` 列 |

多个命中时取 `ModTime` 最新的（最近被使用的）。

### 2.4 未发现

全部来源未命中时，该 Agent 进入"未配置"状态：收集时返回空数据，不阻塞其他 Agent。

## 3. 生命周期与自动更新

### 启动

1. 解析 `appDir` 与回退目录，加载或初始化 `config.json`。
2. 对每个 Agent 校验缓存路径（`os.Stat`；目录检查存在性，文件检查存在性）。
3. 有缺失/失效的 Agent 时，后台启动发现任务。

### 每次刷新前

1. 对每个 Agent 做 `os.Stat` 校验（毫秒级）。
2. 失效且无发现任务在跑 → 触发发现（in-flight 标志防止并发重复扫描）。
3. 发现成功 → 写回 `config.json`，托盘提示"已自动更新 Codex 路径：<新路径>"。
4. 发现失败 → 托盘提示一次"未找到 <Agent> 数据，可手动指定"，并启用「设置 Agent 路径…」入口。

### 界面改动

托盘右键菜单新增两项：

- 「重新扫描路径」：强制对所有 Agent 重新发现，成功后写回配置。用于旧路径仍存在但 Agent 已迁移到新目录的场景。
- 「设置 Agent 路径…」：打开小设置窗口，四个 Agent 各一个路径输入框 + 浏览按钮，确定后校验并写回 `config.json`。

主窗口标题栏保持 "AI Token 统计" 不变。

## 4. 错误处理

- 单个 Agent 数据源不可用（路径失效且未发现、数据库打不开）：只影响该 Agent，汇总中显示无数据，不中断其他 Agent。
- `config.json` 损坏：备份后重扫重写。
- exe 目录与回退目录都不可写：回退目录创建失败时给出错误提示并退出（避免静默丢数据）。
- 发现超时/超量：该根放弃，继续下一根；全部失败按 2.4 处理。

## 5. 测试策略

### 单元测试（新增 `paths_test.go` 等）

- 发现：在临时目录构造各 Agent 特征结构（含多个命中），验证命中与 mtime 选择。
- 校验：目录/文件存在与缺失。
- 配置：读写、JSON 损坏恢复、原子写入。
- 回退目录判定：不可写目录注入。

### 冒烟

- `-smoke` 保持可用：在无 UI 环境同步完成发现并输出汇总。

### 手动验证

1. 首次运行：观察自动发现并生成 `config.json`。
2. 将某 Agent 目录改名：运行后自动重扫并更新配置。
3. 删除全部数据源：弹出提示且不崩溃。
4. 经「设置 Agent 路径…」手动指定后正常采集。

## 6. 变更文件清单

| 文件 | 变更 |
| --- | --- |
| `paths.go`（新增） | 配置模型、读写、目录解析、发现、校验 |
| `paths_test.go`（新增） | 上述单元测试 |
| `settings.go`（新增） | 手动指定路径的设置窗口 |
| `collector.go` | 删除硬编码路径常量，改为从配置读取；`load*` 系列函数签名调整 |
| `cache.go` | 缓存 DB 路径改为运行时解析 |
| `main.go` | 启动初始化配置、刷新前校验、托盘菜单两项 |
| `README.md` | 更新"数据来源"与"说明"章节 |

## 7. 已知边界

- 旧路径仍存在时，工具会继续使用旧路径；无法感知 Agent 迁移到新目录。该场景由「重新扫描路径」手动触发。
- 自动发现依赖特征识别，扫描根之外的极特殊安装位置可能找不到，需手动指定兜底。
