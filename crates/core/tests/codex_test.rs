use ai_token_stats_core::codex::load_codex_records;
use std::collections::BTreeSet;
use std::fs;

#[test]
fn codex_rollout_jsonl_parses_token_count() {
    let dir = std::env::temp_dir().join(format!("ats-codex-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("rollout.jsonl");
    let body = r#"{"type":"session_meta","timestamp":"2026-08-12T10:00:00.000Z","payload":{"session_id":"sess1"}}
{"type":"event_msg","timestamp":"2026-08-12T10:01:00.000Z","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"cached_input_tokens":50,"output_tokens":20,"reasoning_output_tokens":5,"total_tokens":120},"model_context_window":200000}}}
"#;
    fs::write(&file, body).unwrap();
    let set = BTreeSet::from([file.clone()]);
    let records = load_codex_records(&[file.clone()], Some(&set));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].usage.input, 100);
    assert_eq!(records[0].usage.cached, 50);
    assert_eq!(records[0].usage.output, 20);
    assert_eq!(records[0].usage.total, 120);
    assert_eq!(records[0].context_window, Some(200000));
    fs::remove_dir_all(&dir).ok();
}
