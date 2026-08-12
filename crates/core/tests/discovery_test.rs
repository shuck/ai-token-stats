use ai_token_stats_core::config::Agent;
use ai_token_stats_core::discovery::{discover_agent_path, validate_agent_path, ScanLimits};
use std::fs;
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ats-disc-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

struct Guard(PathBuf);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn isolate_env() {
    std::env::set_var(
        "USERPROFILE",
        std::env::temp_dir().join(format!("ats-env-{}", std::process::id())),
    );
    std::env::set_var("CODEX_HOME", "");
    std::env::set_var("ZCODE_DATA", "");
}

#[test]
fn validate_codex_and_claude_dirs() {
    let root = tmp("validate");
    let codex = root.join("codex");
    fs::create_dir_all(codex.join("sessions")).unwrap();
    fs::create_dir_all(codex.join("archived_sessions")).unwrap();
    assert!(validate_agent_path(Agent::Codex, &codex));
    assert!(!validate_agent_path(Agent::Codex, &root));

    let claude = root.join(".claude").join("projects");
    fs::create_dir_all(claude.join("s1")).unwrap();
    fs::write(claude.join("s1").join("a.jsonl"), "{}").unwrap();
    assert!(validate_agent_path(Agent::Claude, &claude));
    assert!(!validate_agent_path(Agent::Claude, &root));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn discover_codex_and_claude_and_zcode_and_opencode() {
    isolate_env();
    let root = tmp("discover");
    let _guard = Guard(root.clone());
    let codex = root.join("ai-data").join("codex");
    fs::create_dir_all(codex.join("sessions")).unwrap();
    fs::create_dir_all(codex.join("archived_sessions")).unwrap();

    let claude = root.join(".claude").join("projects");
    fs::create_dir_all(claude.join("s1")).unwrap();
    fs::write(claude.join("s1").join("a.jsonl"), "{}").unwrap();

    let zdb = root.join("zdata").join("db.sqlite");
    fs::create_dir_all(zdb.parent().unwrap()).unwrap();
    let conn = rusqlite::Connection::open(&zdb).unwrap();
    conn.execute_batch("CREATE TABLE message (id TEXT, data TEXT);").unwrap();
    drop(conn);

    let odb = root.join("opencode.db");
    let conn = rusqlite::Connection::open(&odb).unwrap();
    conn.execute_batch("CREATE TABLE session (id TEXT, tokens_input INTEGER);").unwrap();
    drop(conn);

    let roots = vec![root.clone()];
    assert_eq!(discover_agent_path(Agent::Codex, &roots, &ScanLimits::test()), Some(codex.clone()));
    assert_eq!(discover_agent_path(Agent::Claude, &roots, &ScanLimits::test()), Some(claude));
    assert_eq!(discover_agent_path(Agent::ZCode, &roots, &ScanLimits::test()), Some(zdb));
    assert_eq!(discover_agent_path(Agent::OpenCode, &roots, &ScanLimits::test()), Some(odb));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn discover_prefers_newest() {
    isolate_env();
    let root = tmp("newest");
    for d in ["old", "new"] {
        let p = root.join(d);
        fs::create_dir_all(p.join("sessions")).unwrap();
        fs::create_dir_all(p.join("archived_sessions")).unwrap();
    }
    let old = root.join("old");
    let new = root.join("new");
    let old_t = filetime(&old) - 2 * 86400;
    let new_t = filetime(&new) - 3600;
    set_filetime(&old, old_t);
    set_filetime(&new, new_t);
    let got = discover_agent_path(Agent::Codex, &[root.clone()], &ScanLimits::test());
    assert_eq!(got, Some(new));
    fs::remove_dir_all(&root).ok();
}

fn filetime(p: &PathBuf) -> i64 {
    fs::metadata(p)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn set_filetime(p: &PathBuf, secs: i64) {
    let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64);
    let ft = filetime::FileTime::from_system_time(t);
    filetime::set_file_mtime(p, ft).unwrap();
}
