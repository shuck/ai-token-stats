#![windows_subsystem = "windows"]

use ai_token_stats_core::collect::collect;
use ai_token_stats_core::config::Config;
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod bootstrap;
mod chart;
mod logging;
mod settings;
mod tray;
mod ui;

fn app_dir() -> PathBuf {
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        if writable(&dir) {
            return dir;
        }
    }
    let fallback = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("ai-token-stats");
    std::fs::create_dir_all(&fallback).ok();
    fallback
}

fn writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".write-test-{}", std::process::id()));
    match std::fs::write(&probe, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn single_instance() -> bool {
    use windows_sys::Win32::System::Threading::CreateMutexW;
    let name: Vec<u16> = "Global\\AITokenStatsTray"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return false;
    }
    std::io::Error::last_os_error().raw_os_error() != Some(183) // ERROR_ALREADY_EXISTS
}

fn make_icon_data() -> egui::IconData {
    let w = 32usize;
    let h = 32usize;
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let (r, g, b) = if (5..12).contains(&x) && (12..27).contains(&y) {
                (190, 220, 255)
            } else if (13..20).contains(&x) && (6..27).contains(&y) {
                (255, 255, 255)
            } else if (21..28).contains(&x) && (16..27).contains(&y) {
                (190, 220, 255)
            } else {
                (20, 90, 220)
            };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    egui::IconData {
        rgba,
        width: w as u32,
        height: h as u32,
    }
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\msyh.ttc") {
        fonts
            .font_data
            .insert("msyh".to_owned(), egui::FontData::from_owned(bytes));
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, "msyh".to_owned());
        }
    }
    ctx.set_fonts(fonts);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = app_dir();
    logging::init(&dir);
    logging::log_msg(&format!(
        "startup args={args:?} app_dir={}",
        dir.to_string_lossy()
    ));
    let cfg = Config::load(&dir.join("config.json")).unwrap_or_default();
    if args.iter().any(|a| a == "-smoke") {
        let mut cfg = cfg;
        bootstrap::ensure_discovered(&mut cfg, &dir.join("config.json"));
        logging::log_msg(&format!("smoke config={cfg:?}"));
        let rep = collect(&dir.join("ai-token-stats-cache.db"), &cfg, 30, "all");
        logging::log_msg(&format!(
            "smoke result turns={} agents={:?} models={:?}",
            rep.totals.turns, rep.agents, rep.models
        ));
        println!(
            "SMOKE OK: days={} turns={} agents={:?} models={:?}",
            rep.days, rep.totals.turns, rep.agents, rep.models
        );
        for model in &rep.models {
            if let Some(md) = rep.totals.by_model.get(model) {
                println!(
                    "  {model}: total={} input={} cached={}",
                    md.total, md.input, md.cached
                );
            }
        }
        for agent in &rep.agents {
            if let Some(ad) = rep.totals.by_agent.get(agent) {
                println!("  [{agent}] total={} turns={}", ad.total, ad.turns);
            }
        }
        return;
    }
    if args.iter().any(|a| a == "-hold") {
        std::thread::sleep(std::time::Duration::from_secs(5));
        return;
    }
    if !single_instance() {
        logging::log_msg("single instance already running, exiting");
        return;
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 600.0])
            .with_icon(Arc::new(make_icon_data())),
        ..Default::default()
    };
    let mut app = ui::App::new(dir, cfg);
    let _ = eframe::run_native(
        "AI Token 统计",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            let (state, icon) = tray::create_tray(cc.egui_ctx.clone());
            app.tray = Some(state);
            app._tray_icon = Some(icon);
            Box::new(app)
        }),
    );
}
