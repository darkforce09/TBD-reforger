#!/usr/bin/env python3
"""One-time M2 backfill: stream/targets/executor, T-068 retag, T-113+ milestones."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "tickets" / "registry.json"

PROGRAM_STREAM = {
    "eden": "mission-creator",
    "scale": "mission-creator",
    "platform": "web-platform",
    "backend": "web-platform",
    "infra": "infra",
}

PROGRAM_TARGETS = {
    "eden": ["website"],
    "scale": ["website"],
    "platform": ["website"],
    "backend": ["website"],
    "infra": ["root"],
}

T068_SLICE_PLAN = {
    "T-068.0": {"targets": ["shared", "root"], "executor": "cursor-docs"},
    "T-068.0a": {"targets": ["mod"], "executor": "workbench"},
    "T-068.1": {"targets": ["website"], "executor": "claude-code"},
    "T-068.2": {"targets": ["website"], "executor": "claude-code"},
    "T-068.3": {"targets": ["website"], "executor": "claude-code"},
    "T-068.4": {"targets": ["website"], "executor": "claude-code"},
    "T-068.5": {"targets": ["website"], "executor": "claude-code"},
    "T-068.6": {"targets": ["website"], "executor": "claude-code"},
}

MILESTONE_TICKETS = [
    {
        "id": "T-113",
        "order": 1130,
        "program": "infra",
        "stream": "infra",
        "targets": ["root"],
        "executor": "cursor-docs",
        "status": "shipped",
        "surfaces": ["DATA"],
        "impact": ["schema"],
        "title": "Monorepo migration (M0–M3)",
        "summary": "Merge website + mod into TBD-Reforger; unified ticketing; path rewrites.",
        "milestone": "M1",
    },
    {
        "id": "T-114",
        "order": 1140,
        "program": "platform",
        "stream": "mod-framework",
        "targets": ["mod"],
        "executor": "workbench",
        "status": "queued",
        "surfaces": ["DATA"],
        "impact": ["api", "schema"],
        "title": "Slot roster enforcement",
        "summary": "Roster identity maps to assigned slot; no round-robin fallback.",
        "milestone": "M1",
        "depends_on": [],
    },
    {
        "id": "T-115",
        "order": 1150,
        "program": "platform",
        "stream": "mod-framework",
        "targets": ["mod"],
        "executor": "workbench",
        "status": "queued",
        "surfaces": ["DATA"],
        "impact": ["schema"],
        "title": "Capture win condition",
        "summary": "Real side victory via capture / hold / elimination objective.",
        "milestone": "M1",
        "depends_on": [],
    },
    {
        "id": "T-116",
        "order": 1160,
        "program": "platform",
        "stream": "mod-framework",
        "targets": ["mod", "website"],
        "executor": "human",
        "status": "queued",
        "surfaces": ["DATA"],
        "impact": ["api"],
        "title": "Results POST to backend",
        "summary": "Game server posts match results; visible on event page.",
        "milestone": "M1",
        "depends_on": ["T-115"],
    },
    {
        "id": "T-117",
        "order": 1170,
        "program": "platform",
        "stream": "web-platform",
        "targets": ["website"],
        "executor": "claude-code",
        "status": "queued",
        "surfaces": ["DATA"],
        "impact": ["ui", "api"],
        "title": "Mission upload + validation UI",
        "summary": "Web UI for mission upload and schema validation (API exists).",
        "milestone": "M1",
    },
    {
        "id": "T-118",
        "order": 1180,
        "program": "platform",
        "stream": "web-platform",
        "targets": ["website"],
        "executor": "claude-code",
        "status": "queued",
        "surfaces": ["ORBAT", "DATA"],
        "impact": ["ui", "api"],
        "title": "Event ORBAT + identity linking UI",
        "summary": "Manual ORBAT assignment and Discord identity linking in web admin.",
        "milestone": "M1",
    },
    {
        "id": "T-119",
        "order": 1190,
        "program": "platform",
        "stream": "mod-framework",
        "targets": ["mod"],
        "executor": "workbench",
        "status": "queued",
        "surfaces": ["DATA"],
        "impact": ["schema"],
        "title": "Framework MVP remainder",
        "summary": "Loadouts, safe start, boundary, admin commands for M1 gate.",
        "milestone": "M1",
    },
    {
        "id": "T-120",
        "order": 1200,
        "program": "infra",
        "stream": "infra",
        "targets": ["mod", "website"],
        "executor": "human",
        "status": "queued",
        "surfaces": ["DATA"],
        "impact": ["api"],
        "title": "Staging soak + golden mission smoke",
        "summary": "Pinned game/mod version soak; golden-mission smoke on staging server.",
        "milestone": "M1",
        "depends_on": ["T-114", "T-115", "T-117"],
    },
]


def backfill_row(row: dict) -> None:
    program = row.get("program", "platform")
    if "stream" not in row:
        row["stream"] = PROGRAM_STREAM.get(program, "web-platform")
    if "targets" not in row:
        row["targets"] = list(PROGRAM_TARGETS.get(program, ["website"]))
    if "executor" not in row:
        row["executor"] = "claude-code" if program in ("eden", "scale", "backend") else "cursor-docs"


def main() -> None:
    data = json.loads(REGISTRY.read_text(encoding="utf-8"))
    existing = {t["id"] for t in data["tickets"]}

    for row in data["tickets"]:
        backfill_row(row)
        if row["id"] == "T-068":
            row["stream"] = "mission-creator"
            row["targets"] = ["website", "shared", "root", "mod"]
            row["executor"] = "claude-code"
            row["slices"] = list(T068_SLICE_PLAN.keys())
            row["active_slice"] = "T-068.0"
            row["slice_plan"] = T068_SLICE_PLAN
            row["title"] = "Asset registry + Loadout Forge"
            row["summary"] = "Cross-cutting registry ingest, API, worker, palette, forge, compiler."

    for ticket in MILESTONE_TICKETS:
        if ticket["id"] not in existing:
            backfill_row(ticket)
            data["tickets"].append(ticket)

    data["next_id"] = 121
    REGISTRY.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"Backfill complete: {len(data['tickets'])} tickets, next_id={data['next_id']}")


if __name__ == "__main__":
    main()
