/// 极简语种识别:基于 Unicode 区段计数,不发起任何网络请求
fn classify(text: &str) -> &'static str {
    let (mut cjk, mut kana, mut hangul, mut latin) = (0usize, 0usize, 0usize, 0usize);
    for ch in text.chars() {
        let c = ch as u32;
        if (0x4E00..=0x9FFF).contains(&c)
            || (0x3400..=0x4DBF).contains(&c)
            || (0xF900..=0xFAFF).contains(&c)
        {
            cjk += 1;
        } else if (0x3040..=0x30FF).contains(&c) || (0x31F0..=0x31FF).contains(&c) {
            kana += 1;
        } else if (0xAC00..=0xD7AF).contains(&c) {
            hangul += 1;
        } else if ch.is_ascii_alphabetic() {
            latin += 1;
        }
    }
    if kana > 0 && kana * 2 >= cjk {
        return "ja";
    }
    if hangul > cjk && hangul > latin {
        return "ko";
    }
    if cjk > 0 {
        return "zh";
    }
    "en"
}

pub fn source_label(text: &str) -> &'static str {
    match classify(text) {
        "zh" => "中文",
        "ja" => "日语",
        "ko" => "韩语",
        _ => "英文",
    }
}

/// 词典查询判定:单行、≤3 个空格分词、≤24 字符 → 走词典模式而非整句翻译
pub fn is_dictionary_query(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || t.contains('\n') {
        return false;
    }
    if t.split_whitespace().count() > 3 {
        return false;
    }
    t.chars().count() <= 24
}

/// 解析后的翻译方向
pub struct Direction {
    /// 显式指定的源语言(写进提示词 "from X"),auto 检测时为 None
    pub source: Option<String>,
    /// 提示词里的目标语言名
    pub target: String,
    /// 本地检测到的源语言标签(中文/英文/日语/韩语),供 UI 显示
    pub detected: &'static str,
}

/// 解析翻译方向:
/// - mode = "auto":智能模式,自动识别源语言 → 固定译成中文
/// - mode = "custom":用户指定源/目标语言;source 为 "auto" 时仍自动检测,
///   显式指定时提示词写明 from X into Y
pub fn resolve_direction(text: &str, mode: &str, source: &str, target: &str) -> Direction {
    let detected = source_label(text);
    match mode.trim() {
        "custom" => {
            let tgt = match target.trim() {
                "" | "auto" => "Simplified Chinese",
                other => other,
            };
            let src = match source.trim() {
                "" | "auto" => None,
                other => Some(other.to_string()),
            };
            Direction {
                source: src,
                target: tgt.to_string(),
                detected,
            }
        }
        // 智能模式(含旧配置兼容):自动识别 → 中文
        _ => Direction {
            source: None,
            target: "Simplified Chinese".to_string(),
            detected,
        },
    }
}

/// TTS 选声时用的 bcp47 前缀
pub fn tts_lang_prefix(text: &str) -> &'static str {
    match classify(text) {
        "zh" => "zh",
        "ja" => "ja",
        "ko" => "ko",
        _ => "en",
    }
}
