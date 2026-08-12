use ai_token_stats_core::cache::{insert_records, load_cache_records, Cache};
use ai_token_stats_core::types::{Record, Usage};
use std::fs;

#[test]
fn insert_load_and_watermark() {
    let dir = std::env::temp_dir().join(format!("ats-cache-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cache.db");
    let mut cache = Cache::open(&path).unwrap();

    cache.set_watermark("zcode-updated-ts", 100).unwrap();
    assert_eq!(cache.get_watermark("zcode-updated-ts"), 100);

    let rec = Record {
        thread_id: "zcode".into(),
        agent: "ZCode".into(),
        model: "gpt-4o".into(),
        key: "m1".into(),
        path: "zcode".into(),
        ts: 123,
        date: "2026-08-12".into(),
        usage: Usage {
            total: 271565,
            input: 270000,
            output: 1565,
            ..Default::default()
        },
        context_window: None,
    };
    insert_records(&mut cache, "ZCode", &[rec]).unwrap();

    let rows = load_cache_records(&cache, "ZCode").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].usage.total, 271565);
    assert_eq!(rows[0].date, "2026-08-12");

    fs::remove_dir_all(&dir).ok();
}
