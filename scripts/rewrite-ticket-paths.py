#!/usr/bin/env python3
"""Rewrite registry spec paths: Design_Docs/ -> docs/specs/."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

OLD = "Design_Docs/"
NEW = "docs/specs/"
EXPECTED = 23


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("registry", type=Path)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()

    data = json.loads(args.registry.read_text(encoding="utf-8"))
    count = 0
    for row in data.get("tickets", []):
        spec = row.get("spec")
        if not spec or OLD not in spec:
            continue
        count += 1
        if args.write:
            row["spec"] = spec.replace(OLD, NEW, 1)

    if count != EXPECTED:
        print(f"ERROR: expected {EXPECTED} spec rewrites, found {count}", file=sys.stderr)
        sys.exit(1)

    if args.write:
        args.registry.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
        print(f"Wrote {count} spec path rewrites to {args.registry}")
    else:
        print(f"Dry-run: would rewrite {count} spec paths")


if __name__ == "__main__":
    main()
