package main

import (
	"database/sql"
	"path/filepath"
	"testing"
	"time"
)

func TestLoadZCodeRecordsUpdated(t *testing.T) {
	path := filepath.Join(t.TempDir(), "db.sqlite")
	db, err := sql.Open("sqlite", path)
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	if _, err := db.Exec(`CREATE TABLE message (
		id TEXT PRIMARY KEY,
		time_created INTEGER,
		time_updated INTEGER,
		data TEXT
	)`); err != nil {
		t.Fatal(err)
	}

	sh := time.FixedZone("CST", 8*3600)
	created := time.Date(2026, 8, 12, 9, 0, 0, 0, sh).UnixMilli()
	placeholder := `{"modelID":"gpt-4o","tokens":{"total":0,"input":0,"output":0,"reasoning":0,"cache":{"read":0,"write":0}}}`
	if _, err := db.Exec(
		`INSERT INTO message(id, time_created, time_updated, data) VALUES(?, ?, ?, ?)`,
		"m1", created, created, placeholder); err != nil {
		t.Fatal(err)
	}

	records, maxUpdated := loadZCodeRecords(path, 0)
	if len(records) != 1 {
		t.Fatalf("expected 1 record, got %d", len(records))
	}
	if records[0].Usage.Total != 0 {
		t.Fatalf("expected placeholder total 0, got %d", records[0].Usage.Total)
	}
	if records[0].Ts != created {
		t.Fatalf("expected Ts from time_created, got %d", records[0].Ts)
	}
	if records[0].Date != "2026-08-12" {
		t.Fatalf("expected date 2026-08-12, got %q", records[0].Date)
	}
	if maxUpdated != created {
		t.Fatalf("expected maxUpdated %d, got %d", created, maxUpdated)
	}

	updated := created + 60000
	final := `{"modelID":"gpt-4o","tokens":{"total":271565,"input":270000,"output":1565,"reasoning":0,"cache":{"read":250000,"write":0}}}`
	if _, err := db.Exec(
		`UPDATE message SET data = ?, time_updated = ? WHERE id = 'm1'`,
		final, updated); err != nil {
		t.Fatal(err)
	}

	records, maxUpdated = loadZCodeRecords(path, created)
	if len(records) != 1 {
		t.Fatalf("expected 1 updated record, got %d", len(records))
	}
	if records[0].Usage.Total != 271565 {
		t.Fatalf("expected updated total 271565, got %d", records[0].Usage.Total)
	}
	if records[0].Ts != created {
		t.Fatalf("expected Ts unchanged from time_created, got %d", records[0].Ts)
	}
	if records[0].Date != "2026-08-12" {
		t.Fatalf("expected date still 2026-08-12, got %q", records[0].Date)
	}
	if maxUpdated != updated {
		t.Fatalf("expected maxUpdated %d, got %d", updated, maxUpdated)
	}

	records, _ = loadZCodeRecords(path, updated)
	if len(records) != 0 {
		t.Fatalf("expected no records past watermark, got %d", len(records))
	}
}

func TestAddRecordHitRates(t *testing.T) {
	day := newDaySummary("2026-08-12")
	addRecord(&day, record{
		Agent: "ZCode",
		Model: "gpt-4o",
		Date:  "2026-08-12",
		Usage: usage{Input: 100, Cached: 40, Total: 100},
	})
	addRecord(&day, record{
		Agent: "ZCode",
		Model: "gpt-4o",
		Date:  "2026-08-12",
		Usage: usage{Input: 50, Cached: 10, Total: 50},
	})
	if day.HitRate == nil {
		t.Fatal("day hit rate should be set")
	}
	if day.ByAgent["ZCode"] == nil || day.ByAgent["ZCode"].HitRate == nil {
		t.Fatal("agent hit rate should be set")
	}
	if day.ByModel["gpt-4o"] == nil || day.ByModel["gpt-4o"].HitRate == nil {
		t.Fatal("model hit rate should be set")
	}
	am := day.ByAgent["ZCode"].ByModel["gpt-4o"]
	if am == nil || am.HitRate == nil {
		t.Fatal("agent-model hit rate should be set")
	}
	want := 50.0 / 150.0 // (40+10)/(100+50)
	if *am.HitRate != want {
		t.Fatalf("expected hit rate %v, got %v", want, *am.HitRate)
	}
}
