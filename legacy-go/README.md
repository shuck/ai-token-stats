# legacy-go（Go 旧版）

这是 Rust 重写前的 Go 版本（对应 git 提交 `7c60434`），仅作参考归档，不再维护。

## 构建与运行

前置：Go 1.26+（本机位于 `D:\ai-data\go-sdk\go\bin\go.exe`）。

```powershell
$env:JAVA_HOME = ''  # 不需要
D:\ai-data\go-sdk\go\bin\go.exe build -ldflags "-H windowsgui -s -w" -o ai-token-stats.exe .
.\ai-token-stats.exe
```

参数：`-smoke`（控制台输出汇总）、`-hold`（挂起 5 秒，测试单实例）。

> 已知问题：`loadLogFallback` 的查询写了 `AND ts > ?` 却未传参数，导致 Codex 日志兜底静默失效（Codex 数据少算）；Rust 版已修复。
