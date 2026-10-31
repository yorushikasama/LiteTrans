use std::sync::{Arc, Mutex};

use crate::lang;

pub type SharedTts = Arc<Mutex<Option<tts::Tts>>>;

pub fn speak(shared: &SharedTts, text: &str) -> Result<(), String> {
    let mut guard = shared.lock().map_err(|_| "TTS 内部错误".to_string())?;
    if guard.is_none() {
        let t = tts::Tts::default().map_err(|e| format!("初始化语音合成失败: {e}"))?;
        *guard = Some(t);
    }
    let tts = guard.as_mut().unwrap();

    // 按文本语种挑一个系统语音,失败则保持默认
    let prefix = lang::tts_lang_prefix(text);
    if let Ok(voices) = tts.voices() {
        if let Some(v) = voices.iter().find(|v| {
            let hay = format!("{} {}", v.id(), v.name()).to_lowercase();
            hay.starts_with(prefix) || hay.contains(&format!("{prefix}-")) || hay.contains(&format!("{prefix} "))
        }) {
            let _ = tts.set_voice(v);
        }
    }

    // interrupt = true:再次朗读会打断上一次
    tts.speak(text, true)
        .map(|_| ())
        .map_err(|e| format!("朗读失败: {e}"))
}

pub fn stop(shared: &SharedTts) -> Result<(), String> {
    let mut guard = shared.lock().map_err(|_| "TTS 内部错误".to_string())?;
    if let Some(tts) = guard.as_mut() {
        tts.stop().map_err(|e| e.to_string())?;
    }
    Ok(())
}
