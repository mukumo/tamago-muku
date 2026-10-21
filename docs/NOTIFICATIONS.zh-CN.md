[English](NOTIFICATIONS.md) · [日本語](NOTIFICATIONS.ja.md) · **简体中文**

# AI 通知与 Webhook

面向 Wi-Fi 机型的第一条接收通路。
设备自身在局域网内监听，收到经过认证的 JSON 后，会在 muku 的房间里显示通知。

## 接收规格

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

- `title`：1～64 个字符。必填。
- `body`：0～180 个字符。
- `source`：最多 32 个字符。省略时为 `webhook`。
- `kind`：最多 24 个字符。省略时为 `notice`。
- 正文最多 180 个字符，HTTP 请求体最多 4MiB。拒绝控制字符。
- `audio_wav_base64` 为可选项。接受 Base64 编码的 PCM WAV，最大 2MiB。
- Token 少于 16 个字符时不会启动服务器。
- 成功时返回 `202 Accepted`。认证失败返回 `401`，格式错误返回 `422`。
- 不会打断正在进行的游戏，回到房间时显示 9 秒。
- 日文、英文字母、数字和符号使用随附的 OFL 字体显示。
- 附带语音时，在显示通知的同时异步交给 `aplay` 播放。

## 在 SD 卡上配置

首先生成一个难以猜测的 Token。

```sh
openssl rand -hex 24
```

以 `config/webhook.example.json` 为模板创建 `webhook.json`，并替换 Token。
请勿将机密 Token 提交（commit）到仓库。

```json
{
  "bind": "0.0.0.0:8787",
  "token": "生成的 48 位随机字符串"
}
```

在 R36MAX 2 上放置于 `/storage/muku/webhook.json`。
存在该文件时，Ports 中的 `MUKU` 启动脚本会自动同时启动接收服务器。
没有该文件时，与之前一样完全离线运行。

## 发送测试

### 附带 xAI TTS 语音

有一个本地页面，可以将 Grok 的回复和 xAI TTS 语音一起发送到 MUKU。

```text
http://127.0.0.1:8790
```

API Key 保存在钥匙串中，MUKU Token 保存在不受 Git 管理的 `.local/webhook.json` 中。
配置与限制请参阅 [xAI 聊天桥接](XAI.zh-CN.md)。

### 附带 macOS 系统语音

有一个脚本可以用 `say` 生成日语 WAV，并与正文一起发送到 Webhook。

```sh
./scripts/send-notice-tts.sh \
  --url http://R36_IP:8787/webhook \
  --config /path/to/webhook.json \
  --title "Codexからお知らせ" \
  --body "少し休憩しませんか。温かいお茶をどうぞ。"
```

- 默认语音：`Kyoko`
- 语速：185 词/分钟
- 音频格式：PCM 16bit / mono / 22050Hz
- 可以通过 `--voice`、`--rate`、`--source` 修改
- `--dry-run` 不会发送，只显示生成的 JSON 字符数和音频大小
- Token 从配置文件读取，不会出现在命令行或标准输出中

### 仅文字

以下示例中，设备的局域网地址为 `192.0.2.10`，Token 存放在环境变量 `MUKU_WEBHOOK_TOKEN` 中。

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

AI 服务或外部 Webhook 可能无法直接访问设备的局域网地址。
这种情况下，请在前面加一层 Cloudflare Tunnel 等带认证的隧道，或进行签名校验的中转服务。
请勿在无认证的状态下将接收端口暴露到互联网。

## 目前的限制

- 使用明文 HTTP。仅在同一局域网内，或从可信隧道的终端使用。
- 通知队列只存在于运行中的进程内。重启后仍能保留的持久化队列是下一步工作。
- 尚未接入 AI 端的生成、摘要和发送调度。

2026-10-20 已在 R36MAX 2（EmuELEC 4.7、AIC8800 Wi-Fi）实机上确认带认证的 POST 和
`202 Accepted` 响应。
