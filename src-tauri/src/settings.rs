use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::Manager;

pub const KEYRING_SERVICE: &str = "LiteTrans";

/// 服务商预设:protocol 决定走哪种接口协议(openai / anthropic / gemini)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: &'static str,
    pub name: &'static str,
    pub base_url: &'static str,
    pub model: &'static str,
    pub need_key: bool,
    pub protocol: &'static str,
    /// 静态候选模型(编辑器里还可通过 /models 在线拉取)
    pub models: &'static [&'static str],
}

pub const PRESETS: &[Preset] = &[
    Preset { id: "deepseek", name: "DeepSeek", base_url: "https://api.deepseek.com/v1", model: "deepseek-chat", need_key: true, protocol: "openai",
        models: &["deepseek-chat", "deepseek-reasoner"] },
    Preset { id: "zhipu", name: "智谱 GLM", base_url: "https://open.bigmodel.cn/api/paas/v4", model: "glm-4-flash", need_key: true, protocol: "openai",
        models: &["glm-4-flash", "glm-4-air", "glm-4-plus", "glm-4.5", "glm-4.6"] },
    Preset { id: "moonshot", name: "Kimi", base_url: "https://api.moonshot.cn/v1", model: "moonshot-v1-8k", need_key: true, protocol: "openai",
        models: &["moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k", "kimi-k2-0711-preview", "kimi-k2-turbo-preview"] },
    Preset { id: "qwen", name: "通义千问", base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1", model: "qwen-plus", need_key: true, protocol: "openai",
        models: &["qwen-turbo", "qwen-plus", "qwen-max", "qwen-plus-latest"] },
    Preset { id: "sensenova", name: "商汤 日日新", base_url: "https://token.sensenova.cn/v1", model: "SenseChat-5", need_key: true, protocol: "openai",
        models: &["SenseChat-5"] },
    Preset { id: "siliconflow", name: "硅基流动", base_url: "https://api.siliconflow.cn/v1", model: "Qwen/Qwen2.5-7B-Instruct", need_key: true, protocol: "openai",
        models: &["Qwen/Qwen2.5-7B-Instruct", "Qwen/Qwen2.5-72B-Instruct", "deepseek-ai/DeepSeek-V3", "deepseek-ai/DeepSeek-R1"] },
    Preset { id: "openai", name: "OpenAI", base_url: "https://api.openai.com/v1", model: "gpt-4o-mini", need_key: true, protocol: "openai",
        models: &["gpt-4o-mini", "gpt-4o", "gpt-4.1-mini", "gpt-4.1"] },
    Preset { id: "anthropic", name: "Claude(Anthropic)", base_url: "https://api.anthropic.com", model: "claude-sonnet-4-5", need_key: true, protocol: "anthropic",
        models: &["claude-sonnet-4-5", "claude-opus-4-1", "claude-3-5-haiku-latest"] },
    Preset { id: "gemini", name: "Gemini(Google)", base_url: "https://generativelanguage.googleapis.com", model: "gemini-2.0-flash", need_key: true, protocol: "gemini",
        models: &["gemini-2.5-flash", "gemini-2.5-pro", "gemini-2.0-flash"] },
    Preset { id: "openrouter", name: "OpenRouter", base_url: "https://openrouter.ai/api/v1", model: "openai/gpt-4o-mini", need_key: true, protocol: "openai",
        models: &["openai/gpt-4o-mini", "deepseek/deepseek-chat", "anthropic/claude-sonnet-4.5"] },
    Preset { id: "ollama", name: "Ollama(本地)", base_url: "http://localhost:11434/v1", model: "qwen2.5:7b", need_key: false, protocol: "openai",
        models: &["qwen2.5:7b", "qwen2.5:14b", "llama3.1:8b", "deepseek-r1:7b"] },
    Preset { id: "custom", name: "自定义", base_url: "", model: "", need_key: true, protocol: "openai",
        models: &[] },
];

pub fn preset_by_id(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

pub fn new_profile_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("p{:x}", nanos)
}

/// 一个模型配置档案(参考 CC-Switch:多条配置随时切换)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub id: String,
    pub name: String,
    /// 预设 id 或 "custom"
    pub provider: String,
    /// 接口协议:openai | anthropic | gemini
    pub protocol: String,
    pub base_url: String,
    pub model: String,
    pub temperature: f32,
    /// 内存中的 key:启动时从凭据管理器读入,保存时写回,不落盘
    pub api_key: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: new_profile_id(),
            name: String::new(),
            provider: "deepseek".into(),
            protocol: "openai".into(),
            base_url: String::new(),
            model: String::new(),
            temperature: 0.3,
            api_key: String::new(),
        }
    }
}

impl Profile {
    pub fn effective_base_url(&self) -> String {
        let own = self.base_url.trim().trim_end_matches('/');
        if !own.is_empty() {
            return own.to_string();
        }
        preset_by_id(&self.provider)
            .map(|p| p.base_url.to_string())
            .unwrap_or_default()
    }

    pub fn effective_model(&self) -> String {
        let own = self.model.trim();
        if !own.is_empty() {
            return own.to_string();
        }
        preset_by_id(&self.provider)
            .map(|p| p.model.to_string())
            .unwrap_or_default()
    }

    pub fn has_credentials(&self) -> bool {
        self.provider == "ollama" || !self.api_key.trim().is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub profiles: Vec<Profile>,
    pub active_id: String,
    /// 翻译方向模式:auto = 自动识别→中文;custom = 用户指定源/目标语言
    pub trans_mode: String,
    /// custom 模式下的源语言("auto" 表示仍自动检测)
    pub source_lang: String,
    pub target_lang: String,
    pub hotkey: String,
    pub custom_prompt: String,
    pub autostart: bool,
    /// 划词弹卡翻译完成后自动复制译文
    pub auto_copy: bool,
    /// 主窗口输入即译(防抖)
    pub auto_translate: bool,
    /// 是否启用划词热键(临时停用开关)
    pub hotkey_enabled: bool,
    /// 首次关窗到托盘的提示是否已展示
    pub tray_hint_shown: bool,
    pub setup_done: bool,

    // ── 旧版单配置字段(键名与旧版一致):仅用于读取旧 config.json 做迁移,不再写盘 ──
    #[serde(default, skip_serializing)]
    pub provider: String,
    #[serde(default, skip_serializing)]
    pub base_url: String,
    #[serde(default, skip_serializing)]
    pub model: String,
    #[serde(default, skip_serializing)]
    pub api_key_fallback: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            profiles: Vec::new(),
            active_id: String::new(),
            trans_mode: "auto".into(),
            source_lang: "auto".into(),
            target_lang: "Simplified Chinese".into(),
            hotkey: "alt+q".into(),
            custom_prompt: String::new(),
            autostart: false,
            auto_copy: true,
            auto_translate: false,
            hotkey_enabled: true,
            tray_hint_shown: false,
            setup_done: false,
            provider: String::new(),
            base_url: String::new(),
            model: String::new(),
            api_key_fallback: String::new(),
        }
    }
}

fn keyring_get(account: &str) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, account)
        .and_then(|e| e.get_password())
        .ok()
}

fn keyring_set(account: &str, key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, account).map_err(|e| e.to_string())?;
    entry.set_password(key).map_err(|e| e.to_string())
}

fn keyring_delete(account: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, account) {
        let _ = entry.delete_credential();
    }
}

impl Settings {
    pub fn config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
        app.path().app_config_dir().ok().map(|d| d.join("config.json"))
    }

    pub fn load(app: &tauri::AppHandle) -> Self {
        let mut s = Self::default();
        if let Some(path) = Self::config_path(app) {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(disk) = serde_json::from_str::<Settings>(&content) {
                    s = disk;
                }
                // 旧版只有 targetLang 一个方向字段:迁移为 transMode + source/target
                if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&content) {
                    if raw.get("transMode").is_none() {
                        let old = raw
                            .get("targetLang")
                            .and_then(|v| v.as_str())
                            .unwrap_or("auto");
                        match old {
                            "auto" | "" => {
                                s.trans_mode = "auto".into();
                            }
                            "en" => {
                                s.trans_mode = "custom".into();
                                s.source_lang = "auto".into();
                                s.target_lang = "English".into();
                            }
                            "ja" => {
                                s.trans_mode = "custom".into();
                                s.source_lang = "auto".into();
                                s.target_lang = "Japanese".into();
                            }
                            "ko" => {
                                s.trans_mode = "custom".into();
                                s.source_lang = "auto".into();
                                s.target_lang = "Korean".into();
                            }
                            "zh" => {
                                s.trans_mode = "custom".into();
                                s.source_lang = "auto".into();
                                s.target_lang = "Simplified Chinese".into();
                            }
                            other => {
                                s.trans_mode = "custom".into();
                                s.source_lang = "auto".into();
                                s.target_lang = other.to_string();
                            }
                        }
                    }
                }
            }
        }
        // 旧版单配置字段还在 → 迁移为一条档案
        if s.profiles.is_empty() && (!s.provider.is_empty() || !s.base_url.is_empty()) {
            let mut p = Profile {
                id: new_profile_id(),
                name: preset_by_id(&s.provider)
                    .map(|x| x.name.to_string())
                    .unwrap_or_else(|| "默认配置".into()),
                provider: s.provider.clone(),
                protocol: preset_by_id(&s.provider)
                    .map(|x| x.protocol.to_string())
                    .unwrap_or_else(|| "openai".into()),
                base_url: s.base_url.clone(),
                model: s.model.clone(),
                temperature: 0.3,
                api_key: keyring_get(&s.provider)
                    .or_else(|| {
                        if s.api_key_fallback.is_empty() {
                            None
                        } else {
                            Some(s.api_key_fallback.clone())
                        }
                    })
                    .unwrap_or_default(),
            };
            p.name = if p.name.is_empty() { "默认配置".into() } else { p.name };
            s.active_id = p.id.clone();
            s.profiles.push(p);
        }
        // 从凭据管理器补齐每个档案的 key
        for p in &mut s.profiles {
            if p.api_key.is_empty() {
                p.api_key = keyring_get(&p.id).unwrap_or_default();
            }
        }
        if s.active_id.is_empty() {
            if let Some(first) = s.profiles.first() {
                s.active_id = first.id.clone();
            }
        }
        // 开机自启以系统实际状态为准
        use tauri_plugin_autostart::ManagerExt;
        s.autostart = app.autolaunch().is_enabled().unwrap_or(false);
        s
    }

    pub fn save(&self, app: &tauri::AppHandle) -> Result<(), String> {
        // 每个档案的 key 独立存入凭据管理器
        for p in &self.profiles {
            if p.api_key.trim().is_empty() {
                keyring_delete(&p.id);
            } else {
                keyring_set(&p.id, p.api_key.trim())?;
            }
        }

        let Some(path) = Self::config_path(app) else {
            return Err("无法获取配置目录".into());
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut disk = self.clone();
        disk.profiles = disk
            .profiles
            .iter()
            .map(|p| {
                let mut d = p.clone();
                d.api_key = String::new(); // key 不落盘
                d
            })
            .collect();
        let content = serde_json::to_string_pretty(&disk).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())
    }

    pub fn active_profile(&self) -> Option<&Profile> {
        self.profiles
            .iter()
            .find(|p| p.id == self.active_id)
            .or_else(|| self.profiles.first())
    }

    pub fn active_ready(&self) -> bool {
        self.active_profile()
            .map(|p| p.has_credentials())
            .unwrap_or(false)
    }
}
