#!/usr/bin/env python3
"""Derive compact protocol ID maps from the player's vanilla datagen reports.

The reports and game jars stay outside the public tree. Example:
  python3 tools/generate_registry_maps.py \
    --old /tmp/26.2/reports --new /tmp/26.3/reports \
    --old-version 26.2 --new-version 26.3 \
    --out crates/horizon-protocol/src/registry_26_2.rs
"""

import argparse
import json
import subprocess
from pathlib import Path


def ranges(pairs):
    result = []
    for old, new in sorted(pairs):
        delta = new - old
        if result and old == result[-1][1] + 1 and delta == result[-1][2]:
            result[-1] = (result[-1][0], old, delta)
        else:
            result.append((old, old, delta))
    return result


def blocks(old_dir, new_dir):
    old = json.loads((old_dir / "blocks.json").read_text())
    new = json.loads((new_dir / "blocks.json").read_text())
    pairs = []
    for name, block in old.items():
        if name not in new:
            continue
        target = {
            tuple(sorted(s.get("properties", {}).items())): s["id"]
            for s in new[name]["states"]
        }
        for state in block["states"]:
            key = tuple(sorted(state.get("properties", {}).items()))
            if key in target:
                pairs.append((state["id"], target[key]))
    return pairs


def registry(old_dir, new_dir, name):
    old = json.loads((old_dir / "registries.json").read_text())
    new = json.loads((new_dir / "registries.json").read_text())
    old_entries = old[f"minecraft:{name}"]["entries"]
    new_entries = new[f"minecraft:{name}"]["entries"]
    return [
        (value["protocol_id"], new_entries[key]["protocol_id"])
        for key, value in old_entries.items()
        if key in new_entries
    ]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--old", type=Path, required=True)
    parser.add_argument("--new", type=Path, required=True)
    parser.add_argument("--old-version", required=True)
    parser.add_argument("--new-version", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    tables = {"BLOCK_STATE": blocks(args.old, args.new)}
    for name in ("item", "entity_type", "particle_type", "data_component_type", "block_entity_type", "sound_event"):
        tables[name.upper()] = registry(args.old, args.new, name)
    lines = [
        f"//! Protocol ID mappings from Java Edition {args.old_version} to {args.new_version}.",
        "//! Generated from local vanilla datagen reports by tools/generate_registry_maps.py.",
        "//! The reports are not included in the repository.",
        "#![allow(missing_docs)]",
        "",
        "#[derive(Clone, Copy)]",
        "struct Range { first: i32, last: i32, delta: i32 }",
        "fn map(ranges: &[Range], old: i32) -> Option<i32> {",
        "    let index = ranges.partition_point(|range| range.last < old);",
        "    let range = ranges.get(index)?;",
        "    (old >= range.first).then_some(old + range.delta)",
        "}",
        "",
    ]
    for name, pairs in tables.items():
        compact = ranges(pairs)
        lines.append(f"// {len(pairs)} matching IDs in {len(compact)} runs.")
        lines.append(f"const {name}: &[Range] = &[")
        for first, last, delta in compact:
            lines.append(f"    Range {{ first: {first}, last: {last}, delta: {delta} }},")
        lines.extend([
            "];",
            f"pub fn {name.lower()}(old: i32) -> Option<i32> {{ map({name}, old) }}",
            "",
        ])
        if name in ("ITEM", "DATA_COMPONENT_TYPE"):
            inverse = ranges((new, old) for old, new in pairs)
            lines.append(f"const {name}_TO_OLD: &[Range] = &[")
            for first, last, delta in inverse:
                lines.append(f"    Range {{ first: {first}, last: {last}, delta: {delta} }},")
            lines.extend([
                "];",
                f"pub fn {name.lower()}_to_old(new: i32) -> Option<i32> {{ map({name}_TO_OLD, new) }}",
                "",
            ])
    args.out.write_text("\n".join(lines))
    subprocess.run(["rustfmt", "--edition", "2024", str(args.out)], check=True)
    for name, pairs in tables.items():
        print(f"{name}: {len(pairs)} IDs, {len(ranges(pairs))} runs")


if __name__ == "__main__":
    main()
