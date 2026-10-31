use std::time::Duration;

use futures_util::StreamExt;
use tokio_util::sync::CancellationToken;

/// 一次翻译请求所需的引擎参数(从当前启用的 Profile + 全局设置拼出)
#[derive(Debug, Clone)]
pub struct EngineCfg {
    /// 接口协议:openai | anthropic | gemini
    pub protocol: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub temperature: f32,
    pub custom_prompt: String,
}

impl EngineCfg {
    pub fn protocol(&self) -> &str {
        match self.protocol.as_str() {
            "anthropic" | "gemini" => self.protocol.as_str(),
            _ => "openai",
        }
    }
}

fn build_prompt(custom_prompt: &str, source: Option<&str>, target_name: &str, dict: bool) -> String {
    let core = if dict {
        // 词典模式:固定纯文本结构,保证弹卡渲染一致
        "你是面向中文使用者的双语词典。用户会发送一个单词或极短短语。\n\
         严格按以下纯文本格式输出,除该格式外不要输出任何内容(不要 Markdown 标题、不要前后空行、不要解释):\n\
         第一行:音标(外文词,如 /ˈdevəlpmənt/)或拼音(中文词)\n\
         第二行起:词性. 释义(最多 3 个最常用义项,按常用度排序,义项间用分号分隔)\n\
         最后一行:例. 例句(下一行缩进给例句的中文翻译)\n\
         若为专有名词或品牌:第一行给通行中文名称,之后用一句话说明;中文词则第一行给拼音。\n\
         若该词更常作为缩写/网络用语,在释义后用括号注明。"
            .to_string()
    } else {
        let dir = match source {
            Some(s) => format!("Translate the user's text from {s} into {target_name}."),
            None => format!("Translate the user's text into {target_name}."),
        };
        format!(
            "You are a professional translation engine. {dir}\n\
             Output rules (strict):\n\
             1. Reply with the translation ONLY — no preamble (no \"以下是翻译\", \"Translation:\", etc.), no explanations, no notes, and no surrounding quotes or brackets.\n\
             2. Preserve the original structure exactly: line breaks, blank lines, bullet/numbered lists, indentation, and Markdown syntax.\n\
             3. Do not introduce Markdown (bold, code fences, headers) that the source does not have.\n\
             4. Keep code, commands, URLs, emails, file paths, @mentions, #hashtags and identifiers unchanged.\n\
             5. Keep brand and product names as-is unless an official translation exists.\n\
             6. If the text is already in {target_name}, output it unchanged.\n\
             7. Match the tone and register of the source (formal stays formal, casual stays casual).\n\
             8. If a word is ambiguous, pick its most common sense and stay consistent throughout."
        )
    };
    let mut p = core;
    let extra = custom_prompt.trim();
    if !extra.is_empty() && !dict {
        p.push_str("\n\nAdditional user requirements:\n");
        p.push_str(extra);
    }
    p
}

/// 输出净化:去首尾空白;若模型包了一层源文本没有的对称引号则剥掉
fn sanitize_output(s: String, source: &str) -> String {
    let t = s.trim();
    let chars: Vec<char> = t.chars().collect();
    if chars.len() > 2 {
        let first = chars[0];
        let last = chars[chars.len() - 1];
        let paired = matches!(
            (first, last),
            ('"', '"') | ('“', '”') | ('‘', '’') | ('「', '」') | ('『', '』')
        );
        // 源文本本身以此引号开头时不剥,避免破坏合法的引用内容
        let source_has = source.trim_start().starts_with(first);
        if paired && !source_has {
            let inner: String = chars[1..chars.len() - 1].iter().collect();
            return inner.trim().to_string();
        }
    }
    t.to_string()
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())
}

/// 从非 2xx 响应体里尽力提取可读的错误信息
pub(crate) fn parse_err_body(text: &str) -> String {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| {
            v["error"]["message"]
                .as_str()
                .map(String::from)
                .or_else(|| v["message"].as_str().map(String::from))
                .or_else(|| v["error"].as_str().map(String::from))
        })
        .unwrap_or_else(|| text.chars().take(300).collect())
}

/// 通用 SSE 读取:按行拆 "data:" 负载,由 on_data 按协议解析
/// on_data 返回 Ok(true) 表示流结束;返回 Err 直接终止
async fn read_sse(
    resp: reqwest::Response,
    token: &CancellationToken,
    mut on_data: impl FnMut(&serde_json::Value, &mut String) -> Result<bool, String>,
) -> Result<String, String> {
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    let mut full = String::new();
    let mut done = false;

    loop {
        tokio::select! {
            _ = token.cancelled() => return Err("cancelled".into()),
            chunk = stream.next() => match chunk {
                None => break,
                Some(Err(e)) => {
                    if token.is_cancelled() {
                        return Err("cancelled".into());
                    }
                    return Err(format!("读取响应失败: {e}"));
                }
                Some(Ok(bytes)) => {
                    buf.extend_from_slice(&bytes);
                    while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                        let line_bytes: Vec<u8> = buf.drain(..=pos).collect();
                        let line = String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]);
                        let line = line.trim_end_matches('\r');
                        let Some(data) = line.strip_prefix("data:") else { continue };
                        let data = data.trim();
                        if data.is_empty() || data == "[DONE]" {
                            continue;
                        }
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                            if on_data(&v, &mut full)? {
                                done = true;
                                break;
                            }
                        }
                    }
                    if done {
                        break;
                    }
                }
            }
        }
    }

    if token.is_cancelled() {
        return Err("cancelled".into());
    }
    Ok(full)
}

async fn check_status(resp: reqwest::Response) -> Result<reqwest::Response, String> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let msg = parse_err_body(&body);
        let msg = if msg.is_empty() { format!("HTTP {status}") } else { msg };
        return Err(format!("API 错误 {}: {}", status.as_u16(), msg));
    }
    Ok(resp)
}

fn ensure_non_empty(full: String, what: &str) -> Result<String, String> {
    if full.trim().is_empty() {
        Err(format!("模型未返回内容({what})"))
    } else {
        Ok(full)
    }
}

pub async fn stream_translate(
    cfg: &EngineCfg,
    text: &str,
    dir: &crate::lang::Direction,
    dict: bool,
    token: &CancellationToken,
    on_delta: impl FnMut(String),
) -> Result<String, String> {
    let out = match cfg.protocol() {
        "anthropic" => stream_anthropic(cfg, text, dir, dict, token, on_delta).await,
        "gemini" => stream_gemini(cfg, text, dir, dict, token, on_delta).await,
        _ => stream_openai(cfg, text, dir, dict, token, on_delta).await,
    }?;
    // 最终净化:去首尾空白与多余包裹引号(流式过程中前端看到的仍是原始增量)
    Ok(sanitize_output(out, text))
}

/* ────────── OpenAI 兼容:/chat/completions ────────── */

async fn stream_openai(
    cfg: &EngineCfg,
    text: &str,
    dir: &crate::lang::Direction,
    dict: bool,
    token: &CancellationToken,
    mut on_delta: impl FnMut(String),
) -> Result<String, String> {
    if cfg.base_url.is_empty() {
        return Err("请先在设置中配置 API 地址".into());
    }
    let url = format!("{}/chat/completions", cfg.base_url);
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            { "role": "system", "content": build_prompt(&cfg.custom_prompt, dir.source.as_deref(), &dir.target, dict) },
            { "role": "user", "content": text }
        ],
        "stream": true,
        "temperature": cfg.temperature,
    });

    let resp = client()?
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .header("Accept", "text/event-stream")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {e}"))?;
    let resp = check_status(resp).await?;

    let full = read_sse(resp, token, |v, acc| {
        if let Some(err) = v["error"]["message"].as_str() {
            return Err(format!("API 错误: {err}"));
        }
        if let Some(c) = v["choices"].get(0).and_then(|c| c["delta"]["content"].as_str()) {
            if !c.is_empty() {
                acc.push_str(c);
                on_delta(c.to_string());
            }
        }
        Ok(false)
    })
    .await?;

    ensure_non_empty(full, "请检查配置中的模型名称是否正确")
}

/* ────────── Anthropic 兼容:/v1/messages ────────── */

async fn stream_anthropic(
    cfg: &EngineCfg,
    text: &str,
    dir: &crate::lang::Direction,
    dict: bool,
    token: &CancellationToken,
    mut on_delta: impl FnMut(String),
) -> Result<String, String> {
    if cfg.base_url.is_empty() {
        return Err("请先在设置中配置 API 地址".into());
    }
    // Anthropic 的 base 填主机根,路径自动拼 /v1/messages
    let url = format!("{}/v1/messages", cfg.base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": cfg.model,
        "max_tokens": 8192,
        "system": build_prompt(&cfg.custom_prompt, dir.source.as_deref(), &dir.target, dict),
        "messages": [{ "role": "user", "content": text }],
        "stream": true,
    });

    let resp = client()?
        .post(&url)
        .header("x-api-key", &cfg.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Accept", "text/event-stream")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {e}"))?;
    let resp = check_status(resp).await?;

    let full = read_sse(resp, token, |v, acc| {
        match v["type"].as_str() {
            Some("content_block_delta") => {
                if let Some(t) = v["delta"]["text"].as_str() {
                    if !t.is_empty() {
                        acc.push_str(t);
                        on_delta(t.to_string());
                    }
                }
                Ok(false)
            }
            Some("message_stop") => Ok(true),
            Some("error") => Err(format!(
                "API 错误: {}",
                v["error"]["message"].as_str().unwrap_or("unknown")
            )),
            _ => Ok(false),
        }
    })
    .await?;

    ensure_non_empty(full, "请检查配置中的模型名称是否正确")
}

/* ────────── Gemini 兼容::streamGenerateContent ────────── */

async fn stream_gemini(
    cfg: &EngineCfg,
    text: &str,
    dir: &crate::lang::Direction,
    dict: bool,
    token: &CancellationToken,
    mut on_delta: impl FnMut(String),
) -> Result<String, String> {
    if cfg.base_url.is_empty() {
        return Err("请先在设置中配置 API 地址".into());
    }
    // Gemini 的 base 填主机根,路径自动拼 /v1beta/models/{model}:streamGenerateContent
    let url = format!(
        "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
        cfg.base_url.trim_end_matches('/'),
        cfg.model.trim()
    );
    let body = serde_json::json!({
        "systemInstruction": { "parts": [{ "text": build_prompt(&cfg.custom_prompt, dir.source.as_deref(), &dir.target, dict) }] },
        "contents": [{ "role": "user", "parts": [{ "text": text }] }],
        "generationConfig": { "temperature": cfg.temperature },
    });

    let resp = client()?
        .post(&url)
        .header("x-goog-api-key", &cfg.api_key)
        .header("Accept", "text/event-stream")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {e}"))?;
    let resp = check_status(resp).await?;

    let full = read_sse(resp, token, |v, acc| {
        if let Some(err) = v["error"]["message"].as_str() {
            return Err(format!("API 错误: {err}"));
        }
        if let Some(parts) = v["candidates"].get(0).and_then(|c| c["content"]["parts"].as_array()) {
            for part in parts {
                if let Some(t) = part["text"].as_str() {
                    if !t.is_empty() {
                        acc.push_str(t);
                        on_delta(t.to_string());
                    }
                }
            }
        }
        Ok(v["candidates"]
            .get(0)
            .and_then(|c| c["finishReason"].as_str())
            .is_some())
    })
    .await?;

    ensure_non_empty(full, "请检查配置中的模型名称是否正确")
}

/* ────────── 探活:按协议发一条极短非流式请求 ────────── */

pub async fn test_connection(cfg: &EngineCfg) -> Result<String, String> {
    if cfg.base_url.is_empty() {
        return Err("请先填写 API 地址".into());
    }
    let cl = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(25))
        .build()
        .map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();

    let (url, req) = match cfg.protocol() {
        "anthropic" => (
            format!("{}/v1/messages", cfg.base_url.trim_end_matches('/')),
            cl.post(format!("{}/v1/messages", cfg.base_url.trim_end_matches('/')))
                .header("x-api-key", &cfg.api_key)
                .header("anthropic-version", "2023-06-01")
                .json(&serde_json::json!({
                    "model": cfg.model,
                    "max_tokens": 8,
                    "messages": [{ "role": "user", "content": "ping" }],
                })),
        ),
        "gemini" => {
            let url = format!(
                "{}/v1beta/models/{}:generateContent",
                cfg.base_url.trim_end_matches('/'),
                cfg.model.trim()
            );
            let req = cl
                .post(&url)
                .header("x-goog-api-key", &cfg.api_key)
                .json(&serde_json::json!({
                    "contents": [{ "role": "user", "parts": [{ "text": "ping" }] }],
                }));
            (url, req)
        }
        _ => {
            let url = format!("{}/chat/completions", cfg.base_url);
            let req = cl
                .post(&url)
                .bearer_auth(&cfg.api_key)
                .json(&serde_json::json!({
                    "model": cfg.model,
                    "messages": [{ "role": "user", "content": "ping" }],
                    "max_tokens": 5,
                    "stream": false,
                    "temperature": 0,
                }));
            (url, req)
        }
    };

    let resp = req.send().await.map_err(|e| format!("连接失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        let msg = parse_err_body(&text);
        let msg = if msg.is_empty() { "请求失败".into() } else { msg };
        return Err(format!("API 错误 {}: {}", status.as_u16(), msg));
    }

    let ms = start.elapsed().as_millis();
    let v = serde_json::from_str::<serde_json::Value>(&text).ok();
    let ok_shape = match cfg.protocol() {
        "anthropic" => v.as_ref().map(|v| v["content"].is_array()).unwrap_or(false),
        "gemini" => v.as_ref().map(|v| v["candidates"].is_array()).unwrap_or(false),
        _ => v.as_ref().map(|v| v["choices"].is_array()).unwrap_or(false),
    };
    let _ = url;
    if ok_shape {
        Ok(format!("连接正常 · {ms} ms"))
    } else {
        Err("响应格式与所选协议不符".into())
    }
}
