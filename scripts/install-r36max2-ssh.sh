#!/bin/sh
set -eu

target=${1:?Usage: install-r36max2-ssh.sh root@HOST}
key=${R36_SSH_KEY:-$HOME/.ssh/id_ed25519}
known_hosts=${R36_KNOWN_HOSTS:-$HOME/.ssh/known_hosts}
repo=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
binary="$repo/target/aarch64-unknown-linux-musl/release/muku-pet"

remote() {
    ssh -i "$key" \
        -o IdentitiesOnly=yes \
        -o BatchMode=yes \
        -o StrictHostKeyChecking=yes \
        -o UserKnownHostsFile="$known_hosts" \
        "$target" "$@"
}

cargo build --locked --release --target aarch64-unknown-linux-musl
stage="/storage/muku/.muku-pet.new-$$"
remote 'mkdir -p /storage/muku /storage/roms/ports_scripts'

remote "dd of='$stage' bs=65536 2>/dev/null" <"$binary"

local_hash=$(shasum -a 256 "$binary" | awk '{print $1}')
remote_hash=$(remote "sha256sum '$stage'" | awk '{print $1}')
if [ "$local_hash" != "$remote_hash" ]; then
    echo "Binary transfer verification failed" >&2
    exit 1
fi

tar -C "$repo/packaging/r36max2" -cf - Muku.sh gamelist.xml |
    remote 'tar -xf - -C /storage/roms/ports_scripts && chmod 755 /storage/roms/ports_scripts/Muku.sh'

remote "
mv '$stage' /storage/muku/muku-pet
chmod 755 /storage/muku/muku-pet
sha256sum /storage/muku/muku-pet
"
echo "Installed muku on $target; webhook.json and state.json were preserved."
