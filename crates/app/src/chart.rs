use ai_token_stats_core::report::Report;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke};

const PALETTE: [Color32; 7] = [
    Color32::from_rgb(20, 120, 230),
    Color32::from_rgb(0, 180, 150),
    Color32::from_rgb(150, 100, 220),
    Color32::from_rgb(240, 140, 30),
    Color32::from_rgb(70, 170, 70),
    Color32::from_rgb(230, 70, 120),
    Color32::from_rgb(140, 140, 140),
];

fn fmt_tokens(v: i64) -> String {
    if v >= 100_000_000 {
        format!("{:.2}亿", v as f64 / 100_000_000.0)
    } else if v >= 10_000 {
        format!("{:.2}万", v as f64 / 10_000.0)
    } else {
        v.to_string()
    }
}

fn fmt_percent(v: Option<f64>) -> String {
    match v {
        None => "无数据".to_string(),
        Some(p) => format!("{:.1}%", p * 100.0),
    }
}

fn fmt_context_window(v: Option<i64>) -> String {
    v.map(fmt_tokens).unwrap_or_else(|| "无数据".to_string())
}

#[derive(Clone, Copy)]
struct Geo {
    margin: f32,
    card_gap: f32,
    card_w: f32,
    card_h: f32,
    title_y: f32,
    plot_left: f32,
    plot_right: f32,
    plot_bottom: f32,
    plot_h: f32,
    slot: f32,
    bar_w: f32,
    label_step: usize,
}

fn compute_geo(width: f32, height: f32, days: usize) -> Geo {
    let margin = 20.0;
    let card_gap = 8.0;
    let card_h = 58.0;
    let card_w = if width > margin * 2.0 + card_gap * 4.0 {
        (width - 2.0 * margin - 4.0 * card_gap) / 5.0
    } else {
        0.0
    };
    let summary_bottom = margin + card_h;
    let title_y = summary_bottom + 12.0;
    let plot_left = 24.0;
    let plot_right = width - 24.0;
    let plot_bottom = height - 46.0;
    let plot_w = (plot_right - plot_left).max(1.0);
    let plot_h = (plot_bottom - (title_y + 26.0 + 8.0)).max(1.0);
    let slot = if days > 0 {
        (plot_w / days as f32).max(1.0)
    } else {
        1.0
    };
    let bar_w = (slot * 55.0 / 100.0).max(1.0);
    let label_step = if days > 15 { days.div_ceil(15) } else { 1 };
    Geo {
        margin,
        card_gap,
        card_w,
        card_h,
        title_y,
        plot_left,
        plot_right,
        plot_bottom,
        plot_h,
        slot,
        bar_w,
        label_step,
    }
}

fn center_text(
    painter: &egui::Painter,
    text: &str,
    rect: Rect,
    font_size: f32,
    color: Color32,
) {
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(font_size),
        color,
    );
}

pub fn draw_chart(ui: &mut egui::Ui, rep: &Report, agent: &str) {
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
    let painter = ui.painter();

    let title_font = 12.0;
    let text_font = 9.0;
    let value_font = 11.0;
    let text_color = Color32::from_rgb(60, 60, 60);
    let value_color = Color32::from_rgb(20, 90, 220);

    if rep.daily.is_empty() {
        center_text(painter, "无数据", rect, title_font, Color32::from_rgb(120, 120, 120));
        return;
    }

    let g = compute_geo(rect.width(), rect.height(), rep.daily.len());
    let days = rep.daily.len();
    let keys: Vec<String> = if agent == "all" {
        rep.agents.clone()
    } else {
        rep.models.clone()
    };
    let stack_by_agent = agent == "all";

    // 5 张汇总卡片（与 Go 版几何一致）
    let cards = [
        (format!("最近 {} 天", rep.days), fmt_tokens(rep.totals.total)),
        ("今日".to_string(), fmt_tokens(rep.today.total)),
        ("总命中率".to_string(), fmt_percent(rep.totals.hit_rate)),
        ("今日命中率".to_string(), fmt_percent(rep.today.hit_rate)),
        (
            "今日上下文峰值".to_string(),
            fmt_percent(rep.today.max_usage_percent),
        ),
    ];
    for (i, (title, value)) in cards.iter().enumerate() {
        let card_rect = Rect::from_min_size(
            Pos2::new(
                rect.left() + g.margin + i as f32 * (g.card_w + g.card_gap),
                rect.top() + g.margin,
            ),
            egui::vec2(g.card_w, g.card_h),
        );
        painter.rect_filled(
            card_rect,
            10.0,
            Color32::from_rgb(250, 252, 255),
        );
        painter.rect_stroke(
            card_rect,
            10.0,
            Stroke::new(1.0_f32, Color32::from_rgb(210, 224, 240)),
        );
        let title_rect = Rect::from_min_max(
            Pos2::new(card_rect.left(), card_rect.top() + 4.0),
            Pos2::new(card_rect.right(), card_rect.top() + 24.0),
        );
        center_text(painter, title, title_rect, text_font, text_color);
        let value_rect = Rect::from_min_max(
            Pos2::new(card_rect.left(), card_rect.top() + 26.0),
            Pos2::new(card_rect.right(), card_rect.top() + 52.0),
        );
        center_text(painter, value, value_rect, value_font, value_color);
    }

    // 图表标题
    let chart_title_rect = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top() + g.title_y),
        Pos2::new(rect.right(), rect.top() + g.title_y + 26.0),
    );
    center_text(painter, "AI Token 统计", chart_title_rect, title_font, Color32::from_rgb(30, 30, 30));

    if keys.is_empty() {
        return;
    }
    let plot_left = rect.left() + g.plot_left;
    let plot_bottom = rect.top() + g.plot_bottom;
    let max_token = rep
        .daily
        .iter()
        .map(|d| d.total as f64 / 10000.0)
        .fold(1.0f64, |m, v| m.max(v));

    // 堆叠柱
    for (i, day) in rep.daily.iter().enumerate() {
        let x = plot_left + i as f32 * g.slot + (g.slot - g.bar_w) / 2.0;
        let mut cumulative = 0.0f32;
        for (ki, key) in keys.iter().enumerate() {
            let seg = if stack_by_agent {
                day.by_agent.get(key).map(|s| s.total).unwrap_or(0)
            } else {
                day.by_model.get(key).map(|s| s.total).unwrap_or(0)
            };
            if seg <= 0 {
                continue;
            }
            let h = (seg as f64 / 10000.0 / max_token * g.plot_h as f64) as f32;
            if h <= 0.0 {
                continue;
            }
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(x, plot_bottom - cumulative - h),
                    Pos2::new(x + g.bar_w, plot_bottom - cumulative),
                ),
                0.0,
                PALETTE[ki % PALETTE.len()],
            );
            cumulative += h;
        }
        if i % g.label_step == 0 {
            let label = if day.date.len() > 5 {
                day.date[5..].to_string()
            } else {
                day.date.clone()
            };
            let label_rect = Rect::from_min_max(
                Pos2::new(plot_left + i as f32 * g.slot, plot_bottom + 4.0),
                Pos2::new(plot_left + (i + 1) as f32 * g.slot, plot_bottom + 20.0),
            );
            center_text(painter, &label, label_rect, text_font, text_color);
        }
    }

    // 图例
    let plot_right = rect.left() + g.plot_right;
    let mut legend_x = plot_left;
    let mut legend_y = plot_bottom + 24.0;
    for (ki, key) in keys.iter().enumerate() {
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(legend_x, legend_y + 2.0),
                egui::vec2(10.0, 10.0),
            ),
            0.0,
            PALETTE[ki % PALETTE.len()],
        );
        center_text(
            painter,
            key,
            Rect::from_min_size(Pos2::new(legend_x + 14.0, legend_y), egui::vec2(116.0, 16.0)),
            text_font,
            text_color,
        );
        legend_x += 130.0;
        if legend_x + 130.0 > plot_right {
            legend_x = plot_left;
            legend_y += 18.0;
        }
    }

    // 悬停：橙色柱形描边 + 自定义 tooltip
    if let Some(pos) = response.hover_pos() {
        let idx = ((pos.x - plot_left) / g.slot).floor() as isize;
        if idx >= 0 && (idx as usize) < days && pos.x >= plot_left && pos.x <= plot_right {
            let idx = idx as usize;
            let day = &rep.daily[idx];
            let total = day.total;
            let bar_top = if total > 0 {
                plot_bottom - (total as f64 / 10000.0 / max_token * g.plot_h as f64) as f32
            } else {
                plot_bottom
            };
            let bar_rect = Rect::from_min_max(
                Pos2::new(
                    plot_left + idx as f32 * g.slot + (g.slot - g.bar_w) / 2.0,
                    bar_top,
                ),
                Pos2::new(
                    plot_left + idx as f32 * g.slot + (g.slot - g.bar_w) / 2.0 + g.bar_w,
                    plot_bottom,
                ),
            );
            painter.rect_stroke(
                bar_rect,
                0.0,
                Stroke::new(1.0_f32, Color32::from_rgb(255, 140, 0)),
            );

            let mut lines: Vec<String> = vec![
                day.date.clone(),
                format!("总 token：{}", fmt_tokens(day.total)),
                format!("输入：{} | 缓存：{}", fmt_tokens(day.input), fmt_tokens(day.cached)),
                format!("输出：{} | 推理：{}", fmt_tokens(day.output), fmt_tokens(day.reasoning)),
                format!("轮次：{} | 上下文：{}", day.turns, fmt_context_window(day.max_context_window)),
                format!("使用率峰值：{} | 命中率：{}", fmt_percent(day.max_usage_percent), fmt_percent(day.hit_rate)),
            ];
            if stack_by_agent {
                for agent_name in &rep.agents {
                    if let Some(ad) = day.by_agent.get(agent_name) {
                        if ad.total > 0 {
                            lines.push(format!("{agent_name}：{}", fmt_tokens(ad.total)));
                            for model in &rep.models {
                                if let Some(md) = ad.by_model.get(model) {
                                    if md.total > 0 {
                                        lines.push(format!("  {model}：{}", fmt_tokens(md.total)));
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
                            lines.push(format!("{model}：{}", fmt_tokens(md.total)));
                        }
                    }
                }
            }

            let tip_width = 320.0;
            let tip_height = 10.0 + lines.len() as f32 * 19.0;
            let mut tip_x = pos.x + 14.0;
            let mut tip_y = pos.y - tip_height - 10.0;
            if tip_x + tip_width > rect.right() - 8.0 {
                tip_x = rect.right() - tip_width - 8.0;
            }
            if tip_x < rect.left() + 8.0 {
                tip_x = rect.left() + 8.0;
            }
            if tip_y < rect.top() + 8.0 {
                tip_y = rect.top() + 8.0;
            }
            if tip_y + tip_height > rect.bottom() - 8.0 {
                tip_y = rect.bottom() - tip_height - 8.0;
            }
            let tip_rect = Rect::from_min_size(
                Pos2::new(tip_x, tip_y),
                egui::vec2(tip_width, tip_height),
            );
            painter.rect_filled(tip_rect, 0.0, Color32::from_rgb(255, 253, 247));
            painter.rect_stroke(
                tip_rect,
                0.0,
                Stroke::new(1.0_f32, Color32::from_rgb(200, 160, 90)),
            );
            for (i, line) in lines.iter().enumerate() {
                let line_rect = Rect::from_min_max(
                    Pos2::new(tip_x + 10.0, tip_y + 4.0 + i as f32 * 19.0),
                    Pos2::new(tip_x + tip_width - 10.0, tip_y + 4.0 + i as f32 * 19.0 + 18.0),
                );
                if i == 0 {
                    painter.text(
                        line_rect.left_top(),
                        Align2::LEFT_TOP,
                        line,
                        FontId::proportional(value_font),
                        value_color,
                    );
                } else {
                    painter.text(
                        line_rect.left_top(),
                        Align2::LEFT_TOP,
                        line,
                        FontId::proportional(text_font),
                        text_color,
                    );
                }
            }
        }
    }
}
