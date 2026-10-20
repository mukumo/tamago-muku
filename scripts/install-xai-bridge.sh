#!/bin/sh
set -eu

device_url=${1:?Usage: install-xai-bridge.sh http://R36_IP:8787/webhook}
repo=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
config=${MUKU_WEBHOOK_CONFIG:-$repo/.local/webhook.json}
template="$repo/packaging/xai/com.muku.xai-bridge.plist.template"
agent_dir="$HOME/Library/LaunchAgents"
agent="$agent_dir/com.muku.xai-bridge.plist"
domain="gui/$(id -u)"

case "$device_url$config$repo" in
    *'&'*|*'|'*|*'<'*|*'>'*) echo "Unsupported character in path or URL" >&2; exit 1 ;;
esac

/usr/bin/security find-generic-password \
    -a "${MUKU_XAI_KEYCHAIN_ACCOUNT:-kan}" \
    -s "${MUKU_XAI_KEYCHAIN_SERVICE:-tamago-muku-xai}" \
    -w >/dev/null
[ -r "$config" ] || {
    echo "Muku webhook config not found: $config" >&2
    exit 1
}

mkdir -p "$agent_dir" "$repo/.local"
sed \
    -e "s|__REPO__|$repo|g" \
    -e "s|__HOME__|$HOME|g" \
    -e "s|__DEVICE_URL__|$device_url|g" \
    -e "s|__CONFIG__|$config|g" \
    "$template" >"$agent"
chmod 644 "$agent"
plutil -lint "$agent"

launchctl bootout "$domain" "$agent" >/dev/null 2>&1 || true
launchctl bootstrap "$domain" "$agent"
launchctl kickstart -k "$domain/com.muku.xai-bridge"

sleep 1
curl --fail --silent --show-error http://127.0.0.1:8790/health
printf '\nInstalled com.muku.xai-bridge; open http://127.0.0.1:8790\n'
