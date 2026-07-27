#!/usr/bin/env bash
# T-439 — Class-R: every Objects-palette-eligible workbench kind has a matching
# prop:/comp: row in apps/mod/tbd-framework/Data/registry.json, with guid == resource_name.
#
# Alias derivation MUST mirror apps/website/frontend/src/asset_catalog.rs::derive_object_alias
# (KNOWN reverse-hit for comp:checkpoint_small; else prop:/comp: + display-name slug).
#
# Source of Objects-eligible kinds: packages/tbd-schema/registry/registry-items.workbench.json
# (crate|other, non-abstract) — the same export the API imports. No live Workbench required.
#
# Gate: bash scripts/mod/verify-t439-objects-registry-aliases.sh
# OWNS WIDEN: wave_plan T-439 lists registry.json + asset_catalog.rs; this script is the
# Class-R perturbation guard for the Objects alias ↔ spawn registry invariant.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WB="$ROOT/packages/tbd-schema/registry/registry-items.workbench.json"
MOD="$ROOT/apps/mod/tbd-framework/Data/registry.json"
FE="$ROOT/apps/website/frontend/src/asset_catalog.rs"

for f in "$WB" "$MOD" "$FE"; do
	if [[ ! -f "$f" ]]; then
		echo "FAIL: missing $f"
		exit 1
	fi
done

# Pin the FE derivation contract still exists (slug + KNOWN checkpoint).
if ! grep -q 'pub fn derive_object_alias' "$FE"; then
	echo "FAIL: derive_object_alias missing from asset_catalog.rs"
	exit 1
fi
if ! grep -q 'comp:checkpoint_small' "$FE"; then
	echo "FAIL: KNOWN comp:checkpoint_small reverse-hit missing from asset_catalog.rs"
	exit 1
fi

python3 - "$WB" "$MOD" <<'PY'
import json, re, sys
from pathlib import Path

wb_path, mod_path = Path(sys.argv[1]), Path(sys.argv[2])
GUID_RE = re.compile(r"^\{[0-9A-F]{16}\}[A-Za-z0-9/_.\-]+$")
ALIAS_RE = re.compile(r"^(kit|comp|veh|preset|layer|prop|item):[a-z0-9_]+$")

def object_alias_slug(raw: str) -> str:
    out = []
    prev_repl = False
    for c in raw.lower():
        if c.isascii() and (c.islower() or c.isdigit()):
            out.append(c)
            prev_repl = False
        elif not prev_repl:
            out.append("_")
            prev_repl = True
    trimmed = "".join(out).strip("_")
    return trimmed or "object"

def derive_object_alias(resource_name: str, display_name: str) -> str:
    known = {
        "{E1D01D77D7F47EF3}PrefabsEditable/Auto/Compositions/Misc/SubCompositions/E_Sandbag_Barricade_US_04.et":
            "comp:checkpoint_small",
    }
    if resource_name in known:
        return known[resource_name]
    prefix = (
        "comp"
        if ("Composition" in resource_name or "Compositions" in resource_name)
        else "prop"
    )
    return f"{prefix}:{object_alias_slug(display_name)}"

wb = json.loads(wb_path.read_text())
eligible = [
    i
    for i in wb["items"]
    if i.get("kind") in ("crate", "other") and not i.get("abstract")
]
mod = json.loads(mod_path.read_text())
by_alias = {e["alias"]: e for e in mod["entries"]}

prop_n = sum(1 for a in by_alias if a.startswith("prop:"))
comp_n = sum(1 for a in by_alias if a.startswith("comp:"))

missing = []
guid_mismatch = []
bad_shape = []
for i in eligible:
    alias = derive_object_alias(i["resource_name"], i["display_name"])
    if not ALIAS_RE.match(alias):
        bad_shape.append(("alias", alias, i["display_name"]))
    if not GUID_RE.match(i["resource_name"]):
        bad_shape.append(("guid", i["resource_name"], i["display_name"]))
    ent = by_alias.get(alias)
    if ent is None:
        missing.append(alias)
        continue
    if ent.get("guid") != i["resource_name"]:
        guid_mismatch.append((alias, ent.get("guid"), i["resource_name"]))

# Hard floor measured 2026-07-27: 333 Objects-eligible, 289 prop + 45 comp (incl. POC checkpoint).
if len(eligible) != 333:
    print(f"FAIL: Objects-eligible count {len(eligible)} != 333 (workbench census drift)")
    sys.exit(1)
if prop_n < 289:
    print(f"FAIL: prop: rows {prop_n} < 289")
    sys.exit(1)
if comp_n < 45:
    print(f"FAIL: comp: rows {comp_n} < 45")
    sys.exit(1)
if "comp:checkpoint_small" not in by_alias:
    print("FAIL: POC comp:checkpoint_small missing from mod registry")
    sys.exit(1)
if missing:
    print(f"FAIL: {len(missing)} Objects-eligible aliases missing from mod registry")
    print("  sample:", missing[:10])
    sys.exit(1)
if guid_mismatch:
    print(f"FAIL: {len(guid_mismatch)} alias guid mismatches")
    print("  sample:", guid_mismatch[:5])
    sys.exit(1)
if bad_shape:
    print(f"FAIL: {len(bad_shape)} schema-shape violations")
    print("  sample:", bad_shape[:5])
    sys.exit(1)

print(
    f"PASS: T-439 Objects aliases — eligible={len(eligible)} "
    f"prop={prop_n} comp={comp_n} missing=0 guid_mismatch=0"
)
PY
