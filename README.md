<div align="center">

**English** · [日本語](README.ja.md) · [简体中文](README.zh-CN.md)

<img src="docs/images/device-hello.jpg" alt="A handheld game console in a hand, with muku on screen saying “a good day to be muku.”" width="420">

# tamago-muku

**Live with fluffy muku inside a pocket-sized handheld ☁️**

[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-00C4FF?logo=rust&logoColor=white)](#1-build)
[![Works offline](https://img.shields.io/badge/works-offline-35C838)](#-life-with-muku)
[![Handhelds](https://img.shields.io/badge/RK3562%20%C2%B7%20RK3326-handheld-F3A6FF)](#-supported-handhelds)

</div>

tamago-muku is a Rust app that lets you live with **muku** on small RK3562 / RK3326 handheld consoles.
It draws straight to the screen, so you can care for muku and play mini-games with no network at all.
On the Wi-Fi equipped R36MAX 2, muku can also deliver notices from your AI (optional).

## 🎬 See it running

<div align="center">

<table>
  <tr>
    <td align="center" width="33%">
      <img src="docs/images/device-idle.webp" alt="Video of muku breathing slowly on the handheld screen" width="240">
    </td>
    <td align="center" width="33%">
      <img src="docs/images/device-pet.webp" alt="Video of muku smiling with floating hearts after being petted with the A button" width="240">
    </td>
    <td align="center" width="33%">
      <img src="docs/images/device-care.webp" alt="Video of petting, feeding, and playing with muku in turn" width="240">
    </td>
  </tr>
  <tr>
    <td align="center"><b>🫧 Idle</b><br>Breathes slowly<br>and blinks now and then</td>
    <td align="center"><b>💗 Pet</b><br>Press A to pet,<br>get a smile and hearts</td>
    <td align="center"><b>🍙 Care</b><br>Pet → feed<br>→ play</td>
  </tr>
</table>

<sub>Filmed on the RK3562 4.3-inch handheld (480×272). Clips are silent and loop.</sub>

</div>

<details>
<summary>📺 Watch a screen capture</summary>
<br>
<img src="docs/images/muku-demo.gif" alt="Screen capture of muku's room" width="480">
</details>

## 🐣 Life with muku

muku has three gauges.

| Gauge | On screen | How to fill it |
|---|---|---|
| 🍙 Fullness | `food` | Feed muku |
| 💗 Mood | `happy` | Pet muku or play together |
| 💤 Energy | `rest` | Let muku sleep |

- Breathing, blinking, and reactions to petting keep muku feeling alive beside you
- The gauges change slowly while the app is running
- muku's state is saved automatically
- Time while the power is off is not counted, so **muku never weakens or disappears while you are away** 🕊️

<div align="center">
  <img src="docs/images/device-sleepy.jpg" alt="Photo of muku looking drowsy with the caption “getting sleepy.”" width="360">
  <br>
  <sub>When energy runs low, muku tells you it is “getting sleepy.” 💤</sub>
</div>

## 🎮 How to play

On the home screen, four buttons are all you need.

| Button | What it does |
|:---:|---|
| **A** / ↑ | 💗 Pet |
| **B** / → | 🍙 Feed |
| **X** / ↓ | 🎮 Pick a mini-game |
| **Y** / ← | 💤 Sleep / wake up |
| **START** | 💾 Save and return to the console's own menu |

<details>
<summary>Controls on other screens</summary>

| Screen | Controls |
|---|---|
| Game select | Left/Right: move cursor, A / X: start, B / Y: home, START: home |
| STAR CATCH | Left/Right: move muku, START: end the round and see results |
| BEAT GARDEN | A / B / X / Y: tap the note in each lane, START: end the round and see results |
| Results | A / X: play again, B / Y: game select, START / D-pad: home |

</details>

## 🌟 Mini-games

<table>
  <tr>
    <td align="center" width="50%">
      <img src="docs/images/star-catch.png" alt="STAR CATCH gameplay screen" width="360">
    </td>
    <td align="center" width="50%">
      <img src="docs/images/beat-garden.png" alt="BEAT GARDEN gameplay screen" width="360">
    </td>
  </tr>
  <tr>
    <td>
      <b>⭐ STAR CATCH</b><br>
      Move muku left and right for 25 seconds to catch falling stars.
      Gold stars are worth 3 points. Miss 6 and the round ends.
    </td>
    <td>
      <b>🎵 BEAT GARDEN</b><br>
      Press A/B/X/Y in time with notes falling down 4 lanes.
      Hits closer to the line score higher, and combos boost your score.
    </td>
  </tr>
</table>

Results are shown as 0–5 stars, and every round you play is reflected in muku's mood and energy.

## 💌 Notices from your AI (Wi-Fi model)

<div align="center">
  <img src="docs/images/japanese-notification.png" alt="muku's room showing a Japanese notice" width="480">
</div>

On the R36MAX 2, an HTTP webhook for the local network has been tested on real hardware.
Send a notice from your AI or any other tool, and muku shows it in Japanese inside its room.

- Never interrupts a game; the notice appears for 9 seconds when you return home
- Attach a WAV and the console plays it through its speaker as the notice appears 🔊
- Without `/storage/muku/webhook.json`, the app stays fully offline as before

The endpoint is `POST /webhook` and requires a Bearer token.

```json
{
  "title": "AIからのおしらせ",
  "body": "少し休憩しませんか",
  "source": "ai",
  "kind": "notice"
}
```

Attach the WAV as Base64.
To build a WAV with a macOS Japanese voice and send it to the webhook:

```sh
./scripts/send-notice-tts.sh \
  --url http://R36_IP:8787/webhook \
  --config /path/to/webhook.json \
  --title "AIからのおしらせ" \
  --body "少し休憩しませんか"
```

Audio is generated with the Mac's `say` and `afconvert`. The default voice is `Kyoko`.

📖 Setup, sending examples, and cautions for exposing a webhook outside your network are in
[Webhook notifications](docs/NOTIFICATIONS.md) (Japanese).
SSH and deployment for the R36MAX 2 are in [R36MAX 2 hardware notes](docs/R36MAX2.md) (Japanese).

## 🗣️ Chat with Grok

Run a local chat page on your Mac and have muku show and read Grok's replies aloud.

```sh
# First time only
security add-generic-password -U -a kan -s tamago-muku-xai -w
./scripts/install-xai-bridge.sh http://R36_IP:8787/webhook
```

Then open <http://127.0.0.1:8790> on your Mac.

- The API key is read from the macOS Keychain and never passed to the browser or the R36
- Turn web search on or off for each conversation
- Grok's replies are converted to Japanese WAV with xAI TTS
- Text and audio are sent to the R36 using muku's existing webhook token
- The LaunchAgent starts with a minimal environment and does not inherit unneeded session secrets

📖 Details are in [xAI chat bridge](docs/XAI.md) (Japanese).

## 📟 Supported handhelds

|  | RK3562 4.3-inch handheld | XiFan R36MAX 2 |
|---|---|---|
| SoC | RK3562 (Cortex-A53) | RK3326 (Cortex-A35) |
| Display | 480×272 | 1024×768 (480×272 UI scaled up) |
| Connection | USB (adb) | Wi-Fi (SSH) |
| Autostart | From the SD card | Shown before EmulationStation |
| Notices & TTS | — | ✅ |
| Hardware notes | [Hardware notes](docs/HARDWARE.md) (Japanese) | [R36MAX 2 notes](docs/R36MAX2.md) (Japanese) |

<details>
<summary>✅ What has been verified on real hardware</summary>

On 2026-10-20, the following was confirmed on a USB-connected RK3562 4.3-inch handheld:

- DRM rendering at 480×272 and three evdev input devices
- Rendering 160 frames in 8 seconds (about 20 fps)
- Controls with A/B/X/Y
- Returning to the stock menu after exiting
- Autostart from the SD card
- 13 on-device tests covering state, saving, and Linux rendering

Also on 2026-10-20, the following was confirmed on the R36MAX 2 (RK3326, 1024×768, EmuELEC 4.7):

- Placing a Ports launcher entry, and launching and exiting on hardware while handing over display ownership
- A/B/X input and saving state on the device
- Rendering 331 frames in 18 seconds at 1024×768 (about 18 fps)
- Showing muku automatically before EmulationStation on reboot
- Receiving `202 Accepted` from the webhook
- Public key authentication for a standalone Dropbear SSH service

Clippy for Linux, 23 Rust tests, and a static aarch64 build all pass.

</details>

## 🛠️ Getting started (for developers)

### 1. Build

```sh
rustup target add aarch64-unknown-linux-musl
cargo test
cargo build --locked --release --target aarch64-unknown-linux-musl
```

`.cargo/config.toml` selects Rust's bundled linker and a linker fix for Cortex-A53.
The app runs on the device's 64-bit kernel even with a 32-bit userland.

### 2. Preview the screen only

```sh
cargo run -- --preview /tmp/muku.png
cargo run -- --preview-game select /tmp/games.png
```

### 3. Run temporarily over USB

```sh
./scripts/run-usb.sh YOUR_ADB_SERIAL
```

Replace `YOUR_ADB_SERIAL` with the device serial shown by `adb devices -l`.
The app is copied to `/tmp/muku/`, and the stock menu resumes when it exits.
The internal NAND and the launch scripts on the SD card are left untouched.
Save data for a temporary run lives in `/tmp/muku/state.json` and is lost on power off.

### 4. Launch from the SD card

<details>
<summary>Installing to the SD card and restoring the original</summary>

On the SD card, `/muku/muku-pet` is the app and `/muku/state.json` is the save data.
The stock `/simulator/3562/em_ui.sh` is backed up as `em_ui.before-muku.sh`,
and muku starts from the entry point on the SD card.
Pressing START, or any app failure, returns you to the console's original menu.

Games, ROMs, and the built-in firmware on the card are not modified.
If you boot without the SD card, the original built-in menu runs.

On macOS, install to the target card mounted as writable.
Replace `YOUR_SD_VOLUME_UUID` with the Volume UUID from
`diskutil info /Volumes/Untitled`.

```sh
python3 scripts/install-sd.py \
  --volume /Volumes/Untitled \
  --expected-uuid YOUR_SD_VOLUME_UUID \
  --binary target/aarch64-unknown-linux-musl/release/muku-pet \
  --install
```

To restore the original launch script:

```sh
python3 scripts/install-sd.py \
  --volume /Volumes/Untitled \
  --expected-uuid YOUR_SD_VOLUME_UUID \
  --restore
```

The app and muku's save data are kept.
If the launch script was changed by something other than the installer, automatic overwriting is refused.

</details>

### 5. Update the app on the SD card

<details>
<summary>Updating over USB</summary>

Use this with the SD card inserted in the device and connected over USB.
It checks the current layout and hashes, and backs up the old binary before updating.

```sh
python3 scripts/update-usb.py --serial YOUR_ADB_SERIAL \
  --binary target/aarch64-unknown-linux-musl/release/muku-pet --restart
```

With `--restart`, muku's state is saved and the device reboots.

</details>

### 🗂️ Layout

- `src/state.rs`: state, passage of time, validation, atomic saving
- `src/render.rs`: drawing the screen and text with tiny-skia
- `src/app.rs`: screen transitions and input handling
- `src/minigame.rs`: STAR CATCH and BEAT GARDEN
- `src/device.rs`: evdev input and the ~20 fps run loop
- `src/drm_display.rs`: DRM/KMS rendering and cleanup
- `src/notifications.rs`: token-authenticated webhook receiver
- `src/audio.rs`: asynchronous playback of WAVs attached to webhooks
- `scripts/xai_muku_bridge.py`: Grok chat, web search, xAI TTS, forwarding to muku

## ⚠️ Current limitations

- No AI-generated conversation, growth stages, sound, or syncing other than webhooks yet.
- The webhook is plain HTTP meant for the local network. Do not expose it directly to the internet.
- TTS is not built into the device. The current TTS sender uses macOS voices.
- The notice queue does not survive a reboot.
- On the R36MAX 2, the 480×272 UI is scaled up to 1024×768 keeping its aspect ratio, with margins above and below.
- USB OTG data connections on the R36MAX 2 are unresolved. Deployment and SSH currently use Wi-Fi.
- The loader area at the start of the NAND has not been dumped. Firmware rewriting has not been attempted.
- exFAT on the SD card is not fully resilient to power loss. Power off normally after saving.

## 📜 Credits and license

muku's shape, colors, eyes, breathing, and blinking were ported after reviewing
[Muku.tsx from the mukumo mobile app](https://github.com/TSUQREA/mukumo/blob/0d0b8d7e89069a99fef4f810d0c3688fd813f2cd/apps/mobile/src/components/Muku.tsx)
and the [brand specification](https://github.com/TSUQREA/mukumo/blob/0d0b8d7e89069a99fef4f810d0c3688fd813f2cd/docs/design/brand.md).
No mouth, cheeks, limbs, or outfits have been added to the standard muku.

Japanese text in notices uses
[M PLUS Rounded 1c Regular](assets/fonts/MPLUSRounded1c-Regular.ttf),
under the [SIL Open Font License](assets/fonts/OFL.txt).

The [Linux framebuffer API](https://docs.kernel.org/fb/api.html) and the
[Linux input event codes](https://docs.kernel.org/input/event-codes.html) were also used as references.

<div align="center">
<br>
<sub>☁️ Have a good day with muku.</sub>
</div>
