<div align="center">

[English](README.md) · [日本語](README.ja.md) · **简体中文**

<img src="docs/images/device-hello.jpg" alt="手持掌机的照片，屏幕上的 muku 显示 “a good day to be muku.”" width="420">

# tamago-muku

**在口袋大小的掌机里，和软绵绵的 muku 一起生活吧 ☁️**

[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-00C4FF?logo=rust&logoColor=white)](#1-构建)
[![Works offline](https://img.shields.io/badge/works-offline-35C838)](#-和-muku-一起生活)
[![Handhelds](https://img.shields.io/badge/RK3562%20%C2%B7%20RK3326-handheld-F3A6FF)](#-支持的掌机)

</div>

tamago-muku 是一款用 Rust 编写的电子宠物应用，让你在 RK3562 / RK3326 小型掌机里和 **muku** 一起生活。
应用直接绘制到屏幕，即使没有网络，也能照顾 muku、玩小游戏。
在内置 Wi-Fi 的 R36MAX 2 上，muku 还可以帮你转达来自 AI 的通知（可选）。

## 🎬 实机运行效果

<div align="center">

<table>
  <tr>
    <td align="center" width="33%">
      <img src="docs/images/device-idle.webp" alt="掌机屏幕上 muku 缓慢呼吸的视频" width="240">
    </td>
    <td align="center" width="33%">
      <img src="docs/images/device-pet.webp" alt="按 A 键摸摸后 muku 露出笑脸并冒出爱心的视频" width="240">
    </td>
    <td align="center" width="33%">
      <img src="docs/images/device-care.webp" alt="依次摸摸、喂食、一起玩的视频" width="240">
    </td>
  </tr>
  <tr>
    <td align="center"><b>🫧 日常</b><br>缓慢地呼吸<br>时不时眨眨眼</td>
    <td align="center"><b>💗 摸摸</b><br>按 A 键摸摸<br>露出笑脸和爱心</td>
    <td align="center"><b>🍙 照顾</b><br>摸摸 → 喂食<br>→ 一起玩</td>
  </tr>
</table>

<sub>拍摄于 RK3562 4.3 英寸掌机（480×272）。视频无声，循环播放。<br>拍摄时为早期版本，因此画面下方 X 显示为 “play”（现为 “games”，用于选择小游戏）。</sub>

</div>

<details>
<summary>📺 查看屏幕录制</summary>
<br>
<img src="docs/images/muku-demo.gif" alt="muku 房间的屏幕录制" width="480">
</details>

## 🐣 和 muku 一起生活

muku 有 3 个状态条。

| 状态条 | 屏幕显示 | 如何补充 |
|---|---|---|
| 🍙 饱腹 | `food` | 喂食 |
| 💗 心情 | `happy` | 摸摸、一起玩 |
| 💤 精力 | `rest` | 好好睡一觉 |

- 呼吸、眨眼、被摸摸时的反应，让 muku 一直陪在你身边
- 状态条会在运行期间慢慢变化
- 状态自动保存
- 关机期间不计算时间，所以**就算放着不管，muku 也不会变虚弱或消失** 🕊️

<div align="center">
  <img src="docs/images/device-sleepy.jpg" alt="muku 显示 “getting sleepy.”、看起来很困的照片" width="360">
  <br>
  <sub>精力变低时，muku 会告诉你 “getting sleepy.” 💤</sub>
</div>

## 🎮 玩法

在主界面，用 4 个按键就能照顾 muku。

| 按键 | 功能 |
|:---:|---|
| **A** / ↑ | 💗 摸摸 |
| **B** / → | 🍙 喂食 |
| **X** / ↓ | 🎮 选择小游戏 |
| **Y** / ← | 💤 睡觉 / 起床 |
| **START** | 💾 保存并返回掌机原有菜单 |

<details>
<summary>其他界面的操作</summary>

| 界面 | 操作 |
|---|---|
| 游戏选择 | 左右：移动光标，A / X：开始，B / Y：主界面，START：主界面 |
| STAR CATCH | 左右：移动 muku，START：立即结算 |
| BEAT GARDEN | A / B / X / Y：敲击对应轨道的音符，START：立即结算 |
| 结算 | A / X：再玩一次，B / Y：游戏选择，START / 方向键：主界面 |

</details>

## 🌟 小游戏

<table>
  <tr>
    <td align="center" width="50%">
      <img src="docs/images/star-catch.png" alt="STAR CATCH 游戏画面" width="360">
    </td>
    <td align="center" width="50%">
      <img src="docs/images/beat-garden.png" alt="BEAT GARDEN 游戏画面" width="360">
    </td>
  </tr>
  <tr>
    <td>
      <b>⭐ STAR CATCH</b><br>
      在 25 秒内左右移动 muku，接住掉落的星星。
      金色星星 3 分。漏接 6 次本局结束。
    </td>
    <td>
      <b>🎵 BEAT GARDEN</b><br>
      配合 4 条轨道落下的音符按 A/B/X/Y。
      越接近判定线得分越高，连击可以提升分数。
    </td>
  </tr>
</table>

结算以 1～5 颗星显示，游玩会影响 muku 的心情和精力。

## 💌 来自 AI 的通知（Wi-Fi 机型）

<div align="center">
  <img src="docs/images/japanese-notification.png" alt="muku 的房间里显示日文通知的画面" width="480">
</div>

已在 R36MAX 2 实机上验证面向局域网的 HTTP Webhook。
从 AI 等工具发送通知后，muku 会在房间里显示（支持日文）。

- 文字不会打断游戏，回到主界面时显示 9 秒
- 附带的 WAV 会在收到时立即用机身扬声器播放（游戏中也会播放）🔊
- 没有 `/storage/muku/webhook.json` 时，与之前一样完全离线运行

接收地址为 `POST /webhook`，必须携带 Bearer Token。

```json
{
  "title": "AIからのおしらせ",
  "body": "少し休憩しませんか",
  "source": "ai",
  "kind": "notice"
}
```

> [!NOTE]
> 通知文字使用日文字体 M PLUS Rounded 1c 显示。
> 该字体缺少部分简体中文字形（如“们”“这”“说”），目前建议发送日文内容。

WAV 以 Base64 形式附带。
用 macOS 的日语语音生成 WAV 并发送到 Webhook：

```sh
./scripts/send-notice-tts.sh \
  --url http://R36_IP:8787/webhook \
  --config /path/to/webhook.json \
  --title "AIからのおしらせ" \
  --body "少し休憩しませんか"
```

语音由 Mac 的 `say` 和 `afconvert` 生成，默认使用 `Kyoko`。

📖 配置方法、发送示例以及对外公开 Webhook 时的注意事项，请参阅
[Webhook 通知设置](docs/NOTIFICATIONS.zh-CN.md)。
R36MAX 2 的 SSH 与部署请参阅 [R36MAX 2 实机笔记](docs/R36MAX2.zh-CN.md)。

## 🗣️ 和 Grok 聊天

在 Mac 上运行本地聊天页面，让 muku 显示并朗读 Grok 的回复。

```sh
# 仅首次
security add-generic-password -U -a kan -s tamago-muku-xai -w
# 将包含设备 bind/token 的 webhook.json 放到 .local/webhook.json（权限 0600，参见 docs/XAI.zh-CN.md）
./scripts/install-xai-bridge.sh http://R36_IP:8787/webhook
```

然后在 Mac 上打开 <http://127.0.0.1:8790>。

- API Key 从 macOS 钥匙串读取，不会传给浏览器或 R36
- 每次对话都可以选择是否使用网页搜索
- 用 xAI TTS 将 Grok 的回复转换为日语 WAV
- 使用 muku 现有的 Webhook Token 将文字和语音发送到 R36
- LaunchAgent 以最小环境启动，不继承登录会话中无关的 Token、API Key 等敏感环境变量

📖 详情请参阅 [xAI 聊天桥接](docs/XAI.zh-CN.md)。

## 📟 支持的掌机

|  | RK3562 4.3 英寸掌机 | XiFan R36MAX 2 |
|---|---|---|
| 主控 | RK3562（Cortex-A53） | RK3326（Cortex-A35） |
| 屏幕 | 480×272 | 1024×768（将 480×272 的界面放大） |
| 连接方式 | USB（adb） | Wi-Fi（SSH） |
| 自启动 | 从 SD 卡启动 | 先于 EmulationStation 显示 |
| 通知・TTS | — | ✅ |
| 实机笔记 | [硬件笔记](docs/HARDWARE.zh-CN.md) | [R36MAX 2 实机笔记](docs/R36MAX2.zh-CN.md) |

<details>
<summary>✅ 已在实机上验证的内容</summary>

2026-10-20，在通过 USB 连接的 RK3562 4.3 英寸掌机上确认了：

- 480×272 的 DRM 绘制和 3 个 evdev 输入设备
- 8 秒绘制 160 帧（约 20fps）
- A/B/X/Y 操作
- 退出后返回原厂菜单
- 从 SD 卡自启动
- 机身上运行的状态、存档、Linux 绘制相关测试 13 项

同日，在 R36MAX 2（RK3326、1024×768、EmuELEC 4.7）上确认了：

- 放置 Ports 启动项，并在切换显示所有权后实机启动与退出
- A/B/X 输入和机身存档
- 1024×768 下 18 秒绘制 331 帧（约 18fps）
- 重启时先于 EmulationStation 自动显示 muku
- Webhook 返回 `202 Accepted`
- 独立 Dropbear SSH 服务的公钥认证

面向 Linux 的 Clippy、23 项 Rust 测试和 aarch64 静态构建均已通过。

</details>

## 🛠️ 使用方法（开发者）

### 1. 构建

```sh
rustup target add aarch64-unknown-linux-musl
cargo test
cargo build --locked --release --target aarch64-unknown-linux-musl
```

`.cargo/config.toml` 指定了 Rust 自带的链接器和针对 Cortex-A53 的链接器修正。
即使用户空间是 32 位，也能在实机的 64 位内核上运行。

### 2. 只预览画面

```sh
cargo run -- --preview /tmp/muku.png
cargo run -- --preview-game select /tmp/games.png
```

### 3. 通过 USB 临时启动

```sh
./scripts/run-usb.sh YOUR_ADB_SERIAL
```

将 `YOUR_ADB_SERIAL` 替换为 `adb devices -l` 中显示的目标设备序列号。
应用会被传到 `/tmp/muku/`，退出时恢复原厂菜单。
不会修改内置 NAND 和 SD 卡上的启动脚本。
临时启动的存档位于 `/tmp/muku/state.json`，关机后消失。

### 4. 从 SD 卡启动

<details>
<summary>安装到 SD 卡及恢复原状</summary>

SD 卡上的 `/muku/muku-pet` 是应用，`/muku/state.json` 是存档。
原厂的 `/simulator/3562/em_ui.sh` 会备份为 `em_ui.before-muku.sh`，
然后从 SD 卡上的入口启动 muku。
按 START 或应用出错时，会返回掌机原有菜单。

不会修改卡内的游戏和 ROM，也不会修改掌机的内置固件。
拔出 SD 卡启动时，运行原有的内置菜单。

在 macOS 上，安装到以可写方式挂载的目标卡。
将 `YOUR_SD_VOLUME_UUID` 替换为 `diskutil info /Volumes/Untitled` 中的
Volume UUID。

```sh
python3 scripts/install-sd.py \
  --volume /Volumes/Untitled \
  --expected-uuid YOUR_SD_VOLUME_UUID \
  --binary target/aarch64-unknown-linux-musl/release/muku-pet \
  --install
```

恢复原来的启动脚本：

```sh
python3 scripts/install-sd.py \
  --volume /Volumes/Untitled \
  --expected-uuid YOUR_SD_VOLUME_UUID \
  --restore
```

应用和 muku 的存档会保留。
如果启动脚本被安装程序以外的方式修改过，会拒绝自动覆盖。

</details>

### 5. 更新 SD 卡上的应用

<details>
<summary>通过 USB 更新</summary>

在 SD 卡插入机身并通过 USB 连接的状态下使用。
会先确认当前布局和哈希，备份旧的二进制文件后再更新。

```sh
python3 scripts/update-usb.py --serial YOUR_ADB_SERIAL \
  --binary target/aarch64-unknown-linux-musl/release/muku-pet --restart
```

加上 `--restart` 时，会保存 muku 的状态并重启机身。

</details>

### 🗂️ 目录结构

- `src/state.rs`：状态、时间流逝、校验、原子化保存
- `src/render.rs`：用 tiny-skia 绘制画面和文字
- `src/app.rs`：界面切换与输入处理
- `src/minigame.rs`：STAR CATCH 和 BEAT GARDEN
- `src/device.rs`：evdev 输入和约 20fps 的运行循环
- `src/drm_display.rs`：DRM/KMS 绘制与清理
- `src/notifications.rs`：带 Token 认证的 Webhook 接收
- `src/audio.rs`：Webhook 附带 WAV 的异步播放
- `scripts/xai_muku_bridge.py`：Grok 对话、网页搜索、xAI TTS、转发到 muku

## ⚠️ 目前还不支持

- 机身上还没有 AI 对话生成、成长阶段、音效和背景音乐，以及 Webhook 以外的同步。
- Webhook 是面向局域网的明文 HTTP，请勿直接暴露到互联网。
- 机身不内置 TTS。语音在 Mac 上生成：使用 macOS 语音（`send-notice-tts.sh`）或 xAI TTS（Grok 桥接）。
- 通知队列在重启后不会保留。
- 在 R36MAX 2 上，480×272 的界面按原比例放大到 1024×768，上下留白。
- R36MAX 2 的 USB OTG 数据连接尚未解决，目前通过 Wi-Fi 部署和 SSH。
- 尚未读取 NAND 开头的 Loader 区域，也没有进行固件改写。
- SD 卡的 exFAT 不能完全保证断电安全，保存后请正常关机。

## 📜 出处与许可

muku 的外形、颜色、眼睛、呼吸和眨眼，参照
[mukumo 移动版 Muku.tsx](https://github.com/TSUQREA/mukumo/blob/0d0b8d7e89069a99fef4f810d0c3688fd813f2cd/apps/mobile/src/components/Muku.tsx)
和[品牌规范](https://github.com/TSUQREA/mukumo/blob/0d0b8d7e89069a99fef4f810d0c3688fd813f2cd/docs/design/brand.md)
移植而来（均位于非公开仓库）。
标准的 muku 没有添加嘴巴、腮红、手脚或服装。

通知的日文显示使用
[M PLUS Rounded 1c Regular](assets/fonts/MPLUSRounded1c-Regular.ttf)，
遵循 [SIL Open Font License](assets/fonts/OFL.txt)。

同时参考了 [Linux framebuffer API](https://docs.kernel.org/fb/api.html) 和
[Linux input event 规范](https://docs.kernel.org/input/event-codes.html)。

<div align="center">
<br>
<sub>☁️ 今天也和 muku 一起，度过美好的一天。</sub>
</div>
