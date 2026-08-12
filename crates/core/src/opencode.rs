use crate::types::{Record, Usage};
use crate::zcode::date_key;
use rusqlite::{params, Connection, OpenFlags};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct OpenCodeModel {
    pub id: String,
}

pub fn load_opencode_records(path: &Path, since: i64) -> rusqlite::Result<(Vec<Record>, i64)> {
    if !path.exists() {
        return Ok((Vec::new(), since));
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare(
        "SELECT id, time_updated, model, COALESCE(tokens_input,0), COALESCE(tokens_output,0),
                COALESCE(tokens_reasoning,0), COALESCE(tokens_cache_read,0), COALESCE(tokens_cache_write,0)
         FROM session WHERE time_updated > ?1",
    )?;
    let rows = stmt.query_map(params![since], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, i64>(5)?,
            r.get::<_, i64>(6)?,
            r.get::<_, i64>(7)?,
        ))
    })?;
    let mut records = Vec::new();
    let mut max_updated = since;
    for row in rows {
        let (id, ts, model, input, output, reasoning, cache_read, cache_write) = row?;
        if ts > max_updated {
            max_updated = ts;
        }
        if input == 0 && output == 0 {
            continue;
        }
        let model_name = if model.trim_start().starts_with('{') {
            serde_json::from_str::<OpenCodeModel>(&model)
                .ok()
                .map(|m| m.id)
                .filter(|id| !id.is_empty())
                .unwrap_or_else(|| "unknown".to_string())
        } else if model.is_empty() {
            "unknown".to_string()
        } else {
            model
        };
        records.push(Record {
            thread_id: "opencode".into(),
            agent: "OpenCode".into(),
            model: model_name,
            key: id,
            path: "opencode".into(),
            ts,
            date: date_key(ts),
            usage: Usage {
                input,
                cached: cache_read,
                cache_write,
                output,
                reasoning,
                total: input + output,
            },
            context_window: None,
        });
    }
    Ok((records, max_updated))
}
