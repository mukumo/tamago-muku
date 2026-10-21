[English](XAI.md) · **日本語** · [简体中文](XAI.zh-CN.md)

# xAI会話ブリッジ

Mac上のlocalhostで動く、muku向けのGrok会話画面です。

```text
ブラウザ
  → 127.0.0.1:8790
  → xAI Responses API
  → xAI TTS（日本語WAV）
  → MUKU Webhook
  → R36MAX 2で表示・再生
```

## 前提

- macOS
- Python 3
- bash、curl、jq、security、say、afconvert
- R36MAX 2のWi-Fi接続とMUKU起動

## APIキーの保存

APIキーはmacOS Keychainへ保存します。

```sh
security add-generic-password -U -a kan -s tamago-muku-xai -w
```

入力したキーはシェル履歴、plist、ブラウザ、リポジトリへ保存しません。
`MUKU_XAI_KEYCHAIN_ACCOUNT`はインストールスクリプトの確認にだけ使われます。LaunchAgentはブリッジへ渡さないため、別アカウント名を使う場合は`packaging/xai/com.muku.xai-bridge.plist.template`にも`--keychain-account`を追加します。

## MUKUトークン

MUKU側のWebhook設定をGit管理外へ置きます。

```text
.local/webhook.json
```

```json
{
  "bind": "0.0.0.0:8787",
  "token": "48文字のランダムトークン"
}
```

ディレクトリは0700、設定ファイルは0600にします。

## インストール

```sh
./scripts/install-xai-bridge.sh http://R36_IP:8787/webhook
```

`~/Library/LaunchAgents/com.muku.xai-bridge.plist`を作成し、起動します。
ログイン時にも自動起動し、異常終了時は再起動します。

```text
画面:   http://127.0.0.1:8790
health: http://127.0.0.1:8790/health
```

healthの`environment_isolated`が`true`なら、Token・Secret・API key系の
環境変数を継承していません。

## 会話設定

- モデル: `grok-4.20-0309-non-reasoning`
- TTS voice: `eve`
- 言語: `ja`
- 音声形式: PCM 16bit / mono / 22050Hz
- Web検索: 画面のチェックで送信ごとに指定
- 履歴: 最新16メッセージをMacのメモリ内だけに保持
- xAI Responses API: `store=false`

モデルとvoiceは`xai_muku_bridge.py`の起動引数で変更できます。

## 制約

- MUKUを閉じていると返答の表示・再生はできません
- 現在の会話UIは文字入力です
- R36MAX 2の内蔵マイクは確認できていません
- 音声入力STTと端末単体の音声会話は次の段階です
- Macがスリープすると、ブリッジとMac上で動かしているトンネル（Cloudflare Tunnelなど）は停止します

## 停止と再開

```sh
launchctl bootout gui/$(id -u) "$HOME/Library/LaunchAgents/com.muku.xai-bridge.plist"
./scripts/install-xai-bridge.sh http://R36_IP:8787/webhook
```

APIキーをローテーションした場合は、同じKeychain項目を上書きしてから
LaunchAgentを再起動します。
