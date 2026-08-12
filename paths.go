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
