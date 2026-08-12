# AI Token 统计 Rust 重写 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 AI Token 统计工具从 Go 完整重写为 Rust（eframe/egui + tray-icon），功能与现版 1:1 对齐，数据/缓存格式沿用，根目录替换为 Rust 工程。

**Architecture:** Cargo workspace：`crates/core`（纯逻辑库：config/discovery/cache/collectors/report，可独立单测）+ `crates/app`（eframe GUI：面板/图表/托盘/设置）。实现顺序：先核心（可测试、可对拍），后界面。Go 源码在收尾任务前保留在工作树中，作为移植的参考实现。

**Tech Stack:** Rust（stable GNU 工具链）、eframe/egui 0.27、tray-icon 0.14、rusqlite 0.31（bundled）、serde/serde_json、regex、walkdir、chrono。

**环境前置（必须，见 Task 0）：** 本机无 Rust 工具链、无 C 编译器、无包管理器。需先安装 rustup（GNU stable 工具链）+ WinLibs mingw-w64（免管理员，全装用户目录），所有 cargo 命令前设置 PATH 包含 cargo 与 mingw 的 bin。

**参考实现：** Go 源文件（collector.go/cache.go/paths.go/chart.go/main.go 等）在收尾任务之前保留在工作树，移植语义以它们为准。

---

## Task 0: 安装 Rust 工具链（一次性）

**Files:** 无（系统环境）

- [ ] **Step 1: 下载并安装 rustup（GNU 工具链）**

```powershell
Invoke-WebRequest https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe -OutFile "$env:TEMP\rustup-init.exe"
& "$env:TEMP\rustup-init.exe" -y --default-toolchain stable-x86_64-pc-windows-gnu --profile minimal
```

Expected: `cargo`/`rustc` 安装到 `%USERPROFILE%\.cargo\bin` 与 `%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-gnu`。

- [ ] **Step 2: 下载并解压 WinLibs mingw-w64**

从 `https://github.com/brechtsanders/winlibs_mingw/releases` 下载最新 `winlibs-x86_64-posix-seh-...zip`（含 gcc），解压到 `%USERPROFILE%\mingw64`。若发布页不可用，备选：`https://winlibs.com` 首页链接。

```powershell
$tmp = "$env:TEMP\winlibs.zip"
Invoke-WebRequest <winlibs 下载直链> -OutFile $tmp
Expand-Archive $tmp -DestinationPath "$env:USERPROFILE\mingw64" -Force
```

Expected: `%USERPROFILE%\mingw64\bin\x86_64-w64-mingw32-gcc.exe` 存在。

- [ ] **Step 3: 验证工具链**

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin;$env:USERPROFILE\mingw64\bin;" + $env:PATH
rustc --version
cargo --version
x86_64-w64-mingw32-gcc --version
```

Expected: 三个命令都有版本输出。后续所有 cargo 命令都要先执行本 Step 的 PATH 设置。

- [ ] **Step 4: 提交说明（无代码提交）**

工具链属本机环境，不入库。本任务完成后在 `Cargo.toml` 创建前的验证留到 Task 1。

---

## Phase A：core 纯逻辑库

## Task 1: workspace 脚手架

**Files:**
- Create: `D:\ai-token-stats\Cargo.toml`
- Create: `D:\ai-token-stats\crates\core\Cargo.toml`
- Create: `D:\ai-token-stats\crates\core\src\lib.rs`
- Create: `D:\ai-token-stats\crates\app\Cargo.toml`
- Create: `D:\ai-token-stats\crates\app\src\main.rs`

- [ ] **Step 1: 创建 workspace 与两个 crate**

`D:\ai-token-stats\Cargo.toml`：

```toml
[workspace]
resolver = "2"
members = ["crates/core", "crates/app"]

[workspace.package]
edition = "2021"
version = "0.1.0"
```

`D:\ai-token-stats\crates\core\Cargo.toml`：

```toml
[package]
name = "ai-token-stats-core"
version.workspace = true
edition.workspace = true

[dependencies]
chrono = { version = "0.4", default-features = false, features = ["clock"] }
regex = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
walkdir = "2"
```

`D:\ai-token-stats\crates\core\src\lib.rs`：

```rust
pub mod cache;
pub mod claude;
pub mod codex;
pub mod config;
pub mod discovery;
pub mod opencode;
pub mod report;
pub mod zcode;
```

`D:\ai-token-stats\crates\app\Cargo.toml`：

```toml
[package]
name = "ai-token-stats"
version.workspace = true
edition.workspace = true

[dependencies]
ai-token-stats-core = { path = "../core" }
eframe = "0.27"
egui = "0.27"
tray-icon = "0.14"
windows-sys = { version = "0.59", features = [
    "Win32_Foundation",
    "Win32_System_Threading",
    "Win32_UI_WindowsAndMessaging",
] }
```

`D:\ai-token-stats\crates\app\src\main.rs`：

```rust
fn main() {
    println!("ai-token-stats (rust)");
}
```

注意：`lib.rs` 引用的模块在后续任务逐个创建；在 Task 2 之前编译会因缺文件失败，属预期。

- [ ] **Step 2: 编译验证**

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin;$env:USERPROFILE\mingw64\bin;" + $env:PATH
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --workspace
```

Expected: 首次构建报错仅因 `lib.rs` 缺失模块（`can't find crate` 或 `unresolved import`）。随后按 Task 2-5 逐个补模块。

- [ ] **Step 3: 提交**

```powershell
git add Cargo.toml crates/core/Cargo.toml crates/core/src/lib.rs crates/app/Cargo.toml crates/app/src/main.rs
git commit -m "chore: scaffold rust workspace"
```

---

## Task 2: core config（config.json 读写）

**Files:**
- Create: `D:\ai-token-stats\crates\core\src\config.rs`
- Create: `D:\ai-token-stats\crates\core\tests\config_test.rs`

语义与 Go 版 `paths.go` 的 `agentPath`/`config`/`loadConfig`/`saveConfig` 一致。

- [ ] **Step 1: 写失败测试**

`D:\ai-token-stats\crates\core\tests\config_test.rs`：

```rust
use ai_token_stats_core::config::{Agent, Config};
use std::fs;

#[test]
fn round_trip() {
    let dir = std::env::temp_dir().join(format!("ats-config-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");
    let mut cfg = Config::default();
    cfg.agents.insert(
        Agent::Codex,
        ai_token_stats_core::config::AgentPath {
            path: r"D:\codex".into(),
            detected_at: "2026-08-12T00:00:00+08:00".into(),
        },
    );
    cfg.save(&path).unwrap();
    let got = Config::load(&path).unwrap();
    assert_eq!(got.agents[&Agent::Codex].path, r"D:\codex");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn missing_file_returns_default() {
    let dir = std::env::temp_dir().join(format!("ats-config-missing-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let cfg = Config::load(&dir.join("nope.json")).unwrap();
    assert!(cfg.agents.is_empty());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn corrupt_file_errors() {
    let dir = std::env::temp_dir().join(format!("ats-config-corrupt-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");
    fs::write(&path, "{not json").unwrap();
    assert!(Config::load(&path).is_err());
    fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 2: 运行确认失败**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p ai-token-stats-core --test config_test
```

Expected: FAIL，`unresolved import ai_token_stats_core::config`。

- [ ] **Step 3: 实现 config.rs**

`D:\ai-token-stats\crates\core\src\config.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Agent {
    Codex,
    ZCode,
    Claude,
    OpenCode,
}

impl Agent {
    pub const ALL: [Agent; 4] = [Agent::Codex, Agent::ZCode, Agent::Claude, Agent::OpenCode];

    pub fn name(self) -> &'static str {
        match self {
            Agent::Codex => "Codex",
            Agent::ZCode => "ZCode",
            Agent::Claude => "Claude",
            Agent::OpenCode => "OpenCode",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AgentPath {
    pub path: String,
    pub detected_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub agents: BTreeMap<Agent, AgentPath>,
}

impl Config {
    pub fn load(path: &Path) -> io::Result<Config> {
        match fs::read(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e),
            Ok(data) => serde_json::from_slice(&data).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("corrupt config: {e}"))
            }),
        }
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        let data = serde_json::to_vec_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let tmp = temp_path(dir);
        fs::write(&tmp, &data)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }
}

fn temp_path(dir: &Path) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    dir.join(format!(".config-{}-{n}.tmp", std::process::id()))
}
```

Agent 枚举的 serde 序列化需自定义：`agents` 键在 JSON 里是 `"Codex"` 等字符串。补一个序列化辅助（在 `config.rs` 中追加）：

```rust
impl Serialize for Agent {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Agent {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(match s.as_str() {
            "Codex" => Agent::Codex,
            "ZCode" => Agent::ZCode,
            "Claude" => Agent::Claude,
            "OpenCode" => Agent::OpenCode,
            other => {
                return Err(serde::de::Error::custom(format!("unknown agent {other}")))
            }
        })
    }
}
```

- [ ] **Step 4: 运行确认通过**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p ai-token-stats-core --test config_test
```

Expected: 3 个测试 PASS。

- [ ] **Step 5: 提交**

```powershell
git add crates/core/src/config.rs crates/core/tests/config_test.rs
git commit -m "feat(core): config.json read/write with atomic save"
```

---

## Task 3: core discovery（路径校验与发现）

**Files:**
- Create: `D:\ai-token-stats\crates\core\src\discovery.rs`
- Create: `D:\ai-token-stats\crates\core\tests\discovery_test.rs`

语义与 Go 版 `paths.go` 的 `validateAgentPath`/`knownCandidates`/`discoverAgentPath`/`scanRoots` 一致。

- [ ] **Step 1: 写失败测试**

`D:\ai-token-stats\crates\core\tests\discovery_test.rs`：

```rust
use ai_token_stats_core::config::Agent;
use ai_token_stats_core::discovery::{discover_agent_path, validate_agent_path, ScanLimits};
use std::fs;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ats-disc-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn validate_codex_and_claude_dirs() {
    let root = tmp("validate");
    let codex = root.join("codex");
    fs::create_dir_all(codex.join("sessions")).unwrap();
    fs::create_dir_all(codex.join("archived_sessions")).unwrap();
    assert!(validate_agent_path(Agent::Codex, &codex));
    assert!(!validate_agent_path(Agent::Codex, &root));

    let claude = root.join(".claude").join("projects");
    fs::create_dir_all(claude.join("s1")).unwrap();
    fs::write(claude.join("s1").join("a.jsonl"), "{}").unwrap();
    assert!(validate_agent_path(Agent::Claude, &claude));
    assert!(!validate_agent_path(Agent::Claude, &root));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn discover_codex_and_claude_and_zcode_and_opencode() {
    let root = tmp("discover");
    let codex = root.join("ai-data").join("codex");
    fs::create_dir_all(codex.join("sessions")).unwrap();
    fs::create_dir_all(codex.join("archived_sessions")).unwrap();

    let claude = root.join(".claude").join("projects");
    fs::create_dir_all(claude.join("s1")).unwrap();
    fs::write(claude.join("s1").join("a.jsonl"), "{}").unwrap();

    let zdb = root.join("zdata").join("db.sqlite");
    fs::create_dir_all(zdb.parent().unwrap()).unwrap();
    let conn = rusqlite::Connection::open(&zdb).unwrap();
    conn.execute_batch("CREATE TABLE message (id TEXT, data TEXT);").unwrap();
    drop(conn);

    let odb = root.join("opencode.db");
    let conn = rusqlite::Connection::open(&odb).unwrap();
    conn.execute_batch("CREATE TABLE session (id TEXT, tokens_input INTEGER);").unwrap();
    drop(conn);

    let roots = vec![root.clone()];
    assert_eq!(discover_agent_path(Agent::Codex, &roots, &ScanLimits::test()), Some(codex.clone()));
    assert_eq!(discover_agent_path(Agent::Claude, &roots, &ScanLimits::test()), Some(claude));
    assert_eq!(discover_agent_path(Agent::ZCode, &roots, &ScanLimits::test()), Some(zdb));
    assert_eq!(discover_agent_path(Agent::OpenCode, &roots, &ScanLimits::test()), Some(odb));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn discover_prefers_newest() {
    let root = tmp("newest");
    for d in ["old", "new"] {
        let p = root.join(d);
        fs::create_dir_all(p.join("sessions")).unwrap();
        fs::create_dir_all(p.join("archived_sessions")).unwrap();
    }
    let old = root.join("old");
    let new = root.join("new");
    let old_t = filetime(&old) - 2 * 86400;
    let new_t = filetime(&new) - 3600;
    set_filetime(&old, old_t);
    set_filetime(&new, new_t);
    let got = discover_agent_path(Agent::Codex, &[root.clone()], &ScanLimits::test());
    assert_eq!(got, Some(new));
    fs::remove_dir_all(&root).ok();
}

fn filetime(p: &PathBuf) -> i64 {
    fs::metadata(p).unwrap().modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
}

fn set_filetime(p: &PathBuf, secs: i64) {
    let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64);
    let ft = filetime::FileTime::from_system_time(t);
    filetime::set_file_mtime(p, ft).unwrap();
}
```

测试需要 `filetime` dev 依赖：在 `crates/core/Cargo.toml` 追加：

```toml
[dev-dependencies]
filetime = "0.2"
```

测试使用 `ScanLimits::test()`，其含义见 Step 3。

- [ ] **Step 2: 运行确认失败**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p ai-token-stats-core --test discovery_test
```

Expected: FAIL，`unresolved import`（discovery 模块不存在）。

- [ ] **Step 3: 实现 discovery.rs**

`D:\ai-token-stats\crates\core\src\discovery.rs`（完整文件）：

```rust
use crate::config::Agent;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScanLimits {
    pub max_depth: usize,
    pub max_dirs: usize,
    pub max_duration: Duration,
}

impl Default for ScanLimits {
    fn default() -> Self {
        ScanLimits {
            max_depth: 4,
            max_dirs: 20_000,
            max_duration: Duration::from_secs(20),
        }
    }
}

impl ScanLimits {
    pub fn test() -> Self {
        ScanLimits {
            max_depth: 4,
            max_dirs: 20_000,
            max_duration: Duration::from_secs(20),
        }
    }
}

pub fn validate_agent_path(agent: Agent, path: &Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    let Ok(info) = fs::metadata(path) else {
        return false;
    };
    match agent {
        Agent::Codex => {
            if !info.is_dir() {
                return false;
            }
            if path.join("logs_2.sqlite").exists() {
                return true;
            }
            path.join("sessions").is_dir() && path.join("archived_sessions").is_dir()
        }
        Agent::ZCode => !info.is_dir() && has_message_table(path),
        Agent::Claude => info.is_dir() && is_claude_projects(path),
        Agent::OpenCode => !info.is_dir() && has_session_table(path),
    }
}

fn is_claude_projects(dir: &Path) -> bool {
    dir.file_name().and_then(|n| n.to_str()) == Some("projects")
        && dir
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            == Some(".claude")
}

fn has_message_table(path: &Path) -> bool {
    let Ok(conn) = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    ) else {
        return false;
    };
    let ok = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='message'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    ok && conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('message') WHERE name='data'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false)
}

fn has_session_table(path: &Path) -> bool {
    let Ok(conn) = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    ) else {
        return false;
    };
    let ok = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='session'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    ok && conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('session') WHERE name='tokens_input'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false)
}

pub fn known_candidates(agent: Agent) -> Vec<PathBuf> {
    let home = std::env::var("USERPROFILE").ok();
    match agent {
        Agent::Codex => {
            if let Ok(h) = std::env::var("CODEX_HOME") {
                if !h.is_empty() {
                    return vec![PathBuf::from(h)];
                }
            }
            home.map(|h| vec![PathBuf::from(h).join(".codex")]).unwrap_or_default()
        }
        Agent::ZCode => std::env::var("ZCODE_DATA")
            .ok()
            .filter(|d| !d.is_empty())
            .map(|d| vec![PathBuf::from(d).join("cli").join("db").join("db.sqlite")])
            .unwrap_or_default(),
        Agent::Claude => home
            .map(|h| vec![PathBuf::from(h).join(".claude").join("projects")])
            .unwrap_or_default(),
        Agent::OpenCode => home
            .map(|h| vec![PathBuf::from(h).join(".local").join("share").join("opencode").join("opencode.db")])
            .unwrap_or_default(),
    }
}

pub fn scan_roots() -> Vec<PathBuf> {
    let mut roots: std::collections::BTreeSet<PathBuf> = std::collections::BTreeSet::new();
    for letter in b'A'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if root.exists() {
            roots.insert(root);
        }
    }
    for env in ["USERPROFILE", "APPDATA", "LOCALAPPDATA"] {
        if let Ok(v) = std::env::var(env) {
            if !v.is_empty() {
                roots.insert(PathBuf::from(v));
            }
        }
    }
    roots.into_iter().collect()
}

const SKIP_DIRS: &[&str] = &[
    "$Recycle.Bin",
    "$RECYCLE.BIN",
    "System Volume Information",
    "Windows",
    "Program Files",
    "Program Files (x86)",
];

pub fn discover_agent_path(agent: Agent, roots: &[PathBuf], limits: &ScanLimits) -> Option<PathBuf> {
    for c in known_candidates(agent) {
        if validate_agent_path(agent, &c) {
            return Some(c);
        }
    }
    let mut best: Option<(PathBuf, i64)> = None;
    let started = Instant::now();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let mut visited = 0usize;
        for entry in WalkDir::new(root).max_depth(limits.max_depth).into_iter().filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            !e.file_type().is_dir()
                || !SKIP_DIRS.contains(&e.file_name().to_string_lossy().as_ref())
        }) {
            let Ok(e) = entry else { continue };
            visited += 1;
            if visited > limits.max_dirs || started.elapsed() > limits.max_duration {
                break;
            }
            let path = e.path().to_path_buf();
            let is_match = if e.file_type().is_dir() {
                match_agent_dir(agent, &path)
            } else {
                match_agent_file(agent, &path)
            };
            if !is_match {
                continue;
            }
            let mtime = fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if best.as_ref().map(|(_, m)| mtime > *m).unwrap_or(true) {
                best = Some((path, mtime));
            }
        }
    }
    best.map(|(p, _)| p)
}

fn match_agent_file(agent: Agent, path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    match agent {
        Agent::ZCode => name.eq_ignore_ascii_case("db.sqlite") && has_message_table(path),
        Agent::OpenCode => name.eq_ignore_ascii_case("opencode.db") && has_session_table(path),
        _ => false,
    }
}

fn match_agent_dir(agent: Agent, path: &Path) -> bool {
    match agent {
        Agent::Codex => {
            path.join("logs_2.sqlite").exists()
                || (path.join("sessions").is_dir() && path.join("archived_sessions").is_dir())
        }
        Agent::Claude => is_claude_projects(path),
        _ => false,
    }
}
```

注意：`has_message_table`/`has_session_table` 的 URI 打开方式配合 `path` 可能含反斜杠；rusqlite 打开本地文件用普通路径即可，若 URI 打开失败改为 `Connection::open_with_flags(path, SQLITE_OPEN_READ_ONLY)`（不带 URI 标志），两者选实现后可用的。

- [ ] **Step 4: 运行确认通过**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p ai-token-stats-core --test discovery_test
```

Expected: 3 个测试 PASS。

- [ ] **Step 5: 提交**

```powershell
git add crates/core/src/discovery.rs crates/core/tests/discovery_test.rs crates/core/Cargo.toml
git commit -m "feat(core): agent path validation and discovery"
```

---

## Task 4: core types + cache（SQLite 增量缓存）

**Files:**
- Create: `D:\ai-token-stats\crates\core\src\types.rs`
- Create: `D:\ai-token-stats\crates\core\src\cache.rs`
- Create: `D:\ai-token-stats\crates\core\tests\cache_test.rs`
- Modify: `D:\ai-token-stats\crates\core\src\lib.rs`

语义与 Go 版 `cache.go` 一致（表结构、水位线、changed-files 机制原样移植）。

- [ ] **Step 1: 写失败测试与 types**

先在 `crates/core/src/lib.rs` 追加：

```rust
pub mod types;
```

创建 `D:\ai-token-stats\crates\core\src\types.rs`：

```rust
#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub input: i64,
    pub cached: i64,
    pub cache_write: i64,
    pub output: i64,
    pub reasoning: i64,
    pub total: i64,
}

#[derive(Debug, Clone)]
pub struct Record {
    pub thread_id: String,
    pub agent: String,
    pub model: String,
    pub key: String,
    pub path: String,
    pub ts: i64,
    pub date: String,
    pub usage: Usage,
    pub context_window: Option<i64>,
}
```

创建 `D:\ai-token-stats\crates\core\tests\cache_test.rs`：

```rust
use ai_token_stats_core::cache::{insert_records, load_cache_records, Cache};
use ai_token_stats_core::types::{Record, Usage};
use std::fs;

#[test]
fn insert_load_and_watermark() {
    let dir = std::env::temp_dir().join(format!("ats-cache-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cache.db");
    let mut cache = Cache::open(&path).unwrap();

    cache.set_watermark("zcode-updated-ts", 100).unwrap();
    assert_eq!(cache.get_watermark("zcode-updated-ts"), 100);

    let rec = Record {
        thread_id: "zcode".into(),
        agent: "ZCode".into(),
        model: "gpt-4o".into(),
        key: "m1".into(),
        path: "zcode".into(),
        ts: 123,
        date: "2026-08-12".into(),
        usage: Usage { total: 271565, input: 270000, output: 1565, ..Default::default() },
        context_window: None,
    };
    insert_records(&mut cache, "ZCode", &[rec]).unwrap();

    let rows = load_cache_records(&cache, "ZCode").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].usage.total, 271565);
    assert_eq!(rows[0].date, "2026-08-12");

    fs::remove_dir_all(&dir).ok();
}
```

- [ ] **Step 2: 运行确认失败**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p ai-token-stats-core --test cache_test
```

Expected: FAIL，`unresolved import ai_token_stats_core::cache`。

- [ ] **Step 3: 实现 cache.rs**

`D:\ai-token-stats\crates\core\src\cache.rs`（完整文件）：

```rust
use crate::types::Record;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub struct Cache {
    conn: Connection,
}

impl Cache {
    pub fn open(path: &Path) -> rusqlite::Result<Cache> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS records_v2 (
                source TEXT NOT NULL,
                record_key TEXT NOT NULL,
                path TEXT NOT NULL DEFAULT '',
                agent TEXT NOT NULL,
                model TEXT NOT NULL,
                ts INTEGER NOT NULL,
                date TEXT NOT NULL,
                input INTEGER NOT NULL DEFAULT 0,
                cached INTEGER NOT NULL DEFAULT 0,
                cache_write INTEGER NOT NULL DEFAULT 0,
                output INTEGER NOT NULL DEFAULT 0,
                reasoning INTEGER NOT NULL DEFAULT 0,
                total INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (source, record_key)
            );
            CREATE INDEX IF NOT EXISTS idx_records_v2_date ON records_v2(date);
            CREATE INDEX IF NOT EXISTS idx_records_v2_agent ON records_v2(agent);
            CREATE TABLE IF NOT EXISTS source_files (
                source TEXT NOT NULL,
                path TEXT NOT NULL,
                size INTEGER NOT NULL,
                mtime INTEGER NOT NULL,
                PRIMARY KEY (source, path)
            );
            CREATE TABLE IF NOT EXISTS source_watermarks (
                source TEXT PRIMARY KEY,
                value INTEGER NOT NULL
            );
            "#,
        )?;
        Ok(Cache { conn })
    }

    pub fn get_watermark(&self, source: &str) -> i64 {
        self.conn
            .query_row(
                "SELECT value FROM source_watermarks WHERE source = ?1",
                params![source],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or(0)
    }

    pub fn set_watermark(&mut self, source: &str, value: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO source_watermarks(source, value) VALUES (?1, ?2)
             ON CONFLICT(source) DO UPDATE SET value = excluded.value",
            params![source, value],
        )?;
        Ok(())
    }

    /// 返回发生变化的 jsonl 文件路径；消失的文件会同步删除其缓存记录。
    pub fn changed_file_paths(&mut self, source: &str, roots: &[PathBuf]) -> rusqlite::Result<BTreeSet<PathBuf>> {
        let current = current_file_map(roots);
        let mut stored: BTreeMap<PathBuf, (i64, i64)> = BTreeMap::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT path, size, mtime FROM source_files WHERE source = ?1")?;
            let rows = stmt.query_map(params![source], |r| {
                Ok((PathBuf::from(r.get::<_, String>(0)?), r.get::<_, i64>(1)?, r.get::<_, i64>(2)?))
            })?;
            for row in rows {
                if let Ok((p, size, mtime)) = row {
                    stored.insert(p, (size, mtime));
                }
            }
        }
        let mut changed = BTreeSet::new();
        for (path, meta) in &current {
            let changed_f = match stored.get(path) {
                None => true,
                Some((s, m)) => *s != meta.0 || *m != meta.1,
            };
            if changed_f {
                changed.insert(path.clone());
            }
        }
        for path in stored.keys() {
            if !current.contains_key(path) {
                self.conn.execute(
                    "DELETE FROM records_v2 WHERE source = ?1 AND path = ?2",
                    params![source, path.to_string_lossy()],
                )?;
                self.conn.execute(
                    "DELETE FROM source_files WHERE source = ?1 AND path = ?2",
                    params![source, path.to_string_lossy()],
                )?;
            }
        }
        for (path, meta) in &current {
            self.conn.execute(
                "INSERT INTO source_files(source, path, size, mtime) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(source, path) DO UPDATE SET size = excluded.size, mtime = excluded.mtime",
                params![source, path.to_string_lossy(), meta.0, meta.1],
            )?;
        }
        Ok(changed)
    }

    pub fn delete_records_by_path(&mut self, source: &str, path: &Path) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM records_v2 WHERE source = ?1 AND path = ?2",
            params![source, path.to_string_lossy()],
        )?;
        Ok(())
    }

    pub fn insert_records(&mut self, source: &str, records: &[Record]) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO records_v2
                    (source, record_key, path, agent, model, ts, date, input, cached, cache_write, output, reasoning, total)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )?;
            for r in records {
                stmt.execute(params![
                    source,
                    r.key,
                    r.path,
                    r.agent,
                    r.model,
                    r.ts,
                    r.date,
                    r.usage.input,
                    r.usage.cached,
                    r.usage.cache_write,
                    r.usage.output,
                    r.usage.reasoning,
                    r.usage.total,
                ])?;
            }
        }
        tx.commit()
    }

    pub fn load_cache_records(&self, agent: &str) -> rusqlite::Result<Vec<Record>> {
        let sql = if agent == "all" {
            "SELECT agent, model, ts, date, input, cached, cache_write, output, reasoning, total FROM records_v2".to_string()
        } else {
            "SELECT agent, model, ts, date, input, cached, cache_write, output, reasoning, total FROM records_v2 WHERE agent = ?1".to_string()
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = if agent == "all" {
            stmt.query_map([], |r| {
                Ok(Record {
                    thread_id: String::new(),
                    agent: r.get(0)?,
                    model: r.get(1)?,
                    key: String::new(),
                    path: String::new(),
                    ts: r.get(2)?,
                    date: r.get(3)?,
                    usage: crate::types::Usage {
                        input: r.get(4)?,
                        cached: r.get(5)?,
                        cache_write: r.get(6)?,
                        output: r.get(7)?,
                        reasoning: r.get(8)?,
                        total: r.get(9)?,
                    },
                    context_window: None,
                })
            })?
        } else {
            stmt.query_map(params![agent], |r| {
                Ok(Record {
                    thread_id: String::new(),
                    agent: r.get(0)?,
                    model: r.get(1)?,
                    key: String::new(),
                    path: String::new(),
                    ts: r.get(2)?,
                    date: r.get(3)?,
                    usage: crate::types::Usage {
                        input: r.get(4)?,
                        cached: r.get(5)?,
                        cache_write: r.get(6)?,
                        output: r.get(7)?,
                        reasoning: r.get(8)?,
                        total: r.get(9)?,
                    },
                    context_window: None,
                })
            })?
        };
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

fn current_file_map(roots: &[PathBuf]) -> BTreeMap<PathBuf, (i64, i64)> {
    let mut map = BTreeMap::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let walker = walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok());
        for e in walker {
            if e.file_type().is_file()
                && e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| x.eq_ignore_ascii_case("jsonl"))
                    .unwrap_or(false)
            {
                if let Ok(m) = fs::metadata(e.path()) {
                    let size = m.len() as i64;
                    let mtime = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_nanos() as i64)
                        .unwrap_or(0);
                    map.insert(e.path().to_path_buf(), (size, mtime));
                }
            }
        }
    }
    map
}

// 便捷函数：与测试及 app 直接调用
pub fn insert_records(cache: &mut Cache, source: &str, records: &[Record]) -> rusqlite::Result<()> {
    cache.insert_records(source, records)
}

pub fn load_cache_records(cache: &Cache, agent: &str) -> rusqlite::Result<Vec<Record>> {
    cache.load_cache_records(agent)
}
```

- [ ] **Step 4: 运行确认通过**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p ai-token-stats-core --test cache_test
```

Expected: 1 个测试 PASS。

- [ ] **Step 5: 提交**

```powershell
git add crates/core/src/types.rs crates/core/src/cache.rs crates/core/tests/cache_test.rs crates/core/src/lib.rs
git commit -m "feat(core): sqlite incremental cache with watermarks"
```

---

## Task 5: core collectors（zcode + codex）

**Files:**
- Create: `D:\ai-token-stats\crates\core\src\zcode.rs`
- Create: `D:\ai-token-stats\crates\core\src\codex.rs`
- Create: `D:\ai-token-stats\crates\core\tests\zcode_test.rs`
- Create: `D:\ai-token-stats\crates\core\tests\codex_test.rs`

语义以 Go 版 `collector.go` 的 `loadZCodeRecords`/`loadRolloutRecords`/`loadThreadModels`/`loadLogFallback` 为准。

- [ ] **Step 1: 写失败测试**

`D:\ai-token-stats\crates\core\tests\zcode_test.rs`（重点：按 time_updated 增量 + 日期归属 time_created）：

```rust
use ai_token_stats_core::zcode::load_zcode_records;
use rusqlite::Connection;
use std::fs;

#[test]
fn zcode_placeholder_then_update() {
    let dir = std::env::temp_dir().join(format!("ats-zcode-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("db.sqlite");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
        "CREATE TABLE message (id TEXT PRIMARY KEY, time_created INTEGER, time_updated INTEGER, data TEXT);",
    )
    .unwrap();
    let created = 1_752_800_000_000i64; // 2026 年内的毫秒时间戳
    conn.execute(
        "INSERT INTO message(id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            "m1",
            created,
            created,
            r#"{"modelID":"gpt-4o","tokens":{"total":0,"input":0,"output":0,"reasoning":0,"cache":{"read":0,"write":0}}}"#
        ],
    )
    .unwrap();

    let (records, max_updated) = load_zcode_records(&path, 0).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].usage.total, 0);
    assert_eq!(records[0].ts, created);
    assert_eq!(max_updated, created);

    let updated = created + 60000;
    conn.execute(
        "UPDATE message SET data = ?1, time_updated = ?2 WHERE id = 'm1'",
        rusqlite::params![
            r#"{"modelID":"gpt-4o","tokens":{"total":271565,"input":270000,"output":1565,"reasoning":0,"cache":{"read":250000,"write":0}}}"#,
            updated
        ],
    )
    .unwrap();

    let (records, max_updated) = load_zcode_records(&path, created).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].usage.total, 271565);
    assert_eq!(records[0].ts, created, "Ts 必须仍取 time_created");
    assert_eq!(max_updated, updated);

    let (records, _) = load_zcode_records(&path, updated).unwrap();
    assert!(records.is_empty());
    fs::remove_dir_all(&dir).ok();
}
```

`D:\ai-token-stats\crates\core\tests\codex_test.rs`：

```rust
use ai_token_stats_core::codex::load_codex_records;
use std::fs;

#[test]
fn codex_rollout_jsonl_parses_token_count() {
    let dir = std::env::temp_dir().join(format!("ats-codex-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("rollout.jsonl");
    let body = r#"{"type":"session_meta","timestamp":"2026-08-12T10:00:00.000Z","payload":{"session_id":"sess1"}}
{"type":"event_msg","timestamp":"2026-08-12T10:01:00.000Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":20,"reasoning_output_tokens":5,"total_tokens":120},"model_context_window":200000}}}
"#;
    fs::write(&file, body).unwrap();
    let set = std::collections::BTreeSet::from([file.clone()]);
    let records = load_codex_records(&[file.clone()], Some(&set));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].usage.input, 100);
    assert_eq!(records[0].usage.cached, 50);
    assert_eq!(records[0].usage.output, 20);
    assert_eq!(records[0].usage.total, 120);
    assert_eq!(records[0].context_window, Some(200000));
    fs::remove_dir_all(&dir).ok();
}
```

`codex_test.rs` 中 `load_codex_records` 的第二个参数是"需重读的文件集合"（Go 版 changed map）；为简化，函数签名见 Step 3，`BTreeSet` 为空表示全量扫描（Go 版 `changed == nil` 语义）。

- [ ] **Step 2: 运行确认失败**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p ai-token-stats-core --test zcode_test --test codex_test
```

Expected: FAIL，`unresolved import`。

- [ ] **Step 3: 实现 zcode.rs**

`D:\ai-token-stats\crates\core\src\zcode.rs`（完整文件）：

```rust
use crate::types::{Record, Usage};
use chrono::TimeZone;
use rusqlite::{params, Connection, OpenFlags};
use serde::Deserialize;
use std::path::Path;

pub fn shanghai() -> chrono::FixedOffset {
    chrono::FixedOffset::east_opt(8 * 3600).unwrap()
}

pub fn date_key(ms: i64) -> String {
    let dt = chrono::Utc.timestamp_millis_opt(ms).unwrap().with_timezone(&shanghai());
    dt.format("%Y-%m-%d").to_string()
}

#[derive(Debug, Default, Deserialize)]
pub struct ZCodeToken {
    pub total: i64,
    pub input: i64,
    pub output: i64,
    pub reasoning: i64,
    #[serde(default)]
    pub cache: ZCodeCache,
}

#[derive(Debug, Default, Deserialize)]
pub struct ZCodeCache {
    pub read: i64,
    pub write: i64,
}

#[derive(Debug, Deserialize)]
pub struct ZCodeMessage {
    #[serde(rename = "modelID")]
    pub model_id: String,
    #[serde(default)]
    pub model: ZCodeModel,
    #[serde(default)]
    pub tokens: ZCodeToken,
}

#[derive(Debug, Default, Deserialize)]
pub struct ZCodeModel {
    #[serde(rename = "modelID")]
    pub model_id: String,
}

/// 按 `time_updated` 增量读取 ZCode 消息；记录 Ts/Date 取 `time_created`。
/// 返回（记录，最大 time_updated）。
pub fn load_zcode_records(path: &Path, since: i64) -> rusqlite::Result<(Vec<Record>, i64)> {
    if !path.exists() {
        return Ok((Vec::new(), since));
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare(
        "SELECT id, time_created, time_updated, data FROM message
         WHERE json_extract(data, '$.tokens') IS NOT NULL AND time_updated > ?1",
    )?;
    let rows = stmt.query_map(params![since], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, String>(3)?))
    })?;
    let mut records = Vec::new();
    let mut max_updated = since;
    for row in rows {
        let (id, created, updated, raw) = row?;
        if updated > max_updated {
            max_updated = updated;
        }
        let Ok(msg) = serde_json::from_str::<ZCodeMessage>(&raw) else { continue };
        let model = if !msg.model_id.is_empty() {
            msg.model_id
        } else if !msg.model.model_id.is_empty() {
            msg.model.model_id
        } else {
            "unknown".to_string()
        };
        records.push(Record {
            thread_id: "zcode".into(),
            agent: "ZCode".into(),
            model,
            key: id,
            path: "zcode".into(),
            ts: created,
            date: date_key(created),
            usage: Usage {
                input: msg.tokens.input,
                cached: msg.tokens.cache.read,
                cache_write: msg.tokens.cache.write,
                output: msg.tokens.output,
                reasoning: msg.tokens.reasoning,
                total: msg.tokens.total,
            },
            context_window: None,
        });
    }
    Ok((records, max_updated))
}
```

- [ ] **Step 4: 实现 codex.rs**

`D:\ai-token-stats\crates\core\src\codex.rs`：serde 结构与 Go 版一致，逐字段移植 `collector.go` 的 `loadRolloutRecords`/`loadThreadModels`/`loadLogFallback`。必需结构：

```rust
use crate::types::{Record, Usage};
use crate::zcode::date_key;
use rusqlite::{params, Connection, OpenFlags};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct RawEvent {
    #[serde(rename = "type")]
    pub type_: String,
    pub timestamp: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct TokenUsageJson {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub cache_write_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Deserialize)]
pub struct TokenCountInfo {
    pub last_token_usage: Option<TokenUsageJson>,
    pub total_token_usage: Option<TokenUsageJson>,
    pub model_context_window: Option<i64>,
}

/// changed = None 全量扫描；Some(空集) = 不重读；Some(非空) = 只重读列出的 jsonl（Go 版 changed map 语义）
pub fn load_codex_records(sessions: &[PathBuf], changed: Option<&BTreeSet<PathBuf>>) -> Vec<Record> {
    let files: Vec<PathBuf> = match changed {
        None => walk_jsonl(sessions),
        Some(set) if set.is_empty() => Vec::new(),
        Some(set) => set
            .iter()
            .filter(|p| p.extension().map(|e| e.to_string_lossy().to_lowercase() == "jsonl").unwrap_or(false))
            .cloned()
            .collect(),
    };
    let mut records = Vec::new();
    for f in files {
        parse_rollout_file(&f, &mut records);
    }
    records
}

fn walk_jsonl(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        for e in walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            if e.file_type().is_file()
                && e.path().extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case("jsonl")).unwrap_or(false)
            {
                out.push(e.path().to_path_buf());
            }
        }
    }
    out
}

fn parse_rollout_file(path: &Path, records: &mut Vec<Record>) {
    let Ok(f) = fs::File::open(path) else { return };
    let mut thread_id = String::new();
    for line in BufReader::new(f).lines().map_while(|l| l.ok()) {
        let Ok(ev) = serde_json::from_str::<RawEvent>(&line) else { continue };
        if ev.type_ == "session_meta" {
            if let Some(id) = ev.payload.get("session_id").and_then(|v| v.as_str()) {
                thread_id = id.to_string();
            }
            continue;
        }
        if ev.type_ != "event_msg" {
            continue;
        }
        let Some(mtype) = ev.payload.get("type").and_then(|v| v.as_str()) else { continue };
        if mtype == "token_count" {
            if thread_id.is_empty() {
                continue;
            }
            let Ok(info) = serde_json::from_value::<serde_json::Value>(ev.payload.clone()) else { continue };
            let info = info.get("info").cloned().unwrap_or(serde_json::Value::Null);
            let Ok(info) = serde_json::from_value::<TokenCountInfo>(info) else { continue };
            let u = info.last_token_usage.or(info.total_token_usage);
            let Some(u) = u else { continue };
            let ts = parse_rfc3339_ms(&ev.timestamp);
            if ts == 0 {
                continue;
            }
            let total = if u.total_tokens == 0 { u.input_tokens + u.output_tokens } else { u.total_tokens };
            let mut model = "unknown".to_string();
            // 模型归属：从 state_5.sqlite（threads 表）或 logs 反馈匹配；此处由上层注入
            // 简化实现：本函数不查库，模型留空由 load_log_models 补齐。
            records.push(Record {
                thread_id: thread_id.clone(),
                agent: "Codex".into(),
                model,
                key: format!("{thread_id}:{ts}:{}:{}", u.input_tokens, u.output_tokens),
                path: path.to_string_lossy().into_owned(),
                ts,
                date: date_key(ts),
                usage: Usage {
                    input: u.input_tokens,
                    cached: u.cached_input_tokens,
                    cache_write: u.cache_write_input_tokens,
                    output: u.output_tokens,
                    reasoning: u.reasoning_output_tokens,
                    total,
                },
                context_window: info.model_context_window,
            });
        }
    }
}

pub fn parse_rfc3339_ms(s: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|t| t.timestamp_millis())
        .unwrap_or(0)
}
```

说明：`loadThreadModels`（state_5.sqlite 的 threads.model）与 `loadLogFallback`（logs_2.sqlite 正则匹配 token 用量）按 `collector.go` 同逻辑移植为 `codex.rs` 中的 `load_thread_models(path) -> BTreeMap<String,String>`、`load_log_models(path) -> BTreeMap<String,String>`、`load_log_fallback(logs_db, thread_models, since) -> (Vec<Record>, i64)`；正则与 Go 版一致（`codex\.turn\.token_usage\.([a-z_]+)=(\d+)` 等），日志行 ts 单位为秒、转为毫秒后与日期分桶一致。上述三个函数的具体 SQL/正则照抄 `collector.go` 对应函数。

- [ ] **Step 5: 运行确认通过**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p ai-token-stats-core --test zcode_test --test codex_test
```

Expected: 2 个测试 PASS。

- [ ] **Step 6: 提交**

```powershell
git add crates/core/src/zcode.rs crates/core/src/codex.rs crates/core/tests/zcode_test.rs crates/core/tests/codex_test.rs
git commit -m "feat(core): zcode and codex collectors"
```

---

## Task 6: core collectors（claude + opencode）+ report

**Files:**
- Create: `D:\ai-token-stats\crates\core\src\claude.rs`
- Create: `D:\ai-token-stats\crates\core\src\opencode.rs`
- Create: `D:\ai-token-stats\crates\core\src\report.rs`
- Create: `D:\ai-token-stats\crates\core\tests\report_test.rs`

语义以 Go 版 `collector.go` 的 `loadClaudeRecords`/`loadOpenCodeRecords`/`summarize` 为准。

- [ ] **Step 1: 写失败测试**

`D:\ai-token-stats\crates\core\tests\report_test.rs`：

```rust
use ai_token_stats_core::report::summarize;
use ai_token_stats_core::types::{Record, Usage};

#[test]
fn summarize_buckets_by_agent_and_model() {
    let today = "2026-08-12";
    let mk = |agent: &str, model: &str, date: &str, total: i64, input: i64, cached: i64| Record {
        thread_id: "t".into(),
        agent: agent.into(),
        model: model.into(),
        key: format!("{agent}-{date}-{total}"),
        path: String::new(),
        ts: 0,
        date: date.into(),
        usage: Usage { total, input, cached, ..Default::default() },
        context_window: None,
    };
    let records = vec![
        mk("ZCode", "gpt-4o", today, 100, 100, 40),
        mk("ZCode", "gpt-4o", today, 50, 50, 10),
        mk("Codex", "gpt-5", today, 30, 30, 0),
    ];
    let rep = summarize(records, 30, today.to_string());
    assert_eq!(rep.totals.total, 180);
    assert_eq!(rep.totals.turns, 3);
    assert!(rep.totals.hit_rate.is_some());
    assert_eq!(rep.agents, vec!["ZCode".to_string(), "Codex".to_string()]);
    assert_eq!(rep.models, vec!["gpt-4o".to_string(), "gpt-5".to_string()]);
}
```

- [ ] **Step 2: 运行确认失败**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p ai-token-stats-core --test report_test
```

Expected: FAIL，`unresolved import ai_token_stats_core::report`。

- [ ] **Step 3: 实现 claude.rs 与 opencode.rs**

`D:\ai-token-stats\crates\core\src\claude.rs`：serde 结构与 Go 版 `claudeMessageJSON`/`claudeEventJSON` 一致；`load_claude_records(roots, changed: BTreeSet<PathBuf>) -> Vec<Record>` 逐行解析 jsonl：`message.usage` 存在且 input/output 非零才记录；`ts` 取事件 timestamp（RFC3339 毫秒）；`cached=cache_read_input_tokens`、`cache_write=cache_creation_input_tokens`、`reasoning=output_tokens_details.reasoning_tokens`、`total=input+output`；model 取 `message.model`，空则 "unknown"；ThreadID="claude"。

```rust
use crate::types::{Record, Usage};
use crate::zcode::date_key;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub output_tokens: i64,
    #[serde(default)]
    pub output_tokens_details: ClaudeOutputDetails,
}

#[derive(Debug, Default, Deserialize)]
pub struct ClaudeOutputDetails {
    pub reasoning_tokens: i64,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeMessage {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub usage: ClaudeUsage,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeEvent {
    #[serde(rename = "type")]
    pub type_: String,
    pub timestamp: String,
    pub message: Option<ClaudeMessage>,
}

pub fn load_claude_records(roots: &[PathBuf], changed: Option<&BTreeSet<PathBuf>>) -> Vec<Record> {
    let files: Vec<PathBuf> = match changed {
        None => walk_jsonl(roots),
        Some(set) if set.is_empty() => Vec::new(),
        Some(set) => set
            .iter()
            .filter(|p| p.extension().map(|e| e.to_string_lossy().to_lowercase() == "jsonl").unwrap_or(false))
            .cloned()
            .collect(),
    };
    let mut records = Vec::new();
    for path in files {
        let Ok(f) = fs::File::open(&path) else { continue };
        for line in BufReader::new(f).lines().map_while(|l| l.ok()) {
            let Ok(ev) = serde_json::from_str::<ClaudeEvent>(&line) else { continue };
            let Some(msg) = ev.message else { continue };
            let u = msg.usage;
            if u.input_tokens == 0 && u.output_tokens == 0 {
                continue;
            }
            let ts = crate::codex::parse_rfc3339_ms(&ev.timestamp);
            if ts == 0 {
                continue;
            }
            let model = if msg.model.is_empty() { "unknown".to_string() } else { msg.model };
            records.push(Record {
                thread_id: "claude".into(),
                agent: "Claude".into(),
                model,
                key: format!("{}:{ts}", path.to_string_lossy()),
                path: path.to_string_lossy().into_owned(),
                ts,
                date: date_key(ts),
                usage: Usage {
                    input: u.input_tokens,
                    cached: u.cache_read_input_tokens,
                    cache_write: u.cache_creation_input_tokens,
                    output: u.output_tokens,
                    reasoning: u.output_tokens_details.reasoning_tokens,
                    total: u.input_tokens + u.output_tokens,
                },
                context_window: None,
            });
        }
    }
    records
}

fn walk_jsonl(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in roots {
        if !root.is_dir() { continue; }
        for e in walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            if e.file_type().is_file()
                && e.path().extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case("jsonl")).unwrap_or(false)
            {
                out.push(e.path().to_path_buf());
            }
        }
    }
    out
}
```

`D:\ai-token-stats\crates\core\src\opencode.rs`：`load_opencode_records(path, since) -> rusqlite::Result<(Vec<Record>, i64)>`，SQL 与 Go 版一致（`SELECT id, time_updated, model, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write FROM session WHERE time_updated > ?1`），model 若以 `{` 开头则按 `{"id":...}` 解析取 id；`total = input + output`；返回最大 time_updated。

- [ ] **Step 4: 实现 report.rs**

`D:\ai-token-stats\crates\core\src\report.rs`（完整文件）：

```rust
use crate::types::{Record, Usage};
use crate::zcode::{date_key, shanghai};
use chrono::TimeZone;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct DaySummary {
    pub date: String,
    pub input: i64,
    pub cached: i64,
    pub output: i64,
    pub reasoning: i64,
    pub total: i64,
    pub turns: usize,
    pub max_context_window: Option<i64>,
    pub max_usage_percent: Option<f64>,
    pub hit_rate: Option<f64>,
    pub by_model: BTreeMap<String, DaySummary>,
    pub by_agent: BTreeMap<String, DaySummary>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub days: usize,
    pub range_start: String,
    pub range_end: String,
    pub totals: DaySummary,
    pub today: DaySummary,
    pub daily: Vec<DaySummary>,
    pub models: Vec<String>,
    pub agents: Vec<String>,
}

fn add_record(d: &mut DaySummary, r: &Record) {
    d.input += r.usage.input;
    d.cached += r.usage.cached;
    d.output += r.usage.output;
    d.reasoning += r.usage.reasoning;
    d.total += r.usage.total;
    d.turns += 1;
    if let Some(cw) = r.context_window {
        if d.max_context_window.map(|m| cw > m).unwrap_or(true) {
            d.max_context_window = Some(cw);
        }
        if cw > 0 {
            let pct = r.usage.input as f64 / cw as f64;
            if d.max_usage_percent.map(|m| pct > m).unwrap_or(true) {
                d.max_usage_percent = Some(pct);
            }
        }
    }
    if d.input > 0 {
        d.hit_rate = Some(d.cached as f64 / d.input as f64);
    }
    let m = d.by_model.entry(r.model.clone()).or_default();
    add_leaf(m, r);
    let a = d.by_agent.entry(r.agent.clone()).or_default();
    add_leaf(a, r);
    let am = a.by_model.entry(r.model.clone()).or_default();
    add_leaf(am, r);
}

fn add_leaf(d: &mut DaySummary, r: &Record) {
    d.input += r.usage.input;
    d.cached += r.usage.cached;
    d.output += r.usage.output;
    d.reasoning += r.usage.reasoning;
    d.total += r.usage.total;
    d.turns += 1;
    if let Some(cw) = r.context_window {
        if cw > 0 {
            let pct = r.usage.input as f64 / cw as f64;
            if d.max_usage_percent.map(|m| pct > m).unwrap_or(true) {
                d.max_usage_percent = Some(pct);
            }
        }
    }
    if d.input > 0 {
        d.hit_rate = Some(d.cached as f64 / d.input as f64);
    }
}

pub fn summarize(records: Vec<Record>, days: usize, today: String) -> Report {
    let start_ms = shanghai()
        .with_ymd_and_hms(
            shanghai().timestamp_millis_opt(0).unwrap().year(),
            shanghai().timestamp_millis_opt(0).unwrap().month(),
            shanghai().timestamp_millis_opt(0).unwrap().day(),
            0,
            0,
            0,
        )
        .unwrap();
    let _ = start_ms;
    let now = chrono::Utc::now().with_timezone(&shanghai());
    let start = now - chrono::Duration::days((days - 1) as i64);
    let start_key = start.format("%Y-%m-%d").to_string();

    let mut daily: Vec<DaySummary> = Vec::new();
    for offset in 0..days {
        let d = now - chrono::Duration::days(offset as i64);
        daily.push(DaySummary { date: d.format("%Y-%m-%d").to_string(), ..Default::default() });
    }
    daily.reverse();

    let mut totals = DaySummary { date: "total".into(), ..Default::default() };
    let mut today_sum = DaySummary { date: today.clone(), ..Default::default() };
    for r in &records {
        if r.date < start_key.as_str() || r.date > today.as_str() {
            continue;
        }
        add_record(&mut totals, r);
        if r.date == today {
            add_record(&mut today_sum, r);
        }
        if let Some(day) = daily.iter_mut().find(|d| d.date == r.date) {
            add_record(day, r);
        }
    }

    let mut models: Vec<String> = totals.by_model.keys().cloned().collect();
    models.sort_by(|a, b| totals.by_model[b].total.cmp(&totals.by_model[a].total));
    let mut agents: Vec<String> = totals.by_agent.keys().cloned().collect();
    agents.sort_by(|a, b| totals.by_agent[b].total.cmp(&totals.by_agent[a].total));

    Report {
        days,
        range_start: daily.first().map(|d| d.date.clone()).unwrap_or_default(),
        range_end: daily.last().map(|d| d.date.clone()).unwrap_or_default(),
        totals,
        today: today_sum,
        daily,
        models,
        agents,
    }
}
```

说明：`summarize` 的日期计算以 Go 版 `summarize` 语义为准（今天往前 days-1 天、上海时区）；上面实现若与 Go 版边界有出入，以 Go 版为准调整（`date_key(start_ms + offset*86400000)` 逻辑）。`today` 由调用方传入（`date_key(now_ms)`）。

- [ ] **Step 5: 运行确认通过**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p ai-token-stats-core
```

Expected: 全部 core 测试 PASS（config/discovery/cache/zcode/codex/report）。

- [ ] **Step 6: 提交**

```powershell
git add crates/core/src/claude.rs crates/core/src/opencode.rs crates/core/src/report.rs crates/core/tests/report_test.rs
git commit -m "feat(core): claude/opencode collectors and report summary"
```

---

## Task 7: core collect 编排 + app `-smoke`

**Files:**
- Create: `D:\ai-token-stats\crates\core\src\collect.rs`
- Modify: `D:\ai-token-stats\crates\core\src\lib.rs`
- Modify: `D:\ai-token-stats\crates\app\src\main.rs`

编排逻辑与 Go 版 `cache.go` 的 `ensureCached` + `collector.go` 的 `collect` 一致。

- [ ] **Step 1: 实现 collect.rs**

`D:\ai-token-stats\crates\core\src\collect.rs`（完整文件）：

```rust
use crate::cache::Cache;
use crate::claude::load_claude_records;
use crate::codex::load_codex_records;
use crate::config::{Agent, Config};
use crate::opencode::load_opencode_records;
use crate::report::{summarize, Report};
use crate::types::Record;
use crate::zcode::{date_key, load_zcode_records};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn codex_paths(cfg: &Config) -> Option<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    let home = cfg.agents.get(&Agent::Codex)?.path.clone();
    let home = PathBuf::from(home);
    Some((
        home.join("sessions"),
        home.join("archived_sessions"),
        home.join("logs_2.sqlite"),
        home.join("state_5.sqlite"),
    ))
}

fn records_for_agent(cfg: &Config, agent: Agent, cache: &mut Cache) -> Vec<Record> {
    match agent {
        Agent::Codex => {
            if let Some((sessions, archived, logs_db, state_db)) = codex_paths(cfg) {
                let logs_since = cache.get_watermark("codex-logs");
                let changed = cache
                    .changed_file_paths("Codex", &[sessions.clone(), archived.clone()])
                    .unwrap_or_default();
                let mut records = load_codex_records(&[sessions, archived], Some(&changed));
                // 日志兜底（含线程模型映射），语义同 Go loadLogFallback
                let (log_records, max_ts) = crate::codex::load_log_fallback(&logs_db, &state_db, logs_since);
                records.extend(log_records);
                for p in &changed {
                    cache.delete_records_by_path("Codex", p).ok();
                }
                cache.insert_records("Codex", &records).ok();
                if max_ts > logs_since {
                    cache.set_watermark("codex-logs", max_ts).ok();
                }
                cache.load_cache_records("Codex").unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Agent::Claude => {
            if let Some(root) = cfg.agents.get(&Agent::Claude) {
                let root = PathBuf::from(&root.path);
                let changed = cache
                    .changed_file_paths("Claude", &[root.clone()])
                    .unwrap_or_default();
                let records = load_claude_records(&[root], Some(&changed));
                for p in &changed {
                    cache.delete_records_by_path("Claude", p).ok();
                }
                cache.insert_records("Claude", &records).ok();
                cache.load_cache_records("Claude").unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Agent::ZCode => {
            if let Some(p) = cfg.agents.get(&Agent::ZCode) {
                let db = PathBuf::from(&p.path);
                let since = cache.get_watermark("zcode-updated-ts");
                if let Ok((records, max_updated)) = load_zcode_records(&db, since) {
                    cache.insert_records("ZCode", &records).ok();
                    if max_updated > since {
                        cache.set_watermark("zcode-updated-ts", max_updated).ok();
                    }
                }
                cache.load_cache_records("ZCode").unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Agent::OpenCode => {
            if let Some(p) = cfg.agents.get(&Agent::OpenCode) {
                let db = PathBuf::from(&p.path);
                let since = cache.get_watermark("opencode-ts");
                if let Ok((records, max_updated)) = load_opencode_records(&db, since) {
                    cache.insert_records("OpenCode", &records).ok();
                    if max_updated > since {
                        cache.set_watermark("opencode-ts", max_updated).ok();
                    }
                }
                cache.load_cache_records("OpenCode").unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    }
}

pub fn collect(cache_path: &std::path::Path, cfg: &Config, days: usize, agent: &str) -> Report {
    let mut cache = Cache::open(cache_path).unwrap_or_else(|_| Cache::open(&std::path::Path::new(":memory:")).unwrap());
    let mut records = Vec::new();
    let agents: Vec<Agent> = match agent {
        "Codex" => vec![Agent::Codex],
        "ZCode" => vec![Agent::ZCode],
        "Claude" => vec![Agent::Claude],
        "OpenCode" => vec![Agent::OpenCode],
        _ => Agent::ALL.to_vec(),
    };
    for a in agents {
        records.extend(records_for_agent(cfg, a, &mut cache));
    }
    let today = date_key(chrono::Utc::now().timestamp_millis());
    summarize(records, days, today)
}
```

在 `crates/core/src/lib.rs` 追加：

```rust
pub mod collect;
```

`codex.rs` 需要补充 `load_log_fallback(logs_db, state_db, since) -> (Vec<Record>, i64)`（照抄 Go 版 `loadLogFallback`，返回最大日志行 ts；ts 秒转毫秒存记录）。若 state_db/logs_db 不存在返回 `(vec![], since)`。

- [ ] **Step 2: 修改 app main.rs 支持 -smoke**

`D:\ai-token-stats\crates\app\src\main.rs`：

```rust
use ai_token_stats_core::collect::collect;
use ai_token_stats_core::config::Config;
use std::path::PathBuf;

fn app_dir() -> PathBuf {
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        if writable(&dir) {
            return dir;
        }
    }
    let fallback = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("ai-token-stats");
    std::fs::create_dir_all(&fallback).ok();
    fallback
}

fn writable(dir: &PathBuf) -> bool {
    let probe = dir.join(format!(".write-test-{}", std::process::id()));
    match std::fs::write(&probe, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = app_dir();
    let cfg = Config::load(&dir.join("config.json")).unwrap_or_default();
    if args.iter().any(|a| a == "-smoke") {
        // 先做一次路径发现并写回（语义同 Go 版首次运行）
        crate::bootstrap::ensure_discovered(&cfg, &dir.join("config.json"));
        let rep = collect(&dir.join("ai-token-stats-cache.db"), &cfg, 30, "all");
        println!("SMOKE OK: days={} turns={} agents={:?} models={:?}", rep.days, rep.totals.turns, rep.agents, rep.models);
        for model in &rep.models {
            if let Some(md) = rep.totals.by_model.get(model) {
                println!("  {model}: total={} input={} cached={}", md.total, md.input, md.cached);
            }
        }
        for agent in &rep.agents {
            if let Some(ad) = rep.totals.by_agent.get(agent) {
                println!("  [{agent}] total={} turns={}", ad.total, ad.turns);
            }
        }
        return;
    }
    println!("ai-token-stats (rust)");
}

mod bootstrap;
```

`bootstrap` 模块（`crates/app/src/bootstrap.rs`）负责"发现缺失路径并写回 config"：

```rust
use ai_token_stats_core::config::{Agent, AgentPath, Config};
use ai_token_stats_core::discovery::{discover_agent_path, scan_roots, validate_agent_path, ScanLimits};
use std::path::Path;

pub fn ensure_discovered(cfg: &Config, config_path: &Path) {
    let mut changed = false;
    let roots = scan_roots();
    let limits = ScanLimits::default();
    let mut cfg = cfg.clone();
    for agent in Agent::ALL {
        let valid = cfg.agents.get(&agent).map(|a| validate_agent_path(agent, Path::new(&a.path))).unwrap_or(false);
        if valid {
            continue;
        }
        if let Some(p) = discover_agent_path(agent, &roots, &limits) {
            cfg.agents.insert(agent, AgentPath {
                path: p.to_string_lossy().into_owned(),
                detected_at: chrono::Utc::now().to_rfc3339(),
            });
            changed = true;
        }
    }
    if changed {
        cfg.save(config_path).ok();
    }
}
```

- [ ] **Step 3: 编译与冒烟验证**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p ai-token-stats-core
& "$env:USERPROFILE\.cargo\bin\cargo.exe" run -p ai-token-stats -- -smoke
```

Expected: 编译通过、core 测试 PASS、smoke 输出 `SMOKE OK: ...`（若本机数据源存在）。

- [ ] **Step 4: 提交**

```powershell
git add crates/core/src/collect.rs crates/core/src/lib.rs crates/app/src/main.rs crates/app/src/bootstrap.rs
git commit -m "feat(core): collect orchestration and app -smoke"
```

---

## Phase B：app GUI

## Task 8: eframe 窗口骨架（字体/单实例/图标）

**Files:**
- Modify: `D:\ai-token-stats\crates\app\src\main.rs`
- Create: `D:\ai-token-stats\crates\app\src\ui.rs`

- [ ] **Step 1: 实现单实例 + 图标 + 字体 + 最小窗口**

`crates/app/Cargo.toml` 追加：

```toml
rfd = "0.15"
```

`D:\ai-token-stats\crates\app\src\main.rs` 替换为：

```rust
use ai_token_stats_core::collect::collect;
use ai_token_stats_core::config::Config;
use std::path::PathBuf;
use std::sync::Arc;

mod bootstrap;
mod chart;
mod settings;
mod tray;
mod ui;

fn app_dir() -> PathBuf {
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        if writable(&dir) {
            return dir;
        }
    }
    let fallback = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("ai-token-stats");
    std::fs::create_dir_all(&fallback).ok();
    fallback
}

fn writable(dir: &PathBuf) -> bool {
    let probe = dir.join(format!(".write-test-{}", std::process::id()));
    match std::fs::write(&probe, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn single_instance() -> bool {
    use windows_sys::Win32::System::Threading::CreateMutexW;
    let name: Vec<u16> = "Global\\AITokenStatsTray".encode_utf16().chain(std::iter::once(0)).collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return false;
    }
    std::io::Error::last_os_error().raw_os_error() != Some(183) // ERROR_ALREADY_EXISTS
}

fn make_icon_data() -> egui::IconData {
    let w = 32usize;
    let h = 32usize;
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let (r, g, b) = if (5..12).contains(&x) && (12..27).contains(&y) {
                (190, 220, 255)
            } else if (13..20).contains(&x) && (6..27).contains(&y) {
                (255, 255, 255)
            } else if (21..28).contains(&x) && (16..27).contains(&y) {
                (190, 220, 255)
            } else {
                (20, 90, 220)
            };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    egui::IconData { rgba, width: w as u32, height: h as u32 }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\msyh.ttc") {
        fonts.font_data.insert("msyh".to_owned(), egui::FontData::from_owned(bytes));
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families.entry(family).or_default().insert(0, "msyh".to_owned());
        }
    }
    ctx.set_fonts(fonts);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = app_dir();
    let cfg = Config::load(&dir.join("config.json")).unwrap_or_default();
    if args.iter().any(|a| a == "-smoke") {
        bootstrap::ensure_discovered(&cfg, &dir.join("config.json"));
        let rep = collect(&dir.join("ai-token-stats-cache.db"), &cfg, 30, "all");
        println!("SMOKE OK: days={} turns={} agents={:?} models={:?}", rep.days, rep.totals.turns, rep.agents, rep.models);
        for model in &rep.models {
            if let Some(md) = rep.totals.by_model.get(model) {
                println!("  {model}: total={} input={} cached={}", md.total, md.input, md.cached);
            }
        }
        for agent in &rep.agents {
            if let Some(ad) = rep.totals.by_agent.get(agent) {
                println!("  [{agent}] total={} turns={}", ad.total, ad.turns);
            }
        }
        return;
    }
    if !single_instance() {
        return;
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_icon(Arc::new(make_icon_data())),
        ..Default::default()
    };
    let app = ui::App::new(dir, cfg);
    let _ = eframe::run_native(
        "AI Token 统计",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            tray::create_tray(cc.egui_ctx.clone());
            Ok(Box::new(app))
        }),
    );
}
```

`crates/app/src/ui.rs` 先提供最小实现（后续任务扩展）：

```rust
use ai_token_stats_core::config::Config;
use std::path::PathBuf;

pub struct App {
    pub dir: PathBuf,
    pub cfg: Config,
}

impl App {
    pub fn new(dir: PathBuf, cfg: Config) -> Self {
        App { dir, cfg }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| ui.label("AI Token 统计"));
        });
    }
}
```

`chart.rs`/`settings.rs`/`tray.rs` 先创建带占位函数的模块文件，保证 Task 9 可编译；Task 10-12 逐个填充：

`chart.rs`：

```rust
use ai_token_stats_core::report::Report;

pub fn draw_chart(_ui: &mut egui::Ui, _rep: &Report, _agent: &str) {}
```

`settings.rs`：

```rust
pub fn show_settings(_ctx: &egui::Context, _app: &mut crate::ui::App) {}
```

`tray.rs`：

```rust
pub fn poll_events(_app: &mut crate::ui::App) {}
```

- [ ] **Step 2: 编译验证**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build -p ai-token-stats
```

Expected: 编译通过（首次拉取 eframe/egui 等依赖，耗时较长）。

- [ ] **Step 3: 提交**

```powershell
git add crates/app/src crates/app/Cargo.toml
git commit -m "feat(app): eframe window skeleton with font/icon/single-instance"
```

---

## Task 9: 面板 UI（控件 + 汇总卡片 + 刷新链路）

**Files:**
- Modify: `D:\ai-token-stats\crates\app\src\ui.rs`

- [ ] **Step 1: 实现 ui.rs**

`D:\ai-token-stats\crates\app\src\ui.rs`（完整文件）：

```rust
use ai_token_stats_core::collect::collect;
use ai_token_stats_core::config::Config;
use ai_token_stats_core::report::Report;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct App {
    pub dir: PathBuf,
    pub cfg: Config,
    pub report: Option<Report>,
    pub days: usize,
    pub agent: String,
    pub last_refresh: Option<Instant>,
    pub settings_open: bool,
}

impl App {
    pub fn new(dir: PathBuf, cfg: Config) -> Self {
        let mut app = App {
            dir,
            cfg,
            report: None,
            days: 30,
            agent: "all".to_string(),
            last_refresh: None,
            settings_open: false,
        };
        app.refresh();
        app
    }

    pub fn refresh(&mut self) {
        self.last_refresh = Some(Instant::now());
        let cache_path = self.dir.join("ai-token-stats-cache.db");
        self.report = Some(collect(&cache_path, &self.cfg, self.days, &self.agent));
    }
}

fn fmt_tokens(v: i64) -> String {
    if v >= 100_000_000 {
        format!("{:.2}亿", v as f64 / 100_000_000.0)
    } else if v >= 10_000 {
        format!("{:.2}万", v as f64 / 10_000.0)
    } else {
        v.to_string()
    }
}

fn fmt_percent(v: Option<f64>) -> String {
    match v {
        None => "无数据".to_string(),
        Some(p) => format!("{:.1}%", p * 100.0),
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(t) = self.last_refresh {
            if t.elapsed() >= Duration::from_secs(60) {
                self.refresh();
            }
        }
        ctx.request_repaint_after(Duration::from_secs(60));
        tray::poll_events(self);

        egui::CentralPanel::default().frame(egui::Frame::default().fill(
            egui::Color32::from_rgb(232, 242, 252),
        )).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("最近天数:");
                let mut days = self.days.to_string();
                egui::ComboBox::from_id_salt("days")
                    .selected_text(days.clone())
                    .show_ui(ui, |ui| {
                        for d in ["7", "14", "30", "90"] {
                            ui.selectable_value(&mut days, d.to_string(), d);
                        }
                    });
                if let Ok(v) = days.parse::<usize>() {
                    self.days = v;
                }
                ui.label("Agent:");
                let agents = ["all", "Codex", "ZCode", "Claude", "OpenCode"];
                let mut cur = self.agent.clone();
                egui::ComboBox::from_id_salt("agent")
                    .selected_text(if self.agent == "all" { "全部".to_string() } else { self.agent.clone() })
                    .show_ui(ui, |ui| {
                        for a in agents {
                            let label = if a == "all" { "全部" } else { a };
                            ui.selectable_value(&mut cur, a.to_string(), label);
                        }
                    });
                self.agent = cur;
                if ui.button("刷新").clicked() {
                    self.refresh();
                }
            });

            if let Some(rep) = &self.report {
                ui.horizontal_wrapped(|ui| {
                    let cards = [
                        (format!("最近 {} 天", rep.days), fmt_tokens(rep.totals.total)),
                        ("今日".to_string(), fmt_tokens(rep.today.total)),
                        ("总命中率".to_string(), fmt_percent(rep.totals.hit_rate)),
                        ("今日命中率".to_string(), fmt_percent(rep.today.hit_rate)),
                        ("今日上下文峰值".to_string(), fmt_percent(rep.today.max_usage_percent)),
                    ];
                    for (title, value) in cards {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.set_min_size(egui::vec2(150.0, 54.0));
                            ui.label(title);
                            ui.label(egui::RichText::new(value).size(16.0).strong().color(egui::Color32::from_rgb(20, 90, 220)));
                        });
                    }
                });
                ui.add_space(8.0);
                chart::draw_chart(ui, rep, &self.agent);
            }
        });

        if self.settings_open {
            settings::show_settings(ctx, self);
        }
    }
}
```

- [ ] **Step 2: 编译验证**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build -p ai-token-stats
```

Expected: 编译通过。

- [ ] **Step 3: 提交**

```powershell
git add crates/app/src/ui.rs
git commit -m "feat(app): dashboard controls, summary cards, refresh loop"
```

---

## Task 10: 堆叠柱状图 + 悬停 tooltip

**Files:**
- Modify: `D:\ai-token-stats\crates\app\src\chart.rs`

- [ ] **Step 1: 实现 chart.rs**

`D:\ai-token-stats\crates\app\src\chart.rs`（完整文件）：

```rust
use ai_token_stats_core::report::{DaySummary, Report};

const PALETTE: [egui::Color32; 7] = [
    egui::Color32::from_rgb(20, 120, 230),
    egui::Color32::from_rgb(0, 180, 150),
    egui::Color32::from_rgb(150, 100, 220),
    egui::Color32::from_rgb(240, 140, 30),
    egui::Color32::from_rgb(70, 170, 70),
    egui::Color32::from_rgb(230, 70, 120),
    egui::Color32::from_rgb(140, 140, 140),
];

pub fn draw_chart(ui: &mut egui::Ui, rep: &Report, agent: &str) {
    if rep.daily.is_empty() {
        return;
    }
    let keys: Vec<String> = if agent == "all" {
        rep.agents.clone()
    } else {
        rep.models.clone()
    };
    if keys.is_empty() {
        return;
    }
    let by_agent = agent == "all";

    let available = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(available.x.max(400.0), available.y.max(220.0)),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    let plot = rect.shrink2(egui::vec2(24.0, 16.0));
    let bottom = plot.bottom();
    let max_total = rep.daily.iter().map(|d| d.total).max().unwrap_or(1).max(10_000) as f64;
    let slot = plot.width() / rep.daily.len() as f32;
    let bar_w = (slot * 0.55).max(1.0);

    let mut hover_day: Option<&DaySummary> = None;
    if let Some(pos) = response.hover_pos() {
        let idx = ((pos.x - plot.left()) / slot).floor() as usize;
        if idx < rep.daily.len() && pos.x >= plot.left() && pos.x <= plot.right() {
            hover_day = Some(&rep.daily[idx]);
        }
    }

    for (i, day) in rep.daily.iter().enumerate() {
        let x = plot.left() + i as f32 * slot + (slot - bar_w) / 2.0;
        let mut cumulative = 0.0f32;
        for (ki, key) in keys.iter().enumerate() {
            let seg = if by_agent {
                day.by_agent.get(key).map(|s| s.total).unwrap_or(0)
            } else {
                day.by_model.get(key).map(|s| s.total).unwrap_or(0)
            };
            if seg <= 0 {
                continue;
            }
            let h = (seg as f64 / max_total * plot.height() as f64) as f32;
            let y0 = bottom - cumulative - h;
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(x, y0), egui::pos2(x + bar_w, bottom - cumulative)),
                0.0,
                PALETTE[ki % PALETTE.len()],
            );
            cumulative += h;
        }
        if rep.daily.len() <= 15 || i % 2 == 0 {
            painter.text(
                egui::pos2(plot.left() + i as f32 * slot + slot / 2.0, bottom + 4.0),
                egui::Align2::CENTER_TOP,
                day.date[5..].to_string(),
                egui::FontId::proportional(9.0),
                egui::Color32::from_rgb(60, 60, 60),
            );
        }
    }

    if let Some(day) = hover_day {
        egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), ui.id().with("chart-tip"), |ui| {
            ui.label(egui::RichText::new(&day.date).strong());
            ui.label(format!("总 token：{}", fmt(day.total)));
            ui.label(format!("输入：{} | 缓存：{}", fmt(day.input), fmt(day.cached)));
            ui.label(format!("输出：{} | 推理：{}", fmt(day.output), fmt(day.reasoning)));
            ui.label(format!("轮次：{} | 上下文：{}", day.turns, day.max_context_window.map(fmt).unwrap_or_else(|| "无数据".into())));
            ui.label(format!("使用率峰值：{} | 命中率：{}", pct(day.max_usage_percent), pct(day.hit_rate)));
            let subs: Vec<&(String, DaySummary)> = if by_agent {
                day.by_agent.iter().filter(|(_, s)| s.total > 0).collect()
            } else {
                day.by_model.iter().filter(|(_, s)| s.total > 0).collect()
            };
            for (k, s) in subs {
                ui.label(format!("{k}：{}", fmt(s.total)));
            }
        });
    }
}

fn fmt(v: i64) -> String {
    if v >= 100_000_000 {
        format!("{:.2}亿", v as f64 / 100_000_000.0)
    } else if v >= 10_000 {
        format!("{:.2}万", v as f64 / 10_000.0)
    } else {
        v.to_string()
    }
}

fn pct(v: Option<f64>) -> String {
    match v {
        None => "无数据".to_string(),
        Some(p) => format!("{:.1}%", p * 100.0),
    }
}
```

说明：与 Go 版布局差异（egui 自动布局替代手工几何）属预期，视觉信息一致即可。

- [ ] **Step 2: 编译验证**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build -p ai-token-stats
```

Expected: 编译通过。

- [ ] **Step 3: 提交**

```powershell
git add crates/app/src/chart.rs
git commit -m "feat(app): stacked daily chart with hover tooltip"
```

---

## Task 11: 托盘（菜单/双击/关闭隐藏/置前）

**Files:**
- Modify: `D:\ai-token-stats\crates\app\src\tray.rs`

- [ ] **Step 1: 实现 tray.rs**

`D:\ai-token-stats\crates\app\src\tray.rs`（完整文件）：

```rust
use crate::ui::App;
use egui::Context;
use std::sync::Arc;
use tray_icon::menu::{Menu, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

static DOUBLE_CLICK_MS: u128 = 500;

pub struct TrayState {
    pub _tray: TrayIcon,
    pub last_click: std::time::Instant,
    pub pending_action: std::sync::Mutex<Option<Action>>,
}

#[derive(Clone, Copy)]
pub enum Action {
    Open,
    Refresh,
    Rescan,
    Settings,
    Exit,
}

pub fn create_tray(ctx: Context) -> Arc<TrayState> {
    let open = MenuItem::new("打开面板", true, None);
    let refresh = MenuItem::new("刷新", true, None);
    let rescan = MenuItem::new("重新扫描路径", true, None);
    let settings = MenuItem::new("设置 Agent 路径…", true, None);
    let exit = MenuItem::new("退出", true, None);
    let menu = Menu::new();
    for item in [&open, &refresh, &rescan, &settings, &exit] {
        let _ = menu.append(item);
    }

    let rgba = make_icon_rgba();
    let icon = Icon::from_rgba(rgba, 32, 32).expect("icon");
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("AI Token 统计")
        .with_icon(icon)
        .build()
        .expect("tray");

    let state = Arc::new(TrayState {
        _tray: tray,
        last_click: std::time::Instant::now(),
        pending_action: std::sync::Mutex::new(None),
    });

    let s = state.clone();
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
                let action = if event.id == open.id() {
                    Some(Action::Open)
                } else if event.id == refresh.id() {
                    Some(Action::Refresh)
                } else if event.id == rescan.id() {
                    Some(Action::Rescan)
                } else if event.id == settings.id() {
                    Some(Action::Settings)
                } else if event.id == exit.id() {
                    Some(Action::Exit)
                } else {
                    None
                };
                if let Some(a) = action {
                    *s.pending_action.lock().unwrap() = Some(a);
                    ctx.request_repaint();
                }
            }
            if let Ok(event) = TrayIconEvent::receiver().try_recv() {
                if event.button == MouseButton::Left && event.button_state == MouseButtonState::Up {
                    let now = std::time::Instant::now();
                    let dbl = s.last_click.elapsed().as_millis() <= DOUBLE_CLICK_MS;
                    s.last_click = now;
                    if dbl {
                        *s.pending_action.lock().unwrap() = Some(Action::Open);
                        ctx.request_repaint();
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });
    state
}

fn make_icon_rgba() -> Vec<u8> {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let (r, g, b) = if (5..12).contains(&x) && (12..27).contains(&y) {
                (190, 220, 255)
            } else if (13..20).contains(&x) && (6..27).contains(&y) {
                (255, 255, 255)
            } else if (21..28).contains(&x) && (16..27).contains(&y) {
                (190, 220, 255)
            } else {
                (20, 90, 220)
            };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    rgba
}

pub fn poll_events(app: &mut App) {
    if let Some(state) = app.tray.as_ref() {
        let action = state.pending_action.lock().unwrap().take();
        match action {
            Some(Action::Open) => {
                app.ctx_send_visible(true);
            }
            Some(Action::Refresh) => app.refresh(),
            Some(Action::Rescan) => {
                crate::bootstrap::ensure_discovered_force(&app.cfg, &app.dir.join("config.json"));
                app.refresh();
            }
            Some(Action::Settings) => app.settings_open = true,
            Some(Action::Exit) => {
                app.ctx_send_close();
            }
            None => {}
        }
    }
}
```

`ui.rs` 需要配套改动：

```rust
// App 结构体追加字段：
//   pub tray: Option<Arc<crate::tray::TrayState>>,
// new() 里：
//   app.tray = Some(crate::tray::create_tray(egui_context));
// 注意：create_tray 需要 egui::Context，改为在 eframe 闭包中创建后注入：
//   let mut app = ...; app.tray = Some(tray::create_tray(cc.egui_ctx.clone()));
```

同步修改 `main.rs`：`let app = ui::App::new(dir, cfg);` 改为 `let mut app = ui::App::new(dir, cfg);`，并在 eframe 闭包内、`install_fonts` 之后追加：

```rust
app.tray = Some(tray::create_tray(cc.egui_ctx.clone()));
```

然后 `Ok(Box::new(app))`。`App::new` 中 `tray` 初始为 `None`。

`ui.rs` 追加两个方法：

```rust
pub fn ctx_send_visible(&mut self, visible: bool) {
    if visible {
        self.refresh();
    }
    // 通过 eframe 的 viewport 命令控制显示/最小化/聚焦
    // eframe 0.27 中需在 update 内发送；这里用 pending 标志，在 update 开头处理：
    self.pending_show = true;
}

pub fn ctx_send_close(&mut self) {
    self.pending_close = true;
}
```

`App` 结构体再追加 `pending_show: bool`、`pending_close: bool`；在 `update()` 开头：

```rust
if self.pending_show {
    self.pending_show = false;
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
}
if self.pending_close {
    self.pending_close = false;
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
}
// 关闭按钮 → 隐藏而非退出：
if ctx.input(|i| i.viewport().close_requested()) && !self.pending_close {
    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
}
```

`bootstrap.rs` 追加 `ensure_discovered_force(cfg, config_path)`：无视现有路径是否有效，对 4 个 Agent 全量重扫并写回（对应"重新扫描路径"）。

- [ ] **Step 2: 编译验证**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build -p ai-token-stats
```

Expected: 编译通过（若 tray-icon 事件 API 与 0.14 有出入，按编译错误微调字段名）。

- [ ] **Step 3: 提交**

```powershell
git add crates/app/src/tray.rs crates/app/src/ui.rs crates/app/src/bootstrap.rs
git commit -m "feat(app): tray menu, double-click open, hide-on-close"
```

---

## Task 12: 设置对话框

**Files:**
- Modify: `D:\ai-token-stats\crates\app\src\settings.rs`

- [ ] **Step 1: 实现 settings.rs**

`D:\ai-token-stats\crates\app\src\settings.rs`（完整文件）：

```rust
use ai_token_stats_core::config::{Agent, AgentPath, Config};
use ai_token_stats_core::discovery::validate_agent_path;
use std::path::Path;

use crate::ui::App;

struct Row {
    agent: Agent,
    label: &'static str,
    is_dir: bool,
}

const ROWS: [Row; 4] = [
    Row { agent: Agent::Codex, label: "Codex home 目录", is_dir: true },
    Row { agent: Agent::ZCode, label: "ZCode db.sqlite", is_dir: false },
    Row { agent: Agent::Claude, label: "Claude projects 目录", is_dir: true },
    Row { agent: Agent::OpenCode, label: "OpenCode opencode.db", is_dir: false },
];

pub fn show_settings(ctx: &egui::Context, app: &mut App) {
    let mut open = true;
    let mut save = false;
    let mut paths: Vec<(Agent, String)> = ROWS
        .iter()
        .map(|r| (r.agent, app.cfg.agents.get(&r.agent).map(|a| a.path.clone()).unwrap_or_default()))
        .collect();

    egui::Window::new("设置 Agent 路径")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            for (i, row) in ROWS.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(row.label);
                    ui.add(egui::TextEdit::singleline(&mut paths[i].1).desired_width(280.0));
                    if ui.button("浏览…").clicked() {
                        let picked = if row.is_dir {
                            rfd::FileDialog::new().set_title("选择路径").pick_folder()
                        } else {
                            rfd::FileDialog::new().set_title("选择路径").pick_file()
                        };
                        if let Some(p) = picked {
                            paths[i].1 = p.to_string_lossy().into_owned();
                        }
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.button("确定").clicked() {
                    save = true;
                }
                if ui.button("取消").clicked() {
                    open = false;
                }
            });
        });

    if save {
        open = false;
        let mut cfg = app.cfg.clone();
        for (agent, path) in &paths {
            if path.is_empty() {
                continue;
            }
            if !validate_agent_path(*agent, Path::new(path)) {
                egui::Window::new("路径无效").show(ctx, |ui| {
                    ui.label("路径不存在或不是有效数据源。");
                });
                return;
            }
            cfg.agents.insert(*agent, AgentPath {
                path: path.clone(),
                detected_at: chrono::Utc::now().to_rfc3339(),
            });
        }
        app.cfg = cfg;
        app.cfg.save(&app.dir.join("config.json")).ok();
        app.refresh();
    } else if !open {
        app.settings_open = false;
    }
}
```

说明：无效路径提示用 `egui::Window` 或 `egui::Modal` 均可；确保确定后关闭窗口并刷新。

- [ ] **Step 2: 编译验证**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build -p ai-token-stats
```

Expected: 编译通过。

- [ ] **Step 3: 提交**

```powershell
git add crates/app/src/settings.rs
git commit -m "feat(app): agent path settings dialog"
```

---

## Task 13: 收尾（build.ps1、README、移除 Go、验收）

**Files:**
- Create: `D:\ai-token-stats\build.ps1`
- Modify: `D:\ai-token-stats\README.md`
- Modify: `D:\ai-token-stats\.gitignore`
- Delete: Go 源文件（main.go、collector.go、cache.go、chart.go、paths.go、settings.go、*_test.go、go.mod、go.sum、app.ico、app.manifest、rsrc.syso）

- [ ] **Step 1: build.ps1**

```powershell
$ErrorActionPreference = "Stop"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin;$env:USERPROFILE\mingw64\bin;" + $env:PATH
cargo build --release -p ai-token-stats
Copy-Item target\release\ai-token-stats.exe .\ai-token-stats.exe -Force
Write-Host "Built ai-token-stats.exe"
```

- [ ] **Step 2: .gitignore**

```gitignore
# Rust
target/

# Runtime state, regenerated per machine
ai-token-stats.exe
config.json
ai-token-stats-cache.db
config.json.corrupt-*
```

- [ ] **Step 3: 删除 Go 文件并更新 README**

删除 Go 源文件（git rm）。README 重写为 Rust 版：功能说明、构建（`.\build.ps1`）、数据来源/自动发现、`-smoke`、托盘使用说明；保留 docs/ 下的设计文档引用。

- [ ] **Step 4: 全量验证**

```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy --workspace -- -D warnings
.\build.ps1
.\ai-token-stats.exe -smoke
```

Expected: 测试全过、clippy 无警告、exe 生成、smoke 输出 `SMOKE OK`。

手动验收清单：双击托盘打开/置前、关闭隐藏、菜单各项、设置路径保存后生效、重新扫描、每分钟自动刷新、中文字体渲染、图表悬停明细。

- [ ] **Step 5: 提交**

```powershell
git add -A
git commit -m "feat: complete rust rewrite, remove Go implementation"
```
