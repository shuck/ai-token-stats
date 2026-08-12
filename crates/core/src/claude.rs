use crate::codex::parse_rfc3339_ms;
use crate::types::{Record, Usage};
use crate::zcode::date_key;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: i64,
    pub cache_read_input_tokens: i64,
    pub cache_creation_input_tokens: i64,
    pub output_tokens: i64,
    #[serde(default)]
    pub output_tokens_details: ClaudeOutputDetails,
}

#[derive(Debug, Default, Deserialize)]
pub struct ClaudeOutputDetails {
    pub reasoning_tokens: i64,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeMessage {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub usage: ClaudeUsage,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeEvent {
    #[serde(rename = "type")]
    pub type_: String,
    pub timestamp: String,
    pub message: Option<ClaudeMessage>,
}

pub fn load_claude_records(
    roots: &[PathBuf],
    changed: Option<&BTreeSet<PathBuf>>,
) -> Vec<Record> {
    let files: Vec<PathBuf> = match changed {
        None => walk_jsonl(roots),
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
    for path in files {
        let Ok(f) = fs::File::open(&path) else { continue };
        for line in BufReader::new(f).lines().map_while(|l| l.ok()) {
            let Ok(ev) = serde_json::from_str::<ClaudeEvent>(&line) else {
                continue;
            };
            let Some(msg) = ev.message else { continue };
            let u = msg.usage;
            if u.input_tokens == 0 && u.output_tokens == 0 {
                continue;
            }
            let ts = parse_rfc3339_ms(&ev.timestamp);
            if ts == 0 {
                continue;
            }
            let model = if msg.model.is_empty() {
                "unknown".to_string()
            } else {
                msg.model
            };
            records.push(Record {
                thread_id: "claude".into(),
                agent: "Claude".into(),
                model,
                key: format!("{}:{ts}", path.to_string_lossy()),
                path: path.to_string_lossy().into_owned(),
                ts,
                date: date_key(ts),
                usage: Usage {
                    input: u.input_tokens,
                    cached: u.cache_read_input_tokens,
                    cache_write: u.cache_creation_input_tokens,
                    output: u.output_tokens,
                    reasoning: u.output_tokens_details.reasoning_tokens,
                    total: u.input_tokens + u.output_tokens,
                },
                context_window: None,
            });
        }
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
