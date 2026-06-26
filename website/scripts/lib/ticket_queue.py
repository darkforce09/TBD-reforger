#!/usr/bin/env python3
"""Read/write ticket pipeline state via registry + generated queue.json."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

# Import sibling module
sys.path.insert(0, str(Path(__file__).resolve().parent))
from ticket_registry import (  # noqa: E402
    generate_queue_json,
    load_registry,
    repo_root,
    save_registry,
    ticket_by_id,
)


def queue_path() -> Path:
    return repo_root() / "tickets" / "queue.json"


def load_queue() -> dict[str, Any]:
    path = queue_path()
    if path.is_file():
        with path.open(encoding="utf-8") as f:
            return json.load(f)
    return generate_queue_json(load_registry())


def find_ticket(data: dict[str, Any], ticket_id: str) -> dict[str, Any] | None:
    for t in data.get("tickets", []):
        if t.get("id") == ticket_id:
            return t
    return None


def _sync_queue_from_registry() -> dict[str, Any]:
    data = generate_queue_json(load_registry())
    with queue_path().open("w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
        f.write("\n")
    return data


def cmd_list(_: argparse.Namespace) -> None:
    data = load_queue()
    print(f"batch_size={data.get('batch_size', 10)} concurrency={data.get('concurrency', 3)}")
    print(f"{'ID':<8} {'STATUS':<10} {'SPEC':<50} TITLE")
    print("-" * 100)
    for t in data.get("tickets", []):
        spec = (t.get("spec") or "")[:48]
        print(f"{t.get('id', ''):<8} {t.get('status', ''):<10} {spec:<50} {t.get('title', '')}")


def cmd_ready_ids(args: argparse.Namespace) -> None:
    data = load_queue()
    limit = args.limit or int(data.get("batch_size", 10))
    ids: list[str] = []
    for t in data.get("tickets", []):
        if t.get("status") != "ready":
            continue
        spec = (t.get("spec") or "").strip()
        if not spec:
            continue
        ids.append(t["id"])
        if len(ids) >= limit:
            break
    print("\n".join(ids))


def cmd_set_status(args: argparse.Namespace) -> None:
    registry = load_registry()
    t = ticket_by_id(registry, args.id)
    if not t:
        sys.stderr.write(f"Unknown ticket: {args.id}\n")
        sys.exit(1)
    t["status"] = args.status
    save_registry(registry)
    _sync_queue_from_registry()


def cmd_mark_ready(args: argparse.Namespace) -> None:
    from ticket_registry import cmd_mark_ready  # noqa: E402

    cmd_mark_ready(args)


def cmd_get(args: argparse.Namespace) -> None:
    registry = load_registry()
    t = ticket_by_id(registry, args.id)
    if not t:
        sys.stderr.write(f"Unknown ticket: {args.id}\n")
        sys.exit(1)
    if args.field:
        val = t.get(args.field, "")
        if args.field == "branch" and not val:
            val = f"ticket/{args.id}"
        print(val if val is not None else "")
    else:
        print(json.dumps(t, indent=2))


def cmd_config(args: argparse.Namespace) -> None:
    data = load_queue()
    key = args.key
    defaults = {
        "batch_size": "10",
        "concurrency": "3",
        "worktree_base": "../worktrees",
        "git_base": "main",
    }
    print(str(data.get(key, defaults.get(key, ""))))


def main() -> None:
    parser = argparse.ArgumentParser(description="Ticket queue helper")
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("list").set_defaults(func=cmd_list)

    p_ready = sub.add_parser("ready-ids")
    p_ready.add_argument("--limit", type=int, default=None)
    p_ready.set_defaults(func=cmd_ready_ids)

    p_status = sub.add_parser("set-status")
    p_status.add_argument("id")
    p_status.add_argument("status")
    p_status.set_defaults(func=cmd_set_status)

    p_mr = sub.add_parser("mark-ready")
    p_mr.add_argument("id")
    p_mr.add_argument("spec", nargs="?", default="")
    p_mr.set_defaults(func=cmd_mark_ready)

    p_get = sub.add_parser("get")
    p_get.add_argument("id")
    p_get.add_argument("field", nargs="?", default=None)
    p_get.set_defaults(func=cmd_get)

    p_cfg = sub.add_parser("config")
    p_cfg.add_argument("key")
    p_cfg.set_defaults(func=cmd_config)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
