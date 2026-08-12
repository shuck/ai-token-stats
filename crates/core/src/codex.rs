use crate::types::{Record, Usage};
use crate::zcode::date_key;
use regex::Regex;
use rusqlite::{params, Connection, OpenFlags};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct RawEvent {
    #[serde(rename = "type")]
    pub type_: String,
    pub timestamp: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct TokenUsageJson {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub cache_write_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Deserialize)]
pub struct TokenCountInfo {
    pub last_token_usage: Option<TokenUsageJson>,
    pub total_token_usage: Option<TokenUsageJson>,
    pub model_context_window: Option<i64>,
}

pub fn load_codex_records(
    sessions: &[PathBuf],
    changed: Option<&BTreeSet<PathBuf>>,
) -> Vec<Record> {
    let files: Vec<PathBuf> = match changed {
        None => walk_jsonl(sessions),
        Some(set) if set.is_empty() => Vec::new(),
        Some(set) => set
            .iter()
            .filter(|p| {
                p.extension()
                    .map(|e| e.to_string_lossy().to_lowercase() == "jsonl")
                    .unwrap_or(false)
            })
            .cloned()
            .collect(),
    };
    let mut records = Vec::new();
    for f in files {
        parse_rollout_file(&f, &mut records);
    }
    records
}

fn walk_jsonl(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        for e in walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            if e.file_type().is_file()
                && e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| x.eq_ignore_ascii_case("jsonl"))
                    .unwrap_or(false)
            {
                out.push(e.path().to_path_buf());
            }
        }
    }
    out
}

fn parse_rollout_file(path: &Path, records: &mut Vec<Record>) {
    let Ok(f) = fs::File::open(path) else { return };
    let mut thread_id = String::new();
    for line in BufReader::new(f).lines().map_while(|l| l.ok()) {
        let Ok(ev) = serde_json::from_str::<RawEvent>(&line) else {
            continue;
        };
        if ev.type_ == "session_meta" {
            if let Some(id) = ev.payload.get("session_id").and_then(|v| v.as_str()) {
                thread_id = id.to_string();
            }
            continue;
        }
        if ev.type_ != "event_msg" {
            continue;
        }
        let Some(mtype) = ev.payload.get("type").and_then(|v| v.as_str()) else {
            continue;
        };
        if mtype != "token_count" {
            continue;
        }
        if thread_id.is_empty() {
            continue;
        }
        let Ok(info) = serde_json::from_value::<TokenCountInfo>(
            ev.payload.get("info").cloned().unwrap_or(serde_json::Value::Null),
        ) else {
            continue;
        };
        let Some(u) = info.last_token_usage.or(info.total_token_usage) else {
            continue;
        };
        let ts = parse_rfc3339_ms(&ev.timestamp);
        if ts == 0 {
            continue;
        }
        let total = if u.total_tokens == 0 {
            u.input_tokens + u.output_tokens
        } else {
            u.total_tokens
        };
        records.push(Record {
            thread_id: thread_id.clone(),
            agent: "Codex".into(),
            model: "unknown".to_string(),
            key: format!("{thread_id}:{ts}:{}:{}", u.input_tokens, u.output_tokens),
            path: path.to_string_lossy().into_owned(),
            ts,
            date: date_key(ts),
            usage: Usage {
                input: u.input_tokens,
                cached: u.cached_input_tokens,
                cache_write: u.cache_write_input_tokens,
                output: u.output_tokens,
                reasoning: u.reasoning_output_tokens,
                total,
            },
            context_window: info.model_context_window,
        });
    }
}

pub fn parse_rfc3339_ms(s: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|t| t.timestamp_millis())
        .unwrap_or(0)
}

pub fn load_thread_models(state_db: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if !state_db.exists() {
        return out;
    }
    let Ok(conn) = Connection::open_with_flags(state_db, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return out;
    };
    let Ok(mut stmt) = conn.prepare("SELECT id, model FROM threads WHERE model IS NOT NULL AND model != ''")
    else {
        return out;
    };
    if let Ok(rows) = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    }) {
        for row in rows.flatten() {
            out.insert(row.0, row.1);
        }
    }
    out
}

pub fn load_log_models(logs_db: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if !logs_db.exists() {
        return out;
    }
    let Ok(conn) = Connection::open_with_flags(logs_db, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return out;
    };
    let Ok(mut stmt) = conn.prepare(
        "SELECT thread_id, feedback_log_body FROM logs
         WHERE thread_id IS NOT NULL AND feedback_log_body LIKE '%model=%'",
    ) else {
        return out;
    };
    let model_re = Regex::new(r"model=([A-Za-z0-9_.:-]+)").unwrap();
    if let Ok(rows) = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    }) {
        for row in rows.flatten() {
            if out.contains_key(&row.0) {
                continue;
            }
            if let Some(c) = model_re.captures(&row.1) {
                if let Some(m) = c.get(1) {
                    out.insert(row.0, m.as_str().to_string());
                }
            }
        }
    }
    out
}

pub fn load_log_fallback(
    logs_db: &Path,
    state_db: &Path,
    since: i64,
) -> (Vec<Record>, i64) {
    let mut records = Vec::new();
    if !logs_db.exists() {
        return (records, since);
    }
    let Ok(conn) = Connection::open_with_flags(logs_db, OpenFlags::SQLITE_OPEN_READ_ONLY) else {
        return (records, since);
    };
    let Ok(mut stmt) = conn.prepare(
        "SELECT ts, thread_id, feedback_log_body FROM logs
         WHERE feedback_log_body LIKE '%codex.turn.token_usage.input_tokens%' AND ts > ?1
         ORDER BY ts",
    ) else {
        return (records, since);
    };
    let token_usage_re = Regex::new(r"codex\.turn\.token_usage\.([a-z_]+)=(\d+)").unwrap();
    let turn_id_re = Regex::new(r"turn\.id=([A-Za-z0-9_-]+)").unwrap();
    let turn_id_re2 = Regex::new(r"turn_id=([A-Za-z0-9_-]+)").unwrap();
    let context_limit_re = Regex::new(r"full_context_window_limit=Some\((\d+)\)").unwrap();
    let model_re = Regex::new(r"model=([A-Za-z0-9_.:-]+)").unwrap();
    let log_models = load_log_models(logs_db);
    let thread_models = load_thread_models(state_db);
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut max_ts = since;

    let Ok(rows) = stmt.query_map(params![since], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    }) else {
        return (records, since);
    };
    for row in rows.flatten() {
        let (ts, thread_id, body) = row;
        if ts > max_ts {
            max_ts = ts;
        }
        if thread_id.is_empty() {
            continue;
        }
        let turn_id = turn_id_re
            .captures(&body)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .or_else(|| {
                turn_id_re2
                    .captures(&body)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
            })
            .unwrap_or_else(|| ts.to_string());
        let key = format!("{thread_id}:{turn_id}");
        if !seen.insert(key.clone()) {
            continue;
        }
        let mut u = Usage::default();
        let mut found = false;
        for cap in token_usage_re.captures_iter(&body) {
            let field = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let value: i64 = cap.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            match field {
                "input_tokens" => {
                    u.input = value;
                    found = true;
                }
                "cached_input_tokens" => u.cached = value,
                "cache_write_input_tokens" => u.cache_write = value,
                "output_tokens" => u.output = value,
                "reasoning_output_tokens" => u.reasoning = value,
                "total_tokens" => u.total = value,
                _ => {}
            }
        }
        if !found {
            continue;
        }
        if u.total == 0 {
            u.total = u.input + u.output;
        }
        let model = model_re
            .captures(&body)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .or_else(|| log_models.get(&thread_id).cloned())
            .unwrap_or_else(|| "unknown".to_string());
        let _ = thread_models;
        let ctx = context_limit_re
            .captures(&body)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<i64>().ok());
        let ms = ts * 1000;
        records.push(Record {
            thread_id: thread_id.clone(),
            agent: "Codex".into(),
            model,
            key,
            path: "logs".into(),
            ts: ms,
            date: date_key(ms),
            usage: u,
            context_window: ctx,
        });
    }
    (records, max_ts)
}
