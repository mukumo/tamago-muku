**English** · [日本語](NOTIFICATIONS.ja.md) · [简体中文](NOTIFICATIONS.zh-CN.md)

# AI notices and webhooks

The first receive path for the Wi-Fi model.
The device itself listens on the LAN, and when it receives authenticated JSON, it shows a notice in muku's room.

## Receive specification

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

- `title`: 1–64 characters. Required.
- `body`: 0–180 characters.
- `source`: up to 32 characters. Defaults to `webhook` when omitted.
- `kind`: up to 24 characters. Defaults to `notice` when omitted.
- The body text is limited to 180 characters and the HTTP body to 4MiB. Control characters are rejected.
- `audio_wav_base64` is optional. A Base64-encoded PCM WAV of up to 2MiB is accepted.
- The server does not start if the token is shorter than 16 characters.
- Success returns `202 Accepted`. Authentication failure returns `401`, and malformed input returns `422`.
- A notice never interrupts a running game; it is shown for 9 seconds when you return to the room.
- Japanese, alphanumerics, and symbols are rendered with the bundled OFL font.
- When audio is attached, it is handed to `aplay` asynchronously as soon as the notice is received. It does not wait for the text, so it also plays during a game.

## Configure on the SD card

First, generate a hard-to-guess token.

```sh
openssl rand -hex 24
```

Create `webhook.json` from `config/webhook.example.json` and replace the token.
Do not commit the secret token to the repository.

```json
{
  "bind": "0.0.0.0:8787",
  "token": "the 48-character random string you generated"
}
```

On the R36MAX 2, place it at `/storage/muku/webhook.json`.
If this file exists, the `MUKU` launch script in Ports also starts the receive server automatically.
Without the file, the app keeps running fully offline as before.

## Sending tests

### With xAI TTS audio

A local page sends Grok's replies and xAI TTS audio together to MUKU.

```text
http://127.0.0.1:8790
```

The API key is kept in the Keychain, and the MUKU token in `.local/webhook.json`, which is not tracked by Git.
For setup and limitations, see [xAI chat bridge](XAI.md).

### With the standard macOS voice

A script builds a Japanese WAV with `say` and sends it to the webhook together with the text.

```sh
./scripts/send-notice-tts.sh \
  --url http://R36_IP:8787/webhook \
  --config /path/to/webhook.json \
  --title "Codexからお知らせ" \
  --body "少し休憩しませんか。温かいお茶をどうぞ。"
```

- Default voice: `Kyoko`
- Rate: 185 words/min
- Audio format: PCM 16bit / mono / 22050Hz
- Change these with `--voice`, `--rate`, and `--source`
- `--dry-run` sends nothing and only shows the character counts of the generated JSON and the audio size
- The token is read from the config file and never shown on the command line or standard output

### Text only

An example with the device's LAN address as `192.0.2.10` and the token in the environment variable `MUKU_WEBHOOK_TOKEN`.

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

AI services and external webhooks may not be able to reach the device's LAN address directly.
In that case, put an authenticated tunnel such as Cloudflare Tunnel, or a relay that verifies signatures, in front of it.
Never expose the receive endpoint to the internet without authentication.

## Current limitations

- It is plain HTTP. Use it only within the same LAN or from the end of a trusted tunnel.
- The notice queue lives only inside the running process. A persistent queue that survives restarts is the next step.
- AI-side generation, summarization, and send scheduling are not connected yet.

On 2026-10-20, an authenticated POST to the R36MAX 2 (EmuELEC 4.7, AIC8800 Wi-Fi) and
the `202 Accepted` response were confirmed on real hardware.
