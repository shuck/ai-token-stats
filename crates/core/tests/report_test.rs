use ai_token_stats_core::report::summarize;
use ai_token_stats_core::types::{Record, Usage};

#[test]
fn summarize_buckets_by_agent_and_model() {
    let today = "2026-08-12";
    let mk = |agent: &str, model: &str, date: &str, total: i64, input: i64, cached: i64| Record {
        thread_id: "t".into(),
        agent: agent.into(),
        model: model.into(),
        key: format!("{agent}-{date}-{total}"),
        path: String::new(),
        ts: 0,
        date: date.into(),
        usage: Usage {
            total,
            input,
            cached,
            ..Default::default()
        },
        context_window: None,
    };
    let records = vec![
        mk("ZCode", "gpt-4o", today, 100, 100, 40),
        mk("ZCode", "gpt-4o", today, 50, 50, 10),
        mk("Codex", "gpt-5", today, 30, 30, 0),
    ];
    let rep = summarize(records, 30, today.to_string());
    assert_eq!(rep.totals.total, 180);
    assert_eq!(rep.totals.turns, 3);
    assert!(rep.totals.hit_rate.is_some());
    assert_eq!(rep.agents, vec!["ZCode".to_string(), "Codex".to_string()]);
    assert_eq!(rep.models, vec!["gpt-4o".to_string(), "gpt-5".to_string()]);
}
