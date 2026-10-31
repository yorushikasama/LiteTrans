use std::fs;

use serde::{Deserialize, Serialize};
use tauri::Manager;

/// 一条翻译历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[derive(Default)]
pub struct HistoryItem {
    pub ts: i64,
    pub source: String,
    pub result: String,
    /// 提示词目标语言名或"词典查询"
    pub direction: String,
    pub dict: bool,
    /// 配置名(展示用)
    pub profile: String,
    /// 配置 id:前端据此显示该条记录所属的「调」,旧记录为空
    pub profile_id: String,
}

const MAX_ITEMS: usize = 50;

fn history_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join("history.json"))
}

pub fn record(app: &tauri::AppHandle, item: HistoryItem) {
    let Some(path) = history_path(app) else {
        return;
    };
    let mut list = load(app);
    list.insert(0, item);
    list.truncate(MAX_ITEMS);
    if let Ok(content) = serde_json::to_string(&list) {
        let _ = fs::write(&path, content);
    }
}

pub fn load(app: &tauri::AppHandle) -> Vec<HistoryItem> {
    let Some(path) = history_path(app) else {
        return Vec::new();
    };
    fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

pub fn clear(app: &tauri::AppHandle) {
    if let Some(path) = history_path(app) {
        let _ = fs::remove_file(&path);
    }
}
