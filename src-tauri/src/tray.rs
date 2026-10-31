//! 托盘图标与菜单:主窗口入口、剪贴板翻译、截图翻译、配置切换、热键开关、自启

use tauri::menu::{CheckMenuItem, Menu, MenuBuilder, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::{capture_flow, clipboard_flow, input, AppState};

/// 首次把主窗口关到托盘时给一次性提示
pub fn maybe_show_tray_hint(window: &tauri::Window) {
    let app = window.app_handle().clone();
    let first = {
        let state = app.state::<AppState>();
        let mut s = state.settings.write().unwrap();
        if s.tray_hint_shown {
            false
        } else {
            s.tray_hint_shown = true;
            true
        }
    };
    if first {
        tauri::async_runtime::spawn(async move {
            use tauri::Emitter;
            let _ = app.emit_to("main", "tray-hint", ());
            let state = app.state::<AppState>();
            let snapshot = {
                let s = state.settings.read().unwrap();
                s.clone()
            };
            let _ = snapshot.save(&app);
        });
    }
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_tray_menu(app)?;

    TrayIconBuilder::with_id("tray")
        .icon(tauri::include_image!("icons/32x32.png"))
        .tooltip("LiteTrans · AI 划词翻译")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(on_tray_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// 配置变化 / 切换后重建托盘菜单
pub fn rebuild_tray(app: &AppHandle) {
    let _ = app.remove_tray_by_id("tray");
    if let Err(e) = setup_tray(app) {
        eprintln!("[litetrans] rebuild tray: {e}");
    }
}

fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    use tauri_plugin_autostart::ManagerExt;

    let (profiles, active_id, hotkey_enabled) = {
        let state = app.state::<AppState>();
        let s = state.settings.read().unwrap();
        (
            s.profiles.clone(),
            s.active_profile().map(|p| p.id.clone()).unwrap_or_default(),
            s.hotkey_enabled,
        )
    };

    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let clip = MenuItem::with_id(
        app,
        "translate_clipboard",
        "翻译剪贴板内容",
        true,
        None::<&str>,
    )?;
    let capture_label = {
        let state = app.state::<AppState>();
        let s = state.settings.read().unwrap().capture_hotkey.clone();
        format!(
            "截图翻译({})",
            if s.is_empty() {
                "未设置"
            } else {
                s.as_str()
            }
        )
    };
    let cap = MenuItem::with_id(app, "capture", &capture_label, true, None::<&str>)?;
    let hk = CheckMenuItem::with_id(
        app,
        "hotkey_enabled",
        "启用划词热键",
        true,
        hotkey_enabled,
        None::<&str>,
    )?;
    let conf = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
    let is_auto = app.autolaunch().is_enabled().unwrap_or(false);
    let auto = CheckMenuItem::with_id(app, "autostart", "开机自启", true, is_auto, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let mut mb = MenuBuilder::new(app)
        .item(&show)
        .item(&clip)
        .item(&cap)
        .separator();
    if !profiles.is_empty() {
        for p in &profiles {
            let label = if p.model.trim().is_empty() {
                p.name.clone()
            } else {
                format!("{}  ·  {}", p.name, p.model.trim())
            };
            let item = CheckMenuItem::with_id(
                app,
                format!("profile:{}", p.id),
                label,
                true,
                p.id == active_id,
                None::<&str>,
            )?;
            mb = mb.item(&item);
        }
        mb = mb.separator();
    }
    let menu = mb
        .item(&hk)
        .separator()
        .item(&conf)
        .item(&auto)
        .separator()
        .item(&quit)
        .build()?;
    Ok(menu)
}

fn on_tray_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "show" | "settings" => crate::show_main(app),
        "translate_clipboard" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                match input::read_clipboard_text() {
                    Ok(t) if !t.trim().is_empty() => {
                        let _ = clipboard_flow::show_popup_with_text(&app, t).await;
                    }
                    _ => crate::show_main(&app),
                }
            });
        }
        "hotkey_enabled" => {
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app2.state::<AppState>();
                let new_enabled = {
                    let mut s = match state.settings.write() {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    s.hotkey_enabled = !s.hotkey_enabled;
                    let en = s.hotkey_enabled;
                    let _ = s.save(&app2);
                    en
                };
                if new_enabled {
                    if let Err(e) = crate::register_hotkeys(&app2) {
                        eprintln!("[litetrans] re-enable hotkey: {e}");
                    }
                } else {
                    use tauri_plugin_global_shortcut::GlobalShortcutExt;
                    let _ = app2.global_shortcut().unregister_all();
                }
                let a = app2.clone();
                let b = a.clone();
                let _ = a.run_on_main_thread(move || rebuild_tray(&b));
            });
        }
        "capture" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = capture_flow::open_capture(&app) {
                    eprintln!("[litetrans] {e}");
                }
            });
        }
        "autostart" => {
            use tauri_plugin_autostart::ManagerExt;
            let al = app.autolaunch();
            let enabled = if al.is_enabled().unwrap_or(false) {
                let _ = al.disable();
                false
            } else {
                let _ = al.enable();
                true
            };
            // 同步内存与磁盘,并刷新托盘勾选态
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app2.state::<AppState>();
                if let Ok(mut s) = state.settings.write() {
                    s.autostart = enabled;
                    let _ = s.save(&app2);
                }
                let a = app2.clone();
                let b = a.clone();
                let _ = a.run_on_main_thread(move || rebuild_tray(&b));
            });
        }
        "quit" => app.exit(0),
        other => {
            if let Some(id) = other.strip_prefix("profile:") {
                let id = id.to_string();
                let app2 = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app2.state::<AppState>();
                    let ok = {
                        let mut s = match state.settings.write() {
                            Ok(s) => s,
                            Err(_) => return,
                        };
                        if s.profiles.iter().any(|p| p.id == id) {
                            s.active_id = id;
                            s.save(&app2).is_ok()
                        } else {
                            false
                        }
                    };
                    if ok {
                        let a = app2.clone();
                        let b = a.clone();
                        let _ = a.run_on_main_thread(move || rebuild_tray(&b));
                    }
                });
            }
        }
    }
}
