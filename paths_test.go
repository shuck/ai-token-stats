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
