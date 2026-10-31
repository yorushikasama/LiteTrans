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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_by_script() {
        assert_eq!(classify("你好,世界"), "zh");
        assert_eq!(classify("Hello, world!"), "en");
        assert_eq!(classify("こんにちは世界"), "ja");
        assert_eq!(classify("안녕하세요"), "ko");
    }

    #[test]
    fn smart_mode_always_targets_chinese() {
        let d = resolve_direction("hello world", "auto", "auto", "English");
        assert_eq!(d.target, "Simplified Chinese");
        assert_eq!(d.detected, "英文");
        let d = resolve_direction("你好", "auto", "auto", "English");
        assert_eq!(d.target, "Simplified Chinese");
        assert_eq!(d.detected, "中文");
    }

    #[test]
    fn custom_mode_passes_explicit_source_and_target() {
        let d = resolve_direction("hello", "custom", "English", "Japanese");
        assert_eq!(d.target, "Japanese");
        assert_eq!(d.detected, "英文");
    }

    #[test]
    fn custom_mode_auto_source_detected() {
        let d = resolve_direction("hello", "custom", "auto", "French");
        assert_eq!(d.target, "French");
        assert_eq!(d.detected, "英文");
    }

    #[test]
    fn dictionary_query_detection() {
        assert!(is_dictionary_query("hello"));
        assert!(is_dictionary_query("hello world"));
        assert!(is_dictionary_query("你好世界"));
        // 4 个词不是词典查询
        assert!(!is_dictionary_query("the quick brown fox"));
        // 换行 = 句子
        assert!(!is_dictionary_query("hello\nworld"));
        // 超长
        assert!(!is_dictionary_query("this is a very long sentence indeed"));
    }
}
