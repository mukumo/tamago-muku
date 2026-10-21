**English** · [日本語](R36MAX2.ja.md) · [简体中文](R36MAX2.zh-CN.md)

# XiFan R36MAX 2

The model with built-in Wi-Fi, checked on 2026-10-20.
The box label may not match the actual hardware, so cross-check `/proc/device-tree/model` and the display mode.

## Device configuration

```text
SoC:       Rockchip RK3326 / Cortex-A35 x4
RAM:       about 976 MiB
OS:        EmuELEC 4.7-Nexus devel
Kernel:    Linux 5.10.160 aarch64
Display:   DSI 1024x768, XRGB8888, stride 4096
Wi-Fi:     AIC8800DC 2.4GHz
Audio:     RK817
eMMC:      /storage 2.0GB
ROM SD:    /storage/roms 58.2GB
```

This firmware includes neither OpenSSH `sshd` nor Dropbear.
So a static Dropbear binary and a public key were deployed once from Ports on the SD card.

## SSH

The root password is left unchanged; only public-key login is enabled.

```text
dropbear:       /storage/.config/muku-ssh/dropbear
host key:       /storage/.config/muku-ssh/host_ed25519
authorized key: /storage/.ssh/authorized_keys
service:        /storage/.config/system.d/muku-ssh.service
log:            /storage/.config/muku-ssh/dropbear.log
```

```sh
ssh -i ~/.ssh/id_ed25519 root@R36_IP
```

The IP in this example may change via DHCP.
`muku-ssh.service` is independent of EmulationStation and keeps running after the UI restarts.

Dropbear v2025.89 is built statically for aarch64 on Alpine 3.22/musl.
The download source and build conditions are recorded in `packaging/r36max2/SSH-BOOTSTRAP.md`.

## muku

```text
app:      /storage/muku/muku-pet
state:    /storage/muku/state.json
log:      /storage/muku/muku.log
launcher: /storage/roms/ports_scripts/Muku.sh
```

Launch `MUKU` from `PORTS` in EmulationStation.
Press START to save and return to Ports.

The 480×272 UI is scaled up 2× to 1024×768 with its aspect ratio kept, and centered.
The older RK3562 unit stays at 1× as before.

To update only the app on a device where SSH is already set up:

```sh
./scripts/install-r36max2-ssh.sh root@R36_IP
```

The host key is verified against your regular `known_hosts`.
To use a different key or known_hosts file, set `R36_SSH_KEY` and `R36_KNOWN_HOSTS`.
This script does not overwrite the webhook settings or muku's state.

To open muku automatically at startup:

```sh
./scripts/set-r36max2-autostart.sh root@R36_IP
```

This adds a systemd setting that runs muku once, right before EmulationStation starts.
When you exit muku with START, the regular EmulationStation menu opens.
If muku fails to start, the device still goes on to EmulationStation, and automatic retries during a restart
do not run muku again, so you can keep using the regular menu.
To revert:

```sh
./scripts/set-r36max2-autostart.sh root@R36_IP --disable
```

Both commands back up the previous `emuelec.conf` and existing systemd settings with a timestamp.
The implementation is in `packaging/r36max2/muku-autostart.sh` and
`/storage/.config/system.d/emustation.service.d/muku-autostart.conf`.

## Webhook

```text
config: /storage/muku/webhook.json
listen: 0.0.0.0:8787
path:   POST /webhook
```

`webhook.json` is saved with mode 0600, readable only by root.
Tokens are never written into the repository or public documents.
Attach `audio_wav_base64` and the WAV plays through the RK817 speaker while the notice is shown.

```sh
curl --fail-with-body \
  -X POST http://R36_IP:8787/webhook \
  -H "Authorization: Bearer $MUKU_WEBHOOK_TOKEN" \
  -H "Content-Type: application/json" \
  --data '{
    "title": "AIからのおしらせ",
    "body": "少し休憩しませんか",
    "source": "ai"
  }'
```

On 2026-10-20, a Japanese notice was sent to the device and `202 Accepted` was confirmed.
The server speaks plain HTTP on the LAN, so do not expose it to the internet as-is.

To build a TTS WAV with the Mac's `Kyoko` voice and send it:

```sh
./scripts/send-notice-tts.sh \
  --url http://R36_IP:8787/webhook \
  --config /path/to/webhook.json \
  --title "AIからのおしらせ" \
  --body "少し休憩しませんか"
```

The WAV is generated on the Mac, not on the device.
The device plays the WAV it receives with `aplay`.

A local page that combines text chat with Grok, web search, xAI TTS, and forwarding to MUKU is described in
[xAI chat bridge](XAI.md).

## Verification results

- Wi-Fi after autostart: `TSUQREA-WiFi`, 2437MHz, channel 6
- DHCP: obtained a LAN address, reachable from a Mac on the same subnet
- After a reboot, muku started as the foreground process without EmulationStation being launched
- muku: 1024×768, 331 frames in 18 seconds
- Input: A=304, B=305, X=308 confirmed with physical input
- Webhook: `{"ok":true,"message":"accepted"}`
- SSH: Dropbear 2025.89, Ed25519 public-key authentication

## Recovery

The original `custom_start.sh` is backed up with a timestamp in `/storage/.config/`.
The backup name from the first check is:

```text
custom_start.sh.before-muku-20261020-225144
```

To stop SSH, disable/stop `muku-ssh.service`.
To remove muku from Ports, delete `Muku.sh` and its gamelist entry from the SD card.
The firmware and bootloader have not been modified.
