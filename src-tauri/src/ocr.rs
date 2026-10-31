//! 截图 OCR:GDI 抓屏 + Windows.Media.Ocr 识别(系统组件,零额外体积)

use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Security::Cryptography::CryptographicBuffer;
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

/// 虚拟屏幕范围 (x, y, w, h)
pub fn virtual_screen() -> (i32, i32, i32, i32) {
    unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    }
}

/// 主显示器尺寸 (w, h)
pub fn primary_screen() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN)) }
}

/// 从屏幕抓取指定区域,BGRA 不透明像素(行序 top-down)
pub fn grab_screen(x: i32, y: i32, w: i32, h: i32) -> Result<Vec<u8>, String> {
    if w <= 0 || h <= 0 {
        return Err("无效的截图区域".into());
    }
    unsafe {
        let hdc_screen = GetDC(None);
        if hdc_screen.is_invalid() {
            return Err("GetDC 失败".into());
        }
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut ppv: *mut std::ffi::c_void = std::ptr::null_mut();
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let hbmp = CreateDIBSection(
            Some(hdc_mem),
            &bmi,
            DIB_RGB_COLORS,
            &mut ppv,
            None,
            0,
        )
        .map_err(|e| format!("CreateDIBSection 失败: {e}"))?;
        if hbmp.is_invalid() || ppv.is_null() {
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(None, hdc_screen);
            return Err("创建 DIB 失败".into());
        }
        let old = SelectObject(hdc_mem, hbmp.into());
        let ok = BitBlt(
            hdc_mem,
            0,
            0,
            w,
            h,
            Some(hdc_screen),
            x,
            y,
            SRCCOPY,
        )
        .is_ok();
        SelectObject(hdc_mem, old);
        let data = if ok {
            let slice = std::slice::from_raw_parts(ppv as *const u8, (w * h * 4) as usize);
            Some(slice.to_vec())
        } else {
            None
        };
        let _ = DeleteObject(hbmp.into());
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);
        let _ = &mut bmi;
        data.ok_or_else(|| "BitBlt 抓屏失败".into())
    }
}

/// 获取可用的 OCR 引擎:优先用户界面语言,否则任意已安装语言
fn create_engine() -> Result<OcrEngine, String> {
    unsafe {
        if let Ok(engine) = OcrEngine::TryCreateFromUserProfileLanguages() {
            return Ok(engine);
        }
        let langs = OcrEngine::AvailableRecognizerLanguages()
            .map_err(|e| format!("查询 OCR 语言失败: {e}"))?;
        for i in 0..langs.Size().unwrap_or(0) {
            let lang: Language = langs.GetAt(i).map_err(|e| e.to_string())?;
            if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&lang) {
                return Ok(engine);
            }
        }
        Err("系统未安装 OCR 语言包:请在 Windows 设置 → 时间和语言 → 语言中添加中文或英文语言包".into())
    }
}

/// 对 BGRA 像素做 OCR
pub fn ocr_bgra(bytes: Vec<u8>, w: i32, h: i32) -> Result<String, String> {
    unsafe {
        // WinRT 调用需要 COM;后台线程用 MTA,重复初始化可忽略
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let engine = create_engine()?;
        let buffer = CryptographicBuffer::CreateFromByteArray(&bytes)
            .map_err(|e| format!("像素缓冲创建失败: {e}"))?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
            &buffer,
            BitmapPixelFormat::Bgra8,
            w,
            h,
        )
        .map_err(|e| format!("位图创建失败: {e}"))?;
        let op = engine
            .RecognizeAsync(&bitmap)
            .map_err(|e| format!("OCR 启动失败: {e}"))?;
        // 工作线程阻塞等待,主调用线程轮询超时(15s 保护)
        let worker = std::thread::spawn(move || op.get());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let result = loop {
            if worker.is_finished() {
                break worker
                    .join()
                    .map_err(|_| "OCR 线程崩溃".to_string())?
                    .map_err(|e| format!("OCR 失败: {e}"))?;
            }
            if std::time::Instant::now() > deadline {
                return Err("OCR 超时".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(40));
        };
        let text = result.Text().map_err(|e| e.to_string())?;
        Ok(text.to_string().trim().to_string())
    }
}

/// 抓屏 + OCR 一步完成(区域为屏幕物理坐标)
pub fn grab_and_ocr(x: i32, y: i32, w: i32, h: i32) -> Result<String, String> {
    let bytes = grab_screen(x, y, w, h)?;
    ocr_bgra(bytes, w, h)
}

/// 主屏区域合法性裁剪
pub fn clamp_to_primary(x: i32, y: i32, w: i32, h: i32) -> RECT {
    let (_, _, sw, sh) = virtual_screen();
    let x = x.max(0);
    let y = y.max(0);
    let w = w.min(sw - x).max(1);
    let h = h.min(sh - y).max(1);
    RECT { left: x, top: y, right: x + w, bottom: y + h }
}
