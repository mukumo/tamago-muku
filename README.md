# tamago-muku

RK3562/RK3326の小型ゲーム機で、mukuと暮らすRustアプリです。
480×272と1024×768の画面へ直接描画し、オフラインでもペットの世話とミニゲームを遊べます。
Wi-Fi内蔵のR36MAX 2では、任意でWebhook通知を受け取れます。

![mukuの部屋](docs/images/muku-demo.gif)

## できること

- 呼吸、まばたき、なでられたときの反応
- ごはん、睡眠、遊びによる状態変化
- 状態の自動保存
- 2種類のミニゲームと0〜5個のリザルト評価
- Wi-Fiモデル向けのトークン認証付きWebhook通知と任意のTTS音声

おなか・気分・元気は起動中の時間でゆっくり変化します。
電源を切っていた時間は計算しないため、不在によるペナルティや死はありません。

## 操作方法

| 画面 | 操作 |
|---|---|
| ホーム | A / 上: なでる、B / 右: ごはん、X / 下: ゲーム選択、Y / 左: 寝る・起きる |
| ゲーム選択 | 左右: カーソル移動、A / X: 開始、B / Y: ホーム、START: ホーム |
| STAR CATCH | 左右: mukuを移動、START: その時点でリザルト |
| BEAT GARDEN | A / B / X / Y: 各レーンの音符をタップ、START: その時点でリザルト |
| リザルト | A / X: もう一度、B / Y: ゲーム選択、START / 上下左右: ホーム |
| ホームのSTART | 保存して元のゲーム機メニューへ戻る |

## ミニゲーム

### STAR CATCH

25秒間、mukuを左右に動かして落ちてくる星を拾います。
金の星は3点です。6回取り逃すとラウンド終了です。

### BEAT GARDEN

4本のレーンへ落ちてくる音符に合わせ、A/B/X/Yを押します。
音符が判定線へ近いほど高得点になり、コンボでスコアが伸びます。

結果は0〜5個の星で表示され、遊んだ分はmukuの気分と元気へ反映されます。

## Wi-FiモデルのWebhook通知

R36MAX 2では、LAN内向けのHTTP Webhookを実機で確認済みです。
AIなどから次の形式で送ると、mukuの部屋へ日本語の通知を表示します。

```json
{
  "title": "AIからのおしらせ",
  "body": "少し休憩しませんか",
  "source": "ai",
  "kind": "notice"
}
```

受信先は`POST /webhook`で、Bearerトークンが必須です。
ゲーム中は割り込まず、ホームへ戻ったときに9秒間表示します。
WAVをBase64で添付すると、通知表示と同時に本体のスピーカーで再生します。

macOSの日本語音声からWAVを作り、Webhookへ送る場合:

```sh
./scripts/send-notice-tts.sh \
  --url http://R36_IP:8787/webhook \
  --config /path/to/webhook.json \
  --title "AIからのおしらせ" \
  --body "少し休憩しませんか"
```

音声はMacの`say`と`afconvert`で生成します。標準では`Kyoko`を使います。

R36MAX 2の`/storage/muku/webhook.json`が見つかった場合だけ受信サーバーを開始します。
ファイルがなければ、これまで通り完全オフラインで動作します。

設定、送信例、外部Webhookを公開する場合の注意は
[Webhook通知の設定](docs/NOTIFICATIONS.md)を参照してください。
R36MAX 2のSSHと配備は[R36MAX 2実機メモ](docs/R36MAX2.md)にまとめています。

## 実機で確認済みの範囲

2026-10-20に、USB接続したRK3562の4.3インチ機で次を確認しました。

- 480×272のDRM描画と3つのevdev入力
- 8秒で160フレーム（約20fps）の描画
- A/B/X/Yによる操作
- 終了後に純正メニューへ戻ること
- SDカードからの自動起動
- 本体上での状態・保存・Linux描画処理のテスト13件

2026-10-20にはR36MAX 2（RK3326、1024×768、EmuELEC 4.7）でも次を確認しました。

- Ports起動用エントリの配置と、表示所有権を切り替えた実機起動・終了
- A/B/X入力と、本体への状態保存
- 1024×768で18秒・331フレーム（約18fps）の描画
- Webhookの`202 Accepted`受信
- 独立したDropbear SSHサービスの公開鍵認証

Linux向けClippy、Rustテスト23件、aarch64静的ビルドも通しています。

## ビルド

```sh
rustup target add aarch64-unknown-linux-musl
cargo test
cargo build --locked --release --target aarch64-unknown-linux-musl
```

`.cargo/config.toml`でRust付属リンカーとCortex-A53向けのリンカー修正を指定しています。
32bitのユーザーランドでも、実機の64bitカーネル上で動作します。

画面だけを確認する場合:

```sh
cargo run -- --preview /tmp/muku.png
cargo run -- --preview-game select /tmp/games.png
```

## USBで一時起動

```sh
./scripts/run-usb.sh YOUR_ADB_SERIAL
```

`YOUR_ADB_SERIAL`は`adb devices -l`で確認した対象機器のシリアルへ置き換えます。
この方法はアプリを`/tmp/muku/`へ転送し、終了時に純正メニューを再開します。
内蔵NANDとSD上の起動スクリプトは変更しません。
一時起動の保存データは`/tmp/muku/state.json`にあり、電源を切ると消えます。

## SDから起動

SDの`/muku/muku-pet`がアプリ、`/muku/state.json`が保存データです。
純正の`/simulator/3562/em_ui.sh`を`em_ui.before-muku.sh`へ控え、
SD上の入口からmukuを起動します。
STARTまたはアプリの失敗時には元のゲーム機メニューへ戻ります。

カード内のゲームやROM、内蔵ファームウェアは変更しません。
SDを取り外して起動した場合は、元の内蔵メニューが動きます。

macOSで、書き込み可能にマウントした対象カードへ配置します。
`YOUR_SD_VOLUME_UUID`は`diskutil info /Volumes/Untitled`の
Volume UUIDへ置き換えます。

```sh
python3 scripts/install-sd.py \
  --volume /Volumes/Untitled \
  --expected-uuid YOUR_SD_VOLUME_UUID \
  --binary target/aarch64-unknown-linux-musl/release/muku-pet \
  --install
```

元の起動スクリプトへ戻す場合:

```sh
python3 scripts/install-sd.py \
  --volume /Volumes/Untitled \
  --expected-uuid YOUR_SD_VOLUME_UUID \
  --restore
```

アプリとmukuの保存データは残ります。
起動スクリプトがインストーラー以外で変更されていた場合、自動上書きは拒否します。

## SD上のアプリを更新

SDを本体へ入れてUSB接続した状態で使います。
現在の配置とハッシュを確認し、旧バイナリを控えてから更新します。

```sh
python3 scripts/update-usb.py --serial YOUR_ADB_SERIAL \
  --binary target/aarch64-unknown-linux-musl/release/muku-pet --restart
```

`--restart`を付けると、mukuの状態を保存して本体を再起動します。

## 構成

- `src/state.rs`: 状態、時間経過、検証、原子的な保存
- `src/render.rs`: tiny-skiaによる画面と文字の描画
- `src/app.rs`: 画面遷移と入力処理
- `src/minigame.rs`: STAR CATCHとBEAT GARDEN
- `src/device.rs`: evdev入力と約20fpsの実行ループ
- `src/drm_display.rs`: DRM/KMS描画と後片付け
- `src/notifications.rs`: トークン認証付きWebhook受信
- `src/audio.rs`: Webhook添付WAVの非同期再生

機器の確認結果は[ハードウェアメモ](docs/HARDWARE.md)にあります。

## 制約

- 会話AIの生成、成長段階、音、Webhook以外の同期はまだありません。
- WebhookはLAN向けの平文HTTPです。インターネットへ直接公開しません。
- TTS単体は端末へ内蔵していません。現在のTTS送信ツールはmacOS音声を使います。
- 通知キューは再起動後には残りません。
- R36MAX 2では480×272のUIを1024×768へアスペクト比維持で拡大し、上下を余白にしています。
- R36MAX 2のUSB OTGデータ接続は未解決です。現在の配備とSSHはWi-Fiを使用します。
- NAND先頭のローダー領域は未取得です。ファームウェア書き換えには進んでいません。
- SDのexFATは電源断への完全な耐性を保証しません。保存後に通常の電源操作を行います。

## 出典とライセンス

mukuの形、色、目、呼吸、瞬きは、
[mukumoのmobile版Muku.tsx](https://github.com/TSUQREA/mukumo/blob/0d0b8d7e89069a99fef4f810d0c3688fd813f2cd/apps/mobile/src/components/Muku.tsx)
と[ブランド仕様](https://github.com/TSUQREA/mukumo/blob/0d0b8d7e89069a99fef4f810d0c3688fd813f2cd/docs/design/brand.md)
を確認して移植しています。
標準のmukuに口、ほっぺ、手足、衣装は追加していません。

通知の日本語表示には
[M PLUS Rounded 1c Regular](assets/fonts/MPLUSRounded1c-Regular.ttf)を使用しています。
[SIL Open Font License](assets/fonts/OFL.txt)に従います。

[Linux framebuffer API](https://docs.kernel.org/fb/api.html)と
[Linux input event仕様](https://docs.kernel.org/input/event-codes.html)も参照しています。
