use ai_token_stats_core::config::{Agent, AgentPath, Config};
use ai_token_stats_core::discovery::{
    discover_agent_path, scan_roots, validate_agent_path, ScanLimits,
};
use std::path::Path;

pub fn ensure_discovered(cfg: &mut Config, config_path: &Path) {
    crate::logging::log_msg(&format!("ensure_discovered before cfg={cfg:?}"));
    let changed = discover_missing(cfg);
    crate::logging::log_msg(&format!(
        "ensure_discovered after cfg={cfg:?} changed={changed}"
    ));
    if changed {
        if let Err(e) = cfg.save(config_path) {
            crate::logging::log_msg(&format!("config save failed: {e}"));
        }
    }
}

pub fn ensure_discovered_force(cfg: &mut Config, config_path: &Path) {
    let roots = scan_roots();
    let limits = ScanLimits::default();
    let mut changed = false;
    for agent in Agent::ALL {
        if let Some(p) = discover_agent_path(agent, &roots, &limits) {
            cfg.agents.insert(
                agent,
                AgentPath {
                    path: p.to_string_lossy().into_owned(),
                    detected_at: chrono::Utc::now().to_rfc3339(),
                },
            );
            changed = true;
        }
    }
    if changed {
        if let Err(e) = cfg.save(config_path) {
            crate::logging::log_msg(&format!("force rescan config save failed: {e}"));
        }
    }
}

fn discover_missing(cfg: &mut Config) -> bool {
    let roots = scan_roots();
    let limits = ScanLimits::default();
    let mut changed = false;
    for agent in Agent::ALL {
        let valid = cfg
            .agents
            .get(&agent)
            .map(|a| validate_agent_path(agent, Path::new(&a.path)))
            .unwrap_or(false);
        if valid {
            continue;
        }
        if let Some(p) = discover_agent_path(agent, &roots, &limits) {
            cfg.agents.insert(
                agent,
                AgentPath {
                    path: p.to_string_lossy().into_owned(),
                    detected_at: chrono::Utc::now().to_rfc3339(),
                },
            );
            changed = true;
        }
    }
    changed
}
