//! 数据目录解析:兼顾安装版与便携(绿色)版
//!
//! 便携版的判定不看命令行参数,而是看 **exe 旁边有没有 `portable.txt` 标记文件**:
//!
//! - 标记文件存在 → 数据写在 `exe目录/data/`,拷走整个文件夹就带走了配置,
//!   适合放U盘、放网盘同步目录,不碰 `%APPDATA%`、不写注册表
//! - 标记文件不存在 → 走系统标准位置(`%APPDATA%\LiteTrans`),即安装版行为
//!
//! 用标志文件而不是"检测是否可写",是因为后者在 `C:\Program Files` 下会
//! 因 UAC 虚拟化产生看似可写实则重定向的结果,判定不可靠。

use std::path::PathBuf;
use std::sync::OnceLock;

use tauri::Manager;

/// 便携标记文件名(与 exe 同级)
pub const PORTABLE_MARKER: &str = "portable.txt";

/// 缓存便携模式判定:同一次运行内目录不会变
static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// exe 所在目录(便携模式下数据目录的父目录)
pub fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

/// 是否便携模式:exe 旁边存在 portable.txt
pub fn is_portable() -> bool {
    exe_dir().as_deref().map(marker_present).unwrap_or(false)
}

/// 纯函数版判定:给定 exe 目录,portable.txt 是否存在(便于单测)
pub(crate) fn marker_present(exe_dir: &std::path::Path) -> bool {
    exe_dir.join(PORTABLE_MARKER).exists()
}

/// 纯函数版目录解析:便携 → `<exe>/data`;否则用调用方给出的系统目录
pub(crate) fn resolve(
    exe_dir: Option<&std::path::Path>,
    portable: bool,
    system_dir: Option<PathBuf>,
) -> Option<PathBuf> {
    if portable {
        exe_dir.map(|d| d.join("data"))
    } else {
        system_dir
    }
}

/// 应用数据目录(配置与历史都放这里)
///
/// 便携模式返回 `<exe目录>/data`;否则返回系统 `app_config_dir`。
/// 目录在返回前已确保创建,调用方可以直接写文件。
pub fn data_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Some(dir) = CACHE.get() {
        return dir.clone();
    }

    let resolved = resolve(
        exe_dir().as_deref(),
        is_portable(),
        app.path().app_config_dir().ok(),
    );

    // 首次解析时建目录:便携版首次运行 exe 旁边还没有 data/
    if let Some(dir) = &resolved {
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("[litetrans] 创建数据目录 {} 失败: {e}", dir.display());
        }
    }
    let _ = CACHE.set(resolved.clone());
    resolved
}

/// 数据目录下的某个文件
pub fn data_file(app: &tauri::AppHandle, name: &str) -> Option<PathBuf> {
    data_dir(app).map(|d| d.join(name))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_mode_uses_dir_next_to_exe() {
        let exe = std::path::Path::new(r"D:\Apps\LiteTrans");
        let resolved = resolve(
            Some(exe),
            true,
            Some(PathBuf::from(r"C:\Users\me\AppData\Roaming\LiteTrans")),
        );
        assert_eq!(resolved, Some(exe.join("data")));
    }

    #[test]
    fn installed_mode_uses_system_dir() {
        let exe = std::path::Path::new(r"C:\Program Files\LiteTrans");
        let sys = PathBuf::from(r"C:\Users\me\AppData\Roaming\LiteTrans");
        let resolved = resolve(Some(exe), false, Some(sys.clone()));
        assert_eq!(resolved, Some(sys));
    }

    #[test]
    fn marker_missing_means_not_portable() {
        let tmp = std::env::temp_dir().join("litetrans-paths-test-no-marker");
        let _ = std::fs::create_dir_all(&tmp);
        assert!(!marker_present(&tmp));
    }

    #[test]
    fn marker_present_means_portable() {
        let tmp = std::env::temp_dir().join("litetrans-paths-test-marker");
        let _ = std::fs::create_dir_all(&tmp);
        std::fs::write(tmp.join(PORTABLE_MARKER), b"portable").unwrap();
        assert!(marker_present(&tmp));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn portable_without_exe_dir_yields_none() {
        // 极端情况:拿不到 exe 路径时不应回落到系统目录,否则便携版会
        // 悄悄把配置写到 %APPDATA%,违背"拷走即带走"的承诺
        assert_eq!(resolve(None, true, Some(PathBuf::from(r"C:\x"))), None);
    }
}
