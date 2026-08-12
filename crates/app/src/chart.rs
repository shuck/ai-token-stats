use ai_token_stats_core::report::{DaySummary, Report};

const PALETTE: [egui::Color32; 7] = [
    egui::Color32::from_rgb(20, 120, 230),
    egui::Color32::from_rgb(0, 180, 150),
    egui::Color32::from_rgb(150, 100, 220),
    egui::Color32::from_rgb(240, 140, 30),
    egui::Color32::from_rgb(70, 170, 70),
    egui::Color32::from_rgb(230, 70, 120),
    egui::Color32::from_rgb(140, 140, 140),
];

fn fmt(v: i64) -> String {
    if v >= 100_000_000 {
        format!("{:.2}亿", v as f64 / 100_000_000.0)
    } else if v >= 10_000 {
        format!("{:.2}万", v as f64 / 10_000.0)
    } else {
        v.to_string()
    }
}

fn pct(v: Option<f64>) -> String {
    match v {
        None => "无数据".to_string(),
        Some(p) => format!("{:.1}%", p * 100.0),
    }
}

pub fn draw_chart(ui: &mut egui::Ui, rep: &Report, agent: &str) {
    if rep.daily.is_empty() {
        return;
    }
    let keys: Vec<String> = if agent == "all" {
        rep.agents.clone()
    } else {
        rep.models.clone()
    };
    if keys.is_empty() {
        return;
    }
    let by_agent = agent == "all";

    let available = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(available.x.max(400.0), available.y.max(220.0)),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    let plot = rect.shrink2(egui::vec2(24.0, 16.0));
    let bottom = plot.bottom();
    let max_total = rep
        .daily
        .iter()
        .map(|d| d.total)
        .max()
        .unwrap_or(1)
        .max(10_000) as f64;
    let slot = plot.width() / rep.daily.len() as f32;
    let bar_w = (slot * 0.55).max(1.0);

    let mut hover_day: Option<&DaySummary> = None;
    if let Some(pos) = response.hover_pos() {
        let idx = ((pos.x - plot.left()) / slot).floor() as usize;
        if idx < rep.daily.len() && pos.x >= plot.left() && pos.x <= plot.right() {
            hover_day = Some(&rep.daily[idx]);
        }
    }

    for (i, day) in rep.daily.iter().enumerate() {
        let x = plot.left() + i as f32 * slot + (slot - bar_w) / 2.0;
        let mut cumulative = 0.0f32;
        for (ki, key) in keys.iter().enumerate() {
            let seg = if by_agent {
                day.by_agent.get(key).map(|s| s.total).unwrap_or(0)
            } else {
                day.by_model.get(key).map(|s| s.total).unwrap_or(0)
            };
            if seg <= 0 {
                continue;
            }
            let h = (seg as f64 / max_total * plot.height() as f64) as f32;
            let y0 = bottom - cumulative - h;
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x, y0),
                    egui::pos2(x + bar_w, bottom - cumulative),
                ),
                0.0,
                PALETTE[ki % PALETTE.len()],
            );
            cumulative += h;
        }
        if rep.daily.len() <= 15 || i % 2 == 0 {
            painter.text(
                egui::pos2(
                    plot.left() + i as f32 * slot + slot / 2.0,
                    bottom + 4.0,
                ),
                egui::Align2::CENTER_TOP,
                day.date[5..].to_string(),
                egui::FontId::proportional(9.0),
                egui::Color32::from_rgb(60, 60, 60),
            );
        }
    }

    if let Some(day) = hover_day {
        egui::show_tooltip_at_pointer(ui.ctx(), ui.id().with("chart-tip"), |ui| {
                ui.label(egui::RichText::new(&day.date).strong());
                ui.label(format!("总 token：{}", fmt(day.total)));
                ui.label(format!("输入：{} | 缓存：{}", fmt(day.input), fmt(day.cached)));
                ui.label(format!(
                    "输出：{} | 推理：{}",
                    fmt(day.output),
                    fmt(day.reasoning)
                ));
                ui.label(format!(
                    "轮次：{} | 上下文：{}",
                    day.turns,
                    day.max_context_window
                        .map(fmt)
                        .unwrap_or_else(|| "无数据".into())
                ));
                ui.label(format!(
                    "使用率峰值：{} | 命中率：{}",
                    pct(day.max_usage_percent),
                    pct(day.hit_rate)
                ));
                let subs: Vec<(&String, &DaySummary)> = if by_agent {
                    day.by_agent
                        .iter()
                        .filter(|(_, s)| s.total > 0)
                        .collect()
                } else {
                    day.by_model
                        .iter()
                        .filter(|(_, s)| s.total > 0)
                        .collect()
                };
                for (k, s) in subs {
                    ui.label(format!("{k}：{}", fmt(s.total)));
                }
        });
    }
}
