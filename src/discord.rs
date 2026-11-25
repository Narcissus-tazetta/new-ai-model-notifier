use crate::openrouter::Model;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::time::Duration;

// Discord は 1 メッセージあたり Embed 10 件まで。
const EMBEDS_PER_MESSAGE: usize = 10;
const DESCRIPTION_LIMIT: usize = 300;

fn per_million(price: &Option<String>) -> String {
    match price.as_deref().and_then(|s| s.parse::<f64>().ok()) {
        Some(p) if p >= 0.0 => format!("${:.2}", p * 1_000_000.0),
        _ => "-".to_string(),
    }
}

fn truncate(s: &str, limit: usize) -> String {
    if s.chars().count() <= limit {
        return s.to_string();
    }
    let head: String = s.chars().take(limit).collect();
    format!("{head}…")
}

// 本体が無く派生しかないモデルでも、通知の名前にはサフィックスを出さない。
fn display_name(m: &Model) -> &str {
    m.name
        .strip_suffix(" (batch)")
        .or_else(|| m.name.strip_suffix(" (free)"))
        .unwrap_or(&m.name)
}

pub fn embed(m: &Model) -> Value {
    let (provider, color) = match m.provider() {
        "openai" => ("OpenAI", 0x10a37f),
        "anthropic" => ("Anthropic", 0xd97757),
        "google" => ("Google", 0x4285f4),
        other => (other, 0x5865f2),
    };
    let context = m
        .context_length
        .map(|c| format!("{} tokens", c))
        .unwrap_or_else(|| "-".to_string());
    let (input, output) = match &m.pricing {
        Some(p) => (per_million(&p.prompt), per_million(&p.completion)),
        None => ("-".to_string(), "-".to_string()),
    };
    json!({
        "title": display_name(m),
        "url": format!("https://openrouter.ai/{}", m.base_id()),
        "description": truncate(m.description.as_deref().unwrap_or(""), DESCRIPTION_LIMIT),
        "color": color,
        "fields": [
            { "name": "Provider", "value": provider, "inline": true },
            { "name": "Context", "value": context, "inline": true },
            { "name": "Input / 1M tokens", "value": input, "inline": true },
            { "name": "Output / 1M tokens", "value": output, "inline": true },
        ],
    })
}

pub fn payloads(models: &[Model]) -> Vec<Value> {
    models
        .chunks(EMBEDS_PER_MESSAGE)
        .map(|chunk| {
            json!({
                "content": "🆕 New models on OpenRouter",
                "embeds": chunk.iter().map(embed).collect::<Vec<_>>(),
            })
        })
        .collect()
}

pub async fn send(client: &reqwest::Client, webhook_url: &str, payloads: &[Value]) -> Result<()> {
    for (i, payload) in payloads.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        // reqwest のエラーには URL(トークン入り)が含まれるため、外してから返す。
        client
            .post(webhook_url)
            .json(payload)
            .send()
            .await
            .map_err(reqwest::Error::without_url)
            .context("Discord webhook request failed")?
            .error_for_status()
            .map_err(reqwest::Error::without_url)
            .context("Discord webhook returned an error status")?;
    }
    Ok(())
}

/// 全 Webhook に送る。一部が失敗しても残りには送り、最後にまとめてエラーにする。
/// URL にはトークンが含まれるため、エラーには番号だけを出す。
pub async fn send_all(
    client: &reqwest::Client,
    webhook_urls: &[String],
    payloads: &[Value],
) -> Result<()> {
    let mut failed = Vec::new();
    for (i, url) in webhook_urls.iter().enumerate() {
        if let Err(e) = send(client, url, payloads).await {
            eprintln!("webhook #{}: {e:#}", i + 1);
            failed.push(i + 1);
        }
    }
    if !failed.is_empty() {
        bail!("{} of {} webhooks failed: #{:?}", failed.len(), webhook_urls.len(), failed);
    }
    Ok(())
}
