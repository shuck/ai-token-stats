use ai_token_stats_core::zcode::load_zcode_records;
use rusqlite::Connection;
use std::fs;

#[test]
fn zcode_placeholder_then_update() {
    let dir = std::env::temp_dir().join(format!("ats-zcode-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("db.sqlite");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
        "CREATE TABLE message (id TEXT PRIMARY KEY, time_created INTEGER, time_updated INTEGER, data TEXT);",
    )
    .unwrap();
    let created = 1_752_800_000_000i64;
    conn.execute(
        "INSERT INTO message(id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            "m1",
            created,
            created,
            r#"{"modelID":"gpt-4o","tokens":{"total":0,"input":0,"output":0,"reasoning":0,"cache":{"read":0,"write":0}}}"#
        ],
    )
    .unwrap();

    let (records, max_updated) = load_zcode_records(&path, 0).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].usage.total, 0);
    assert_eq!(records[0].ts, created);
    assert_eq!(max_updated, created);

    let updated = created + 60000;
    conn.execute(
        "UPDATE message SET data = ?1, time_updated = ?2 WHERE id = 'm1'",
        rusqlite::params![
            r#"{"modelID":"gpt-4o","tokens":{"total":271565,"input":270000,"output":1565,"reasoning":0,"cache":{"read":250000,"write":0}}}"#,
            updated
        ],
    )
    .unwrap();

    let (records, max_updated) = load_zcode_records(&path, created).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].usage.total, 271565);
    assert_eq!(records[0].ts, created, "Ts 必须仍取 time_created");
    assert_eq!(max_updated, updated);

    let (records, _) = load_zcode_records(&path, updated).unwrap();
    assert!(records.is_empty());
    fs::remove_dir_all(&dir).ok();
}
