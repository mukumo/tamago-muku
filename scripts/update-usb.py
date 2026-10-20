#!/usr/bin/env python3
"""Update an installed SD copy over adb, preserving state and optionally rebooting."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import time


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serial", required=True)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--restart", action="store_true")
    args = parser.parse_args()
    adb = ["adb", "-s", args.serial]

    def run(*parts, timeout=20):
        return subprocess.check_output([*adb, *parts], timeout=timeout)

    def shell(command):
        return run("shell", command).decode().strip()

    payload = args.binary.read_bytes()
    if payload[:6] != b"\x7fELF\x02\x01" or payload[18:20] != b"\xb7\x00":
        raise RuntimeError("expected an aarch64 little-endian ELF binary")
    if shell("uname -m") != "aarch64":
        raise RuntimeError("unexpected target architecture")
    raw_manifest = run("exec-out", "cat", "/sdcard/muku/install-manifest.json")
    manifest = json.loads(raw_manifest)
    if manifest.get("version") != 1 or manifest.get("status") != "installed":
        raise RuntimeError("expected an existing version 1 SD installation")
    for key, path in [
        ("binary_sha256", "/sdcard/muku/muku-pet"),
        ("launcher_sha256", "/sdcard/simulator/3562/em_ui.sh"),
        ("original_sha256", "/sdcard/simulator/3562/em_ui.before-muku.sh"),
    ]:
        if shell(f"sha256sum {path}").split()[0] != manifest[key]:
            raise RuntimeError(f"preserving unexpected changes to {path}")

    # A fixed lock makes concurrent updates fail rather than race each other.
    shell("mkdir /sdcard/muku/.usb-update-lock")
    try:
        print(run("push", str(args.binary), "/sdcard/muku/muku-pet.update").decode(), end="")
        if shell("sha256sum /sdcard/muku/muku-pet.update").split()[0] != sha(payload):
            raise RuntimeError("binary transfer verification failed")
        updated = dict(manifest, binary_sha256=sha(payload),
                       previous_binary_sha256=manifest["binary_sha256"])
        data = (json.dumps(updated, indent=2, sort_keys=True) + "\n").encode()
        with tempfile.TemporaryDirectory(prefix="muku-usb-") as temp:
            path = Path(temp) / "install-manifest.json"
            path.write_bytes(data)
            print(run("push", str(path), "/sdcard/muku/install-manifest.update").decode(), end="")
        if run("exec-out", "cat", "/sdcard/muku/install-manifest.update") != data:
            raise RuntimeError("manifest transfer verification failed")
        # Keep the previous executable and manifest for interrupted-update recovery.
        shell("cp /sdcard/muku/muku-pet /sdcard/muku/muku-pet.previous && "
              "cp /sdcard/muku/install-manifest.json /sdcard/muku/install-manifest.previous && "
              "chmod 755 /sdcard/muku/muku-pet.update && sync && "
              "mv /sdcard/muku/muku-pet.update /sdcard/muku/muku-pet && "
              "mv /sdcard/muku/install-manifest.update /sdcard/muku/install-manifest.json && sync")
        if shell("sha256sum /sdcard/muku/muku-pet").split()[0] != sha(payload):
            raise RuntimeError("installed binary readback failed")
        if run("exec-out", "cat", "/sdcard/muku/install-manifest.json") != data:
            raise RuntimeError("installed manifest readback failed")
        print("Updated and verified; pet state preserved.")
    finally:
        shell("rmdir /sdcard/muku/.usb-update-lock")

    if args.restart:
        # Termination is handled in Rust: save the pet before rebooting.
        shell("for p in $(pidof muku-pet 2>/dev/null); do kill -TERM \"$p\"; done")
        for _ in range(50):
            if not shell("pidof muku-pet 2>/dev/null || true"):
                break
            time.sleep(0.1)
        else:
            raise RuntimeError("muku did not exit; reboot cancelled")
        shell("sync")
        run("reboot")
        print("Reboot requested after saving. The SD launcher will start the update.")


if __name__ == "__main__":
    main()
