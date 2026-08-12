use crate::config::Agent;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScanLimits {
    pub max_depth: usize,
    pub max_dirs: usize,
    pub max_duration: Duration,
}

impl Default for ScanLimits {
    fn default() -> Self {
        ScanLimits {
            max_depth: 4,
            max_dirs: 20_000,
            max_duration: Duration::from_secs(20),
        }
    }
}

impl ScanLimits {
    pub fn test() -> Self {
        ScanLimits {
            max_depth: 4,
            max_dirs: 20_000,
            max_duration: Duration::from_secs(20),
        }
    }
}

pub fn validate_agent_path(agent: Agent, path: &Path) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    let Ok(info) = fs::metadata(path) else {
        return false;
    };
    match agent {
        Agent::Codex => {
            if !info.is_dir() {
                return false;
            }
            if path.join("logs_2.sqlite").exists() {
                return true;
            }
            path.join("sessions").is_dir() && path.join("archived_sessions").is_dir()
        }
        Agent::ZCode => !info.is_dir() && has_message_table(path),
        Agent::Claude => info.is_dir() && is_claude_projects(path),
        Agent::OpenCode => !info.is_dir() && has_session_table(path),
    }
}

fn is_claude_projects(dir: &Path) -> bool {
    dir.file_name().and_then(|n| n.to_str()) == Some("projects")
        && dir
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            == Some(".claude")
}

fn has_message_table(path: &Path) -> bool {
    let Ok(conn) = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    ) else {
        return false;
    };
    let ok = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='message'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    ok && conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('message') WHERE name='data'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false)
}

fn has_session_table(path: &Path) -> bool {
    let Ok(conn) = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    ) else {
        return false;
    };
    let ok = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='session'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    ok && conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info('session') WHERE name='tokens_input'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false)
}

pub fn known_candidates(agent: Agent) -> Vec<PathBuf> {
    let home = std::env::var("USERPROFILE").ok();
    match agent {
        Agent::Codex => {
            if let Ok(h) = std::env::var("CODEX_HOME") {
                if !h.is_empty() {
                    return vec![PathBuf::from(h)];
                }
            }
            home.map(|h| vec![PathBuf::from(h).join(".codex")])
                .unwrap_or_default()
        }
        Agent::ZCode => std::env::var("ZCODE_DATA")
            .ok()
            .filter(|d| !d.is_empty())
            .map(|d| vec![PathBuf::from(d).join("cli").join("db").join("db.sqlite")])
            .unwrap_or_default(),
        Agent::Claude => home
            .map(|h| vec![PathBuf::from(h).join(".claude").join("projects")])
            .unwrap_or_default(),
        Agent::OpenCode => home
            .map(|h| {
                vec![
                    PathBuf::from(h)
                        .join(".local")
                        .join("share")
                        .join("opencode")
                        .join("opencode.db"),
                ]
            })
            .unwrap_or_default(),
    }
}

pub fn scan_roots() -> Vec<PathBuf> {
    let mut roots: std::collections::BTreeSet<PathBuf> = std::collections::BTreeSet::new();
    for letter in b'A'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if root.exists() {
            roots.insert(root);
        }
    }
    for env in ["USERPROFILE", "APPDATA", "LOCALAPPDATA"] {
        if let Ok(v) = std::env::var(env) {
            if !v.is_empty() {
                roots.insert(PathBuf::from(v));
            }
        }
    }
    roots.into_iter().collect()
}

const SKIP_DIRS: &[&str] = &[
    "$Recycle.Bin",
    "$RECYCLE.BIN",
    "System Volume Information",
    "Windows",
    "Program Files",
    "Program Files (x86)",
];

pub fn discover_agent_path(
    agent: Agent,
    roots: &[PathBuf],
    limits: &ScanLimits,
) -> Option<PathBuf> {
    for c in known_candidates(agent) {
        if validate_agent_path(agent, &c) {
            return Some(c);
        }
    }
    let mut best: Option<(PathBuf, i64)> = None;
    let started = Instant::now();
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let mut visited = 0usize;
        // Go 版语义：目录下降深度 > max_depth 时不再深入，但深度恰为 max_depth
        // 的目录内的文件仍会被访问；目录数/时间超限时跳过当前子树而不是中断整盘。
        let walker = WalkDir::new(root)
            .max_depth(limits.max_depth + 1)
            .into_iter()
            .filter_entry(move |e| {
                if e.depth() == 0 {
                    return true;
                }
                if e.file_type().is_dir() {
                    if e.depth() > limits.max_depth
                        || visited >= limits.max_dirs
                        || started.elapsed() > limits.max_duration
                    {
                        return false;
                    }
                    visited += 1;
                    return !SKIP_DIRS.contains(&e.file_name().to_string_lossy().as_ref());
                }
                true
            });
        for entry in walker {
            let Ok(e) = entry else { continue };
            let path = e.path().to_path_buf();
            let is_match = if e.file_type().is_dir() {
                match_agent_dir(agent, &path)
            } else {
                match_agent_file(agent, &path)
            };
            if !is_match {
                continue;
            }
            let mtime = fs::metadata(&path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if best.as_ref().map(|(_, m)| mtime > *m).unwrap_or(true) {
                best = Some((path, mtime));
            }
        }
    }
    best.map(|(p, _)| p)
}

fn match_agent_file(agent: Agent, path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    match agent {
        Agent::ZCode => name.eq_ignore_ascii_case("db.sqlite") && has_message_table(path),
        Agent::OpenCode => name.eq_ignore_ascii_case("opencode.db") && has_session_table(path),
        _ => false,
    }
}

fn match_agent_dir(agent: Agent, path: &Path) -> bool {
    match agent {
        Agent::Codex => {
            path.join("logs_2.sqlite").exists()
                || (path.join("sessions").is_dir() && path.join("archived_sessions").is_dir())
        }
        Agent::Claude => is_claude_projects(path),
        _ => false,
    }
}
