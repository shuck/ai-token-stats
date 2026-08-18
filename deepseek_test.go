package main

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"

	"github.com/klauspost/compress/zstd"
)

func writeDeepSeekSession(t *testing.T, path string, lines ...string) {
	t.Helper()
	var buf bytes.Buffer
	w, err := zstd.NewWriter(&buf)
	if err != nil {
		t.Fatal(err)
	}
	for _, l := range lines {
		if _, err := w.Write([]byte(l + "\n")); err != nil {
			t.Fatal(err)
		}
	}
	if err := w.Close(); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, buf.Bytes(), 0o644); err != nil {
		t.Fatal(err)
	}
}

func TestLoadDeepSeekRecordsIncremental(t *testing.T) {
	dir := t.TempDir()
	agentPaths[agentDeepSeek] = dir
	defer delete(agentPaths, agentDeepSeek)

	session := filepath.Join(dir, "sessions", "--proj--", "s1", "session.jsonl.zstd")
	base := []string{
		`{"type":"session","id":"s1","createdAt":1786950991034}`,
		`{"type":"request/context","data":{"contextWindow":1000000}}`,
		`{"type":"request/header","data":{"header":{"config":{"model":"mimo-v2.5"}}}}`,
		`{"type":"assistant/chunk","seq":1,"time":1786951000000,"data":{"chunk":{"type":"usage","usage":{"inputTokens":100,"outputTokens":10,"cacheReadTokens":50}}}}`,
		`{"type":"assistant/chunk","seq":2,"time":1786952000000,"data":{"chunk":{"type":"usage","usage":{"inputTokens":200,"outputTokens":20,"cacheReadTokens":100}}}}`,
	}
	writeDeepSeekSession(t, session, base...)

	records := loadDeepSeekRecords(0)
	if len(records) != 2 {
		t.Fatalf("expected 2 records, got %d", len(records))
	}
	if records[0].Model != "mimo-v2.5" {
		t.Fatalf("model: %s", records[0].Model)
	}
	if records[0].ContextWindow == nil || *records[0].ContextWindow != 1000000 {
		t.Fatal("context window should be set")
	}
	if records[0].Usage.Input != 150 { // 100 未缓存 + 50 缓存读取
		t.Fatalf("input should include cache read: %d", records[0].Usage.Input)
	}
	if records[0].Usage.Cached != 50 {
		t.Fatalf("cached: %d", records[0].Usage.Cached)
	}
	if records[0].Usage.Total != 160 {
		t.Fatalf("total: %d", records[0].Usage.Total)
	}
	if records[0].Key == records[1].Key {
		t.Fatal("record keys must be unique per event")
	}

	// 增量：追加一条新事件，since 用上次最大 ts，应只返回新事件且不覆盖旧记录
	extra := `{"type":"assistant/chunk","seq":3,"time":1786953000000,"data":{"chunk":{"type":"usage","usage":{"inputTokens":300,"outputTokens":30,"cacheReadTokens":150}}}}`
	writeDeepSeekSession(t, session, append(base, extra)...)

	records2 := loadDeepSeekRecords(1786952000001)
	if len(records2) != 1 {
		t.Fatalf("expected 1 incremental record, got %d", len(records2))
	}
	if records2[0].Usage.Input != 450 { // 300 未缓存 + 150 缓存读取
		t.Fatalf("incremental input: %d", records2[0].Usage.Input)
	}
}
