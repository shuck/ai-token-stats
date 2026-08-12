use crate::types::Record;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub struct Cache {
    conn: Connection,
}

impl Cache {
    pub fn open(path: &Path) -> rusqlite::Result<Cache> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
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
            "#,
        )?;
        Ok(Cache { conn })
    }

    pub fn get_watermark(&self, source: &str) -> i64 {
        self.conn
            .query_row(
                "SELECT value FROM source_watermarks WHERE source = ?1",
                params![source],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten()
            .unwrap_or(0)
    }

    pub fn set_watermark(&mut self, source: &str, value: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO source_watermarks(source, value) VALUES (?1, ?2)
             ON CONFLICT(source) DO UPDATE SET value = excluded.value",
            params![source, value],
        )?;
        Ok(())
    }

    pub fn changed_file_paths(
        &mut self,
        source: &str,
        roots: &[PathBuf],
    ) -> rusqlite::Result<BTreeSet<PathBuf>> {
        let current = current_file_map(roots);
        let mut stored: BTreeMap<PathBuf, (i64, i64)> = BTreeMap::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT path, size, mtime FROM source_files WHERE source = ?1")?;
            let rows = stmt.query_map(params![source], |r| {
                Ok((
                    PathBuf::from(r.get::<_, String>(0)?),
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })?;
            for row in rows {
                if let Ok((p, size, mtime)) = row {
                    stored.insert(p, (size, mtime));
                }
            }
        }
        let mut changed = BTreeSet::new();
        for (path, meta) in &current {
            let changed_f = match stored.get(path) {
                None => true,
                Some((s, m)) => *s != meta.0 || *m != meta.1,
            };
            if changed_f {
                changed.insert(path.clone());
            }
        }
        for path in stored.keys() {
            if !current.contains_key(path) {
                self.conn.execute(
                    "DELETE FROM records_v2 WHERE source = ?1 AND path = ?2",
                    params![source, path.to_string_lossy()],
                )?;
                self.conn.execute(
                    "DELETE FROM source_files WHERE source = ?1 AND path = ?2",
                    params![source, path.to_string_lossy()],
                )?;
            }
        }
        for (path, meta) in &current {
            self.conn.execute(
                "INSERT INTO source_files(source, path, size, mtime) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(source, path) DO UPDATE SET size = excluded.size, mtime = excluded.mtime",
                params![source, path.to_string_lossy(), meta.0, meta.1],
            )?;
        }
        Ok(changed)
    }

    pub fn delete_records_by_path(
        &mut self,
        source: &str,
        path: &Path,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM records_v2 WHERE source = ?1 AND path = ?2",
            params![source, path.to_string_lossy()],
        )?;
        Ok(())
    }

    pub fn insert_records(&mut self, source: &str, records: &[Record]) -> rusqlite::Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO records_v2
                    (source, record_key, path, agent, model, ts, date, input, cached, cache_write, output, reasoning, total)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )?;
            for r in records {
                stmt.execute(params![
                    source,
                    r.key,
                    r.path,
                    r.agent,
                    r.model,
                    r.ts,
                    r.date,
                    r.usage.input,
                    r.usage.cached,
                    r.usage.cache_write,
                    r.usage.output,
                    r.usage.reasoning,
                    r.usage.total,
                ])?;
            }
        }
        tx.commit()
    }

    pub fn load_cache_records(&self, agent: &str) -> rusqlite::Result<Vec<Record>> {
        let sql = if agent == "all" {
            "SELECT agent, model, ts, date, input, cached, cache_write, output, reasoning, total FROM records_v2".to_string()
        } else {
            "SELECT agent, model, ts, date, input, cached, cache_write, output, reasoning, total FROM records_v2 WHERE agent = ?1".to_string()
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let mapper = |r: &rusqlite::Row<'_>| record_from_row(r);
        let rows = if agent == "all" {
            stmt.query_map([], mapper)?
        } else {
            stmt.query_map(params![agent], mapper)?
        };
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

fn record_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Record> {
    Ok(Record {
        thread_id: String::new(),
        agent: r.get(0)?,
        model: r.get(1)?,
        key: String::new(),
        path: String::new(),
        ts: r.get(2)?,
        date: r.get(3)?,
        usage: crate::types::Usage {
            input: r.get(4)?,
            cached: r.get(5)?,
            cache_write: r.get(6)?,
            output: r.get(7)?,
            reasoning: r.get(8)?,
            total: r.get(9)?,
        },
        context_window: None,
    })
}

fn current_file_map(roots: &[PathBuf]) -> BTreeMap<PathBuf, (i64, i64)> {
    let mut map = BTreeMap::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let walker = walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok());
        for e in walker {
            if e.file_type().is_file()
                && e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| x.eq_ignore_ascii_case("jsonl"))
                    .unwrap_or(false)
            {
                if let Ok(m) = fs::metadata(e.path()) {
                    let size = m.len() as i64;
                    let mtime = m
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_nanos() as i64)
                        .unwrap_or(0);
                    map.insert(e.path().to_path_buf(), (size, mtime));
                }
            }
        }
    }
    map
}

pub fn insert_records(cache: &mut Cache, source: &str, records: &[Record]) -> rusqlite::Result<()> {
    cache.insert_records(source, records)
}

pub fn load_cache_records(cache: &Cache, agent: &str) -> rusqlite::Result<Vec<Record>> {
    cache.load_cache_records(agent)
}
