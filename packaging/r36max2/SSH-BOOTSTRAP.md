# R36MAX 2 SSH bootstrap

The stock EmuELEC 4.7 image checked on 2026-10-20 did not include OpenSSH
`sshd`, Dropbear, or an SSH service unit.

## Tested package

```text
Dropbear:       v2025.89
Source:         https://matt.ucc.asn.au/dropbear/releases/dropbear-2025.89.tar.bz2
Source SHA-256: 0d1f7ca711cfc336dc8a85e672cab9cfd8223a02fe2da0a4a7aeb58c9e113634
Runtime:        Alpine 3.22, aarch64, musl 1.2.5
Dropbear SHA:   7ef066441914d7e5be1b000450c9a25d058b822ae32a0786178a3b874d0583c1
Keygen SHA:     f69c27d1f67bc769f8fe0d221711df6cd3e9836f2e7ee5670e07e25c2e5c66d1
```

Build configuration:

```sh
./configure \
  --build=aarch64-linux-musl \
  --host=aarch64-linux-musl \
  CC=gcc \
  CFLAGS="-Os -W -Wall" \
  LDFLAGS="-static" \
  --disable-harden \
  --disable-zlib \
  --disable-lastlog \
  --disable-utmp \
  --disable-wtmp \
  --disable-syslog
make PROGRAMS="dropbear dropbearkey"
strip dropbear dropbearkey
```

The `--disable-syslog` build does not accept Dropbear's `-E` option.
The runtime command therefore uses:

```sh
dropbear -F -s -g \
  -p 0.0.0.0:22 \
  -P /storage/.config/muku-ssh/dropbear.pid \
  -r /storage/.config/muku-ssh/host_ed25519
```

Password authentication is disabled. The authorized key is installed into
`/storage/.ssh/authorized_keys`, and the service runs from
`muku-ssh.service` rather than inside the EmulationStation process cgroup.

The binaries are not committed to this repository. Their source URL, build
flags, and hashes are recorded here so the same build can be reproduced.
