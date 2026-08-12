package main

import (
	"database/sql"
	"os"
	"path/filepath"
	"testing"
	"time"
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
	// Isolate default-location candidates (e.g. the real ~/.claude/projects
	// on this machine) so discovery must go through the scan roots.
	t.Setenv("USERPROFILE", t.TempDir())

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
	if err := os.MkdirAll(filepath.Dir(zdb), 0o755); err != nil {
		t.Fatal(err)
	}
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
