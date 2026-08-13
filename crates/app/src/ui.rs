use ai_token_stats_core::collect::collect;
use ai_token_stats_core::config::Config;
use ai_token_stats_core::report::Report;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct App {
    pub dir: PathBuf,
    pub cfg: Config,
    pub report: Option<Report>,
    pub days: usize,
    pub agent: String,
    pub last_refresh: Option<Instant>,
    pub settings_open: bool,
    pub tray: Option<Arc<crate::tray::TrayState>>,
    pub _tray_icon: Option<tray_icon::TrayIcon>,
    pub pending_show: bool,
    pub pending_close: bool,
    pub exiting: bool,
}

impl App {
    pub fn new(dir: PathBuf, cfg: Config) -> Self {
        let mut cfg = cfg;
        crate::bootstrap::ensure_discovered(&mut cfg, &dir.join("config.json"));
        let mut app = App {
            dir,
            cfg,
            report: None,
            days: 30,
            agent: "all".to_string(),
            last_refresh: None,
            settings_open: false,
            tray: None,
            _tray_icon: None,
            pending_show: false,
            pending_close: false,
            exiting: false,
        };
        app.refresh();
        app
    }

    pub fn ctx_send_visible(&mut self, visible: bool) {
        if visible {
            self.refresh();
        }
        self.pending_show = true;
    }

    pub fn ctx_send_close(&mut self) {
        self.exiting = true;
        self.pending_close = true;
        // 兜底：eframe/winit 关窗链路若被卡住，1.5 秒后强制退出，
        // 保证托盘「退出」永远可用。
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_secs(1) + Duration::from_millis(500));
            std::process::exit(0);
        });
    }

    pub fn refresh(&mut self) {
        self.last_refresh = Some(Instant::now());
        let cache_path = self.dir.join("ai-token-stats-cache.db");
        crate::bootstrap::ensure_discovered(&mut self.cfg, &self.dir.join("config.json"));
        crate::logging::log_msg(&format!(
            "refresh days={} agent={} cfg={:?}",
            self.days, self.agent, self.cfg
        ));
        self.report = Some(collect(&cache_path, &self.cfg, self.days, &self.agent));
        if let Some(rep) = &self.report {
            crate::logging::log_msg(&format!(
                "refresh done turns={} agents={:?} models={:?} zcode={} codex={} claude={} opencode={}",
                rep.totals.turns,
                rep.agents,
                rep.models,
                rep.totals
                    .by_agent
                    .get("ZCode")
                    .map(|s| s.total)
                    .unwrap_or(0),
                rep.totals
                    .by_agent
                    .get("Codex")
                    .map(|s| s.total)
                    .unwrap_or(0),
                rep.totals
                    .by_agent
                    .get("Claude")
                    .map(|s| s.total)
                    .unwrap_or(0),
                rep.totals
                    .by_agent
                    .get("OpenCode")
                    .map(|s| s.total)
                    .unwrap_or(0),
            ));
        }
        self.update_tray_tooltip();
    }

    pub fn update_tray_tooltip(&mut self) {
        if let Some(icon) = &self._tray_icon {
            let tip = if let Some(rep) = &self.report {
                format!(
                    "AI Token 统计 | 今日 {} | 命中 {}",
                    fmt_tokens(rep.today.total),
                    fmt_percent(rep.today.hit_rate)
                )
            } else {
                "AI Token 统计".to_string()
            };
            crate::logging::log_msg(&format!("tray tooltip set: {tip}"));
            let _ = icon.set_tooltip(Some(tip));
        }
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

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.pending_show {
            self.pending_show = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if self.pending_close {
            self.pending_close = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.exiting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        if let Some(t) = self.last_refresh {
            if t.elapsed() >= Duration::from_secs(60) {
                self.refresh();
            }
        }
        ctx.request_repaint_after(Duration::from_secs(60));
        crate::tray::poll_events(self);

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::WHITE))
            .show(ctx, |ui| {
                // 垂直渐变背景（浅蓝 → 白）
                let rect = ui.max_rect();
                let top_c = egui::Color32::from_rgb(232, 242, 252);
                let bottom_c = egui::Color32::WHITE;
                let bands = 24;
                for i in 0..bands {
                    let t = i as f32 / bands as f32;
                    let t1 = (i + 1) as f32 / bands as f32;
                    let c = egui::Color32::from_rgb(
                        lerp_u8(top_c.r(), bottom_c.r(), t),
                        lerp_u8(top_c.g(), bottom_c.g(), t),
                        lerp_u8(top_c.b(), bottom_c.b(), t),
                    );
                    let y0 = rect.top() + rect.height() * t;
                    let y1 = rect.top() + rect.height() * t1;
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(rect.left(), y0),
                            egui::pos2(rect.right(), y1),
                        ),
                        0.0,
                        c,
                    );
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    crate::widgets::label(ui, "最近天数:");
                    let day_items = ["7", "14", "30", "90"];
                    let mut day_idx = day_items
                        .iter()
                        .position(|d| d.parse::<usize>().ok() == Some(self.days))
                        .unwrap_or(2);
                    crate::widgets::combo(ui, "days", day_items[day_idx], &day_items, &mut day_idx);
                    self.days = day_items[day_idx].parse().unwrap_or(30);
                    ui.add_space(12.0);
                    crate::widgets::label(ui, "Agent:");
                    let agent_items = ["全部", "Codex", "ZCode", "Claude", "OpenCode"];
                    let mut agent_idx = match self.agent.as_str() {
                        "Codex" => 1,
                        "ZCode" => 2,
                        "Claude" => 3,
                        "OpenCode" => 4,
                        _ => 0,
                    };
                    crate::widgets::combo(
                        ui,
                        "agent",
                        agent_items[agent_idx],
                        &agent_items,
                        &mut agent_idx,
                    );
                    self.agent = match agent_idx {
                        1 => "Codex",
                        2 => "ZCode",
                        3 => "Claude",
                        4 => "OpenCode",
                        _ => "all",
                    }
                    .to_string();
                    ui.add_space(12.0);
                    if crate::widgets::button(ui, "刷新") {
                        self.refresh();
                    }
                    ui.add_space(6.0);
                    if crate::widgets::button(ui, "退出") {
                        self.ctx_send_close();
                    }
                });
                ui.add_space(10.0);

                if let Some(rep) = &self.report {
                    crate::chart::draw_chart(ui, rep, &self.agent);
                }
            });

        if self.settings_open {
            crate::settings::show_settings(ctx, self);
        }
    }
}
