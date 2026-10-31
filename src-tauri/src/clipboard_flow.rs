use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::input;
use crate::AppState;

/// 热键划词:模拟 Ctrl+C → 读剪贴板 → 光标附近弹卡
pub async fn capture_selection(app: &AppHandle) -> Result<(), String> {
    // 弹卡自身持有焦点时不触发,避免自己复制自己
    if let Some(p) = app.get_webview_window("popup") {
        if p.is_focused().unwrap_or(false) {
            return Ok(());
        }
    }

    let saved = input::save_clipboard();
    input::press_ctrl_c()?;
    input::wait_clipboard_change(saved.seq, 400).await;
    let text = input::read_clipboard_text().unwrap_or_default();
    input::restore_clipboard_later(saved);

    if text.trim().is_empty() {
        return Err("未获取到选中文本(目标应用可能不支持 Ctrl+C,或当前没有选中文字)".into());
    }
    show_popup_with_text(app, text).await
}

/// 在光标附近显示弹卡并触发翻译
pub async fn show_popup_with_text(app: &AppHandle, text: String) -> Result<(), String> {
    let existed = app.get_webview_window("popup").is_some();
    let win = ensure_popup(app)?;

    let (cx, cy) = input::cursor_pos();
    let (l, t, r, b) = input::work_area_at(cx, cy);
    let size = win.outer_size().unwrap_or_default();
    let w = if size.width > 0 { size.width as f64 } else { 420.0 };
    let h = if size.height > 0 { size.height as f64 } else { 330.0 };
    let mut px = cx as f64 + 16.0;
    let mut py = cy as f64 + 16.0;
    if px + w > r as f64 {
        px = r as f64 - w - 8.0;
    }
    if py + h > b as f64 {
        py = (cy as f64 - 16.0 - h).max(t as f64 + 8.0);
    }
    if px < l as f64 {
        px = l as f64 + 8.0;
    }
    if py < t as f64 {
        py = t as f64 + 8.0;
    }
    win.set_position(PhysicalPosition::new(px as i32, py as i32))
        .map_err(|e| e.to_string())?;
    win.show().map_err(|e| e.to_string())?;
    let _ = win.set_focus();

    if existed {
        // 弹卡页面已就绪,直接发事件
        app.emit_to("popup", "do-translate", serde_json::json!({ "text": text }))
            .map_err(|e| e.to_string())?;
    } else {
        // 首次创建:页面尚未加载完成,由前端挂载后主动取
        let state = app.state::<AppState>();
        *state.pending_popup_text.lock().unwrap() = Some(text);
    }
    Ok(())
}

/// 惰性创建弹卡窗口(创建一次,之后隐藏/复用)
pub fn ensure_popup(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(w) = app.get_webview_window("popup") {
        return Ok(w);
    }
    let win = match WebviewWindowBuilder::new(app, "popup", WebviewUrl::App("popup.html".into()))
        .title("LiteTrans")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(true)
        .visible(false)
        .inner_size(420.0, 330.0)
        .effects(tauri_utils::config::WindowEffectsConfig {
            effects: vec![tauri::window::Effect::Acrylic],
            state: Some(tauri::window::EffectState::Active),
            radius: None,
            color: None,
            interactive: false,
        })
        .build()
    {
        Ok(w) => w,
        // 并发热键可能重复创建,已有则复用
        Err(_) => app.get_webview_window("popup").ok_or("创建翻译弹卡失败")?,
    };

    let w = win.clone();
    win.on_window_event(move |event| {
        // 失焦自动隐藏(钉住时除外)
        if let tauri::WindowEvent::Focused(false) = event {
            let pinned = w
                .app_handle()
                .state::<AppState>()
                .pinned
                .load(std::sync::atomic::Ordering::Relaxed);
            if !pinned {
                let _ = w.hide();
            }
        }
    });
    Ok(win)
}
