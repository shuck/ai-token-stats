use crate::ui::App;
use egui::Context;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

const DOUBLE_CLICK_MS: u128 = 500;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    Refresh,
    Rescan,
    Settings,
    Exit,
}

pub struct TrayState {
    pub last_click: Mutex<std::time::Instant>,
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
        .with_tooltip("AI Token 统计")
        .with_icon(icon)
        .build()
        .expect("tray");

    let state = Arc::new(TrayState {
        last_click: Mutex::new(std::time::Instant::now()),
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
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click { button, button_state, .. } = event {
                if button == MouseButton::Left && button_state == MouseButtonState::Up {
                    let now = std::time::Instant::now();
                    let mut last = s.last_click.lock().unwrap();
                    let dbl = last.elapsed().as_millis() <= DOUBLE_CLICK_MS;
                    *last = now;
                    if dbl {
                        *s.pending_action.lock().unwrap() = Some(Action::Open);
                        ctx.request_repaint();
                    }
                }
            }
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
            Some(Action::Open) => app.ctx_send_visible(true),
            Some(Action::Refresh) => app.refresh(),
            Some(Action::Rescan) => {
                crate::bootstrap::ensure_discovered_force(
                    &mut app.cfg,
                    &app.dir.join("config.json"),
                );
                app.refresh();
            }
            Some(Action::Settings) => app.settings_open = true,
            Some(Action::Exit) => app.ctx_send_close(),
            None => {}
        }
    }
}
