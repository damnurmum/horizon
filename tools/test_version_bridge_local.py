#!/usr/bin/env python3
"""Run bridge smoke tests against disposable, loopback-only vanilla release servers.

Example: python3 tools/test_version_bridge_local.py \
  --server 26.1=/path/to/26.1/server.jar --server 26.2=/path/to/26.2/server.jar
The test never connects to a public server or uses the player's profile.
"""

import argparse
import gzip
import json
import os
import re
import socket
import struct
import subprocess
import sys
import tempfile
import time
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RELEASE = re.compile(r"(?:[0-9]+\.)+[0-9]+\Z")


def server_arg(value):
    version, separator, raw_jar = value.partition("=")
    if not separator or not RELEASE.fullmatch(version):
        raise argparse.ArgumentTypeError("use RELEASE=/absolute/path/server.jar")
    jar = Path(raw_jar).expanduser().resolve()
    if not jar.is_file():
        raise argparse.ArgumentTypeError(f"server jar does not exist: {jar}")
    return version, jar


def verify_jar(jar, expected):
    with zipfile.ZipFile(jar) as archive:
        try:
            version = json.loads(archive.read("version.json"))["id"]
        except (KeyError, ValueError) as exc:
            raise RuntimeError(f"cannot verify server version in {jar}") from exc
    if version != expected:
        raise RuntimeError(f"{jar} is {version}, expected {expected}")


def free_loopback_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait_for_server(process, log, port):
    deadline = time.monotonic() + 80
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"server exited with {process.returncode}; see {log}")
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.25):
                return
        except OSError:
            time.sleep(0.3)
    raise TimeoutError(f"server did not listen on loopback; see {log}")


def saved_player_state(case_dir):
    """Read the position and inventory from the temporary vanilla player's NBT."""
    files = list((case_dir / "world/players/data").glob("*.dat"))
    if len(files) != 1:
        raise RuntimeError(f"expected one saved test player, found {len(files)}")
    with gzip.open(files[0], "rb") as source:
        data = source.read()
    offset = 0

    def read(fmt):
        nonlocal offset
        size = struct.calcsize(fmt)
        value = struct.unpack_from(fmt, data, offset)[0]
        offset += size
        return value

    def string():
        nonlocal offset
        length = read(">H")
        value = data[offset:offset + length].decode("utf-8")
        offset += length
        return value

    def payload(kind):
        nonlocal offset
        primitives = {1: ">b", 2: ">h", 3: ">i", 4: ">q", 5: ">f", 6: ">d"}
        if kind in primitives:
            return read(primitives[kind])
        if kind == 8:
            return string()
        if kind in (7, 11, 12):
            length = read(">i")
            if length < 0:
                raise ValueError("negative NBT array length")
            offset += length * {7: 1, 11: 4, 12: 8}[kind]
            return None
        if kind == 9:
            item_kind, length = read(">b"), read(">i")
            if length < 0:
                raise ValueError("negative NBT list length")
            return [payload(item_kind) for _ in range(length)]
        if kind == 10:
            values = {}
            while (member_kind := read(">b")) != 0:
                member_name = string()
                values[member_name] = payload(member_kind)
            return values
        raise ValueError(f"unexpected NBT tag type {kind}")

    if read(">b") != 10:
        raise ValueError("player NBT root is not a compound")
    string()  # root name
    return payload(10)


def run_case(version, jar, hmod, probe, scratch):
    verify_jar(jar, version)
    case_dir = Path(tempfile.mkdtemp(prefix=f"{version}-", dir=scratch))
    (case_dir / "eula.txt").write_text("eula=true\n")
    port = free_loopback_port()
    (case_dir / "server.properties").write_text(
        f"server-ip=127.0.0.1\nserver-port={port}\nonline-mode=false\n"
        "gamemode=creative\nforce-gamemode=true\ndifficulty=peaceful\n"
        "spawn-protection=0\nview-distance=3\nsimulation-distance=3\n"
        "enable-command-block=false\nmax-players=2\n"
    )
    log = case_dir / "server.log"
    client_home = case_dir / "client-home"
    client_home.mkdir()
    probe_env = os.environ.copy()
    probe_env.update({
        "HORIZON_HOME": str(client_home),
        "HORIZON_LAUNCHER_HOME": str(client_home / "launcher"),
        "XDG_DATA_HOME": str(client_home / "xdg-data"),
        "XDG_CONFIG_HOME": str(client_home / "xdg-config"),
        "HORIZON_LAUNCHER_NO_SELF_UPDATE": "1",
        "HORIZON_NO_AUDIO": "1",
        "HORIZON_LAUNCHER_TRASH": str(client_home / "trash"),
        "HORIZON_LAUNCHER_CLIPBOARD": str(client_home / "clipboard"),
        "HORIZON_LAUNCHER_DESKTOP": str(client_home / "desktop"),
    })
    with log.open("wb") as output:
        process = subprocess.Popen(
            ["java", "-Xmx1G", "-jar", str(jar), "nogui"],
            cwd=case_dir, stdin=subprocess.PIPE, stdout=output,
            stderr=subprocess.STDOUT,
        )
        try:
            wait_for_server(process, log, port)
            result = subprocess.run(
                [str(probe), f"127.0.0.1:{port}", version, str(hmod)],
                cwd=ROOT, env=probe_env, capture_output=True, text=True, timeout=55,
            )
            (case_dir / "probe.log").write_text(result.stdout + result.stderr)
            if result.returncode:
                raise RuntimeError(f"{version} probe failed ({result.returncode}); see {case_dir / 'probe.log'}")
        finally:
            if process.poll() is None:
                try:
                    process.stdin.write(b"stop\n")
                    process.stdin.flush()
                    process.wait(timeout=15)
                except (OSError, subprocess.TimeoutExpired):
                    process.terminate()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
    state = saved_player_state(case_dir)
    starts = re.findall(r"^START_POS ([^ ]+) ([^ ]+) ([^\n]+)$", result.stdout, re.MULTILINE)
    if not starts:
        raise RuntimeError(f"probe did not report its starting position; see {case_dir / 'probe.log'}")
    start_x = float(starts[-1][0])
    saved_x = float(state["Pos"][0])
    if abs(saved_x - (start_x + 0.2)) > 0.01:
        raise RuntimeError(f"{version} movement was not saved by server: {start_x} -> {saved_x}")
    inventory = state["Inventory"]
    if not any(item.get("id") == "minecraft:diamond" and item.get("count") == 1
               for item in inventory):
        raise RuntimeError(f"{version} server did not save the remaining diamond")
    print(result.stdout.strip())
    print(f"PASS {version}: server saved movement and one remaining diamond")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server", action="append", required=True, type=server_arg)
    parser.add_argument("--hmod", type=Path, default=ROOT / "target/horizon-mod/version_bridge.hmod")
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--keep", type=Path, help="keep disposable server worlds and logs here")
    args = parser.parse_args()
    versions = [version for version, _ in args.server]
    if len(versions) != len(set(versions)):
        parser.error("each release may appear only once")
    for version, jar in args.server:
        verify_jar(jar, version)
    hmod = args.hmod.resolve()
    probe = ROOT / "target/debug/examples/version_bridge_probe"
    if not args.skip_build:
        builder = ROOT / "target/debug/cargo-horizon-mod"
        subprocess.run(["cargo", "build", "-p", "cargo-horizon-mod"], cwd=ROOT, check=True)
        subprocess.run(
            [str(builder), "build"],
            cwd=ROOT / "crates/horizon-mod-sdk/examples/version-bridge", check=True,
        )
        subprocess.run(
            ["cargo", "build", "-p", "horizon-client", "--features", "mods", "--example", "version_bridge_probe"],
            cwd=ROOT, check=True,
        )
    if not hmod.is_file() or not probe.is_file():
        parser.error("build the mod and probe first, or omit --skip-build")
    if args.keep:
        scratch = args.keep.resolve()
        scratch.mkdir(parents=True, exist_ok=True)
        for version, jar in args.server:
            run_case(version, jar, hmod, probe, scratch)
        print(f"logs and test worlds: {scratch}")
    else:
        with tempfile.TemporaryDirectory(prefix="horizon-bridge-local-") as directory:
            scratch = Path(directory)
            for version, jar in args.server:
                run_case(version, jar, hmod, probe, scratch)
    print(f"PASS: {len(args.server)} local release server(s)")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.SubprocessError) as exc:
        sys.exit(f"FAIL: {exc}")
