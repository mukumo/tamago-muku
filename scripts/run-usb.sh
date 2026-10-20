#!/bin/sh
set -eu
serial=${1:?Usage: run-usb.sh ADB_SERIAL}
cd "$(dirname "$0")/.."
adb -s "$serial" shell 'if pidof muku-pet >/dev/null 2>&1; then echo "muku is already running; exit with START before transferring." >&2; exit 1; fi'
case "$(adb -s "$serial" shell uname -m | tr -d '\r')" in
    aarch64) ;;
    *) echo "Expected the aarch64 RK3562 handheld" >&2; exit 1 ;;
esac
adb -s "$serial" shell 'test "$(cat /sys/class/graphics/fb0/virtual_size)" = "480,272"'
cargo build --locked --release --target aarch64-unknown-linux-musl
adb -s "$serial" shell 'mkdir -p /tmp/muku'
adb -s "$serial" push target/aarch64-unknown-linux-musl/release/muku-pet /tmp/muku/muku-pet
adb -s "$serial" push scripts/run-temporary.sh /tmp/muku/run.sh
adb -s "$serial" shell 'chmod 755 /tmp/muku/muku-pet; sh /tmp/muku/run.sh'
