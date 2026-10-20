#!/bin/sh
set -eu

target=${1:?Usage: set-r36max2-autostart.sh root@HOST [--disable]}
mode=${2:-enable}
case "$mode" in
    enable|--disable) ;;
    *) echo "Unknown mode: $mode" >&2; exit 2 ;;
esac

key=${R36_SSH_KEY:-$HOME/.ssh/id_ed25519}
known_hosts=${R36_KNOWN_HOSTS:-$HOME/.ssh/known_hosts}

remote() {
    ssh -i "$key" \
        -o IdentitiesOnly=yes \
        -o BatchMode=yes \
        -o StrictHostKeyChecking=yes \
        -o UserKnownHostsFile="$known_hosts" \
        "$target" "$@"
}

remote "
set -eu
config=/storage/.config/emuelec/configs/emuelec.conf
port=/storage/roms/ports_scripts/Muku.sh
helper=/storage/muku/muku-autostart.sh
dropin_dir=/storage/.config/system.d/emustation.service.d
dropin=\"\$dropin_dir/muku-autostart.conf\"
[ -f \"\$config\" ] || { echo 'EmuELEC config not found' >&2; exit 1; }
[ -x \"\$port\" ] || { echo 'Muku Port launcher not found' >&2; exit 1; }
[ -x \"\$helper\" ] || { echo 'Muku autostart helper not found; run install-r36max2-ssh.sh first' >&2; exit 1; }
stamp=\$(date +%Y%m%d-%H%M%S)
backup=\"\$config.before-muku-autostart-\$stamp\"
cp -p \"\$config\" \"\$backup\"
temporary=\"\$config.muku-new\"
awk '
  !/^global\.bootgame\.path=/ &&
  !/^global\.bootgame\.cmd=/
' \"\$config\" >\"\$temporary\"
chmod 644 \"\$temporary\"
systemctl stop emustation
mv \"\$temporary\" \"\$config\"
mkdir -p \"\$dropin_dir\"
if [ -f \"\$dropin\" ]; then
  cp -p \"\$dropin\" \"\$dropin.before-muku-autostart-\$stamp\"
fi
if [ '$mode' = 'enable' ]; then
  printf '%s\n' \
    '[Service]' \
    'ExecStartPre=-/storage/muku/muku-autostart.sh' \
    'TimeoutStartSec=infinity' >\"\$dropin.tmp\"
  chmod 644 \"\$dropin.tmp\"
  mv \"\$dropin.tmp\" \"\$dropin\"
  rm -f /run/tamago-muku-autostart-done
else
  rm -f \"\$dropin\"
  rm -f /run/tamago-muku-autostart-done
fi
sync
systemctl daemon-reload
printf 'backup=%s\n' \"\$backup\"
if [ '$mode' = 'enable' ]; then
  printf 'dropin=%s\n' \"\$dropin\"
  cat \"\$dropin\"
  systemctl start --no-block emustation
else
  printf 'dropin=disabled\n'
  systemctl start emustation
fi
"

echo "muku autostart mode: $mode"
