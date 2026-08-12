use ai_token_stats_core::config::Config;
use std::path::PathBuf;

pub struct App {
    pub dir: PathBuf,
    pub cfg: Config,
}

impl App {
    pub fn new(dir: PathBuf, cfg: Config) -> Self {
        App { dir, cfg }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| ui.label("AI Token 统计"));
        });
    }
}
