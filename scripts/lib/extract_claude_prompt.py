#!/usr/bin/env python3
"""Extract the first fenced Claude Code prompt from a t0xx spec markdown file."""

from __future__ import annotations

import re
import sys
from pathlib import Path


def extract_prompt(markdown: str) -> str:
    section = re.search(
        r"^##\s+Claude Code prompt[^\n]*\n+(?:[^\n]*\n)*?```(?:\w*\n)?(.*?)```",
        markdown,
        re.MULTILINE | re.DOTALL,
    )
    if section:
        return section.group(1).strip()

    idx = markdown.find("## Claude Code prompt")
    if idx == -1:
        raise ValueError("No '## Claude Code prompt' section found")
    rest = markdown[idx:]
    block = re.search(r"```(?:\w*\n)?(.*?)```", rest, re.DOTALL)
    if not block:
        raise ValueError("No fenced code block in Claude Code prompt section")
    return block.group(1).strip()


def main() -> None:
    if len(sys.argv) != 2:
        sys.stderr.write("Usage: extract_claude_prompt.py <spec.md>\n")
        sys.exit(1)
    path = Path(sys.argv[1])
    if not path.is_file():
        sys.stderr.write(f"Not found: {path}\n")
        sys.exit(1)
    text = path.read_text(encoding="utf-8")
    try:
        print(extract_prompt(text))
    except ValueError as e:
        sys.stderr.write(f"{e}\n")
        sys.exit(1)


if __name__ == "__main__":
    main()
