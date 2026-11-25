mod discord;
mod openrouter;

use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::path::PathBuf;

const DEFAULT_STATE_PATH: &str = "state/seen.json";

struct Args {
    dry_run: bool,
    state_path: PathBuf,
}

fn parse_args() -> Result<Args> {
    let mut dry_run = false;
    let mut state_path = PathBuf::from(DEFAULT_STATE_PATH);
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "--state" => {
                state_path = it.next().context("--state requires a path")?.into();
            }
            other => bail!("unknown argument: {other}"),
        }
    }
    Ok(Args { dry_run, state_path })
}

fn load_state(path: &PathBuf) -> Result<Option<BTreeSet<String>>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let ids: BTreeSet<String> = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(ids))
}

fn save_state(path: &PathBuf, ids: &BTreeSet<String>) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut text = serde_json::to_string_pretty(ids)?;
    text.push('\n');
    std::fs::write(path, text).with_context(|| format!("failed to write {}", path.display()))
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args()?;
    let client = reqwest::Client::builder()
        .user_agent("new-ai-model-notifier")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let targets = openrouter::select_targets(openrouter::fetch(&client).await?);
    let current: BTreeSet<String> = targets.iter().map(|m| m.base_id().to_string()).collect();
    println!("targets: {}", current.len());

    let Some(seen) = load_state(&args.state_path)? else {
        // 初回は既存モデルを通知せず、基準だけ記録する。
        println!("no state file; recording {} models without notifying", current.len());
        if !args.dry_run {
            save_state(&args.state_path, &current)?;
        }
        return Ok(());
    };

    let mut new_models: Vec<_> = targets
        .into_iter()
        .filter(|m| !seen.contains(m.base_id()))
        .collect();
    new_models.sort_by_key(|m| m.created.unwrap_or(0));
    println!("new models: {}", new_models.len());
    if new_models.is_empty() {
        return Ok(());
    }

    let payloads = discord::payloads(&new_models);
    if args.dry_run {
        for p in &payloads {
            println!("{}", serde_json::to_string_pretty(p)?);
        }
        return Ok(());
    }

    let webhook = std::env::var("DISCORD_WEBHOOK_URL")
        .context("DISCORD_WEBHOOK_URL is not set")?;
    discord::send(&client, &webhook, &payloads).await?;

    // 送信成功後にだけ更新する。失敗時は次回の実行で再送される。
    // 一時的に API から消えたモデルが再登場しても再通知しないよう、既知 ID は残す。
    let updated: BTreeSet<String> = seen.union(&current).cloned().collect();
    save_state(&args.state_path, &updated)
}
