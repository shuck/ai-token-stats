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
    // 标题区（顶部 30px）+ 图例区（底部 22px）
    let title_bottom = rect.top() + 30.0;
    let legend_top = rect.bottom() - 22.0;
    let plot = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 24.0, title_bottom),
        egui::pos2(rect.right() - 24.0, legend_top - 8.0),
    );
    // 绘图面板背景与浅色网格
    painter.rect_filled(
        plot.expand2(egui::vec2(6.0, 6.0)),
        8.0,
        egui::Color32::from_rgb(250, 252, 255),
    );
    painter.rect_stroke(
        plot.expand2(egui::vec2(6.0, 6.0)),
        8.0,
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(210, 224, 240)),
    );
    for k in 0..=4 {
        let y = plot.top() + plot.height() * k as f32 / 4.0;
        painter.line_segment(
            [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(228, 234, 242)),
        );
    }
    painter.text(
        egui::pos2(rect.center().x, rect.top() + 6.0),
        egui::Align2::CENTER_TOP,
        "AI Token 统计",
        egui::FontId::proportional(14.0),
        egui::Color32::from_rgb(30, 30, 30),
    );
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
    let mut hover_idx: Option<usize> = None;
    if let Some(pos) = response.hover_pos() {
        ui.ctx().request_repaint();
        let idx = ((pos.x - plot.left()) / slot).floor() as usize;
        if idx < rep.daily.len() && pos.x >= plot.left() && pos.x <= plot.right() {
            hover_day = Some(&rep.daily[idx]);
            hover_idx = Some(idx);
        }
    }

    if let Some(idx) = hover_idx {
        let x0 = plot.left() + idx as f32 * slot;
        painter.rect_stroke(
            egui::Rect::from_min_max(
                egui::pos2(x0, plot.top()),
                egui::pos2(x0 + slot, plot.bottom()),
            ),
            4.0,
            egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(255, 140, 0)),
        );
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

    // 图例
    let mut legend_x = plot.left();
    let mut legend_y = legend_top + 2.0;
    for (ki, key) in keys.iter().enumerate() {
        if legend_x + 130.0 > plot.right() {
            legend_x = plot.left();
            legend_y += 18.0;
        }
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(legend_x, legend_y + 2.0),
                egui::vec2(10.0, 10.0),
            ),
            0.0,
            PALETTE[ki % PALETTE.len()],
        );
        painter.text(
            egui::pos2(legend_x + 14.0, legend_y),
            egui::Align2::LEFT_TOP,
            key,
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(60, 60, 60),
        );
        legend_x += 130.0;
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
                // 与 Go 版一致：按 Agent 列出总额，其下缩进列出各模型
                if by_agent {
                    for agent in &rep.agents {
                        if let Some(ad) = day.by_agent.get(agent) {
                            if ad.total > 0 {
                                ui.label(format!("{agent}：{}", fmt(ad.total)));
                                for model in &rep.models {
                                    if let Some(md) = ad.by_model.get(model) {
                                        if md.total > 0 {
                                            ui.label(format!("  {model}：{}", fmt(md.total)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    for model in &rep.models {
                        if let Some(md) = day.by_model.get(model) {
                            if md.total > 0 {
                                ui.label(format!("{model}：{}", fmt(md.total)));
                            }
                        }
                    }
                }
        });
    }
}
