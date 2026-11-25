use anyhow::{Context, Result};
use serde::Deserialize;

const MODELS_URL: &str = "https://openrouter.ai/api/v1/models";

#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub created: Option<i64>,
    pub context_length: Option<u64>,
    pub description: Option<String>,
    pub pricing: Option<Pricing>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pricing {
    pub prompt: Option<String>,
    pub completion: Option<String>,
}

#[derive(Deserialize)]
struct ModelsResponse {
    data: Vec<Model>,
}

impl Model {
    /// `:free` / `:batch` などの派生を除いた ID。通知・状態管理の単位。
    pub fn base_id(&self) -> &str {
        self.id.split(':').next().unwrap_or(&self.id)
    }

    pub fn provider(&self) -> &str {
        self.base_id().split('/').next().unwrap_or("")
    }
}

pub async fn fetch(client: &reqwest::Client) -> Result<Vec<Model>> {
    let res: ModelsResponse = client
        .get(MODELS_URL)
        .send()
        .await
        .context("OpenRouter models request failed")?
        .error_for_status()
        .context("OpenRouter models returned an error status")?
        .json()
        .await
        .context("failed to parse OpenRouter models response")?;
    Ok(res.data)
}

/// OpenAI / Anthropic / Gemini のみを残し、派生 ID をベース ID に畳む。
/// 同じベース ID が複数あるときは、サフィックスなしの本体を優先する。
pub fn select_targets(models: Vec<Model>) -> Vec<Model> {
    let mut out: Vec<Model> = Vec::new();
    for m in models {
        let base = m.base_id();
        let wanted = base.starts_with("openai/")
            || base.starts_with("anthropic/")
            || base.starts_with("google/gemini");
        if !wanted {
            continue;
        }
        match out.iter().position(|x| x.base_id() == base) {
            None => out.push(m),
            Some(i) if m.id == base && out[i].id != base => out[i] = m,
            Some(_) => {}
        }
    }
    out
}
