use ai_token_stats_core::collect::collect;
use ai_token_stats_core::config::Config;
use ai_token_stats_core::report::Report;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct App {
    pub dir: PathBuf,
    pub cfg: Config,
    pub report: Option<Report>,
    pub days: usize,
    pub agent: String,
    pub last_refresh: Option<Instant>,
    pub settings_open: bool,
}

impl App {
    pub fn new(dir: PathBuf, cfg: Config) -> Self {
        let mut app = App {
            dir,
            cfg,
            report: None,
            days: 30,
            agent: "all".to_string(),
            last_refresh: None,
            settings_open: false,
        };
        app.refresh();
        app
    }

    pub fn refresh(&mut self) {
        self.last_refresh = Some(Instant::now());
        let cache_path = self.dir.join("ai-token-stats-cache.db");
        self.report = Some(collect(&cache_path, &self.cfg, self.days, &self.agent));
    }
}

pub fn fmt_tokens(v: i64) -> String {
    if v >= 100_000_000 {
        format!("{:.2}亿", v as f64 / 100_000_000.0)
    } else if v >= 10_000 {
        format!("{:.2}万", v as f64 / 10_000.0)
    } else {
        v.to_string()
    }
}

pub fn fmt_percent(v: Option<f64>) -> String {
    match v {
        None => "无数据".to_string(),
        Some(p) => format!("{:.1}%", p * 100.0),
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(t) = self.last_refresh {
            if t.elapsed() >= Duration::from_secs(60) {
                self.refresh();
            }
        }
        ctx.request_repaint_after(Duration::from_secs(60));
        crate::tray::poll_events(self);

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::from_rgb(232, 242, 252)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("最近天数:");
                    let mut days = self.days.to_string();
                    egui::ComboBox::from_id_source("days")
                        .selected_text(days.clone())
                        .show_ui(ui, |ui| {
                            for d in ["7", "14", "30", "90"] {
                                ui.selectable_value(&mut days, d.to_string(), d);
                            }
                        });
                    if let Ok(v) = days.parse::<usize>() {
                        self.days = v;
                    }
                    ui.label("Agent:");
                    let agents = ["all", "Codex", "ZCode", "Claude", "OpenCode"];
                    let mut cur = self.agent.clone();
                    egui::ComboBox::from_id_source("agent")
                        .selected_text(if self.agent == "all" {
                            "全部".to_string()
                        } else {
                            self.agent.clone()
                        })
                        .show_ui(ui, |ui| {
                            for a in agents {
                                let label = if a == "all" { "全部" } else { a };
                                ui.selectable_value(&mut cur, a.to_string(), label);
                            }
                        });
                    self.agent = cur;
                    if ui.button("刷新").clicked() {
                        self.refresh();
                    }
                });

                if let Some(rep) = &self.report {
                    ui.horizontal_wrapped(|ui| {
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
                        for (title, value) in cards {
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.set_min_size(egui::vec2(150.0, 54.0));
                                ui.label(title);
                                ui.label(
                                    egui::RichText::new(value)
                                        .size(16.0)
                                        .strong()
                                        .color(egui::Color32::from_rgb(20, 90, 220)),
                                );
                            });
                        }
                    });
                    ui.add_space(8.0);
                    crate::chart::draw_chart(ui, rep, &self.agent);
                }
            });

        if self.settings_open {
            crate::settings::show_settings(ctx, self);
        }
    }
}
