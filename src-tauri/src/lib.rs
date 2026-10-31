//! LiteTrans 后端入口:应用组装、全局状态与热键注册
//!
//! 模块划分:
//! - `tray`:托盘图标与菜单
//! - `clipboard_flow`:划词捕获 → 弹卡
//! - `capture_flow`:截图框选窗口
//! - `ocr`:GDI 抓屏 + Windows OCR
//! - `engine`:AI 翻译引擎(多协议 SSE)
//! - `settings` / `history`:配置与历史持久化
//! - `lang`:语种识别与方向解析
//! - `tts_win`:系统 TTS

mod capture_flow;
mod clipboard_flow;
mod commands;
mod engine;
mod history;
mod input;
mod lang;
mod ocr;
mod settings;
mod tray;
mod tts_win;

pub use tray::rebuild_tray;

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use tauri::{AppHandle, Manager};

use tokio_util::sync::CancellationToken;

use settings::Settings;
use tts_win::SharedTts;

/// 启动后延迟预热弹卡窗口的秒数(消除首次划词的 WebView2 冷启动)
const PREWARM_DELAY_SECS: u64 = 30;

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

            tray::setup_tray(&handle).map_err(|e| e.to_string())?;

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

            // 启动一段时间后后台预热弹卡窗口,消除首次划词的冷启动
            let h2 = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(PREWARM_DELAY_SECS)).await;
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
                    tray::maybe_show_tray_hint(window);
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
