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
[ -f \"\$config\" ] || { echo 'EmuELEC config not found' >&2; exit 1; }
[ -x \"\$port\" ] || { echo 'Muku Port launcher not found' >&2; exit 1; }
service_stopped=0
restore_service() {
  [ \"\$service_stopped\" -eq 0 ] || systemctl start emustation >/dev/null 2>&1 || true
}
trap restore_service EXIT INT TERM
stamp=\$(date +%Y%m%d-%H%M%S)
backup=\"\$config.before-muku-autostart-\$stamp\"
cp -p \"\$config\" \"\$backup\"
temporary=\"\$config.muku-new\"
awk '
  !/^global\.bootgame\.path=/ &&
  !/^global\.bootgame\.cmd=/
' \"\$config\" >\"\$temporary\"
if [ '$mode' = 'enable' ]; then
  printf '%s\n' 'global.bootgame.path=/storage/roms/ports_scripts/Muku.sh' >>\"\$temporary\"
  printf '%s\n' 'global.bootgame.cmd=/usr/bin/bash %ROM% -Pports --core=ports --emulator=ports --controllers=\"%CONTROLLERSCONFIG%\"' >>\"\$temporary\"
fi
chmod 644 \"\$temporary\"
sync
systemctl stop emustation
service_stopped=1
mv \"\$temporary\" \"\$config\"
sync
printf 'backup=%s\n' \"\$backup\"
grep '^global\\.bootgame\\.' \"\$config\" || true
systemctl start emustation
service_stopped=0
trap - EXIT INT TERM
"

echo "muku autostart mode: $mode"
