package main

import (
	"database/sql"
	"path/filepath"
	"testing"
)

func TestLoadOpenCodeRecordsMapsCacheRead(t *testing.T) {
	dir := t.TempDir()
	dbPath := filepath.Join(dir, "opencode.db")
	db, err := sql.Open("sqlite", dbPath)
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	if _, err := db.Exec(`CREATE TABLE session (
		id TEXT PRIMARY KEY,
		time_updated INTEGER,
		model TEXT,
		tokens_input INTEGER,
		tokens_output INTEGER,
		tokens_reasoning INTEGER,
		tokens_cache_read INTEGER,
		tokens_cache_write INTEGER
	)`); err != nil {
		t.Fatal(err)
	}
	if _, err := db.Exec(
		`INSERT INTO session(id, time_updated, model, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write)
		 VALUES ('s1', 1787222111819, 'gpt-4o', 100, 10, 5, 200, 0)`); err != nil {
		t.Fatal(err)
	}

	agentPaths[agentOpenCode] = dbPath
	defer delete(agentPaths, agentOpenCode)

	records := loadOpenCodeRecords(0)
	if len(records) != 1 {
		t.Fatalf("expected 1 record, got %d", len(records))
	}
	r := records[0]
	if r.Usage.Input != 300 { // 100 未缓存 + 200 缓存读取
		t.Fatalf("input should include cache read: %d", r.Usage.Input)
	}
	if r.Usage.Cached != 200 {
		t.Fatalf("cached: %d", r.Usage.Cached)
	}
	if r.Usage.Total != 310 {
		t.Fatalf("total: %d", r.Usage.Total)
	}
}
