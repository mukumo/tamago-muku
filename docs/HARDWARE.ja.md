# 確認したゲーム機

2026-10-20の実機確認。箱の型番表記と実際の中身が異なる機器です。
ほかの同じ見た目のゲーム機にそのまま適用できるとは限りません。

| 項目 | 実機で確認した値 |
|---|---|
| SoC | Rockchip RK3562 / Cortex-A53 |
| カーネル | Linux 5.10.209、aarch64 |
| ユーザーランド | Buildroot由来、32bit armhf |
| メモリー | 約756 MiB |
| 画面 | DPI、480×272 |
| 描画 | `/dev/dri/card0`、DPI-1、DRM dumb buffer |
| fb0の形式 | 32bpp、stride 1920、R16/G8/B0、各8bit |
| ボタン | `/dev/input/event0..2`、キーボード形式 |
| 通信 | USB adb。現在のカーネルにTCP/IPなし |
| 時計 | RTCなし |
| 保存先 | SDのexFAT、`/sdcard`からアクセス可能 |

静的リンクした `aarch64-unknown-linux-musl` バイナリは実機で動作しました。
DRM描画の前に、既存メニューが持っている画面の所有権を解放する必要があります。
一時起動用スクリプトはメニューの再起動ループを止め、終了時に再開します。

## 起動の仕組み

純正の `/etc/init.d/game_ui.sh` は、SDに
`/sdcard/simulator/3562/em_ui.sh` があればそれを実行します。
このSD上の入口からmukuを起動し、終了したら控えておいた元の入口へ戻ります。
NAND、ブートローダー、カーネル、内蔵の起動スクリプトは書き換えません。

元のSDイメージと、OSから見えるNANDパーティションのバックアップは別途保管しています。
NAND先頭のローダー領域は未取得なので、完全な復旧用バックアップではありません。
このプロジェクトの動作にファームウェアの書き換えは必要ありません。

## 実入力の対応

| 物理操作 | Linux key code |
|---|---|
| A | 30 |
| B | 48 |
| X | 45 |
| Y | 21 |
| START | 28 |
| 上 / 右 / 下 / 左 | 103 / 106 / 108 / 105 |

A/B/X/Yは本体を押したログで確認済みです。
一部の操作は複数デバイスから重複して届くため、同じアクションの160ms以内の重複を抑制します。

参考: [Linux input events](https://docs.kernel.org/input/event-codes.html)、
[framebuffer API](https://docs.kernel.org/fb/api.html)。

## XiFan R36MAX 2（Wi-Fiモデル）

2026-10-20に接続した別個体。箱の表記はR36MAX 2、4.5インチ、Wi-Fi 2.4GHzです。

| 項目 | 実機で確認した値 |
|---|---|
| SoC | Rockchip RK3326 / Cortex-A35 ×4 |
| カーネル | Linux 5.10.160、aarch64 |
| OS | EmuELEC 4.7-Nexus devel |
| メモリー | 約976 MiB |
| 画面 | DSI、1024×768 |
| 描画 | `/dev/dri/card0`、DSI-1、DRM dumb buffer |
| fb0の形式 | 32bpp、stride 4096、BGRX |
| 入力 | `gamepad` event0、ほか6イベントデバイス |
| Wi-Fi | AIC8800DC、2.4GHz、`TSUQREA-WiFi`へ接続 |
| 音声 | RK817 |
| 保存先 | eMMC `/storage` 2.0GB、ROM用SD 58.2GB |

実測では1024×768で18秒・331フレーム（約18fps）を描画しました。
A/B/Xは実入力ログで確認し、十字キーは`BTN_DPAD_*`にも対応しています。
USB OTGのデータ接続は未解決のため、配備とSSHにはWi-Fiを使用しています。

SSH、Ports、Webhookの詳細は[R36MAX2.md](R36MAX2.md)を参照してください。
