use ai_token_stats_core::config::{Agent, AgentPath, Config};
use std::fs;

#[test]
fn round_trip() {
    let dir = std::env::temp_dir().join(format!("ats-config-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");
    let mut cfg = Config::default();
    cfg.agents.insert(
        Agent::Codex,
        AgentPath {
            path: r"D:\codex".into(),
            detected_at: "2026-08-12T00:00:00+08:00".into(),
        },
    );
    cfg.save(&path).unwrap();
    let got = Config::load(&path).unwrap();
    assert_eq!(got.agents[&Agent::Codex].path, r"D:\codex");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn missing_file_returns_default() {
    let dir = std::env::temp_dir().join(format!("ats-config-missing-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let cfg = Config::load(&dir.join("nope.json")).unwrap();
    assert!(cfg.agents.is_empty());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn corrupt_file_errors() {
    let dir = std::env::temp_dir().join(format!("ats-config-corrupt-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");
    fs::write(&path, "{not json").unwrap();
    assert!(Config::load(&path).is_err());
    fs::remove_dir_all(&dir).ok();
}
