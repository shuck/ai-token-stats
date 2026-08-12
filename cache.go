package main

import (
	"database/sql"
	"os"
	"path/filepath"
	"strings"
)

type fileMeta struct {
	Size  int64
	Mtime int64
}

func openCache() (*sql.DB, error) {
	db, err := sql.Open("sqlite", cacheDB())
	if err != nil {
		return nil, err
	}
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS records_v2 (
			source TEXT NOT NULL,
			record_key TEXT NOT NULL,
			path TEXT NOT NULL DEFAULT '',
			agent TEXT NOT NULL,
			model TEXT NOT NULL,
			ts INTEGER NOT NULL,
			date TEXT NOT NULL,
			input INTEGER NOT NULL DEFAULT 0,
			cached INTEGER NOT NULL DEFAULT 0,
			cache_write INTEGER NOT NULL DEFAULT 0,
			output INTEGER NOT NULL DEFAULT 0,
			reasoning INTEGER NOT NULL DEFAULT 0,
			total INTEGER NOT NULL DEFAULT 0,
			PRIMARY KEY (source, record_key)
		);
		CREATE INDEX IF NOT EXISTS idx_records_v2_date ON records_v2(date);
		CREATE INDEX IF NOT EXISTS idx_records_v2_agent ON records_v2(agent);
		CREATE TABLE IF NOT EXISTS source_files (
			source TEXT NOT NULL,
			path TEXT NOT NULL,
			size INTEGER NOT NULL,
			mtime INTEGER NOT NULL,
			PRIMARY KEY (source, path)
		);
		CREATE TABLE IF NOT EXISTS source_watermarks (
			source TEXT PRIMARY KEY,
			value INTEGER NOT NULL
		);
	`)
	if err != nil {
		db.Close()
		return nil, err
	}
	return db, nil
}

func currentFileMap(roots ...string) map[string]fileMeta {
	result := map[string]fileMeta{}
	for _, root := range roots {
		filepath.Walk(root, func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return nil
			}
			if info.IsDir() || !strings.HasSuffix(strings.ToLower(info.Name()), ".jsonl") {
				return nil
			}
			result[path] = fileMeta{Size: info.Size(), Mtime: info.ModTime().UnixNano()}
			return nil
		})
	}
	return result
}

func changedFilePaths(db *sql.DB, source string, roots ...string) (map[string]bool, error) {
	current := currentFileMap(roots...)
	changed := map[string]bool{}
	stored := map[string]fileMeta{}
	rows, err := db.Query(`SELECT path, size, mtime FROM source_files WHERE source = ?`, source)
	if err != nil {
		return nil, err
	}
	for rows.Next() {
		var path string
		var meta fileMeta
		if rows.Scan(&path, &meta.Size, &meta.Mtime) == nil {
			stored[path] = meta
		}
	}
	rows.Close()

	for path, meta := range current {
		old, ok := stored[path]
		if !ok || old.Size != meta.Size || old.Mtime != meta.Mtime {
			changed[path] = true
		}
	}
	for path := range stored {
		if _, ok := current[path]; !ok {
			if _, err := db.Exec(`DELETE FROM records_v2 WHERE source = ? AND path = ?`, source, path); err != nil {
				return nil, err
			}
			if _, err := db.Exec(`DELETE FROM source_files WHERE source = ? AND path = ?`, source, path); err != nil {
				return nil, err
			}
		}
	}
	for path, meta := range current {
		if _, err := db.Exec(
			`INSERT INTO source_files(source, path, size, mtime) VALUES (?, ?, ?, ?)
			 ON CONFLICT(source, path) DO UPDATE SET size = excluded.size, mtime = excluded.mtime`,
			source, path, meta.Size, meta.Mtime); err != nil {
			return nil, err
		}
	}
	return changed, nil
}

func deleteRecordsByPath(db *sql.DB, source, path string) error {
	_, err := db.Exec(`DELETE FROM records_v2 WHERE source = ? AND path = ?`, source, path)
	return err
}

func insertRecords(db *sql.DB, source string, records []record) error {
	tx, err := db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()
	stmt, err := tx.Prepare(`
		INSERT OR REPLACE INTO records_v2
			(source, record_key, path, agent, model, ts, date, input, cached, cache_write, output, reasoning, total)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`)
	if err != nil {
		return err
	}
	defer stmt.Close()
	for _, r := range records {
		if _, err := stmt.Exec(source, r.Key, r.Path, r.Agent, r.Model, r.Ts, r.Date,
			r.Usage.Input, r.Usage.Cached, r.Usage.CacheWrite,
			r.Usage.Output, r.Usage.Reasoning, r.Usage.Total); err != nil {
			return err
		}
	}
	return tx.Commit()
}

func getWatermark(db *sql.DB, source string) int64 {
	var value int64
	err := db.QueryRow(`SELECT value FROM source_watermarks WHERE source = ?`, source).Scan(&value)
	if err != nil {
		return 0
	}
	return value
}

func setWatermark(db *sql.DB, source string, value int64) error {
	_, err := db.Exec(
		`INSERT INTO source_watermarks(source, value) VALUES (?, ?)
		 ON CONFLICT(source) DO UPDATE SET value = excluded.value`,
		source, value)
	return err
}

func ensureCached(agent string) error {
	db, err := openCache()
	if err != nil {
		return err
	}
	defer db.Close()

	if agent == agentAll || agent == agentCodex {
		changed, err := changedFilePaths(db, agentCodex, sessionsRoot(), archivedRoot())
		if err != nil {
			return err
		}
		logsSince := getWatermark(db, "codex-logs")
		records, maxLogsTs := loadCodexRecords(changed, logsSince)
		for path := range changed {
			if err := deleteRecordsByPath(db, agentCodex, path); err != nil {
				return err
			}
		}
		if err := insertRecords(db, agentCodex, records); err != nil {
			return err
		}
		if maxLogsTs > logsSince {
			if err := setWatermark(db, "codex-logs", maxLogsTs); err != nil {
				return err
			}
		}
	}

	if agent == agentAll || agent == agentClaude {
		changed, err := changedFilePaths(db, agentClaude, claudeRoot())
		if err != nil {
			return err
		}
		records := loadClaudeRecords(changed)
		for path := range changed {
			if err := deleteRecordsByPath(db, agentClaude, path); err != nil {
				return err
			}
		}
		if err := insertRecords(db, agentClaude, records); err != nil {
			return err
		}
	}

	if agent == agentAll || agent == agentZcode {
		since := getWatermark(db, "zcode-updated-ts")
		records, maxUpdated := loadZCodeRecords(zcodeDB(), since)
		if err := insertRecords(db, agentZcode, records); err != nil {
			return err
		}
		if maxUpdated > since {
			if err := setWatermark(db, "zcode-updated-ts", maxUpdated); err != nil {
				return err
			}
		}
	}

	if agent == agentAll || agent == agentOpenCode {
		since := getWatermark(db, "opencode-ts")
		records := loadOpenCodeRecords(since)
		maxTs := since
		for _, r := range records {
			if r.Ts > maxTs {
				maxTs = r.Ts
			}
		}
		if err := insertRecords(db, agentOpenCode, records); err != nil {
			return err
		}
		if maxTs > since {
			if err := setWatermark(db, "opencode-ts", maxTs); err != nil {
				return err
			}
		}
	}

	return nil
}

func loadCacheRecords(agent string) []record {
	db, err := openCache()
	if err != nil {
		return nil
	}
	defer db.Close()

	query := `SELECT agent, model, ts, date, input, cached, cache_write, output, reasoning, total FROM records_v2`
	var args []any
	if agent != agentAll {
		query += ` WHERE agent = ?`
		args = append(args, agent)
	}
	rows, err := db.Query(query, args...)
	if err != nil {
		return nil
	}
	defer rows.Close()

	records := []record{}
	for rows.Next() {
		var r record
		if err := rows.Scan(&r.Agent, &r.Model, &r.Ts, &r.Date,
			&r.Usage.Input, &r.Usage.Cached, &r.Usage.CacheWrite,
			&r.Usage.Output, &r.Usage.Reasoning, &r.Usage.Total); err != nil {
			continue
		}
		records = append(records, r)
	}
	return records
}
