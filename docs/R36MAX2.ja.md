# XiFan R36MAX 2

2026-10-20に確認したWi-Fi内蔵モデルです。
箱表記と実機が異なる場合があるため、`/proc/device-tree/model`と画面モードを照合してください。

## 実機構成

```text
SoC:       Rockchip RK3326 / Cortex-A35 x4
RAM:       約976 MiB
OS:        EmuELEC 4.7-Nexus devel
Kernel:    Linux 5.10.160 aarch64
Display:   DSI 1024x768, XRGB8888, stride 4096
Wi-Fi:     AIC8800DC 2.4GHz
Audio:     RK817
eMMC:      /storage 2.0GB
ROM SD:    /storage/roms 58.2GB
```

このファームウェアにはOpenSSHの`sshd`もDropbearも含まれていません。
そのためSDカードのPortsから、静的Dropbearと公開鍵を一度だけ配備しました。

## SSH

rootパスワードは変更せず、公開鍵ログインだけを有効にしています。

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

この例のIPはDHCPで変わる可能性があります。
`muku-ssh.service`はEmulationStationから独立しており、UI再起動後も維持されます。

Dropbear v2025.89をAlpine 3.22/muslでaarch64静的ビルドしています。
配布元とビルド条件は`packaging/r36max2/SSH-BOOTSTRAP.md`に記録しています。

## muku

```text
app:      /storage/muku/muku-pet
state:    /storage/muku/state.json
log:      /storage/muku/muku.log
launcher: /storage/roms/ports_scripts/Muku.sh
```

EmulationStationの`PORTS`から`MUKU`を起動します。
STARTで保存してPortsへ戻ります。

480×272の描画UIを、1024×768へアスペクト比を保って2倍拡大し、中央配置しています。
旧RK3562機では従来どおり1倍です。

SSH設定済みの端末へアプリだけを更新する場合:

```sh
./scripts/install-r36max2-ssh.sh root@R36_IP
```

ホスト鍵は通常の`known_hosts`で検証します。
鍵やknown_hostsを変える場合は`R36_SSH_KEY`と`R36_KNOWN_HOSTS`を指定します。
このスクリプトはWebhook設定とmukuの状態を上書きしません。

起動時にmukuを自動で開く場合:

```sh
./scripts/set-r36max2-autostart.sh root@R36_IP
```

EmulationStationの起動直前に、1回だけmukuを実行するsystemd設定を追加します。
mukuのSTARTで終了すると、そのまま通常のEmulationStationメニューが開きます。
mukuが起動できなかった場合もEmulationStationへ進み、再起動中の自動再試行では
mukuを繰り返さないため、通常メニューから操作を続けられます。
元に戻す場合:

```sh
./scripts/set-r36max2-autostart.sh root@R36_IP --disable
```

どちらも変更前の`emuelec.conf`と既存のsystemd設定を日時付きでバックアップします。
実装は`packaging/r36max2/muku-autostart.sh`と
`/storage/.config/system.d/emustation.service.d/muku-autostart.conf`です。

## Webhook

```text
config: /storage/muku/webhook.json
listen: 0.0.0.0:8787
path:   POST /webhook
```

`webhook.json`はrootだけが読める0600で保存しています。
トークンはリポジトリや公開文書へ記載しません。
`audio_wav_base64`を添付すると、通知を表示しながらRK817スピーカーでWAVを再生します。

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

2026-10-20に日本語通知を実機へ送信し、`202 Accepted`を確認しました。
サーバーは平文LAN HTTPなので、そのままインターネットへ公開しません。

Macの`Kyoko`音声からTTS WAVを作って送る場合:

```sh
./scripts/send-notice-tts.sh \
  --url http://R36_IP:8787/webhook \
  --config /path/to/webhook.json \
  --title "AIからのおしらせ" \
  --body "少し休憩しませんか"
```

TTSは端末内ではなく、Mac側でWAVを生成します。
端末は受け取ったWAVを`aplay`で再生します。

Grokとのテキスト会話、Web検索、xAI TTS、MUKU転送をまとめたローカル画面は
[xAI会話ブリッジ](XAI.md)にあります。

## 検証結果

- 自動起動後のWi-Fi: `TSUQREA-WiFi`、2437MHz、channel 6
- DHCP: LANアドレスを取得し、同一サブネットのMacから到達可能
- 再起動後、EmulationStationを起動せずmukuが前面プロセスとして開始
- muku: 1024×768、18秒で331フレーム
- 入力: A=304、B=305、X=308を実入力で確認
- Webhook: `{"ok":true,"message":"accepted"}`
- SSH: Dropbear 2025.89、Ed25519公開鍵認証

## 復旧

元の`custom_start.sh`は`/storage/.config/`に日時付きでバックアップしています。
初回確認時のバックアップ名は:

```text
custom_start.sh.before-muku-20261020-225144
```

SSHを止める場合は`muku-ssh.service`をdisable/stopします。
Portsからmukuを外す場合は、SD上の`Muku.sh`とgamelistエントリを削除します。
ファームウェアとブートローダーは変更していません。
