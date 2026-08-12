use crate::cache::Cache;
use crate::claude::load_claude_records;
use crate::codex::load_codex_records;
use crate::config::{Agent, Config};
use crate::opencode::load_opencode_records;
use crate::report::{summarize, Report};
use crate::types::Record;
use crate::zcode::{date_key, load_zcode_records};
use std::path::PathBuf;

fn codex_paths(cfg: &Config) -> Option<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    let home = cfg.agents.get(&Agent::Codex)?.path.clone();
    let home = PathBuf::from(home);
    Some((
        home.join("sessions"),
        home.join("archived_sessions"),
        home.join("logs_2.sqlite"),
        home.join("state_5.sqlite"),
    ))
}

fn records_for_agent(cfg: &Config, agent: Agent, cache: &mut Cache) -> Vec<Record> {
    match agent {
        Agent::Codex => {
            if let Some((sessions, archived, logs_db, state_db)) = codex_paths(cfg) {
                let logs_since = cache.get_watermark("codex-logs");
                let changed = cache
                    .changed_file_paths("Codex", &[sessions.clone(), archived.clone()])
                    .unwrap_or_default();
                let mut records = load_codex_records(&[sessions, archived], Some(&changed));
                let thread_models = crate::codex::load_thread_models(&state_db);
                let log_models = crate::codex::load_log_models(&logs_db);
                for r in &mut records {
                    if r.model == "unknown" {
                        if let Some(m) = thread_models.get(&r.thread_id) {
                            r.model = m.clone();
                        } else if let Some(m) = log_models.get(&r.thread_id) {
                            r.model = m.clone();
                        }
                    }
                }
                let (log_records, max_ts) = crate::codex::load_log_fallback(&logs_db, &state_db, logs_since);
                records.extend(log_records);
                for p in &changed {
                    cache.delete_records_by_path("Codex", p).ok();
                }
                cache.insert_records("Codex", &records).ok();
                if max_ts > logs_since {
                    cache.set_watermark("codex-logs", max_ts).ok();
                }
                cache.load_cache_records("Codex").unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Agent::Claude => {
            if let Some(p) = cfg.agents.get(&Agent::Claude) {
                let root = PathBuf::from(&p.path);
                let changed = cache
                    .changed_file_paths("Claude", std::slice::from_ref(&root))
                    .unwrap_or_default();
                let records = load_claude_records(&[root], Some(&changed));
                for p in &changed {
                    cache.delete_records_by_path("Claude", p).ok();
                }
                cache.insert_records("Claude", &records).ok();
                cache.load_cache_records("Claude").unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Agent::ZCode => {
            if let Some(p) = cfg.agents.get(&Agent::ZCode) {
                let db = PathBuf::from(&p.path);
                let since = cache.get_watermark("zcode-updated-ts");
                if let Ok((records, max_updated)) = load_zcode_records(&db, since) {
                    cache.insert_records("ZCode", &records).ok();
                    if max_updated > since {
                        cache.set_watermark("zcode-updated-ts", max_updated).ok();
                    }
                }
                cache.load_cache_records("ZCode").unwrap_or_default()
            } else {
                Vec::new()
            }
        }
        Agent::OpenCode => {
            if let Some(p) = cfg.agents.get(&Agent::OpenCode) {
                let db = PathBuf::from(&p.path);
                let since = cache.get_watermark("opencode-ts");
                if let Ok((records, max_updated)) = load_opencode_records(&db, since) {
                    cache.insert_records("OpenCode", &records).ok();
                    if max_updated > since {
                        cache.set_watermark("opencode-ts", max_updated).ok();
                    }
                }
                cache.load_cache_records("OpenCode").unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    }
}

pub fn collect(cache_path: &std::path::Path, cfg: &Config, days: usize, agent: &str) -> Report {
    let mut cache =
        Cache::open(cache_path).unwrap_or_else(|_| Cache::open(std::path::Path::new(":memory:")).unwrap());
    let mut records = Vec::new();
    let agents: Vec<Agent> = match agent {
        "Codex" => vec![Agent::Codex],
        "ZCode" => vec![Agent::ZCode],
        "Claude" => vec![Agent::Claude],
        "OpenCode" => vec![Agent::OpenCode],
        _ => Agent::ALL.to_vec(),
    };
    for a in agents {
        records.extend(records_for_agent(cfg, a, &mut cache));
    }
    let today = date_key(chrono::Utc::now().timestamp_millis());
    summarize(records, days, today)
}
