# Agent 数据路径动态发现 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 AI Token 统计工具的硬编码 Agent 数据路径改为首次自动发现 + 缓存到 exe 旁 `config.json`，路径失效时自动重扫更新，并提供手动指定兜底。

**Architecture:** 新增 `paths.go` 集中管理路径：`config` 模型（JSON 读写，原子写入）、目录解析（exe 目录 / `%APPDATA%` 回退）、路径校验、按 Agent 的发现逻辑（环境变量 → 默认位置 → 受限目录扫描）。`collector.go` / `cache.go` 中的路径常量改为同名函数，从运行时全局 `agentPaths` 读取；`main.go` 在启动时初始化配置、刷新前校验路径，失效时后台重扫；新增 `settings.go` 提供手动指定路径的对话框。

**Tech Stack:** Go 1.26、lxn/walk（GUI）、modernc.org/sqlite（纯 Go SQLite）、golang.org/x/sys/windows（逻辑盘枚举）。

**环境前置：** Go 1.26.5 位于 `D:\ai-data\go-sdk\go\bin\go.exe`，不在 PATH 中。本计划所有 Go 命令均用完整路径形式：

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' <args>
```

Git 操作因沙箱限制需要提权执行（`sandbox_permissions: require_escalated`）。

---

## Task 1: 配置模型与读写（paths.go 骨架）

**Files:**
- Create: `D:\ai-token-stats\paths.go`
- Create: `D:\ai-token-stats\paths_test.go`

- [ ] **Step 1: 写失败测试**

在 `D:\ai-token-stats\paths_test.go` 写入：

```go
package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestConfigRoundTrip(t *testing.T) {
	path := filepath.Join(t.TempDir(), "config.json")
	cfg := newConfig()
	cfg.Agents[agentCodex] = agentPath{Path: `D:\codex`, DetectedAt: "2026-08-12T00:00:00+08:00"}
	if err := saveConfig(path, cfg); err != nil {
		t.Fatal(err)
	}
	got, err := loadConfig(path)
	if err != nil {
		t.Fatal(err)
	}
	if got.Agents[agentCodex].Path != `D:\codex` {
		t.Fatalf("unexpected path: %q", got.Agents[agentCodex].Path)
	}
}

func TestLoadConfigMissing(t *testing.T) {
	cfg, err := loadConfig(filepath.Join(t.TempDir(), "nope.json"))
	if err != nil {
		t.Fatal(err)
	}
	if len(cfg.Agents) != 0 {
		t.Fatalf("expected empty agents, got %v", cfg.Agents)
	}
}

func TestLoadConfigCorrupt(t *testing.T) {
	path := filepath.Join(t.TempDir(), "config.json")
	if err := os.WriteFile(path, []byte("{not json"), 0o644); err != nil {
		t.Fatal(err)
	}
	if _, err := loadConfig(path); err == nil {
		t.Fatal("expected error for corrupt config")
	}
}

func TestPathExists(t *testing.T) {
	dir := t.TempDir()
	if !pathExists(dir) {
		t.Fatal("dir should exist")
	}
	if pathExists(filepath.Join(dir, "missing")) {
		t.Fatal("missing should not exist")
	}
}
```

注意：测试引用了 `agentCodex`，该常量当前已存在于 `collector.go`。

- [ ] **Step 2: 运行测试确认失败**

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' test ./... -run 'TestConfig|TestLoadConfig|TestPathExists' -v
```

Expected: FAIL，编译错误 `undefined: newConfig` / `undefined: saveConfig` / `undefined: loadConfig` / `undefined: pathExists`。

- [ ] **Step 3: 实现最小代码**

在 `D:\ai-token-stats\paths.go` 写入：

```go
package main

import (
	"encoding/json"
	"os"
	"path/filepath"
)

type agentPath struct {
	Path       string `json:"path"`
	DetectedAt string `json:"detected_at"`
}

type config struct {
	Agents map[string]agentPath `json:"agents"`
}

func newConfig() *config {
	return &config{Agents: map[string]agentPath{}}
}

func loadConfig(path string) (*config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return newConfig(), nil
		}
		return nil, err
	}
	cfg := newConfig()
	if err := json.Unmarshal(data, cfg); err != nil {
		return nil, err
	}
	if cfg.Agents == nil {
		cfg.Agents = map[string]agentPath{}
	}
	return cfg, nil
}

func saveConfig(path string, cfg *config) error {
	data, err := json.MarshalIndent(cfg, "", "  ")
	if err != nil {
		return err
	}
	tmp, err := os.CreateTemp(filepath.Dir(path), ".config-*.tmp")
	if err != nil {
		return err
	}
	tmpName := tmp.Name()
	if _, err := tmp.Write(data); err != nil {
		tmp.Close()
		os.Remove(tmpName)
		return err
	}
	if err := tmp.Close(); err != nil {
		os.Remove(tmpName)
		return err
	}
	return os.Rename(tmpName, path)
}

func pathExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}
```

- [ ] **Step 4: 运行测试确认通过**

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' test ./... -run 'TestConfig|TestLoadConfig|TestPathExists' -v
```

Expected: 4 个测试全部 PASS。

- [ ] **Step 5: 提交**

```powershell
git add paths.go paths_test.go
git commit -m "feat: add config model and persistence"
```

---

## Task 2: Agent 路径校验

**Files:**
- Modify: `D:\ai-token-stats\paths.go`
- Modify: `D:\ai-token-stats\paths_test.go`

- [ ] **Step 1: 写失败测试**

在 `D:\ai-token-stats\paths_test.go` 末尾追加：

```go
func TestValidateAgentPath(t *testing.T) {
	root := t.TempDir()

	codex := filepath.Join(root, "codex")
	if err := os.MkdirAll(filepath.Join(codex, "sessions"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(codex, "archived_sessions"), 0o755); err != nil {
		t.Fatal(err)
	}
	if !validateAgentPath(agentCodex, codex) {
		t.Fatal("codex home should be valid")
	}
	if validateAgentPath(agentCodex, root) {
		t.Fatal("root should not be a codex home")
	}

	claude := filepath.Join(root, ".claude", "projects")
	if err := os.MkdirAll(filepath.Join(claude, "s1"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(claude, "s1", "a.jsonl"), []byte("{}"), 0o644); err != nil {
		t.Fatal(err)
	}
	if !validateAgentPath(agentClaude, claude) {
		t.Fatal("claude projects should be valid")
	}
	if validateAgentPath(agentClaude, root) {
		t.Fatal("root should not be a claude projects dir")
	}
}

func TestValidateZCodeAndOpenCode(t *testing.T) {
	dir := t.TempDir()
	zdb := filepath.Join(dir, "db.sqlite")
	createTestDB(t, zdb, `CREATE TABLE message (id TEXT, data TEXT)`)
	if !validateAgentPath(agentZcode, zdb) {
		t.Fatal("zcode db should be valid")
	}

	odb := filepath.Join(dir, "opencode.db")
	createTestDB(t, odb, `CREATE TABLE session (id TEXT, tokens_input INTEGER)`)
	if !validateAgentPath(agentOpenCode, odb) {
		t.Fatal("opencode db should be valid")
	}
}

func createTestDB(t *testing.T, path, ddl string) {
	t.Helper()
	db, err := sql.Open("sqlite", path)
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	if _, err := db.Exec(ddl); err != nil {
		t.Fatal(err)
	}
}
```

同时把 `paths_test.go` 的 import 改为：

```go
import (
	"database/sql"
	"os"
	"path/filepath"
	"testing"
)
```

- [ ] **Step 2: 运行测试确认失败**

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' test ./... -run 'TestValidateAgentPath|TestValidateZCodeAndOpenCode' -v
```

Expected: FAIL，编译错误 `undefined: validateAgentPath`、`undefined: hasMessageTable`、`undefined: hasSessionTable`、`undefined: isClaudeProjects`。

- [ ] **Step 3: 实现校验逻辑**

在 `D:\ai-token-stats\paths.go` 的 `pathExists` 函数后追加：

```go
func validateAgentPath(agent, path string) bool {
	if path == "" {
		return false
	}
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	switch agent {
	case agentCodex:
		if !info.IsDir() {
			return false
		}
		if _, err := os.Stat(filepath.Join(path, "logs_2.sqlite")); err == nil {
			return true
		}
		s1, e1 := os.Stat(filepath.Join(path, "sessions"))
		s2, e2 := os.Stat(filepath.Join(path, "archived_sessions"))
		return e1 == nil && s1.IsDir() && e2 == nil && s2.IsDir()
	case agentZcode:
		if info.IsDir() {
			return false
		}
		return hasMessageTable(path)
	case agentClaude:
		if !info.IsDir() {
			return false
		}
		return isClaudeProjects(path)
	case agentOpenCode:
		if info.IsDir() {
			return false
		}
		return hasSessionTable(path)
	}
	return false
}

func isClaudeProjects(dir string) bool {
	return filepath.Base(dir) == "projects" && filepath.Base(filepath.Dir(dir)) == ".claude"
}

func hasMessageTable(path string) bool {
	db, err := sql.Open("sqlite", "file:"+path+"?mode=ro&_pragma=query_only(1)")
	if err != nil {
		return false
	}
	defer db.Close()
	var n int
	if err := db.QueryRow(`SELECT count(*) FROM sqlite_master WHERE type='table' AND name='message'`).Scan(&n); err != nil || n == 0 {
		return false
	}
	var cols int
	err = db.QueryRow(`SELECT count(*) FROM pragma_table_info('message') WHERE name='data'`).Scan(&cols)
	return err == nil && cols > 0
}

func hasSessionTable(path string) bool {
	db, err := sql.Open("sqlite", "file:"+path+"?mode=ro&_pragma=query_only(1)")
	if err != nil {
		return false
	}
	defer db.Close()
	var n int
	if err := db.QueryRow(`SELECT count(*) FROM sqlite_master WHERE type='table' AND name='session'`).Scan(&n); err != nil || n == 0 {
		return false
	}
	var cols int
	err = db.QueryRow(`SELECT count(*) FROM pragma_table_info('session') WHERE name='tokens_input'`).Scan(&cols)
	return err == nil && cols > 0
}
```

把 `paths.go` 的 import 改为：

```go
import (
	"database/sql"
	"encoding/json"
	"os"
	"path/filepath"
)
```

- [ ] **Step 4: 运行测试确认通过**

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' test ./... -run 'TestValidateAgentPath|TestValidateZCodeAndOpenCode' -v
```

Expected: 2 个测试全部 PASS。

- [ ] **Step 5: 提交**

```powershell
git add paths.go paths_test.go
git commit -m "feat: add agent path validation"
```

---

## Task 3: Agent 路径发现

**Files:**
- Modify: `D:\ai-token-stats\paths.go`
- Modify: `D:\ai-token-stats\paths_test.go`

- [ ] **Step 1: 写失败测试**

在 `D:\ai-token-stats\paths_test.go` 末尾追加：

```go
func TestKnownCandidatesCodexEnv(t *testing.T) {
	dir := t.TempDir()
	t.Setenv("CODEX_HOME", dir)
	got := knownCandidates(agentCodex)
	if len(got) != 1 || got[0] != dir {
		t.Fatalf("unexpected candidates: %v", got)
	}
}

func TestDiscoverAgentPath(t *testing.T) {
	root := t.TempDir()

	codex := filepath.Join(root, "ai-data", "codex")
	if err := os.MkdirAll(filepath.Join(codex, "sessions"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(codex, "archived_sessions"), 0o755); err != nil {
		t.Fatal(err)
	}

	claude := filepath.Join(root, ".claude", "projects")
	if err := os.MkdirAll(filepath.Join(claude, "s1"), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(claude, "s1", "a.jsonl"), []byte("{}"), 0o644); err != nil {
		t.Fatal(err)
	}

	zdb := filepath.Join(root, "zdata", "db.sqlite")
	createTestDB(t, zdb, `CREATE TABLE message (id TEXT, data TEXT)`)
	odb := filepath.Join(root, "opencode.db")
	createTestDB(t, odb, `CREATE TABLE session (id TEXT, tokens_input INTEGER)`)

	roots := []string{root}
	if got := discoverAgentPath(agentCodex, roots); got != codex {
		t.Fatalf("codex: got %q want %q", got, codex)
	}
	if got := discoverAgentPath(agentClaude, roots); got != claude {
		t.Fatalf("claude: got %q want %q", got, claude)
	}
	if got := discoverAgentPath(agentZcode, roots); got != zdb {
		t.Fatalf("zcode: got %q want %q", got, zdb)
	}
	if got := discoverAgentPath(agentOpenCode, roots); got != odb {
		t.Fatalf("opencode: got %q want %q", got, odb)
	}
}

func TestDiscoverPrefersNewest(t *testing.T) {
	root := t.TempDir()
	old := filepath.Join(root, "old")
	newd := filepath.Join(root, "new")
	for _, d := range []string{old, newd} {
		if err := os.MkdirAll(filepath.Join(d, "sessions"), 0o755); err != nil {
			t.Fatal(err)
		}
		if err := os.MkdirAll(filepath.Join(d, "archived_sessions"), 0o755); err != nil {
			t.Fatal(err)
		}
	}
	oldT := time.Now().Add(-48 * time.Hour)
	if err := os.Chtimes(old, oldT, oldT); err != nil {
		t.Fatal(err)
	}
	newT := time.Now().Add(-1 * time.Hour)
	if err := os.Chtimes(newd, newT, newT); err != nil {
		t.Fatal(err)
	}
	if got := discoverAgentPath(agentCodex, []string{root}); got != newd {
		t.Fatalf("expected newest %q, got %q", newd, got)
	}
}
```

把 `paths_test.go` 的 import 改为：

```go
import (
	"database/sql"
	"os"
	"path/filepath"
	"testing"
	"time"
)
```

- [ ] **Step 2: 运行测试确认失败**

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' test ./... -run 'TestKnownCandidates|TestDiscover' -v
```

Expected: FAIL，编译错误 `undefined: knownCandidates`、`undefined: discoverAgentPath`。

- [ ] **Step 3: 实现发现逻辑**

在 `D:\ai-token-stats\paths.go` 的 `hasSessionTable` 函数后追加：

```go
const (
	maxScanDepth    = 4
	maxScanDirs     = 20000
	maxScanDuration = 20 * time.Second
)

func knownCandidates(agent string) []string {
	home := os.Getenv("USERPROFILE")
	switch agent {
	case agentCodex:
		if h := os.Getenv("CODEX_HOME"); h != "" {
			return []string{h}
		}
		if home != "" {
			return []string{filepath.Join(home, ".codex")}
		}
	case agentZcode:
		if d := os.Getenv("ZCODE_DATA"); d != "" {
			return []string{filepath.Join(d, "cli", "db", "db.sqlite")}
		}
	case agentClaude:
		if home != "" {
			return []string{filepath.Join(home, ".claude", "projects")}
		}
	case agentOpenCode:
		if home != "" {
			return []string{filepath.Join(home, ".local", "share", "opencode", "opencode.db")}
		}
	}
	return nil
}

func scanRoots() []string {
	roots := map[string]bool{}
	for _, d := range fixedDrives() {
		roots[d] = true
	}
	for _, env := range []string{"USERPROFILE", "APPDATA", "LOCALAPPDATA"} {
		if v := os.Getenv(env); v != "" {
			roots[v] = true
		}
	}
	out := make([]string, 0, len(roots))
	for r := range roots {
		out = append(out, r)
	}
	sort.Strings(out)
	return out
}

func fixedDrives() []string {
	bits, err := windows.GetLogicalDrives()
	if err != nil {
		return nil
	}
	var drives []string
	for i := 0; i < 26; i++ {
		if bits&(1<<uint(i)) != 0 {
			drives = append(drives, string(rune('A'+i))+":\\")
		}
	}
	return drives
}

var skipDirs = map[string]bool{
	"$Recycle.Bin":              true,
	"$RECYCLE.BIN":              true,
	"System Volume Information": true,
	"Windows":                   true,
	"Program Files":             true,
	"Program Files (x86)":       true,
}

func discoverAgentPath(agent string, roots []string) string {
	for _, p := range knownCandidates(agent) {
		if validateAgentPath(agent, p) {
			return p
		}
	}
	best := ""
	var bestMtime time.Time
	deadline := time.Now().Add(maxScanDuration)
	for _, root := range roots {
		info, err := os.Stat(root)
		if err != nil || !info.IsDir() {
			continue
		}
		visited := 0
		_ = filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
			if err != nil {
				return filepath.SkipDir
			}
			if !d.IsDir() {
				if matchAgentFile(agent, path) {
					fi, e := d.Info()
					if e == nil && (best == "" || fi.ModTime().After(bestMtime)) {
						best = path
						bestMtime = fi.ModTime()
					}
				}
				return nil
			}
			visited++
			if visited > maxScanDirs || time.Now().After(deadline) {
				return filepath.SkipDir
			}
			rel, e := filepath.Rel(root, path)
			if e != nil {
				return filepath.SkipDir
			}
			if depth := len(strings.Split(rel, string(os.PathSeparator))); depth > maxScanDepth {
				return filepath.SkipDir
			}
			if path != root && skipDirs[d.Name()] {
				return filepath.SkipDir
			}
			if matchAgentDir(agent, path) {
				fi, e := d.Info()
				if e == nil && (best == "" || fi.ModTime().After(bestMtime)) {
					best = path
					bestMtime = fi.ModTime()
				}
			}
			return nil
		})
	}
	return best
}

func matchAgentFile(agent, path string) bool {
	name := strings.ToLower(filepath.Base(path))
	switch agent {
	case agentZcode:
		return name == "db.sqlite" && hasMessageTable(path)
	case agentOpenCode:
		return name == "opencode.db" && hasSessionTable(path)
	}
	return false
}

func matchAgentDir(agent, path string) bool {
	switch agent {
	case agentCodex:
		if _, err := os.Stat(filepath.Join(path, "logs_2.sqlite")); err == nil {
			return true
		}
		s1, e1 := os.Stat(filepath.Join(path, "sessions"))
		s2, e2 := os.Stat(filepath.Join(path, "archived_sessions"))
		return e1 == nil && s1.IsDir() && e2 == nil && s2.IsDir()
	case agentClaude:
		return isClaudeProjects(path)
	}
	return false
}
```

把 `paths.go` 的 import 改为：

```go
import (
	"database/sql"
	"encoding/json"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"golang.org/x/sys/windows"
)
```

- [ ] **Step 4: 运行测试确认通过**

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' test ./... -run 'TestKnownCandidates|TestDiscover' -v
```

Expected: 3 个测试全部 PASS。

- [ ] **Step 5: 提交**

```powershell
git add paths.go paths_test.go
git commit -m "feat: add agent path discovery"
```

---

## Task 4: 目录解析与 collector/cache 接线

**Files:**
- Modify: `D:\ai-token-stats\paths.go`
- Modify: `D:\ai-token-stats\collector.go`
- Modify: `D:\ai-token-stats\cache.go`

- [ ] **Step 1: 修改 collector.go 常量块**

`D:\ai-token-stats\collector.go` 顶部当前为：

```go
const (
	codexHome     = `D:\ai-data\codex`
	sessionsRoot  = codexHome + `\sessions`
	archivedRoot  = codexHome + `\archived_sessions`
	logsDB        = codexHome + `\logs_2.sqlite`
	stateDB       = codexHome + `\state_5.sqlite`
	zcodeDB       = `D:\ai-data\zcode-data\cli\db\db.sqlite`
	claudeRoot    = `C:\Users\zc\.claude\projects`
	opencodeDB    = `C:\Users\zc\.local\share\opencode\opencode.db`
	shanghaiZone  = "Asia/Shanghai"
	agentAll      = "all"
	agentCodex    = "Codex"
	agentZcode    = "ZCode"
	agentClaude   = "Claude"
	agentOpenCode = "OpenCode"
)
```

替换为：

```go
const (
	shanghaiZone  = "Asia/Shanghai"
	agentAll      = "all"
	agentCodex    = "Codex"
	agentZcode    = "ZCode"
	agentClaude   = "Claude"
	agentOpenCode = "OpenCode"
)
```

- [ ] **Step 2: 修改 cache.go**

删除 `D:\ai-token-stats\cache.go` 顶部：

```go
const cacheDB = `D:\ai-data\codex\codex-usage-tool\ai-token-stats-cache.db`
```

并把 `openCache` 中的：

```go
	db, err := sql.Open("sqlite", cacheDB)
```

改为：

```go
	db, err := sql.Open("sqlite", cacheDB())
```

- [ ] **Step 3: 在 paths.go 追加目录解析、初始化与路径 getter**

在 `D:\ai-token-stats\paths.go` 文件末尾追加：

```go
var allAgents = []string{agentCodex, agentZcode, agentClaude, agentOpenCode}

// Runtime locations, set once by initPaths.
var (
	appDir      string
	configPath  string
	cacheDBPath string
	agentPaths  = map[string]string{}
)

func initPaths() (*config, error) {
	dir, err := resolveAppDir()
	if err != nil {
		return nil, err
	}
	appDir = dir
	configPath = filepath.Join(appDir, "config.json")
	cacheDBPath = filepath.Join(appDir, "ai-token-stats-cache.db")

	cfg, err := loadConfig(configPath)
	if err != nil {
		backup := fmt.Sprintf("%s.corrupt-%d", configPath, time.Now().Unix())
		if renameErr := os.Rename(configPath, backup); renameErr == nil {
			fmt.Fprintf(os.Stderr, "config corrupt, backed up to %s\n", backup)
		}
		cfg = newConfig()
	}
	for _, agent := range allAgents {
		if p := cfg.Agents[agent].Path; pathExists(p) {
			agentPaths[agent] = p
		} else {
			agentPaths[agent] = ""
		}
	}
	return cfg, nil
}

func resolveAppDir() (string, error) {
	exe, err := os.Executable()
	if err != nil {
		return "", err
	}
	dir := filepath.Dir(exe)
	if testWritable(dir) == nil {
		return dir, nil
	}
	fallback := filepath.Join(os.Getenv("APPDATA"), "ai-token-stats")
	if err := os.MkdirAll(fallback, 0o755); err != nil {
		return "", err
	}
	if err := testWritable(fallback); err != nil {
		return "", fmt.Errorf("no writable location: %s and %s", dir, fallback)
	}
	return fallback, nil
}

func testWritable(dir string) error {
	f, err := os.CreateTemp(dir, ".write-test-*")
	if err != nil {
		return err
	}
	name := f.Name()
	if err := f.Close(); err != nil {
		return err
	}
	return os.Remove(name)
}

// Data source getters, backed by the runtime-discovered config.
func codexHome() string    { return agentPaths[agentCodex] }
func sessionsRoot() string { return filepath.Join(codexHome(), "sessions") }
func archivedRoot() string { return filepath.Join(codexHome(), "archived_sessions") }
func logsDB() string       { return filepath.Join(codexHome(), "logs_2.sqlite") }
func stateDB() string      { return filepath.Join(codexHome(), "state_5.sqlite") }
func zcodeDB() string      { return agentPaths[agentZcode] }
func claudeRoot() string   { return agentPaths[agentClaude] }
func opencodeDB() string   { return agentPaths[agentOpenCode] }
func cacheDB() string      { return cacheDBPath }
```

`paths.go` 的 import 改为：

```go
import (
	"database/sql"
	"encoding/json"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"golang.org/x/sys/windows"
)
```

- [ ] **Step 4: 在 paths_test.go 追加 testWritable 测试**

```go
func TestTestWritable(t *testing.T) {
	if err := testWritable(t.TempDir()); err != nil {
		t.Fatal(err)
	}
}
```

- [ ] **Step 5: 编译验证**

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' build ./...
```

Expected: 无输出，退出码 0。

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' vet ./...
```

Expected: 无输出，退出码 0。

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' test ./... -v
```

Expected: 全部 PASS（含新增 TestTestWritable）。

- [ ] **Step 6: 提交**

```powershell
git add paths.go paths_test.go collector.go cache.go
git commit -m "refactor: resolve data paths at runtime"
```

---

## Task 5: 主程序集成（自动重扫 + 托盘入口）

**Files:**
- Modify: `D:\ai-token-stats\main.go`

- [ ] **Step 1: 修改 app 结构体**

`D:\ai-token-stats\main.go` 的 `type app struct` 末尾（`lastClick time.Time` 之后）追加两个字段：

```go
	cfg        *config
	scanning   bool
```

- [ ] **Step 2: 修改 main()**

当前 `main()` 中：

```go
	defer windows.CloseHandle(mutex)

	hold := false
	a := &app{}
```

改为：

```go
	defer windows.CloseHandle(mutex)

	cfg, err := initPaths()
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	hold := false
	a := &app{cfg: cfg}
```

- [ ] **Step 3: refresh() 开头加路径校验**

当前 `refresh()`：

```go
func (a *app) refresh() {
	if a.agent == "" {
		a.agent = agentAll
	}
	r := collect(a.days, a.agent)
```

改为：

```go
func (a *app) refresh() {
	a.ensurePaths()
	if a.agent == "" {
		a.agent = agentAll
	}
	r := collect(a.days, a.agent)
```

- [ ] **Step 4: 追加 ensurePaths 与 scanAll 方法**

在 `refresh()` 函数后追加：

```go
func (a *app) ensurePaths() {
	needScan := false
	for _, agent := range allAgents {
		if !pathExists(agentPaths[agent]) {
			needScan = true
			break
		}
	}
	if !needScan {
		return
	}
	if a.smoke {
		a.scanAll(false)
		return
	}
	if a.scanning {
		return
	}
	a.scanning = true
	go func() {
		defer func() { a.scanning = false }()
		changed := a.scanAll(false)
		if a.mw != nil {
			a.mw.Synchronize(func() {
				if changed && a.ni != nil {
					_ = a.ni.ShowMessage("AI Token 统计", "Agent 数据路径已自动更新。")
				}
				a.refresh()
			})
		}
	}()
}

func (a *app) scanAll(force bool) bool {
	changed := false
	roots := scanRoots()
	for _, agent := range allAgents {
		if !force && pathExists(agentPaths[agent]) {
			continue
		}
		if p := discoverAgentPath(agent, roots); p != "" {
			if p != agentPaths[agent] {
				agentPaths[agent] = p
				a.cfg.Agents[agent] = agentPath{Path: p, DetectedAt: time.Now().Format(time.RFC3339)}
				changed = true
			}
		}
	}
	if changed {
		if err := saveConfig(configPath, a.cfg); err != nil {
			fmt.Fprintln(os.Stderr, "save config:", err)
		}
	}
	return changed
}
```

- [ ] **Step 5: 托盘菜单新增两项**

当前 `run()` 中：

```go
	refreshAction := walk.NewAction()
	_ = refreshAction.SetText("刷新")
	refreshAction.Triggered().Attach(a.refresh)
	exitAction := walk.NewAction()
```

改为：

```go
	refreshAction := walk.NewAction()
	_ = refreshAction.SetText("刷新")
	refreshAction.Triggered().Attach(a.refresh)
	rescanAction := walk.NewAction()
	_ = rescanAction.SetText("重新扫描路径")
	rescanAction.Triggered().Attach(func() {
		if a.scanning {
			return
		}
		a.scanning = true
		go func() {
			defer func() { a.scanning = false }()
			changed := a.scanAll(true)
			if a.mw != nil {
				a.mw.Synchronize(func() {
					if a.ni != nil {
						if changed {
							_ = a.ni.ShowMessage("AI Token 统计", "Agent 数据路径已更新。")
						} else {
							_ = a.ni.ShowMessage("AI Token 统计", "未发现新的 Agent 数据路径。")
						}
					}
					a.refresh()
				})
			}
		}()
	})
	settingsAction := walk.NewAction()
	_ = settingsAction.SetText("设置 Agent 路径…")
	settingsAction.Triggered().Attach(func() { a.showSettingsDialog() })
	exitAction := walk.NewAction()
```

当前 `run()` 中：

```go
	if err := menu.Add(refreshAction); err != nil {
		return err
	}
	if err := menu.Add(exitAction); err != nil {
```

改为：

```go
	if err := menu.Add(refreshAction); err != nil {
		return err
	}
	if err := menu.Add(rescanAction); err != nil {
		return err
	}
	if err := menu.Add(settingsAction); err != nil {
		return err
	}
	if err := menu.Add(exitAction); err != nil {
```

- [ ] **Step 6: 编译与测试**

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' build ./...
& 'D:\ai-data\go-sdk\go\bin\go.exe' vet ./...
& 'D:\ai-data\go-sdk\go\bin\go.exe' test ./...
```

Expected: build/vet 无输出退出码 0；测试全部 PASS。

注意：本任务引用的 `a.showSettingsDialog()` 由「设置对话框」任务先行实现并提交。执行顺序：先完成设置对话框任务，再执行本任务。

- [ ] **Step 7: 提交**

```powershell
git add main.go
git commit -m "feat: auto re-detect agent paths in app"
```

---

## Task 6: 设置对话框（settings.go）

**Files:**
- Create: `D:\ai-token-stats\settings.go`

- [ ] **Step 1: 创建 settings.go**

写入完整文件：

```go
package main

import (
	"time"

	"github.com/lxn/walk"
)

type pathRow struct {
	label *walk.Label
	edit  *walk.LineEdit
	agent string
	isDir bool
}

func (a *app) showSettingsDialog() {
	if a.mw == nil {
		return
	}
	dlg, err := walk.NewDialogWithFixedSize(a.mw)
	if err != nil {
		return
	}
	_ = dlg.SetTitle("设置 Agent 路径")
	if err := dlg.SetClientSize(walk.Size{Width: 640, Height: 280}); err != nil {
		return
	}
	root := walk.NewVBoxLayout()
	if err := dlg.SetLayout(root); err != nil {
		return
	}

	var rows []*pathRow
	for _, def := range []struct {
		agent string
		label string
		isDir bool
	}{
		{agentCodex, "Codex home 目录", true},
		{agentZcode, "ZCode db.sqlite", false},
		{agentClaude, "Claude projects 目录", true},
		{agentOpenCode, "OpenCode opencode.db", false},
	} {
		comp, err := walk.NewComposite(dlg)
		if err != nil {
			return
		}
		h := walk.NewHBoxLayout()
		_ = comp.SetLayout(h)
		lbl, err := walk.NewLabel(comp)
		if err != nil {
			return
		}
		_ = lbl.SetText(def.label)
		edit, err := walk.NewLineEdit(comp)
		if err != nil {
			return
		}
		_ = edit.SetText(agentPaths[def.agent])
		_ = h.SetStretchFactor(edit, 3)
		browse, err := walk.NewPushButton(comp)
		if err != nil {
			return
		}
		_ = browse.SetText("浏览…")
		r := &pathRow{label: lbl, edit: edit, agent: def.agent, isDir: def.isDir}
		browse.Clicked().Attach(func() {
			r.pickPath(a.mw)
		})
		rows = append(rows, r)
	}

	btnComp, err := walk.NewComposite(dlg)
	if err != nil {
		return
	}
	hb := walk.NewHBoxLayout()
	_ = btnComp.SetLayout(hb)
	okBtn, err := walk.NewPushButton(btnComp)
	if err != nil {
		return
	}
	_ = okBtn.SetText("确定")
	okBtn.Clicked().Attach(func() {
		for _, r := range rows {
			p := r.edit.Text()
			if p == "" {
				continue
			}
			if !validateAgentPath(r.agent, p) {
				walk.MsgBox(dlg, "路径无效", r.label.Text()+" 不存在或不是有效数据源。", walk.MsgBoxIconError)
				return
			}
			agentPaths[r.agent] = p
			a.cfg.Agents[r.agent] = agentPath{Path: p, DetectedAt: time.Now().Format(time.RFC3339)}
		}
		if err := saveConfig(configPath, a.cfg); err != nil {
			walk.MsgBox(dlg, "保存失败", err.Error(), walk.MsgBoxIconError)
			return
		}
		dlg.Accept()
		a.refresh()
	})
	cancelBtn, err := walk.NewPushButton(btnComp)
	if err != nil {
		return
	}
	_ = cancelBtn.SetText("取消")
	cancelBtn.Clicked().Attach(func() {
		dlg.Cancel()
	})
	_ = dlg.Run()
}

func (r *pathRow) pickPath(owner walk.Form) {
	var dlg walk.FileDialog
	dlg.Title = "选择路径"
	dlg.FilePath = r.edit.Text()
	if r.isDir {
		if accepted, _ := dlg.ShowBrowseFolder(owner); accepted && dlg.FilePath != "" {
			_ = r.edit.SetText(dlg.FilePath)
		}
		return
	}
	if accepted, _ := dlg.ShowOpen(owner); accepted && dlg.FilePath != "" {
		_ = r.edit.SetText(dlg.FilePath)
	}
}
```

- [ ] **Step 2: 编译与测试**

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' build ./...
& 'D:\ai-data\go-sdk\go\bin\go.exe' vet ./...
& 'D:\ai-data\go-sdk\go\bin\go.exe' test ./...
```

Expected: build/vet 无输出退出码 0；测试全部 PASS。

- [ ] **Step 3: 提交**

```powershell
git add settings.go
git commit -m "feat: add agent path settings dialog"
```

---

## Task 7: 更新 README

**Files:**
- Modify: `D:\ai-token-stats\README.md`

- [ ] **Step 1: 更新"数据来源"章节**

把 README 中：

```markdown
## 数据来源

数据路径为源码中硬编码的本机路径，按 Agent 汇总：

| Agent | 来源 |
| --- | --- |
| Codex | `D:\ai-data\codex\sessions` 和 `D:\ai-data\codex\archived_sessions` 下的 JSONL 会话文件，以及 `logs_2.sqlite`、`state_5.sqlite` |
| ZCode | `D:\ai-data\zcode-data\cli\db\db.sqlite` |
| Claude Code | `C:\Users\zc\.claude\projects` 下的 JSONL 会话文件 |
| OpenCode | `C:\Users\zc\.local\share\opencode\opencode.db` |

首次运行时会在 `D:\ai-data\codex\codex-usage-tool\ai-token-stats-cache.db` 自动创建增量缓存数据库，之后只读取发生变化的文件。
```

替换为：

```markdown
## 数据来源

各 Agent 数据路径由程序自动发现：

1. 环境变量：Codex 查 `CODEX_HOME`，ZCode 查 `ZCODE_DATA`。
2. 默认位置：`~/.codex`、`~/.claude/projects`、`~/.local/share/opencode/opencode.db`。
3. 受限目录扫描（深度 ≤ 4、限时限量）：按特征识别 Codex（`logs_2.sqlite` 或 `sessions`+`archived_sessions`）、ZCode（含 `message` 表的 `db.sqlite`）、Claude（`.claude\projects`）、OpenCode（含 `session` 表的 `opencode.db`）。

发现结果保存在 exe 同目录的 `config.json` 中。缓存的路径失效时，程序自动重新发现并更新；仍找不到时可通过托盘菜单「设置 Agent 路径…」手动指定。「重新扫描路径」可强制重扫全部 Agent。

工具自身的增量缓存数据库（`ai-token-stats-cache.db`）与 `config.json` 一起放在 exe 同目录（exe 目录不可写时回退到 `%APPDATA%\ai-token-stats\`）。
```

- [ ] **Step 2: 更新"使用"章节**

在 README 使用章节末尾追加：

```markdown
5. 右键托盘图标可选择「重新扫描路径」强制重扫，或「设置 Agent 路径…」手动指定各 Agent 数据源。
```

- [ ] **Step 3: 更新"说明"章节**

把 README 中：

```markdown
## 说明

- 各 Agent 的数据路径均为本机硬编码路径，换机器使用需修改 `collector.go` 顶部的常量。
- 模型归属通过会话元数据或日志匹配得出，无法识别时记为 `unknown`。
```

替换为：

```markdown
## 说明

- 各 Agent 数据路径自动发现并缓存到 exe 同目录的 `config.json`，无需手动配置；路径失效会自动重扫更新。
- 旧路径仍存在时程序会继续使用旧路径，此时可用托盘菜单「重新扫描路径」强制重扫。
- 模型归属通过会话元数据或日志匹配得出，无法识别时记为 `unknown`。
```

- [ ] **Step 4: 提交**

```powershell
git add README.md
git commit -m "docs: update README for dynamic paths"
```

---

## Task 8: 全量验证

**Files:** 无（只运行命令）

- [ ] **Step 1: 单元测试全量**

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' test ./... -v
```

Expected: 全部 PASS。

- [ ] **Step 2: 构建**

```powershell
& 'D:\ai-data\go-sdk\go\bin\go.exe' build -ldflags "-H windowsgui -s -w" -o ai-token-stats.exe
```

Expected: 退出码 0，生成 `ai-token-stats.exe`。

- [ ] **Step 3: 冒烟运行（可选，首次可能较慢）**

```powershell
& '.\ai-token-stats.exe' -smoke
```

Expected: 输出 `SMOKE OK: ...` 后退出；首次运行会扫描磁盘（每根最多 20 秒），并生成 exe 同目录的 `config.json`。

- [ ] **Step 4: 手动验证（可选）**

1. 运行 exe，确认托盘出现，双击打开面板且图表有数据。
2. 把某 Agent 目录改名后运行，观察自动重扫并更新 `config.json`。
3. 删除全部数据源后运行，程序不崩溃，托盘提示后可手动指定。
4. 通过「设置 Agent 路径…」手动指定后正常采集。

- [ ] **Step 5: 提交构建产物（如有变化）**

```powershell
git add ai-token-stats.exe
git commit -m "build: rebuild exe with dynamic path support"
```
