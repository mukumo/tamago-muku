**English** · [日本語](XAI.ja.md) · [简体中文](XAI.zh-CN.md)

# xAI chat bridge

A Grok chat page for muku that runs on localhost on your Mac.

```text
Browser
  → 127.0.0.1:8790
  → xAI Responses API
  → xAI TTS (Japanese WAV)
  → MUKU Webhook
  → Shown and played on the R36MAX 2
```

## Requirements

- macOS
- Python 3
- bash, curl, jq, security, say, afconvert
- R36MAX 2 connected to Wi-Fi with MUKU running

## Storing the API key

Store the API key in the macOS Keychain.

```sh
security add-generic-password -U -a kan -s tamago-muku-xai -w
```

The key you enter is never saved to shell history, plists, the browser, or the repository.
`MUKU_XAI_KEYCHAIN_ACCOUNT` only changes the account the install script checks. The LaunchAgent does not pass it to the bridge, so to use a different account name also add `--keychain-account` to `packaging/xai/com.muku.xai-bridge.plist.template`.

## MUKU token

Keep MUKU's webhook settings outside Git.

```text
.local/webhook.json
```

```json
{
  "bind": "0.0.0.0:8787",
  "token": "48-character random token"
}
```

Set the directory to 0700 and the config file to 0600.

## Installation

```sh
./scripts/install-xai-bridge.sh http://R36_IP:8787/webhook
```

This creates `~/Library/LaunchAgents/com.muku.xai-bridge.plist` and starts it.
It also starts automatically at login and restarts if it exits abnormally.

```text
Page:   http://127.0.0.1:8790
health: http://127.0.0.1:8790/health
```

If `environment_isolated` in health is `true`, the bridge has not inherited any token, secret, or API key
environment variables.

## Chat settings

- Model: `grok-4.20-0309-non-reasoning`
- TTS voice: `eve`
- Language: `ja`
- Audio format: PCM 16bit / mono / 22050Hz
- Web search: toggled per message with the checkbox on the page
- History: the latest 16 messages, kept only in memory on the Mac
- xAI Responses API: `store=false`

The model and voice can be changed with the launch arguments of `xai_muku_bridge.py`.

## Limitations

- Replies cannot be shown or played while MUKU is closed
- The current chat UI takes text input
- The R36MAX 2's built-in microphone has not been confirmed
- Voice input (STT) and standalone voice chat on the device are the next step
- The bridge, and any tunnel running on the Mac (such as Cloudflare Tunnel), stop when the Mac sleeps

## Stopping and restarting

```sh
launchctl bootout gui/$(id -u) "$HOME/Library/LaunchAgents/com.muku.xai-bridge.plist"
./scripts/install-xai-bridge.sh http://R36_IP:8787/webhook
```

If you rotate the API key, overwrite the same Keychain item and then
restart the LaunchAgent.
