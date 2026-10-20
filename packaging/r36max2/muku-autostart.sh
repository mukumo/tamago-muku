#!/bin/sh
set -u

marker=/run/tamago-muku-autostart-done
launcher=/storage/roms/ports_scripts/Muku.sh
binary=/storage/muku/muku-pet

# EmulationStation retries its service after a crash. Show muku once per boot,
# then let those retries go straight to the normal frontend.
[ ! -e "$marker" ] || exit 0
[ -x "$launcher" ] || exit 0
[ -x "$binary" ] || exit 0

: >"$marker" 2>/dev/null || true
exec "$launcher"
