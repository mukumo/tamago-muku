#!/bin/bash
# Launch tamago-muku from the EmuELEC Ports menu.

set -u

BASE=/storage/muku
mkdir -p "$BASE"

args=(
    --device
    --state "$BASE/state.json"
)
if [ -f "$BASE/webhook.json" ]; then
    args+=(--webhook-config "$BASE/webhook.json")
fi

exec "$BASE/muku-pet" "${args[@]}" >>"$BASE/muku.log" 2>&1
