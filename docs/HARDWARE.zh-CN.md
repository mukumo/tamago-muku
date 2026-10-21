[English](HARDWARE.md) · [日本語](HARDWARE.ja.md) · **简体中文**

# 已验证的掌机

2026-10-20 实机确认。这台机器包装盒上标注的型号与实际硬件不一致。
不一定能直接套用到其他外观相同的掌机上。

| 项目 | 实机确认值 |
|---|---|
| 主控 | Rockchip RK3562 / Cortex-A53 |
| 内核 | Linux 5.10.209，aarch64 |
| 用户空间 | 基于 Buildroot，32 位 armhf |
| 内存 | 约 756 MiB |
| 屏幕 | DPI，480×272 |
| 绘制 | `/dev/dri/card0`，DPI-1，DRM dumb buffer |
| fb0 格式 | 32bpp，stride 1920，R16/G8/B0，各 8 位 |
| 按键 | `/dev/input/event0..2`，键盘类型 |
| 通信 | USB adb。当前内核不支持 TCP/IP |
| 时钟 | 无 RTC |
| 存储 | SD 卡 exFAT，可通过 `/sdcard` 访问 |

静态链接的 `aarch64-unknown-linux-musl` 二进制文件可以在实机上运行。
进行 DRM 绘制前，需要让现有菜单释放其持有的显示所有权。
临时启动脚本会停止菜单的重启循环，并在退出时恢复。

## 启动机制

原厂的 `/etc/init.d/game_ui.sh` 会在 SD 卡上存在
`/sdcard/simulator/3562/em_ui.sh` 时执行它。
从 SD 卡上的这个入口启动 muku，退出后返回事先备份的原始入口。
不会改写 NAND、Bootloader、内核和内置启动脚本。

原始 SD 卡镜像，以及系统可见的 NAND 分区备份，均已另行保存。
尚未读取 NAND 开头的 Loader 区域，因此这不是完整的恢复用备份。
本项目无需改写固件即可运行。

## 实体按键映射

| 实体操作 | Linux key code |
|---|---|
| A | 30 |
| B | 48 |
| X | 45 |
| Y | 21 |
| START | 28 |
| 上 / 右 / 下 / 左 | 103 / 106 / 108 / 105 |

A/B/X/Y 已通过按下机身按键时的日志确认。
部分操作会从多个设备重复传入，因此同一动作在 160ms 内的重复输入会被过滤。

参考：[Linux input events](https://docs.kernel.org/input/event-codes.html)、
[framebuffer API](https://docs.kernel.org/fb/api.html)。

## XiFan R36MAX 2（Wi-Fi 机型）

2026-10-20 连接的另一台机器。包装盒标注为 R36MAX 2、4.5 英寸、Wi-Fi 2.4GHz。

| 项目 | 实机确认值 |
|---|---|
| 主控 | Rockchip RK3326 / Cortex-A35 ×4 |
| 内核 | Linux 5.10.160，aarch64 |
| 系统 | EmuELEC 4.7-Nexus devel |
| 内存 | 约 976 MiB |
| 屏幕 | DSI，1024×768 |
| 绘制 | `/dev/dri/card0`，DSI-1，DRM dumb buffer |
| fb0 格式 | 32bpp，stride 4096，BGRX |
| 输入 | `gamepad` event0，另有 6 个事件设备 |
| Wi-Fi | AIC8800DC，2.4GHz，已连接 `TSUQREA-WiFi` |
| 音频 | RK817 |
| 存储 | eMMC `/storage` 2.0GB，ROM 用 SD 卡 58.2GB |

实测在 1024×768 下 18 秒绘制 331 帧（约 18fps）。
A/B/X 已通过实际输入日志确认，十字键也支持 `BTN_DPAD_*`。
USB OTG 数据连接尚未解决，因此部署和 SSH 使用 Wi-Fi。

SSH、Ports、Webhook 的详情请参阅 [R36MAX 2 实机笔记](R36MAX2.zh-CN.md)。
