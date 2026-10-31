use std::borrow::Cow;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

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
    let ups: [INPUT; 5] =
        [VK_MENU, VK_LWIN, VK_RWIN, VK_SHIFT, VK_CONTROL].map(|vk| key_event(vk, true));
    unsafe { SendInput(&ups, std::mem::size_of::<INPUT>() as i32) };
    std::thread::sleep(Duration::from_millis(40));

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
