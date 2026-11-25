# new-ai-model-notifier

OpenRouter に新しく載った OpenAI / Anthropic / Gemini のモデルを Discord Webhook に通知する。

- 対象: `openai/*`、`anthropic/*`、`google/gemini*`。`:free` / `:batch` などの派生はベース ID にまとめる。
- 判定: `state/seen.json` にない ID を新規とみなす。状態ファイルがない初回は通知せず記録だけ行う。
- 実行: GitHub Actions が15分ごとに実行し、`state/seen.json` を commit する。

## セットアップ

1. Discord のチャンネルで Webhook を作る。
2. リポジトリの Secret `DISCORD_WEBHOOK_URL` に Webhook URL を登録する。複数サーバーに送るときは、URL を改行区切りで並べる。全サーバーに同じ通知が届く。
3. Actions の `notify` を手動実行して初回の基準を作る。

## ローカル実行

```bash
cargo run --release -- --dry-run --state /tmp/seen.json
```

`--dry-run` は送信と状態更新をしない。通常実行には環境変数 `DISCORD_WEBHOOK_URL` が必要。

一部の Webhook だけ失敗したときは、状態を更新せずエラー終了する。次回の実行で全サーバーに再送されるため、成功済みのサーバーには重複して届く。
