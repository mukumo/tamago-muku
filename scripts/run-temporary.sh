#!/bin/sh
# Run from /tmp/muku on this known RK3562. All state is temporary.
# Suspend the stock respawn loop; resume it on normal exit or termination.
set -eu
cd /tmp/muku
if [ -e runner.pid ] && kill -0 "$(cat runner.pid)" 2>/dev/null; then
    echo "muku temporary runner is already active" >&2
    exit 1
fi
echo $$ > runner.pid
loops=$(ps | awk '/\/bin\/sh \/etc\/init.d\/game_ui.sh start/ && !/awk/ {print $1}')
if [ -z "$loops" ]; then
    echo "stock game_ui loop not found; refusing automatic takeover" >&2
    rm -f runner.pid
    exit 1
fi
child=
restore() {
    trap - EXIT INT TERM HUP
    if [ -n "$child" ] && kill -0 "$child" 2>/dev/null; then
        kill -TERM "$child" 2>/dev/null || true
        wait "$child" 2>/dev/null || true
    fi
    for pid in $loops; do kill -CONT "$pid" 2>/dev/null || true; done
    rm -f runner.pid
}
trap restore EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP
for pid in $loops; do kill -STOP "$pid"; done
running_games() {
    for pid in $(pidof game 2>/dev/null || true); do
        [ -r "/proc/$pid/stat" ] || continue
        state=$(awk '{print $3}' "/proc/$pid/stat")
        # The suspended parent cannot reap zombies until cleanup resumes it.
        case "$state" in Z|X) continue ;; esac
        printf '%s\n' "$pid"
    done
}
for pid in $(running_games); do kill -TERM "$pid"; done
# The vendor's own game_ui.sh stop path uses SIGKILL: this menu ignores TERM.
# Give it a chance to exit before applying that same process-only stop.
count=0
while [ -n "$(running_games)" ]; do
    count=$((count + 1))
    if [ "$count" -eq 30 ]; then
        for pid in $(running_games); do
            kill -KILL "$pid"
        done
    fi
    [ "$count" -lt 50 ] || { echo "stock menu did not stop" >&2; exit 1; }
    sleep 0.1
done
./muku-pet --device --state /tmp/muku/state.json "$@" &
child=$!
wait "$child"
