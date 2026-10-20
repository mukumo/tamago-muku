#!/bin/bash
set -euo pipefail

usage() {
    cat >&2 <<'EOF'
Usage:
  send-notice-tts.sh --url URL --config webhook.json \
    --title TITLE --body TEXT [--voice Kyoko] [--rate 185] [--source codex] [--dry-run]

The config file is the same JSON used by muku-pet:
  {"bind":"0.0.0.0:8787","token":"..."}
The token is read locally and is never printed.
EOF
    exit 2
}

url=
config=
title="AIからのおしらせ"
body=
voice=${MUKU_TTS_VOICE:-Kyoko}
rate=185
source=codex
dry_run=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --url) url=${2:-}; shift 2 ;;
        --config) config=${2:-}; shift 2 ;;
        --title) title=${2:-}; shift 2 ;;
        --body) body=${2:-}; shift 2 ;;
        --voice) voice=${2:-}; shift 2 ;;
        --rate) rate=${2:-}; shift 2 ;;
        --source) source=${2:-}; shift 2 ;;
        --dry-run) dry_run=1; shift ;;
        *) usage ;;
    esac
done

[ -n "$url" ] && [ -n "$config" ] && [ -n "$body" ] || usage
[ -r "$config" ] || { echo "Cannot read webhook config: $config" >&2; exit 1; }
[ "$(printf '%s' "$body" | wc -m | tr -d ' ')" -le 180 ] ||
    { echo "Body must be 180 characters or fewer" >&2; exit 1; }

token=$(jq -er '.token | select(length >= 16)' "$config")
case "$url" in
    */webhook) ;;
    *) url=${url%/}/webhook ;;
esac

temporary=$(mktemp -d "${TMPDIR:-/tmp}/muku-tts.XXXXXX")
umask 077
aiff="$temporary/message.aiff"
wav="$temporary/message.wav"
audio_b64="$temporary/audio.b64"
json="$temporary/notice.json"
headers="$temporary/headers"
cleanup() {
    rm -f "$aiff" "$wav" "$audio_b64" "$json" "$headers"
    rmdir "$temporary"
}
trap cleanup EXIT

say -v "$voice" -r "$rate" -o "$aiff" -- "$body"
afconvert -f WAVE -d LEI16@22050 -c 1 "$aiff" "$wav"
[ "$(wc -c <"$wav" | tr -d ' ')" -le 2097152 ] ||
    { echo "Generated WAV is larger than 2 MiB" >&2; exit 1; }
base64 <"$wav" | tr -d '\n' >"$audio_b64"

jq -n \
    --arg title "$title" \
    --arg body "$body" \
    --arg source "$source" \
    --rawfile audio "$audio_b64" \
    '{
      title: $title,
      body: $body,
      source: $source,
      kind: "notice",
      audio_wav_base64: $audio
    }' >"$json"

if [ "$dry_run" -eq 1 ]; then
    jq '{title, body, source, kind, audio_wav_bytes: (.audio_wav_base64 | length)}' "$json"
    exit 0
fi

printf 'Authorization: Bearer %s\n' "$token" >"$headers"
curl --fail-with-body --connect-timeout 5 --max-time 30 \
    -X POST "$url" \
    -H @"$headers" \
    -H 'Content-Type: application/json' \
    --data-binary @"$json"
printf '\n'
