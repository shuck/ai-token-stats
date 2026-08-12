use crate::types::{Record, Usage};
use chrono::TimeZone;
use rusqlite::{params, Connection, OpenFlags};
use serde::Deserialize;
use std::path::Path;

pub fn shanghai() -> chrono::FixedOffset {
    chrono::FixedOffset::east_opt(8 * 3600).unwrap()
}

pub fn date_key(ms: i64) -> String {
    let dt = chrono::Utc
        .timestamp_millis_opt(ms)
        .unwrap()
        .with_timezone(&shanghai());
    dt.format("%Y-%m-%d").to_string()
}

#[derive(Debug, Default, Deserialize)]
pub struct ZCodeToken {
    pub total: i64,
    pub input: i64,
    pub output: i64,
    pub reasoning: i64,
    #[serde(default)]
    pub cache: ZCodeCache,
}

#[derive(Debug, Default, Deserialize)]
pub struct ZCodeCache {
    pub read: i64,
    pub write: i64,
}

#[derive(Debug, Deserialize)]
pub struct ZCodeMessage {
    #[serde(rename = "modelID")]
    pub model_id: String,
    #[serde(default)]
    pub model: ZCodeModel,
    #[serde(default)]
    pub tokens: ZCodeToken,
}

#[derive(Debug, Default, Deserialize)]
pub struct ZCodeModel {
    #[serde(rename = "modelID")]
    pub model_id: String,
}

pub fn load_zcode_records(path: &Path, since: i64) -> rusqlite::Result<(Vec<Record>, i64)> {
    if !path.exists() {
        return Ok((Vec::new(), since));
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare(
        "SELECT id, time_created, time_updated, data FROM message
         WHERE json_extract(data, '$.tokens') IS NOT NULL AND time_updated > ?1",
    )?;
    let rows = stmt.query_map(params![since], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut records = Vec::new();
    let mut max_updated = since;
    for row in rows {
        let (id, created, updated, raw) = row?;
        if updated > max_updated {
            max_updated = updated;
        }
        let Ok(msg) = serde_json::from_str::<ZCodeMessage>(&raw) else {
            continue;
        };
        let model = if !msg.model_id.is_empty() {
            msg.model_id
        } else if !msg.model.model_id.is_empty() {
            msg.model.model_id
        } else {
            "unknown".to_string()
        };
        records.push(Record {
            thread_id: "zcode".into(),
            agent: "ZCode".into(),
            model,
            key: id,
            path: "zcode".into(),
            ts: created,
            date: date_key(created),
            usage: Usage {
                input: msg.tokens.input,
                cached: msg.tokens.cache.read,
                cache_write: msg.tokens.cache.write,
                output: msg.tokens.output,
                reasoning: msg.tokens.reasoning,
                total: msg.tokens.total,
            },
            context_window: None,
        });
    }
    Ok((records, max_updated))
}
