mod capture_flow;
mod clipboard_flow;
mod commands;
mod engine;
mod history;
mod input;
mod ocr;
mod lang;
mod settings;
mod tts_win;

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use tauri::menu::{CheckMenuItem, Menu, MenuBuilder, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use tokio_util::sync::CancellationToken;

use settings::Settings;
use tts_win::SharedTts;

pub struct AppState {
    pub settings: RwLock<Settings>,
    /// 每个窗口同一时刻只允许一个进行中的翻译请求
    pub cancels: Mutex<HashMap<String, Arc<CancellationToken>>>,
    /// 弹卡是否钉住(钉住则失焦不隐藏)
    pub pinned: AtomicBool,
    /// 首次创建弹卡时暂存的待翻译文本
    pub pending_popup_text: Mutex<Option<String>>,
    /// 首次创建弹卡时暂存的错误信息(OCR 无结果等)
    pub pending_popup_error: Mutex<Option<String>>,
    pub tts: SharedTts,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::translate,
            commands::cancel_translate,
            commands::get_settings,
            commands::save_settings,
            commands::get_presets,
            commands::set_active_profile,
            commands::test_profile,
            commands::fetch_models,
            commands::toggle_pin,
            commands::take_pending_text,
            commands::take_pending_error,
            commands::ocr_translate,
            commands::hide_capture,
            commands::write_clipboard,
            commands::read_clipboard,
            commands::get_history,
            commands::clear_history,
            commands::hide_popup,
            commands::open_main_with_text,
            commands::open_main_settings,
            commands::tts_speak,
            commands::tts_stop,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            let settings = Settings::load(&handle);
            let setup_done = settings.setup_done;
            app.manage(AppState {
                settings: RwLock::new(settings),
                cancels: Mutex::new(HashMap::new()),
                pinned: AtomicBool::new(false),
                pending_popup_text: Mutex::new(None),
                pending_popup_error: Mutex::new(None),
                tts: Arc::new(Mutex::new(None)),
            });

            setup_tray(&handle).map_err(|e| e.to_string())?;

            let hotkey = {
                let state = handle.state::<AppState>();
                let s = state.settings.read().unwrap();
                s.hotkey.clone()
            };
            if let Err(e) = register_hotkeys(&handle) {
                eprintln!("[litetrans] {e}");
                // 热键失败必须让用户看见:打开主窗口并推送通知
                show_main(&handle);
                use tauri::Emitter;
                let _ = handle.emit(
                    "app-notify",
                    serde_json::json!({ "message": format!("热键注册失败:{e}。请在设置中检查热键设置。") }),
                );
            }
            let _ = hotkey;

            // 启动 30s 后后台预热弹卡窗口,消除首次划词的 1-2s 冷启动
            let h2 = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                let app = h2.clone();
                let value = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Err(e) = clipboard_flow::ensure_popup(&value) {
                        eprintln!("[litetrans] prewarm popup: {e}");
                    }
                });
            });

            // 首次使用(尚未完成配置)时打开主窗口引导
            if !setup_done {
                show_main(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // 主窗口点关闭 = 隐藏到托盘
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                    // 首次关窗给一次性提示
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
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// 注册全部全局热键:划词 + 截图翻译
pub fn register_hotkeys(app: &AppHandle) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;
    let (hk, enabled, cap) = {
        let state = app.state::<AppState>();
        let s = state.settings.read().unwrap();
        (s.hotkey.clone(), s.hotkey_enabled, s.capture_hotkey.clone())
    };
    if enabled && !hk.trim().is_empty() {
        app.global_shortcut()
            .on_shortcut(hk.as_str(), |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = clipboard_flow::capture_selection(&app).await {
                            eprintln!("[litetrans] {e}");
                        }
                    });
                }
            })
            .map_err(|e| format!("注册划词热键 {hk} 失败: {e}"))?;
    }
    if !cap.trim().is_empty() {
        app.global_shortcut()
            .on_shortcut(cap.as_str(), |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = capture_flow::open_capture(&app) {
                            eprintln!("[litetrans] {e}");
                        }
                    });
                }
            })
            .map_err(|e| format!("注册截图热键 {cap} 失败: {e}"))?;
    }
    Ok(())
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
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
                show_main(tray.app_handle());
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
    let clip = MenuItem::with_id(app, "translate_clipboard", "翻译剪贴板内容", true, None::<&str>)?;
    let capture_label = {
        let state = app.state::<AppState>();
        let s = state.settings.read().unwrap().capture_hotkey.clone();
        format!("截图翻译({})", if s.is_empty() { "未设置" } else { s.as_str() })
    };
    let cap = MenuItem::with_id(app, "capture", &capture_label, true, None::<&str>)?;
    let hk = CheckMenuItem::with_id(app, "hotkey_enabled", "启用划词热键", true, hotkey_enabled, None::<&str>)?;
    let conf = MenuItem::with_id(app, "settings", "设置…", true, None::<&str>)?;
    let is_auto = app.autolaunch().is_enabled().unwrap_or(false);
    let auto = CheckMenuItem::with_id(app, "autostart", "开机自启", true, is_auto, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let mut mb = MenuBuilder::new(app).item(&show).item(&clip).item(&cap).separator();
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
    let menu = mb.item(&hk).separator().item(&conf).item(&auto).separator().item(&quit).build()?;
    Ok(menu)
}

fn on_tray_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "show" | "settings" => show_main(app),
        "translate_clipboard" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                match input::read_clipboard_text() {
                    Ok(t) if !t.trim().is_empty() => {
                        let _ = clipboard_flow::show_popup_with_text(&app, t).await;
                    }
                    _ => show_main(&app),
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
                    let hk = state.settings.read().unwrap().hotkey.clone();
                    if let Err(e) = register_hotkeys(&app2) {
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
