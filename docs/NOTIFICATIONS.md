# AI通知とWebhook

Wi-Fi内蔵モデル用の最初の受信経路です。
端末自身がLAN内で待ち受け、認証済みのJSONを受け取るとmukuの部屋に通知を表示します。

## 受信仕様

```text
POST /webhook HTTP/1.1
Authorization: Bearer <token>
Content-Type: application/json

{
  "id": "optional-id",
  "title": "AIからのおしらせ",
  "body": "15時から予定があります",
  "source": "ai",
  "kind": "notice",
  "audio_wav_base64": "UklGRi..."
}
```

- `title`: 1〜64文字。必須。
- `body`: 0〜180文字。
- `source`: 最大32文字。省略時は`webhook`。
- `kind`: 最大24文字。省略時は`notice`。
- 本文は180文字、HTTP本文は4MiBまで。制御文字は拒否します。
- `audio_wav_base64`は任意です。Base64で符号化したPCM WAVを最大2MiBまで受け付けます。
- トークンが16文字未満ならサーバーを開始しません。
- 成功時は`202 Accepted`。認証失敗は`401`、形式不正は`422`です。
- 受信中のゲームを止めず、部屋に戻ったときに9秒間表示します。
- 日本語、英数字、記号を同梱のOFLフォントで表示します。
- 音声付きの場合、表示と同じタイミングで`aplay`へ非同期で渡します。

## SDへ設定

まず推測されにくいトークンを作ります。

```sh
openssl rand -hex 24
```

`config/webhook.example.json`を元に`webhook.json`を作り、トークンを差し替えます。
秘密のトークンをリポジトリへcommitしないでください。

```json
{
  "bind": "0.0.0.0:8787",
  "token": "生成した48文字の乱数"
}
```

R36MAX 2では`/storage/muku/webhook.json`へ置きます。
Portsの`MUKU`起動スクリプトは、このファイルがあれば自動で受信サーバーも開始します。
ファイルがなければ、これまで通り完全オフラインで動きます。

## 送信テスト

### xAIのTTS音声付き

Grokの返答とxAI TTSをまとめてMUKUへ送るローカル画面があります。

```text
http://127.0.0.1:8790
```

APIキーはKeychain、MUKUトークンはGit管理外の`.local/webhook.json`に保持します。
設定と制約は[xAI会話ブリッジ](XAI.md)を参照してください。

### macOS標準音声付き

`say`で日本語WAVを作り、本文と一緒にWebhookへ送るスクリプトがあります。

```sh
./scripts/send-notice-tts.sh \
  --url http://R36_IP:8787/webhook \
  --config /path/to/webhook.json \
  --title "Codexからお知らせ" \
  --body "少し休憩しませんか。温かいお茶をどうぞ。"
```

- 標準音声: `Kyoko`
- 速度: 185語/分
- 音声形式: PCM 16bit / mono / 22050Hz
- `--voice`、`--rate`、`--source`で変更できます
- `--dry-run`は送信せず、生成JSONの文字数と音声サイズだけを表示します
- トークンは設定ファイルから読み、コマンドラインや標準出力へ表示しません

### テキストのみ

端末のLANアドレスを`192.0.2.10`、トークンを環境変数`MUKU_WEBHOOK_TOKEN`に入れた例です。

```sh
curl --fail-with-body \
  -X POST http://192.0.2.10:8787/webhook \
  -H "Authorization: Bearer $MUKU_WEBHOOK_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{
    "title": "AIからのおしらせ",
    "body": "少し休憩しませんか",
    "source": "ai"
  }'
```

AIサービスや外部Webhookは、端末のLANアドレスへ直接届かない場合があります。
その場合はCloudflare Tunnelなどの認証付きトンネル、または署名検証を行う中継を前段に置きます。
受信口を無認証のままインターネットへ公開しません。

## 現時点の制約

- 平文HTTPです。同じLAN内、または信頼できるトンネルの終端からだけ使います。
- 通知キューは実行中のプロセス内だけです。再起動後にも残す永続キューは次の段階です。
- AI側の生成、要約、送信スケジュールはまだ接続していません。

2026-10-20にR36MAX 2（EmuELEC 4.7、AIC8800 Wi-Fi）への認証付きPOSTと
`202 Accepted`応答を実機で確認しました。
