#!/usr/bin/env python3
"""Derive compact protocol ID maps from vanilla reports or Via Mappings data.

The reports and game jars stay outside the public tree. Example:
  python3 tools/generate_registry_maps.py \
    --old /tmp/26.2/reports --new /tmp/26.3/reports \
    --old-version 26.2 --new-version 26.3 \
    --out crates/horizon-protocol/src/registry_26_2.rs

ViaVersion/Mappings' ``mappings/`` JSON files can be used instead of reports:
  python3 tools/generate_registry_maps.py \
    --via-old /tmp/mapping-26.2.json --via-new /tmp/mapping-26.3.json \
    --via-diff /tmp/mapping-26.2to26.3.json \
    --via-reverse-diff /tmp/mapping-26.3to26.2.json \
    --old-version 26.2 --new-version 26.3 \
    --out crates/horizon-protocol/src/registry_26_2.rs
"""

import argparse
import json
import re
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


VIA_TABLES = {
    "BLOCK_STATE": "blockstates",
    "ITEM": "items",
    "ENTITY_TYPE": "entities",
    "PARTICLE_TYPE": "particles",
    "DATA_COMPONENT_TYPE": "data_component_type",
    "BLOCK_ENTITY_TYPE": "blockentities",
    "SOUND_EVENT": "sounds",
}


def material_target(name, new_ids, materials):
    """Apply Via's documented whole-word material fallbacks when the result exists."""
    base, separator, properties = name.partition("[")
    matches = []
    for source, rule in materials.items():
        target = rule if isinstance(rule, str) else rule["to"]
        words = {} if isinstance(rule, str) else rule.get("words", {})
        pattern = rf"(?<![a-z0-9]){re.escape(source)}(?![a-z0-9])"
        replaced, count = re.subn(pattern, target, base)
        if not count:
            continue
        for old_word, new_word in words.items():
            pattern = rf"(?<![a-z0-9]){re.escape(old_word)}(?![a-z0-9])"
            replaced = re.sub(pattern, new_word, replaced)
        candidate = replaced + separator + properties
        if candidate in new_ids:
            matches.append((len(source.split("_")), candidate))
    return max(matches, default=(0, None))[1]


def via_pairs(old, new, diff, key):
    """Map protocol IDs by identifier, including documented explicit renames."""
    old_names = old[key]
    overrides = diff.get(key, {})
    materials = diff.get("materials", {})
    if not isinstance(old_names, list) or not isinstance(new[key], list):
        raise ValueError(f"Via mapping {key} must be an identifier array")
    if not isinstance(overrides, dict):
        raise ValueError(f"Via diff {key} must be an object")
    if key == "blockstates" and any(name not in old_names for name in overrides):
        raise ValueError("Via diff has partial blockstate rules that need a state-rule converter")
    new_ids = {name: index for index, name in enumerate(new[key])}
    pairs = []
    for source_id, source_name in enumerate(old_names):
        target_name = overrides.get(source_name, source_name)
        if not isinstance(target_name, str):
            raise ValueError(f"Via diff for {key}/{source_name} needs a state-rule converter")
        if not target_name:
            continue  # explicitly removed in the target version
        if target_name.startswith("minecraft:"):
            target_name = target_name[len("minecraft:") :]
        if target_name not in new_ids:
            if source_name in overrides:
                raise ValueError(f"Via diff for {key}/{source_name} names unknown {target_name}")
            target_name = material_target(source_name, new_ids, materials)
            if target_name is None:
                continue  # introduced/removed between versions, with no alias
        pairs.append((source_id, new_ids[target_name]))
    return pairs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--old", type=Path, help="old vanilla datagen reports directory")
    parser.add_argument("--new", type=Path, help="new vanilla datagen reports directory")
    parser.add_argument("--via-old", type=Path, help="old Via Mappings JSON file")
    parser.add_argument("--via-new", type=Path, help="new Via Mappings JSON file")
    parser.add_argument("--via-diff", type=Path, help="optional Via Mappings diff JSON file")
    parser.add_argument("--via-reverse-diff", type=Path, help="optional new-to-old Via diff for outgoing item fallbacks")
    parser.add_argument("--old-version", required=True)
    parser.add_argument("--new-version", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    reports = args.old is not None and args.new is not None
    via = args.via_old is not None and args.via_new is not None
    if reports == via or ((args.via_diff or args.via_reverse_diff) and not via):
        parser.error("supply either --old/--new or --via-old/--via-new with optional Via diffs")
    if via:
        old = json.loads(args.via_old.read_text())
        new = json.loads(args.via_new.read_text())
        diff = json.loads(args.via_diff.read_text()) if args.via_diff else {}
        tables = {name: via_pairs(old, new, diff, key) for name, key in VIA_TABLES.items()}
        reverse_diff = json.loads(args.via_reverse_diff.read_text()) if args.via_reverse_diff else None
        reverse = (
            {
                name: via_pairs(new, old, reverse_diff, VIA_TABLES[name])
                for name in ("ITEM", "DATA_COMPONENT_TYPE")
            }
            if reverse_diff is not None
            else {}
        )
        source = "ViaVersion/Mappings mapping JSON (mappings/), freely reusable per its README"
    else:
        tables = {"BLOCK_STATE": blocks(args.old, args.new)}
        for name in ("item", "entity_type", "particle_type", "data_component_type", "block_entity_type", "sound_event"):
            tables[name.upper()] = registry(args.old, args.new, name)
        reverse = {}
        source = "local vanilla datagen reports"
    lines = [
        f"//! Protocol ID mappings from Java Edition {args.old_version} to {args.new_version}.",
        f"//! Generated from {source} by tools/generate_registry_maps.py.",
        "//! Source data is not included in the repository.",
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
            inverse_pairs = [(new, old) for old, new in pairs]
            inverse = ranges(inverse_pairs)
            lines.append(f"const {name}_TO_OLD: &[Range] = &[")
            for first, last, delta in inverse:
                lines.append(f"    Range {{ first: {first}, last: {last}, delta: {delta} }},")
            lines.append("];")
            original = dict(inverse_pairs)
            overrides = sorted(
                (source_id, target_id)
                for source_id, target_id in reverse.get(name, [])
                if original.get(source_id) != target_id
            )
            if overrides:
                lines.append("// Explicit backwards fallbacks from ViaVersion/Mappings.")
                lines.append(f"const {name}_TO_OLD_OVERRIDES: &[(i32, i32)] = &[")
                for source_id, target_id in overrides:
                    lines.append(f"    ({source_id}, {target_id}),")
                lines.append("];")
                lines.append(f"pub fn {name.lower()}_to_old(new: i32) -> Option<i32> {{")
                lines.append(f"    {name}_TO_OLD_OVERRIDES.binary_search_by_key(&new, |pair| pair.0)")
                lines.append(f"        .ok().map(|index| {name}_TO_OLD_OVERRIDES[index].1)")
                lines.append(f"        .or_else(|| map({name}_TO_OLD, new))")
                lines.append("}")
            else:
                lines.append(f"pub fn {name.lower()}_to_old(new: i32) -> Option<i32> {{ map({name}_TO_OLD, new) }}")
            lines.append("")
    args.out.write_text("\n".join(lines))
    subprocess.run(["rustfmt", "--edition", "2024", str(args.out)], check=True)
    for name, pairs in tables.items():
        print(f"{name}: {len(pairs)} IDs, {len(ranges(pairs))} runs")


if __name__ == "__main__":
    main()
