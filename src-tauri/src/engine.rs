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
        // 词典模式:词/短语的释义而非整句翻译
        "You are a concise dictionary engine. The user will send a single word or a very short phrase.\n\
         Explain it for a native Chinese speaker:\n\
         - Non-Chinese word: give the IPA pronunciation in slashes, then part of speech and core meanings in Simplified Chinese, then ONE example sentence with its Chinese translation.\n\
         - Chinese word: give the pinyin, the meanings, and one example sentence.\n\
         - Output plain text only, compact (under 100 words), no markdown headings, no extra commentary."
            .to_string()
    } else {
        let dir = match source {
            Some(s) => format!("Translate the user's text from {s} into {target_name}."),
            None => format!("Translate the user's text into {target_name}."),
        };
        format!(
            "{dir}\n\
             Rules:\n\
             - Output ONLY the translation. No explanations, no surrounding quotes.\n\
             - Preserve line breaks and original formatting.\n\
             - Keep code, URLs, identifiers and proper nouns intact unless context demands translation.\n\
             - If the text is already in the target language, output it unchanged."
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
    match cfg.protocol() {
        "anthropic" => stream_anthropic(cfg, text, dir, dict, token, on_delta).await,
        "gemini" => stream_gemini(cfg, text, dir, dict, token, on_delta).await,
        _ => stream_openai(cfg, text, dir, dict, token, on_delta).await,
    }
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
