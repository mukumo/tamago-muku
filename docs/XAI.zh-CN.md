[English](XAI.md) · [日本語](XAI.ja.md) · **简体中文**

# xAI 聊天桥接

在 Mac 的 localhost 上运行、面向 muku 的 Grok 聊天页面。

```text
浏览器
  → 127.0.0.1:8790
  → xAI Responses API
  → xAI TTS（日语 WAV）
  → MUKU Webhook
  → 在 R36MAX 2 上显示与播放
```

## 前提条件

- macOS
- Python 3
- bash、curl、jq、security、say、afconvert
- R36MAX 2 已连接 Wi-Fi 并启动 MUKU

## 保存 API Key

将 API Key 保存到 macOS 钥匙串。

```sh
security add-generic-password -U -a kan -s tamago-muku-xai -w
```

输入的 Key 不会保存到 Shell 历史、plist、浏览器或仓库中。
`MUKU_XAI_KEYCHAIN_ACCOUNT` 只影响安装脚本的检查。LaunchAgent 不会把它传给桥接服务，因此如需使用其他账户名，还需在 `packaging/xai/com.muku.xai-bridge.plist.template` 中添加 `--keychain-account`。

## MUKU Token

将 MUKU 端的 Webhook 配置放在 Git 管理之外。

```text
.local/webhook.json
```

```json
{
  "bind": "0.0.0.0:8787",
  "token": "48 位随机 Token"
}
```

目录权限设为 0700，配置文件设为 0600。

## 安装

```sh
./scripts/install-xai-bridge.sh http://R36_IP:8787/webhook
```

创建并启动 `~/Library/LaunchAgents/com.muku.xai-bridge.plist`。
登录时也会自动启动，异常退出时会自动重启。

```text
页面:   http://127.0.0.1:8790
health: http://127.0.0.1:8790/health
```

如果 health 中的 `environment_isolated` 为 `true`，表示没有继承 Token、Secret、API Key 类的环境变量。

## 对话设置

- 模型：`grok-4.20-0309-non-reasoning`
- TTS 语音：`eve`
- 语言：`ja`
- 音频格式：PCM 16bit / mono / 22050Hz
- 网页搜索：通过页面上的复选框逐条消息指定
- 历史记录：仅在 Mac 的内存中保留最新 16 条消息
- xAI Responses API：`store=false`

模型和语音可以通过 `xai_muku_bridge.py` 的启动参数修改。

## 限制

- MUKU 关闭时无法显示和播放回复
- 当前的对话 UI 为文字输入
- 尚未确认 R36MAX 2 的内置麦克风
- 语音输入 STT 和设备单机语音对话是下一步工作
- Mac 休眠时，桥接服务以及在 Mac 上运行的隧道（如 Cloudflare Tunnel）会停止

## 停止与重新启动

```sh
launchctl bootout gui/$(id -u) "$HOME/Library/LaunchAgents/com.muku.xai-bridge.plist"
./scripts/install-xai-bridge.sh http://R36_IP:8787/webhook
```

轮换 API Key 后，请先覆盖同一钥匙串项目，然后重启 LaunchAgent。
