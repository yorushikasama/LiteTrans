use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::input;
use crate::AppState;

/// 弹卡显示后多久之内的失焦事件不算"用户点了别处"。
/// 需要覆盖"show → 系统补发一次 Focused(false)"这段窗口期。
const SHOW_GRACE_MS: u128 = 350;

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// 热键划词:模拟 Ctrl+C → 读剪贴板 → 光标附近弹卡
pub async fn capture_selection(app: &AppHandle) -> Result<(), String> {
    // 弹卡自身持有焦点时不触发,避免自己复制自己
    if let Some(p) = app.get_webview_window("popup") {
        if p.is_focused().unwrap_or(false) {
            return Err("翻译弹卡当前处于焦点状态:先在别的窗口选中文字,再按热键".into());
        }
    }

    // 单飞:等待目标应用写入剪贴板最多 3 秒,这期间用户很可能以为没反应
    // 又按一次热键。两路并发会互相踩剪贴板(后一路的 save/restore 会覆盖
    // 前一路刚拿到的文本),所以新的触发直接忽略。
    {
        let state = app.state::<AppState>();
        if state.capturing.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
    }
    let result = capture_selection_inner(app).await;
    app.state::<AppState>()
        .capturing
        .store(false, Ordering::SeqCst);
    result
}

async fn capture_selection_inner(app: &AppHandle) -> Result<(), String> {
    // save_clipboard 会打开系统剪贴板并读内容,属于阻塞调用;
    // press_ctrl_c 里还有"等物理修饰键松开"的等待。都放到阻塞线程池,
    // 避免占住 tokio 工作线程(否则热键响应会被拖慢到数秒)。
    let mut saved = tauri::async_runtime::spawn_blocking(input::save_clipboard)
        .await
        .map_err(|e| e.to_string())?;
    // save_clipboard 里的 arboard 会打开剪贴板,可能自身推进序列号。
    // 这里重新取一次基线,确保它反映的是"按下 Ctrl+C 之前"的真实状态,
    // 否则会把本地那次打开误判成"目标应用已复制"。
    saved.seq = input::clipboard_seq();
    tauri::async_runtime::spawn_blocking(input::press_ctrl_c)
        .await
        .map_err(|e| e.to_string())??;
    // 必须等待剪贴板真的变化:不判断的话,模拟 Ctrl+C 未生效时会读到
    // 用户上一次复制的旧内容,并把它当成"选中的文本"翻译,表现为译错内容。
    //
    // 等待上限必须放得很宽。实测(本机,IDEA 打开一个 Java 文件、Ctrl+A 全选):
    // 剪贴板在 Ctrl+C 之后 **1893ms** 才更新;原先的 400ms 必然超时,
    // 用户看到的就是"明明选中了却说没取到文本"。浏览器里选中大段文字同理。
    // wait_clipboard_change 一检测到变化就返回,放宽上限不增加正常路径的延迟。
    let changed = input::wait_clipboard_change(saved.seq, 3000).await;
    let text = input::read_clipboard_text().unwrap_or_default();
    input::restore_clipboard_later(saved);

    if !changed {
        return Err("没有取到新的选中文本。请确认:\n\
             · 目标窗口里确实选中了文字(蓝色高亮)\n\
             · 目标应用没把 Ctrl+C 改绑成别的功能(某些终端、游戏、看图软件)\n\
             图片、PDF、视频字幕这类选不中的内容,请改用截图翻译(默认 Alt+X)。"
            .into());
    }
    if text.trim().is_empty() {
        return Err("选中内容为空,请重新选中文字后再按热键".into());
    }
    // 剪贴板内容与原文相同(应用忽略 Ctrl+C 复制了别的东西)时也提示,
    // 避免把无关内容当成译文来源
    show_popup_with_text(app, text).await
}

/// 在光标附近显示弹卡并触发翻译
pub async fn show_popup_with_text(app: &AppHandle, text: String) -> Result<(), String> {
    show_popup_payload(app, serde_json::json!({ "text": text }), "do-translate").await
}

/// 在光标附近显示弹卡但只展示错误信息(如 OCR 无结果)
pub async fn show_popup_with_error(app: &AppHandle, message: String) -> Result<(), String> {
    show_popup_payload(
        app,
        serde_json::json!({ "message": message }),
        "translate-error",
    )
    .await
}

async fn show_popup_payload(
    app: &AppHandle,
    payload: serde_json::Value,
    event: &str,
) -> Result<(), String> {
    let existed = app.get_webview_window("popup").is_some();
    let win = ensure_popup(app)?;

    let (cx, cy) = input::cursor_pos();
    let (l, t, r, b) = input::work_area_at(cx, cy);
    let size = win.outer_size().unwrap_or_default();
    let w = if size.width > 0 {
        size.width as f64
    } else {
        420.0
    };
    let h = if size.height > 0 {
        size.height as f64
    } else {
        330.0
    };
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
    // 先记录显示时刻再 show:窗口事件可能同步派发,写晚了会被当成"已经显示很久"
    app.state::<AppState>()
        .popup_shown_at
        .store(now_ms() as u64, Ordering::Relaxed);
    win.show().map_err(|e| e.to_string())?;
    // 抢焦点:Windows 的前台锁定策略会让后台进程的 set_focus 静默失败。
    // 失败也不强求 —— 弹卡始终可见,并且下面挂了"点外部即隐藏"的兜底监视。
    let _ = win.set_focus();
    spawn_outside_click_watcher(app, win.clone());

    if existed {
        // 弹卡页面已就绪,直接发事件
        app.emit_to("popup", event, payload)
            .map_err(|e| e.to_string())?;
    } else {
        // 首次创建:页面尚未加载完成,由前端挂载后主动取
        let state = app.state::<AppState>();
        if event == "do-translate" {
            *state.pending_popup_text.lock().unwrap() = payload
                .get("text")
                .and_then(|t| t.as_str())
                .map(String::from);
        } else {
            *state.pending_popup_error.lock().unwrap() = payload
                .get("message")
                .and_then(|t| t.as_str())
                .map(String::from);
        }
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
        .build()
    {
        Ok(w) => w,
        // 并发热键可能重复创建,已有则复用
        Err(_) => app.get_webview_window("popup").ok_or("创建翻译弹卡失败")?,
    };

    let w = win.clone();
    win.on_window_event(move |event| {
        // 只关心失焦:点窗口外 = 弹卡失去焦点 = 隐藏。
        // 点窗口本体不会让弹卡失焦,所以什么也不会发生 —— 这正是要的行为。
        if let tauri::WindowEvent::Focused(false) = event {
            let state = w.app_handle().state::<AppState>();
            if state.pinned.load(Ordering::Relaxed) {
                return;
            }
            // 鼠标还按着就失焦,是「正在拖动窗口」而不是「点了别处」。
            //
            // 拖动顶条时 Tauri 走 startDragging,Windows 会在这期间让窗口
            // 短暂失去焦点;照字面执行 hide 的后果是用户一按下去弹卡就没了
            // —— 实测日志:press inside=true → Focused(false) down=true
            // → 隐藏。点一下(按下随即松开)不会触发这条,所以
            // "点击窗口本体不消失"一直是对的,只有拖动会踩到。
            if input::mouse_button_down() {
                return;
            }
            // 刚显示后的一小段时间内忽略失焦:窗口还没真正进入屏幕时,
            // 系统会补发一次 Focused(false),照做会把弹卡瞬间藏起来
            // (实测 Alt+Q 后剪贴板变了、窗口也建了,但 IsWindowVisible
            // 一直是 False,用户看到的就是"弹卡不出来")。
            let shown_at = state.popup_shown_at.load(Ordering::Relaxed);
            if shown_at != 0 && now_ms().saturating_sub(shown_at as u128) < SHOW_GRACE_MS {
                return;
            }
            let _ = w.hide();
        }
    });
    Ok(win)
}

/// 弹卡显示期间盯着鼠标:在窗口**外面**按下就隐藏,在窗口里面按下什么都不做。
///
/// 为什么不能只靠失焦事件:Windows 的前台锁定策略会让后台进程的 set_focus
/// 静默失败。此时弹卡以 always_on_top 显示出来,却从未拥有焦点 ——
/// 用户点窗口外,系统不会产生任何 Focused(false),弹卡就赖着不走。
/// 这个轮询补上那条路径,同时保证"点窗口本体不触发消失"。
pub fn spawn_outside_click_watcher(app: &AppHandle, win: WebviewWindow) {
    let app = app.clone();
    let generation = app
        .state::<AppState>()
        .popup_watch_gen
        .fetch_add(1, Ordering::SeqCst)
        + 1;

    tauri::async_runtime::spawn(async move {
        // 等一小会儿再开始:热键过后用户可能还在按着鼠标,别把旧按键当新点击
        tokio::time::sleep(std::time::Duration::from_millis(SHOW_GRACE_MS as u64)).await;

        loop {
            // 每次唤起都会起一个新的 watcher;旧的发现代号变了就自行退出,
            // 否则连续按热键会让多个 watcher 同时盯着同一个窗口。
            if app
                .state::<AppState>()
                .popup_watch_gen
                .load(Ordering::SeqCst)
                != generation
            {
                break;
            }
            // 弹卡不可见(或已销毁)就结束监视,下次唤起会重新起一个
            if !win.is_visible().unwrap_or(false) {
                break;
            }
            if app.state::<AppState>().pinned.load(Ordering::Relaxed) {
                break;
            }

            if input::mouse_button_down() {
                let (cx, cy) = input::cursor_pos();
                if !cursor_inside(&win, cx, cy) {
                    let _ = win.hide();
                    break;
                }
                // 在窗口内部按下:等这一次点击结束,避免拖拽/连点期间反复判断。
                // 拖动窗口时鼠标会一路移出边界,但按键还没松开,这段等待
                // 保证拖动过程不会被误判成"点了外面"。
                while input::mouse_button_down() {
                    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        }
    });
}

/// 光标是否落在窗口的可见矩形内(用外框,含边框与圆角外的极小误差)
fn cursor_inside(win: &WebviewWindow, x: i32, y: i32) -> bool {
    let Ok(pos) = win.outer_position() else {
        return false;
    };
    let Ok(size) = win.outer_size() else {
        return false;
    };
    let (l, t) = (pos.x, pos.y);
    let (r, b) = (l + size.width as i32, t + size.height as i32);
    x >= l && x < r && y >= t && y < b
}
