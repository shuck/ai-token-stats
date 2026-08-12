use crate::types::Record;
use crate::zcode::shanghai;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct DaySummary {
    pub date: String,
    pub input: i64,
    pub cached: i64,
    pub output: i64,
    pub reasoning: i64,
    pub total: i64,
    pub turns: usize,
    pub max_context_window: Option<i64>,
    pub max_usage_percent: Option<f64>,
    pub hit_rate: Option<f64>,
    pub by_model: BTreeMap<String, DaySummary>,
    pub by_agent: BTreeMap<String, DaySummary>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub days: usize,
    pub range_start: String,
    pub range_end: String,
    pub totals: DaySummary,
    pub today: DaySummary,
    pub daily: Vec<DaySummary>,
    pub models: Vec<String>,
    pub agents: Vec<String>,
}

fn add_record(d: &mut DaySummary, r: &Record) {
    d.input += r.usage.input;
    d.cached += r.usage.cached;
    d.output += r.usage.output;
    d.reasoning += r.usage.reasoning;
    d.total += r.usage.total;
    d.turns += 1;
    if let Some(cw) = r.context_window {
        if d.max_context_window.map(|m| cw > m).unwrap_or(true) {
            d.max_context_window = Some(cw);
        }
        if cw > 0 {
            let pct = r.usage.input as f64 / cw as f64;
            if d.max_usage_percent.map(|m| pct > m).unwrap_or(true) {
                d.max_usage_percent = Some(pct);
            }
        }
    }
    if d.input > 0 {
        d.hit_rate = Some(d.cached as f64 / d.input as f64);
    }
    add_leaf(d.by_model.entry(r.model.clone()).or_default(), r);
    let agent = d.by_agent.entry(r.agent.clone()).or_default();
    add_leaf(agent, r);
    add_leaf(agent.by_model.entry(r.model.clone()).or_default(), r);
}

fn add_leaf(d: &mut DaySummary, r: &Record) {
    d.input += r.usage.input;
    d.cached += r.usage.cached;
    d.output += r.usage.output;
    d.reasoning += r.usage.reasoning;
    d.total += r.usage.total;
    d.turns += 1;
    if let Some(cw) = r.context_window {
        if cw > 0 {
            let pct = r.usage.input as f64 / cw as f64;
            if d.max_usage_percent.map(|m| pct > m).unwrap_or(true) {
                d.max_usage_percent = Some(pct);
            }
        }
    }
    if d.input > 0 {
        d.hit_rate = Some(d.cached as f64 / d.input as f64);
    }
}

pub fn summarize(records: Vec<Record>, days: usize, today: String) -> Report {
    let now = chrono::Utc::now().with_timezone(&shanghai());
    let start = now - chrono::Duration::days((days - 1) as i64);
    let start_key = start.format("%Y-%m-%d").to_string();

    let mut daily: Vec<DaySummary> = Vec::new();
    for offset in (0..days).rev() {
        let d = now - chrono::Duration::days(offset as i64);
        daily.push(DaySummary {
            date: d.format("%Y-%m-%d").to_string(),
            ..Default::default()
        });
    }

    let mut totals = DaySummary {
        date: "total".into(),
        ..Default::default()
    };
    let mut today_sum = DaySummary {
        date: today.clone(),
        ..Default::default()
    };
    for r in &records {
        if r.date.as_str() < start_key.as_str() || r.date.as_str() > today.as_str() {
            continue;
        }
        add_record(&mut totals, r);
        if r.date == today {
            add_record(&mut today_sum, r);
        }
        if let Some(day) = daily.iter_mut().find(|d| d.date == r.date) {
            add_record(day, r);
        }
    }

    let mut models: Vec<String> = totals.by_model.keys().cloned().collect();
    models.sort_by(|a, b| totals.by_model[b].total.cmp(&totals.by_model[a].total));
    let mut agents: Vec<String> = totals.by_agent.keys().cloned().collect();
    agents.sort_by(|a, b| totals.by_agent[b].total.cmp(&totals.by_agent[a].total));

    Report {
        days,
        range_start: daily.first().map(|d| d.date.clone()).unwrap_or_default(),
        range_end: daily.last().map(|d| d.date.clone()).unwrap_or_default(),
        totals,
        today: today_sum,
        daily,
        models,
        agents,
    }
}
