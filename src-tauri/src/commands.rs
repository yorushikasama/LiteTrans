use std::sync::Arc;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use crate::engine::{self, EngineCfg};
use crate::lang;
use crate::settings::{Preset, Profile, Settings, PRESETS};
use crate::tts_win;
use crate::AppState;

/// 流式翻译消息(与前端 StreamMsg 对应)
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamMsg {
    Start {
        detected: String,
        target: String,
        dict: bool,
    },
    Delta {
        text: String,
    },
    Done {
        text: String,
    },
    Error {
        message: String,
    },
    Cancelled,
}

#[tauri::command]
pub async fn translate(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
    source: String,
    on_event: Channel<StreamMsg>,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }
    let s = state.settings.read().map_err(|e| e.to_string())?.clone();

    let dir = lang::resolve_direction(&text, &s.trans_mode, &s.source_lang, &s.target_lang);
    // 词典模式:词/短语走释义而非整句翻译,释义固定用中文
    let dict = lang::is_dictionary_query(&text);
    let dir = if dict {
        crate::lang::Direction {
            source: None,
            target: "Simplified Chinese".into(),
            detected: dir.detected,
        }
    } else {
        dir
    };

    let Some(p) = s.active_profile().cloned() else {
        let _ = on_event.send(StreamMsg::Error {
            message: "尚未添加模型配置,请打开设置新增".into(),
        });
        return Ok(());
    };
    if !p.has_credentials() {
        let _ = on_event.send(StreamMsg::Error {
            message: format!("当前配置「{}」尚未填写 API Key,请打开设置完成", p.name),
        });
        return Ok(());
    }

    let cfg = EngineCfg {
        protocol: p.protocol.clone(),
        base_url: p.effective_base_url(),
        model: p.effective_model(),
        api_key: p.api_key.trim().to_string(),
        temperature: p.temperature,
        custom_prompt: s.custom_prompt.clone(),
    };

    // 同一窗口的新请求会取消旧请求
    let token = Arc::new(CancellationToken::new());
    if let Some(prev) = state
        .cancels
        .lock()
        .unwrap()
        .insert(source.clone(), token.clone())
    {
        prev.cancel();
    }

    let _ = on_event.send(StreamMsg::Start {
        detected: dir.detected.to_string(),
        target: dir.target.clone(),
        dict,
    });

    let ev = on_event.clone();
    let result = engine::stream_translate(&cfg, &text, &dir, dict, &token, move |delta| {
        let _ = ev.send(StreamMsg::Delta { text: delta });
    })
    .await;

    // 仅当 map 里还是本次 token 时才移除,避免误删新请求的 token
    {
        let mut map = state.cancels.lock().unwrap();
        if map
            .get(&source)
            .map(|t| Arc::ptr_eq(t, &token))
            .unwrap_or(false)
        {
            map.remove(&source);
        }
    }

    match result {
        Ok(full) => {
            // 弹卡场景:按设置自动复制译文(防失焦丢失)
            if s.auto_copy && source == "popup" {
                if let Ok(mut clip) = arboard::Clipboard::new() {
                    let _ = clip.set_text(full.clone());
                }
            }
            // 记录翻译历史
            let direction = if dict {
                "词典查询".to_string()
            } else {
                format!("{} → {}", dir.detected, dir.target)
            };
            crate::history::record(
                &app,
                crate::history::HistoryItem {
                    ts: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0),
                    source: text.clone(),
                    result: full.clone(),
                    direction,
                    dict,
                    profile: p.name.clone(),
                    profile_id: p.id.clone(),
                },
            );
            let _ = on_event.send(StreamMsg::Done { text: full });
        }
        Err(e) => {
            if token.is_cancelled() {
                let _ = on_event.send(StreamMsg::Cancelled);
            } else {
                let _ = on_event.send(StreamMsg::Error { message: e });
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_translate(state: State<'_, AppState>, source: String) {
    if let Some(t) = state.cancels.lock().unwrap().get(&source) {
        t.cancel();
    }
}

#[tauri::command]
pub fn get_presets() -> &'static [Preset] {
    PRESETS
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.read().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), String> {
    // 开机自启
    use tauri_plugin_autostart::ManagerExt;
    let want = settings.autostart;
    let cur = app.autolaunch().is_enabled().unwrap_or(false);
    if want != cur {
        if want {
            app.autolaunch().enable().map_err(|e| e.to_string())?;
        } else {
            app.autolaunch().disable().map_err(|e| e.to_string())?;
        }
    }

    let old = {
        let s = state.settings.read().unwrap();
        (s.hotkey.clone(), s.hotkey_enabled, s.capture_hotkey.clone())
    };
    settings.save(&app)?;
    *state.settings.write().unwrap() = settings;

    let new = {
        let s = state.settings.read().unwrap();
        (s.hotkey.clone(), s.hotkey_enabled, s.capture_hotkey.clone())
    };
    if old != new {
        if let Err(e) = crate::register_hotkeys(&app) {
            // 注册失败则回滚旧热键设置
            let (old_hotkey, _, old_cap) = old;
            {
                let mut s = state.settings.write().unwrap();
                s.hotkey = old_hotkey;
                s.capture_hotkey = old_cap;
            }
            let _ = crate::register_hotkeys(&app);
            return Err(e);
        }
    }
    crate::rebuild_tray(&app);
    Ok(())
}

/// 一键切换启用的配置(弹卡/托盘/设置页共用)
#[tauri::command]
pub fn set_active_profile(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    {
        let mut s = state.settings.write().unwrap();
        if !s.profiles.iter().any(|p| p.id == id) {
            return Err("配置不存在".into());
        }
        s.active_id = id;
        s.save(&app)?;
    }
    crate::rebuild_tray(&app);
    Ok(())
}

/// 探活:测试某条配置是否能连通
#[tauri::command]
pub async fn test_profile(profile: Profile) -> Result<String, String> {
    let cfg = EngineCfg {
        protocol: profile.protocol.clone(),
        base_url: profile.effective_base_url(),
        model: profile.effective_model(),
        api_key: profile.api_key.trim().to_string(),
        temperature: 0.0,
        custom_prompt: String::new(),
    };
    engine::test_connection(&cfg).await
}

/// 在线拉取模型列表(GET /models,按协议区分)
#[tauri::command]
pub async fn fetch_models(profile: Profile) -> Result<Vec<String>, String> {
    use std::time::Duration;

    let base = profile.effective_base_url();
    if base.is_empty() {
        return Err("请先填写 API 地址".into());
    }
    let key = profile.api_key.trim();
    let cl = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let mut ids: Vec<String> = match profile.protocol.as_str() {
        "anthropic" => {
            let url = format!("{}/v1/models?limit=100", base.trim_end_matches('/'));
            let mut req = cl.get(&url).header("anthropic-version", "2023-06-01");
            if !key.is_empty() {
                req = req.header("x-api-key", key);
            }
            let v = send_json(req).await?;
            v["data"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|m| m["id"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        }
        "gemini" => {
            let url = format!("{}/v1beta/models", base.trim_end_matches('/'));
            let mut req = cl.get(&url);
            if !key.is_empty() {
                req = req.header("x-goog-api-key", key);
            }
            let v = send_json(req).await?;
            v["models"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|m| {
                            // 只保留支持文本生成的模型
                            if let Some(methods) = m["supportedGenerationMethods"].as_array() {
                                if !methods
                                    .iter()
                                    .any(|x| x.as_str() == Some("generateContent"))
                                {
                                    return None;
                                }
                            }
                            let name = m["name"].as_str()?;
                            Some(name.strip_prefix("models/").unwrap_or(name).to_string())
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
        _ => {
            let url = format!("{}/models", base.trim_end_matches('/'));
            let mut req = cl.get(&url);
            if !key.is_empty() {
                req = req.bearer_auth(key);
            }
            let v = send_json(req).await?;
            v["data"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|m| m["id"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        }
    };
    ids.sort();
    ids.dedup();
    Ok(ids)
}

async fn send_json(req: reqwest::RequestBuilder) -> Result<serde_json::Value, String> {
    let resp = req.send().await.map_err(|e| format!("获取失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        let msg = engine::parse_err_body(&text);
        let msg = if msg.is_empty() {
            "请求失败".into()
        } else {
            msg
        };
        return Err(format!("API 错误 {}: {}", status.as_u16(), msg));
    }
    serde_json::from_str(&text).map_err(|e| format!("响应解析失败: {e}"))
}

#[tauri::command]
pub fn toggle_pin(state: State<'_, AppState>) -> bool {
    let old = state
        .pinned
        .fetch_xor(true, std::sync::atomic::Ordering::Relaxed);
    !old
}

#[tauri::command]
pub fn take_pending_text(state: State<'_, AppState>) -> Option<String> {
    state.pending_popup_text.lock().unwrap().take()
}

#[tauri::command]
pub fn take_pending_error(state: State<'_, AppState>) -> Option<String> {
    state.pending_popup_error.lock().unwrap().take()
}

#[tauri::command]
pub fn hide_capture(app: AppHandle) {
    crate::capture_flow::hide_capture(&app);
}

/// 框选完成:隐藏遮罩 → 抓屏 → OCR → 弹卡翻译
#[tauri::command]
pub async fn ocr_translate(app: AppHandle, x: i32, y: i32, w: i32, h: i32) -> Result<(), String> {
    crate::capture_flow::hide_capture(&app);
    // 等遮罩从屏幕上消失,避免污染截图
    tokio::time::sleep(std::time::Duration::from_millis(180)).await;
    let rect = crate::ocr::clamp_to_primary(x, y, w, h);
    let (rx, ry) = (rect.left, rect.top);
    let (rw, rh) = (rect.right - rect.left, rect.bottom - rect.top);
    let text =
        tauri::async_runtime::spawn_blocking(move || crate::ocr::grab_and_ocr(rx, ry, rw, rh))
            .await
            .map_err(|e| e.to_string())??;
    if text.trim().is_empty() {
        crate::clipboard_flow::show_popup_with_error(
            &app,
            "未识别到文字,请框选更清晰的区域".to_string(),
        )
        .await?;
    } else {
        crate::clipboard_flow::show_popup_with_text(&app, text).await?;
    }
    Ok(())
}

#[tauri::command]
pub fn write_clipboard(text: String) -> Result<(), String> {
    let mut clip = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clip.set_text(text).map_err(|e| e.to_string())
}

/// 读取剪贴板文本(主窗口「粘贴」按钮用)
#[tauri::command]
pub fn read_clipboard() -> Result<Option<String>, String> {
    let mut clip = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    clip.get_text().map(Some).or(Ok(None))
}

#[tauri::command]
pub fn get_history(app: AppHandle) -> Vec<crate::history::HistoryItem> {
    crate::history::load(&app)
}

#[tauri::command]
pub fn clear_history(app: AppHandle) {
    crate::history::clear(&app);
}

#[tauri::command]
pub fn hide_popup(app: AppHandle) {
    if let Some(w) = app.get_webview_window("popup") {
        let _ = w.hide();
    }
}

#[tauri::command]
pub fn open_main_with_text(app: AppHandle, text: String) {
    crate::show_main(&app);
    let _ = app.emit_to("main", "fill-source", serde_json::json!({ "text": text }));
}

/// 打开主窗口并定位到设置页(错误卡「去设置」用)
#[tauri::command]
pub fn open_main_settings(app: AppHandle) {
    crate::show_main(&app);
    let _ = app.emit_to("main", "open-settings", ());
}

#[tauri::command]
pub async fn tts_speak(state: State<'_, AppState>, text: String) -> Result<(), String> {
    let shared = state.tts.clone();
    tauri::async_runtime::spawn_blocking(move || tts_win::speak(&shared, &text))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn tts_stop(state: State<'_, AppState>) -> Result<(), String> {
    let shared = state.tts.clone();
    tauri::async_runtime::spawn_blocking(move || tts_win::stop(&shared))
        .await
        .map_err(|e| e.to_string())?
}
