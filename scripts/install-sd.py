#!/usr/bin/env python3
"""Install or restore Muku's SD entrypoint; never mount, format, or touch firmware."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import uuid


def digest(data):
    return hashlib.sha256(data).hexdigest()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def validate_volume(volume, expected_uuid):
    result = subprocess.run(
        ["/usr/sbin/diskutil", "info", "-plist", str(volume)],
        check=True, capture_output=True, timeout=20,
    )
    info = plistlib.loads(result.stdout)
    require(str(info.get("VolumeUUID", "")).upper() == expected_uuid,
            "volume UUID does not match; nothing was installed")
    require(info.get("FilesystemType", "").lower() == "exfat", "volume must be exFAT")
    require(info.get("Mounted") is not False and info.get("MountPoint") == str(volume)
            and os.path.ismount(volume), "volume is not mounted at the requested path")
    require(all(info.get(key) is True for key in ("Writable", "WritableMedia", "WritableVolume")),
            "volume is read-only; this installer will not remount it")
    return info


def checked_path(volume, relative):
    path = volume
    for component in Path(relative).parts:
        path = path / component
        require(not path.is_symlink(), f"symlink is not allowed: {path}")
    return path


def replace_verified(path, data, mode=0o644):
    """Stage beside destination, sync bytes, replace, and independently read back."""
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as output:
            output.write(data)
            os.fchmod(output.fileno(), mode)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, path)
        readback = path.read_bytes()
        require(readback == data and digest(readback) == digest(data),
                f"readback failed: {path}")
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def write_manifest(path, manifest):
    replace_verified(path, (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode())


def operate(volume, expected_uuid, binary, restore):
    info = validate_volume(volume, expected_uuid)
    entry = checked_path(volume, "simulator/3562/em_ui.sh")
    backup = checked_path(volume, "simulator/3562/em_ui.before-muku.sh")
    directory = checked_path(volume, "muku")
    manifest_path = checked_path(volume, "muku/install-manifest.json")
    target = checked_path(volume, "muku/muku-pet")
    require(entry.is_file(), "original menu entrypoint is missing")
    require(not directory.exists() or directory.is_dir(), "/muku must be a directory")
    launcher = Path(__file__).with_name("sd-launcher.sh").read_bytes()
    payload = None
    if not restore:
        payload = binary.read_bytes()
        require(len(payload) >= 64 and payload[:6] == b"\x7fELF\x02\x01"
                and payload[18:20] == b"\xb7\x00", "binary must be little-endian aarch64 ELF")
    require(not restore or manifest_path.is_file(), "no Muku installation manifest to restore")
    directory.mkdir(exist_ok=True)
    lock = checked_path(volume, "muku/.install-lock")
    lock.mkdir()  # Atomic exclusion. A stale lock requires inspection, never forced deletion.
    try:
        current = entry.read_bytes()
        current_hash = digest(current)
        manifest = None
        if manifest_path.exists():
            manifest = json.loads(manifest_path.read_bytes())
            require(isinstance(manifest, dict), "installation manifest must be an object")
            require(manifest.get("version") == 1 and manifest.get("volume_uuid") == expected_uuid,
                    "manifest version or volume UUID does not match")
            require(manifest.get("status") in ("prepared", "installed", "restored"),
                    "unknown installation status")
            require(backup.is_file(), "original menu backup is missing")
            original = backup.read_bytes()
            require(digest(original) == manifest.get("original_sha256"), "original backup changed")
            known = {manifest.get("launcher_sha256"), manifest.get("previous_launcher_sha256")}
            unchanged_original = current_hash == digest(original)
            require(current_hash in known or (unchanged_original and
                    (restore or manifest["status"] in ("prepared", "restored"))),
                    "entrypoint changed outside this installer; refusing to overwrite it")
        else:
            require(not backup.exists(), "backup exists without a manifest; inspect it first")
            require(not target.exists(), "binary exists without a manifest; refusing to overwrite it")
            original = current
        if restore:
            if current_hash != digest(original):
                require(entry.read_bytes() == current, "entrypoint changed during restore")
                replace_verified(entry, original, 0o755)
            manifest["status"] = "restored"
            write_manifest(manifest_path, manifest)
            print(f"Restored original menu on {info.get('DeviceIdentifier')}; /muku and state preserved.")
            return
        if manifest is not None and target.exists():
            require(target.is_file() and digest(target.read_bytes()) == manifest.get("binary_sha256"),
                    "installed binary changed outside this installer")
        if manifest is None:
            require(entry.read_bytes() == current, "entrypoint changed during backup")
            replace_verified(backup, original, 0o755)
        previous = manifest.get("launcher_sha256") if manifest else None
        manifest = {
            "version": 1, "volume_uuid": expected_uuid, "status": "prepared",
            "original_sha256": digest(original), "launcher_sha256": digest(launcher),
            "previous_launcher_sha256": previous, "binary_sha256": digest(payload),
        }
        # The journal knows both entrypoint versions before either boot file changes.
        write_manifest(manifest_path, manifest)
        replace_verified(target, payload, 0o755)
        require(entry.read_bytes() == current, "entrypoint changed during installation")
        require(backup.read_bytes() == original, "backup changed during installation")
        replace_verified(entry, launcher, 0o755)
        manifest["status"] = "installed"
        write_manifest(manifest_path, manifest)
        print(f"Installed and verified Muku on {info.get('DeviceIdentifier')} ({expected_uuid}).")
        print(f"Original menu backup: {backup}")
    finally:
        lock.rmdir()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--volume", required=True, type=Path)
    parser.add_argument("--expected-uuid", required=True)
    parser.add_argument("--binary", type=Path, help="required with --install")
    operation = parser.add_mutually_exclusive_group(required=True)
    operation.add_argument("--install", action="store_true")
    operation.add_argument("--restore", action="store_true")
    args = parser.parse_args(argv)
    if args.install and args.binary is None:
        parser.error("--install requires --binary")
    try:
        expected_uuid = str(uuid.UUID(args.expected_uuid)).upper()
        volume = args.volume.resolve(strict=True)
        operate(volume, expected_uuid, args.binary, args.restore)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
