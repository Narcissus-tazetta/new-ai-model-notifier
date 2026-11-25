# new-ai-model-notifier

Posts new OpenAI / Anthropic / Gemini models listed on OpenRouter to Discord webhooks.

- Targets: `openai/*`, `anthropic/*`, and `google/gemini*`. Variants such as `:free` and `:batch` are folded into the base ID.
- Detection: any ID not in `state/seen.json` is treated as new. When the state file does not exist (first run), models are recorded without notifying.
- Japanese notifications: descriptions are summarized in Japanese (up to 120 characters) with `claude-haiku-5-5`. If summarization fails or no API key is set, the original English description is sent and the embed footer says so.
- Schedule: GitHub Actions runs every 15 minutes and commits `state/seen.json`.

## Setup

1. Create a webhook in a Discord channel.
2. Add the webhook URL to the repository secret `DISCORD_WEBHOOK_URL`. To post to multiple servers, put one URL per line; every server receives the same notification.
3. To summarize descriptions in Japanese, add a Claude API key to the secret `ANTHROPIC_API_KEY`. Without it, descriptions are sent untranslated.
4. Run the `notify` workflow manually once to record the initial baseline.

## Local run

```bash
cargo run --release -- --dry-run --state /tmp/seen.json
```

`--dry-run` sends nothing to Discord and does not update the state file. It still calls the Claude API when `ANTHROPIC_API_KEY` is set. A normal run requires the environment variable `DISCORD_WEBHOOK_URL`.

If only some webhooks fail, the run exits with an error without updating the state. The next run resends to every server, so servers that already succeeded receive duplicates.
