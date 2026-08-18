package main

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
	case agentDeepSeek:
		if !info.IsDir() {
			return false
		}
		_, err := os.Stat(filepath.Join(path, "storages", "session_projcache.json"))
		return err == nil
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
	case agentDeepSeek:
		if h := os.Getenv("DSH_HOME"); h != "" {
			return []string{h}
		}
		if home != "" {
			return []string{filepath.Join(home, ".dsh")}
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

var allAgents = []string{agentCodex, agentZcode, agentClaude, agentOpenCode, agentDeepSeek}

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
	delete(cfg.Agents, "DeepSeek") // 迁移：旧命名残留键
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
func opencodeDB() string      { return agentPaths[agentOpenCode] }
func deepSeekHome() string    { return agentPaths[agentDeepSeek] }
func cacheDB() string         { return cacheDBPath }
