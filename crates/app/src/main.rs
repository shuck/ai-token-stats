use ai_token_stats_core::collect::collect;
use ai_token_stats_core::config::Config;
use std::path::PathBuf;

mod bootstrap;

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

fn writable(dir: &PathBuf) -> bool {
    let probe = dir.join(format!(".write-test-{}", std::process::id()));
    match std::fs::write(&probe, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = app_dir();
    let mut cfg = Config::load(&dir.join("config.json")).unwrap_or_default();
    if args.iter().any(|a| a == "-smoke") {
        bootstrap::ensure_discovered(&mut cfg, &dir.join("config.json"));
        let rep = collect(&dir.join("ai-token-stats-cache.db"), &cfg, 30, "all");
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
    println!("ai-token-stats (rust)");
}
