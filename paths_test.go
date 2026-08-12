package main

import (
	"database/sql"
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
