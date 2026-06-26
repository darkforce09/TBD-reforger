#!/usr/bin/env python3
"""Rewrite Design_Docs/ -> docs/specs/ across git-tracked text files."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

OLD = "Design_Docs/"
NEW = "docs/specs/"
EXPECTED_FILES = 33
EXPECTED_REPLACEMENTS = 213

SKIP_SUFFIXES = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".et", ".rdb"}
SKIP_PARTS = {
    "docs/platform/MONOREPO_MIGRATION.md",
    "scripts/rewrite-doc-links.py",
    "scripts/verify-monorepo-migration.sh",
}


def git_ls_files(root: Path) -> list[str]:
    out = subprocess.check_output(["git", "-C", str(root), "ls-files"], text=True)
    return [line.strip() for line in out.splitlines() if line.strip()]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    root = args.root

    files_changed = 0
    replacements = 0
    for rel in git_ls_files(root):
        if rel in SKIP_PARTS:
            continue
        if any(rel.endswith(s) for s in SKIP_SUFFIXES):
            continue
        path = root / rel
        if not path.is_file():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        if OLD not in text:
            continue
        n = text.count(OLD)
        files_changed += 1
        replacements += n
        if args.write:
            path.write_text(text.replace(OLD, NEW), encoding="utf-8")

    if files_changed != EXPECTED_FILES or replacements != EXPECTED_REPLACEMENTS:
        print(
            f"ERROR: expected {EXPECTED_FILES} files / {EXPECTED_REPLACEMENTS} replacements, "
            f"got {files_changed} / {replacements}",
            file=sys.stderr,
        )
        sys.exit(1)

    mode = "Wrote" if args.write else "Dry-run"
    print(f"{mode}: {replacements} replacements in {files_changed} files")


if __name__ == "__main__":
    main()
