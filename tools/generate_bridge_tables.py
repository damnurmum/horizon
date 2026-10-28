#!/usr/bin/env python3
"""Regenerate packet and registry tables for implemented Java release bridges.

The input packet reports come from local vanilla server jars; Via mapping JSON
stays outside the repository. Example (repeat --target for a batch):

  python3 tools/generate_bridge_tables.py \
    --via-dir /tmp/horizon-via-mappings-26x --current-version 26.3 \
    --target 26.1=775=/tmp/26.1/reports/packets.json \
    --target 26.2=776=/tmp/26.2/reports/packets.json --check

Omit --check to write the generated Rust files. Generating tables does not
enable a version: a packet adapter and a tested route are also required.
"""

import argparse
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RELEASE = re.compile(r"(?:[0-9]+\.)+[0-9]+\Z")


def target(value):
    parts = value.split("=", 2)
    if len(parts) != 3 or not RELEASE.fullmatch(parts[0]):
        raise argparse.ArgumentTypeError("use RELEASE=PROTOCOL=PACKETS_JSON (stable releases only)")
    try:
        protocol = int(parts[1])
    except ValueError as exc:
        raise argparse.ArgumentTypeError("protocol must be a positive integer") from exc
    if protocol <= 0:
        raise argparse.ArgumentTypeError("protocol must be a positive integer")
    return parts[0], protocol, Path(parts[2])


def generate(version, protocol, report, current, via_dir, output):
    old = via_dir / f"mapping-{version}.json"
    new = via_dir / f"mapping-{current}.json"
    for path in (old, new, report):
        if not path.is_file():
            raise FileNotFoundError(path)
    diff = via_dir / f"mapping-{version}to{current}.json"
    reverse = via_dir / f"mapping-{current}to{version}.json"
    suffix = version.replace(".", "_")
    packet_file = output / f"ids_{suffix}.rs"
    registry_file = output / f"registry_{suffix}.rs"
    subprocess.run(
        [sys.executable, str(ROOT / "tools/generate_packet_ids.py"),
         "--report", str(report), "--out", str(packet_file),
         "--version-name", version, "--protocol-version", str(protocol)],
        check=True,
    )
    command = [
        sys.executable, str(ROOT / "tools/generate_registry_maps.py"),
        "--via-old", str(old), "--via-new", str(new),
        "--old-version", version, "--new-version", current,
        "--out", str(registry_file),
    ]
    if diff.is_file():
        command.extend(["--via-diff", str(diff)])
    if reverse.is_file():
        command.extend(["--via-reverse-diff", str(reverse)])
    subprocess.run(command, check=True)
    subprocess.run(["rustfmt", "--edition", "2024", str(packet_file), str(registry_file)], check=True)
    return packet_file, registry_file


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--via-dir", required=True, type=Path)
    parser.add_argument("--current-version", required=True)
    parser.add_argument("--target", action="append", required=True, type=target)
    parser.add_argument("--out-dir", type=Path, default=ROOT / "crates/horizon-protocol/src")
    parser.add_argument("--check", action="store_true", help="verify tracked tables without writing them")
    args = parser.parse_args()
    if not RELEASE.fullmatch(args.current_version):
        parser.error("--current-version must be a stable release")
    versions = [version for version, _, _ in args.target]
    if len(versions) != len(set(versions)) or args.current_version in versions:
        parser.error("targets must be unique and differ from the current version")
    with tempfile.TemporaryDirectory(prefix="horizon-bridge-tables-") as scratch:
        generated = Path(scratch)
        for version, protocol, report in args.target:
            for candidate in generate(version, protocol, report, args.current_version,
                                      args.via_dir, generated):
                destination = args.out_dir / candidate.name
                if args.check:
                    if not destination.exists() or destination.read_bytes() != candidate.read_bytes():
                        raise SystemExit(f"outdated bridge table: {destination}")
                    print(f"up to date: {destination}")
                else:
                    args.out_dir.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(candidate, destination)
                    print(f"wrote: {destination}")


if __name__ == "__main__":
    main()
