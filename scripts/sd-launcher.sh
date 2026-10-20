#!/bin/sh
# This SD-only entrypoint returns to the preserved menu on START or failure.
# No firmware settings or internal-storage files are changed.
webhook_config=
if [ -f /sdcard/muku/webhook.json ]; then
    webhook_config="--webhook-config /sdcard/muku/webhook.json"
fi
# shellcheck disable=SC2086
/sdcard/muku/muku-pet --device --state /sdcard/muku/state.json $webhook_config \
    >>/tmp/muku-sd.log 2>&1
status=$?
printf 'muku exited (%s); opening the original menu\n' "$status" >>/tmp/muku-sd.log
exec /bin/sh /sdcard/simulator/3562/em_ui.before-muku.sh "$@"
