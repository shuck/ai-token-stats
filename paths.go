package main

import (
	"database/sql"
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
