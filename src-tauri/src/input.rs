use std::borrow::Cow;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// 等待物理修饰键松开的上限。热键是 Alt+Q,正常松手在 100ms 内;
/// 留足余量以防用户按住不放,超时就照常发(至少补发了 KEYUP)。
const MODIFIER_WAIT_MS: u64 = 260;

/// 划词前保存的剪贴板内容,用于事后恢复
pub struct SavedClipboard {
    pub seq: u32,
    pub text: Option<String>,
    pub image: Option<(usize, usize, Vec<u8>)>,
}

pub fn cursor_pos() -> (i32, i32) {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
    }
    (p.x, p.y)
}

/// 光标所在显示器的工作区 (left, top, right, bottom),用于把弹卡限制在屏幕内
pub fn work_area_at(x: i32, y: i32) -> (i32, i32, i32, i32) {
    unsafe {
        let hmon = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            (
                mi.rcWork.left,
                mi.rcWork.top,
                mi.rcWork.right,
                mi.rcWork.bottom,
            )
        } else {
            (0, 0, 1920, 1080)
        }
    }
}

fn key_event(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// 释放修饰键后模拟一次 Ctrl+C,把前台选中文本复制进剪贴板
pub fn press_ctrl_c() -> Result<(), String> {
    // 关键:用户是按着 Alt 触发热键的。即使我们补发 Alt 的 KEYUP,
    // 物理键仍处于按下状态,系统随后会把 Alt 状态重新同步给目标应用,
    // 目标应用收到的就是 Alt+Ctrl+C(IDEA 里是另一个命令,浏览器里是
    // "检查元素"之类),于是"选中了却复制不到东西"。
    // 解法:先等物理修饰键真的松开(最多等 MODIFIER_WAIT_MS)。
    //
    // 用 sleep 而不是忙等:热键回调跑在 tokio 工作线程上,空转会把
    // 同线程上的其它任务一起拖住。
    let deadline = Instant::now() + Duration::from_millis(MODIFIER_WAIT_MS);
    while modifier_still_down() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }

    let ups: [INPUT; 5] =
        [VK_MENU, VK_LWIN, VK_RWIN, VK_SHIFT, VK_CONTROL].map(|vk| key_event(vk, true));
    unsafe { SendInput(&ups, std::mem::size_of::<INPUT>() as i32) };
    // 补发 KEYUP 之后还要给目标应用一点时间处理,否则它可能仍在 Alt 态
    std::thread::sleep(Duration::from_millis(60));

    let seq = [
        key_event(VK_CONTROL, false),
        key_event(VIRTUAL_KEY(0x43), false), // 'C'
        key_event(VIRTUAL_KEY(0x43), true),
        key_event(VK_CONTROL, true),
    ];
    let sent = unsafe { SendInput(&seq, std::mem::size_of::<INPUT>() as i32) };
    if sent != seq.len() as u32 {
        return Err("模拟按键被系统拦截(若目标应用以管理员运行,LiteTrans 也需以管理员运行)".into());
    }
    Ok(())
}

/// 物理修饰键(Alt/Win/Shift/Ctrl)是否还有按下的
fn modifier_still_down() -> bool {
    let keys = [VK_MENU, VK_LWIN, VK_RWIN, VK_SHIFT, VK_CONTROL];
    keys.iter().any(|&vk| {
        // 最高位为 1 表示当前按下
        unsafe { (GetAsyncKeyState(vk.0 as i32) as u16) & 0x8000 != 0 }
    })
}

/// 当前系统剪贴板序列号
pub fn clipboard_seq() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

/// 等待剪贴板序列号变化(目标应用完成复制)
pub async fn wait_clipboard_change(prev_seq: u32, timeout_ms: u64) -> bool {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        let cur = unsafe { GetClipboardSequenceNumber() };
        if prev_seq != cur {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

pub fn save_clipboard() -> SavedClipboard {
    // 必须在 arboard 之前取:arboard::Clipboard::new() 在某些实现里会
    // 打开/占用剪贴板,可能自身就推进序列号,导致随后"变化"被误判为
    // 目标应用已复制(实测基线要取最早的)。
    let seq = unsafe { GetClipboardSequenceNumber() };
    let mut saved = SavedClipboard {
        seq,
        text: None,
        image: None,
    };
    if let Ok(mut clip) = arboard::Clipboard::new() {
        saved.text = clip.get_text().ok();
        if saved.text.is_none() {
            if let Ok(img) = clip.get_image() {
                saved.image = Some((img.width, img.height, img.bytes.into_owned()));
            }
        }
    }
    saved
}

/// 读取文本后延迟恢复用户原有剪贴板内容
pub fn restore_clipboard_later(saved: SavedClipboard) {
    if saved.text.is_none() && saved.image.is_none() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        if let Ok(mut clip) = arboard::Clipboard::new() {
            if let Some(t) = saved.text {
                let _ = clip.set_text(t);
            } else if let Some((w, h, b)) = saved.image {
                let _ = clip.set_image(arboard::ImageData {
                    width: w,
                    height: h,
                    bytes: Cow::Owned(b),
                });
            }
        }
    });
}

pub fn read_clipboard_text() -> Result<String, String> {
    let mut clip = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clip.get_text().map_err(|e| e.to_string())
}
