use crate::ui::App;
use egui::Context;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuItem};
use tray_icon::{Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    Refresh,
    Rescan,
    Settings,
    Exit,
}

pub struct TrayState {
    pub pending_action: Mutex<Option<Action>>,
}

pub fn create_tray(ctx: Context) -> (Arc<TrayState>, TrayIcon) {
    let open = MenuItem::new("打开面板", true, None);
    let refresh = MenuItem::new("刷新", true, None);
    let rescan = MenuItem::new("重新扫描路径", true, None);
    let settings = MenuItem::new("设置 Agent 路径…", true, None);
    let exit = MenuItem::new("退出", true, None);
    let menu = Menu::new();
    for item in [&open, &refresh, &rescan, &settings, &exit] {
        let _ = menu.append(item);
    }

    let rgba = make_icon_rgba();
    let icon = Icon::from_rgba(rgba, 32, 32).expect("icon");
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip("AI Token 统计")
        .with_icon(icon)
        .build()
        .expect("tray");

    let state = Arc::new(TrayState {
        pending_action: Mutex::new(None),
    });

    let s = state.clone();
    let open_id = open.id().clone();
    let refresh_id = refresh.id().clone();
    let rescan_id = rescan.id().clone();
    let settings_id = settings.id().clone();
    let exit_id = exit.id().clone();
    std::thread::spawn(move || loop {
        if let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            let action = if event.id == open_id {
                Some(Action::Open)
            } else if event.id == refresh_id {
                Some(Action::Refresh)
            } else if event.id == rescan_id {
                Some(Action::Rescan)
            } else if event.id == settings_id {
                Some(Action::Settings)
            } else if event.id == exit_id {
                Some(Action::Exit)
            } else {
                None
            };
            if let Some(a) = action {
                *s.pending_action.lock().unwrap() = Some(a);
                ctx.request_repaint();
            }
        }
        if let Ok(TrayIconEvent::Click {
            button: MouseButton::Left,
            ..
        }) = TrayIconEvent::receiver().try_recv()
        {
            // tray-icon 在 Windows 上不处理 WM_LBUTTONDBLCLK，
            // 双击事件时序不可靠；改为任意左键点击即打开面板（双击同样生效）。
            *s.pending_action.lock().unwrap() = Some(Action::Open);
            ctx.request_repaint();
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    });
    (state, tray)
}

fn make_icon_rgba() -> Vec<u8> {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
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
    rgba
}

pub fn poll_events(app: &mut App) {
    if let Some(state) = app.tray.as_ref() {
        let action = state.pending_action.lock().unwrap().take();
        match action {
            Some(Action::Open) => {
                crate::logging::log_msg("tray action: open");
                app.ctx_send_visible(true);
            }
            Some(Action::Refresh) => {
                crate::logging::log_msg("tray action: refresh");
                app.refresh();
            }
            Some(Action::Rescan) => {
                crate::logging::log_msg("tray action: rescan");
                crate::bootstrap::ensure_discovered_force(
                    &mut app.cfg,
                    &app.dir.join("config.json"),
                );
                app.refresh();
            }
            Some(Action::Settings) => {
                crate::logging::log_msg("tray action: settings");
                app.settings_open = true;
            }
            Some(Action::Exit) => {
                crate::logging::log_msg("tray action: exit");
                app.ctx_send_close();
            }
            None => {}
        }
    }
}
