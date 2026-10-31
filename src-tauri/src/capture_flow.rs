//! 截图翻译框选窗口:覆盖主显示器的全屏遮罩,拖拽框选后交给 OCR

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::ocr;

/// 惰性创建框选窗口(隐藏,覆盖主显示器)
pub fn ensure_capture(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(w) = app.get_webview_window("capture") {
        return Ok(w);
    }
    let (w, h) = ocr::primary_screen();
    let win =
        match WebviewWindowBuilder::new(app, "capture", WebviewUrl::App("capture.html".into()))
            .title("LiteTrans Capture")
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .focused(true)
            .visible(false)
            .position(0.0, 0.0)
            .inner_size(w as f64, h as f64)
            .build()
        {
            Ok(w) => w,
            Err(_) => app
                .get_webview_window("capture")
                .ok_or("创建截图窗口失败")?,
        };
    // 物理像素精确对齐主屏(逻辑坐标在高 DPI 下会错位)
    let _ = win.set_size(PhysicalSize::new(w as u32, h as u32));
    let _ = win.set_position(PhysicalPosition::new(0i32, 0i32));

    let w2 = win.clone();
    win.on_window_event(move |event| {
        // 失焦即取消框选
        if let tauri::WindowEvent::Focused(false) = event {
            let _ = w2.hide();
        }
    });
    Ok(win)
}

/// 打开框选窗口(每次唤起都重置前端状态)
pub fn open_capture(app: &AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    let win = ensure_capture(app)?;
    let _ = win.show();
    let _ = win.set_focus();
    app.emit_to("capture", "capture-reset", ())
        .map_err(|e| e.to_string())
}

pub fn hide_capture(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("capture") {
        let _ = w.hide();
    }
}
