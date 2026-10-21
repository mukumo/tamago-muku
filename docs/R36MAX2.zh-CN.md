[English](R36MAX2.md) · [日本語](R36MAX2.ja.md) · **简体中文**

# XiFan R36MAX 2

2026-10-20 确认的内置 Wi-Fi 机型。
包装盒标注可能与实机不一致，请核对 `/proc/device-tree/model` 和屏幕模式。

## 实机配置

```text
SoC:       Rockchip RK3326 / Cortex-A35 x4
RAM:       约 976 MiB
OS:        EmuELEC 4.7-Nexus devel
Kernel:    Linux 5.10.160 aarch64
Display:   DSI 1024x768, XRGB8888, stride 4096
Wi-Fi:     AIC8800DC 2.4GHz
Audio:     RK817
eMMC:      /storage 2.0GB
ROM SD:    /storage/roms 58.2GB
```

该固件既不包含 OpenSSH 的 `sshd`，也不包含 Dropbear。
因此通过 SD 卡上的 Ports，一次性部署了静态 Dropbear 和公钥。

## SSH

不修改 root 密码，仅启用公钥登录。

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

示例中的 IP 可能会因 DHCP 而变化。
`muku-ssh.service` 独立于 EmulationStation，UI 重启后仍保持运行。

Dropbear v2025.89 在 Alpine 3.22/musl 上以 aarch64 静态构建。
下载来源和构建条件记录在 `packaging/r36max2/SSH-BOOTSTRAP.md`。

## muku

```text
app:      /storage/muku/muku-pet
state:    /storage/muku/state.json
log:      /storage/muku/muku.log
launcher: /storage/roms/ports_scripts/Muku.sh
```

从 EmulationStation 的 `PORTS` 中启动 `MUKU`。
按 START 保存并返回 Ports。

480×272 的绘制界面保持宽高比放大 2 倍到 1024×768，并居中显示。
旧的 RK3562 机器仍与之前一样为 1 倍。

只向已配置好 SSH 的设备更新应用：

```sh
./scripts/install-r36max2-ssh.sh root@R36_IP
```

主机密钥通过常规的 `known_hosts` 校验。
如需更换密钥或 known_hosts，请指定 `R36_SSH_KEY` 和 `R36_KNOWN_HOSTS`。
该脚本不会覆盖 Webhook 配置和 muku 的状态。

开机时自动打开 muku：

```sh
./scripts/set-r36max2-autostart.sh root@R36_IP
```

会添加一项 systemd 配置，在 EmulationStation 启动前只运行一次 muku。
在 muku 中按 START 退出后，会直接打开常规的 EmulationStation 菜单。
即使 muku 无法启动，也会继续进入 EmulationStation；重启过程中的自动重试
不会再次运行 muku，因此可以继续在常规菜单中操作。
恢复原状：

```sh
./scripts/set-r36max2-autostart.sh root@R36_IP --disable
```

两者都会将修改前的 `emuelec.conf` 和现有 systemd 配置按日期时间备份。
实现位于 `packaging/r36max2/muku-autostart.sh` 和
`/storage/.config/system.d/emustation.service.d/muku-autostart.conf`。

## Webhook

```text
config: /storage/muku/webhook.json
listen: 0.0.0.0:8787
path:   POST /webhook
```

`webhook.json` 以仅 root 可读的 0600 权限保存。
Token 不会写入仓库或公开文档。
附带 `audio_wav_base64` 时，会在显示通知的同时通过 RK817 扬声器播放 WAV。

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

2026-10-20 向实机发送了日文通知，并确认返回 `202 Accepted`。
服务器是局域网内的明文 HTTP，请勿直接暴露到互联网。

用 Mac 的 `Kyoko` 语音生成 TTS WAV 并发送：

```sh
./scripts/send-notice-tts.sh \
  --url http://R36_IP:8787/webhook \
  --config /path/to/webhook.json \
  --title "AIからのおしらせ" \
  --body "少し休憩しませんか"
```

TTS 的 WAV 在 Mac 端生成，而不是在设备上生成。
设备使用 `aplay` 播放收到的 WAV。

整合了与 Grok 的文字对话、网页搜索、xAI TTS 和 MUKU 转发的本地页面，请参阅
[xAI 聊天桥接](XAI.zh-CN.md)。

## 验证结果

- 自启动后的 Wi-Fi：`TSUQREA-WiFi`，2437MHz，信道 6
- DHCP：获取到局域网地址，同一子网的 Mac 可以访问
- 重启后，muku 在未启动 EmulationStation 的情况下作为前台进程启动
- muku：1024×768，18 秒 331 帧
- 输入：通过实际输入确认 A=304、B=305、X=308
- Webhook：`{"ok":true,"message":"accepted"}`
- SSH：Dropbear 2025.89，Ed25519 公钥认证

## 恢复

原来的 `custom_start.sh` 已按日期时间备份在 `/storage/.config/`。
首次确认时的备份名为：

```text
custom_start.sh.before-muku-20261020-225144
```

如需停止 SSH，请 disable/stop `muku-ssh.service`。
如需从 Ports 中移除 muku，请删除 SD 卡上的 `Muku.sh` 和 gamelist 条目。
固件和 Bootloader 均未改动。
