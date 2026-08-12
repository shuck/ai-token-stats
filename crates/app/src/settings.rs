use crate::ui::App;
use ai_token_stats_core::config::{Agent, AgentPath};
use ai_token_stats_core::discovery::validate_agent_path;
use std::path::Path;

const ROWS: [(Agent, &str, bool); 4] = [
    (Agent::Codex, "Codex home 目录", true),
    (Agent::ZCode, "ZCode db.sqlite", false),
    (Agent::Claude, "Claude projects 目录", true),
    (Agent::OpenCode, "OpenCode opencode.db", false),
];

pub fn show_settings(ctx: &egui::Context, app: &mut App) {
    let mut open = true;
    let mut save = false;
    let mut cancel = false;
    let mut paths: Vec<(Agent, String)> = ROWS
        .iter()
        .map(|(agent, _, _)| {
            (
                *agent,
                app.cfg
                    .agents
                    .get(agent)
                    .map(|a| a.path.clone())
                    .unwrap_or_default(),
            )
        })
        .collect();
    let mut error: Option<String> = None;

    egui::Window::new("设置 Agent 路径")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            for (i, (_, label, is_dir)) in ROWS.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(*label);
                    ui.add(egui::TextEdit::singleline(&mut paths[i].1).desired_width(280.0));
                    if ui.button("浏览…").clicked() {
                        let picked = if *is_dir {
                            rfd::FileDialog::new()
                                .set_title("选择路径")
                                .pick_folder()
                        } else {
                            rfd::FileDialog::new()
                                .set_title("选择路径")
                                .pick_file()
                        };
                        if let Some(p) = picked {
                            paths[i].1 = p.to_string_lossy().into_owned();
                        }
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.button("确定").clicked() {
                    save = true;
                }
                if ui.button("取消").clicked() {
                    cancel = true;
                }
            });
        });

    if cancel {
        open = false;
    }

    if save {
        let mut cfg = app.cfg.clone();
        for (agent, path) in &paths {
            if path.is_empty() {
                continue;
            }
            if !validate_agent_path(*agent, Path::new(path)) {
                error = Some(format!(
                    "{} 不存在或不是有效数据源。",
                    ROWS.iter().find(|(a, _, _)| a == agent).map(|(_, l, _)| *l).unwrap_or("")
                ));
                break;
            }
            cfg.agents.insert(
                *agent,
                AgentPath {
                    path: path.clone(),
                    detected_at: chrono::Utc::now().to_rfc3339(),
                },
            );
        }
        if error.is_none() {
            app.cfg = cfg;
            app.cfg.save(&app.dir.join("config.json")).ok();
            app.refresh();
            open = false;
        }
    }

    if !open {
        app.settings_open = false;
    }

    if let Some(msg) = error {
        egui::Window::new("路径无效").collapsible(false).resizable(false).show(ctx, |ui| {
            ui.label(msg);
            if ui.button("关闭").clicked() {
                // 关闭由 Window 的关闭按钮处理
            }
        });
    }
}
