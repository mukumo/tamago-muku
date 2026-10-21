**English** · [日本語](HARDWARE.ja.md) · [简体中文](HARDWARE.zh-CN.md)

# Verified handhelds

Checked on real hardware on 2026-10-20. On this unit, the model number printed on the box differs from what is actually inside.
These findings may not apply as-is to other handhelds that look the same.

| Item | Value confirmed on the device |
|---|---|
| SoC | Rockchip RK3562 / Cortex-A53 |
| Kernel | Linux 5.10.209, aarch64 |
| Userland | Buildroot-based, 32-bit armhf |
| Memory | About 756 MiB |
| Display | DPI, 480×272 |
| Rendering | `/dev/dri/card0`, DPI-1, DRM dumb buffer |
| fb0 format | 32bpp, stride 1920, R16/G8/B0, 8 bits each |
| Buttons | `/dev/input/event0..2`, keyboard-style |
| Connectivity | USB adb. No TCP/IP in the current kernel |
| Clock | No RTC |
| Storage | exFAT on the SD card, accessible at `/sdcard` |

A statically linked `aarch64-unknown-linux-musl` binary ran on the device.
Before DRM rendering, the existing menu has to release its ownership of the display.
The temporary launch script stops the menu's restart loop and resumes it on exit.

## How startup works

The stock `/etc/init.d/game_ui.sh` runs
`/sdcard/simulator/3562/em_ui.sh` if it exists on the SD card.
muku starts from this entry point on the SD card, and on exit it hands back to the original entry point that was set aside.
The NAND, bootloader, kernel, and built-in startup scripts are not modified.

The original SD image and a backup of the NAND partitions visible from the OS are stored separately.
The loader area at the start of the NAND has not been dumped, so this is not a complete recovery backup.
This project does not need any firmware rewriting to run.

## Physical input mapping

| Physical control | Linux key code |
|---|---|
| A | 30 |
| B | 48 |
| X | 45 |
| Y | 21 |
| START | 28 |
| Up / Right / Down / Left | 103 / 106 / 108 / 105 |

A/B/X/Y were confirmed from logs of pressing the buttons on the device.
Some controls arrive in duplicate from multiple devices, so repeats of the same action within 160ms are suppressed.

References: [Linux input events](https://docs.kernel.org/input/event-codes.html),
[framebuffer API](https://docs.kernel.org/fb/api.html).

## XiFan R36MAX 2 (Wi-Fi model)

A separate unit connected on 2026-10-20. The box says R36MAX 2, 4.5-inch, Wi-Fi 2.4GHz.

| Item | Value confirmed on the device |
|---|---|
| SoC | Rockchip RK3326 / Cortex-A35 ×4 |
| Kernel | Linux 5.10.160, aarch64 |
| OS | EmuELEC 4.7-Nexus devel |
| Memory | About 976 MiB |
| Display | DSI, 1024×768 |
| Rendering | `/dev/dri/card0`, DSI-1, DRM dumb buffer |
| fb0 format | 32bpp, stride 4096, BGRX |
| Input | `gamepad` event0, plus 6 other event devices |
| Wi-Fi | AIC8800DC, 2.4GHz, connected to `TSUQREA-WiFi` |
| Audio | RK817 |
| Storage | eMMC `/storage` 2.0GB, ROM SD card 58.2GB |

Measured: 331 frames rendered in 18 seconds at 1024×768 (about 18fps).
A/B/X were confirmed from physical input logs, and the D-pad also supports `BTN_DPAD_*`.
USB OTG data connections are still unresolved, so Wi-Fi is used for deployment and SSH.

For SSH, Ports, and webhook details, see [R36MAX 2 notes](R36MAX2.md).
