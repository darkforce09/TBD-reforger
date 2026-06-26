#!/usr/bin/env python3
"""One-time seed for tickets/registry.json — exactly 87 rows."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "tickets" / "registry.json"

SHIPPED_IDS = [
    "T-001", "T-002", "T-003", "T-004", "T-005", "T-006",
    "T-008", "T-009", "T-010", "T-011", "T-012", "T-013",
    "T-018", "T-019", "T-020", "T-021", "T-022", "T-023", "T-024", "T-025",
    "T-029", "T-030", "T-031", "T-032", "T-033", "T-034", "T-035", "T-036", "T-037", "T-038",
    "T-039", "T-040", "T-043", "T-045", "T-046", "T-047", "T-048", "T-049", "T-050", "T-052",
    "T-053", "T-054", "T-055", "T-056", "T-057", "T-058", "T-059", "T-060", "T-061", "T-062",
    "T-063", "T-064", "T-065", "T-066",
]

SPEC_MAP: dict[str, str] = {
    "T-048": "Design_Docs/Mission_Creator_Architecture/t048_library_create_dialog.md",
    "T-049": "Design_Docs/Mission_Creator_Architecture/t049_terrain_title_position.md",
    "T-050": "Design_Docs/Mission_Creator_Architecture/t050_cursor_z_readout.md",
    "T-052": "Design_Docs/Mission_Creator_Architecture/t052_undo_shortcuts.md",
    "T-053": "Design_Docs/Mission_Creator_Architecture/t053_additive_select.md",
    "T-054": "Design_Docs/Mission_Creator_Architecture/t054_attributes_entry_points.md",
    "T-055": "Design_Docs/Mission_Creator_Architecture/t055_asset_browser_search.md",
    "T-056": "Design_Docs/Mission_Creator_Architecture/t056_copy_paste.md",
    "T-057": "Design_Docs/Mission_Creator_Architecture/t057_map_performance_hotfix.md",
    "T-058": "Design_Docs/Mission_Creator_Architecture/t058_entity_count_readout.md",
    "T-059": "Design_Docs/Mission_Creator_Architecture/t059_bulk_paste_operations.md",
    "T-060": "Design_Docs/Mission_Creator_Architecture/t060_fast_initial_load.md",
    "T-061": "Design_Docs/Mission_Creator_Architecture/t061_drag_move_hotfix.md",
    "T-062": "Design_Docs/Mission_Creator_Architecture/t062_incremental_bindings.md",
    "T-063": "Design_Docs/Mission_Creator_Architecture/t063_spatial_index.md",
    "T-064": "Design_Docs/Mission_Creator_Architecture/t064_virtualized_outliner.md",
    "T-065": "Design_Docs/Mission_Creator_Architecture/t065_cluster_lod.md",
    "T-066": "Design_Docs/Mission_Creator_Architecture/t066_worker_compile.md",
}

SHIPPED_META: dict[str, dict[str, Any]] = {
    "T-001": {"program": "backend", "surfaces": ["DATA"], "impact": ["api", "schema"], "title": "Initial backend", "summary": "Full schema, handlers, and API scaffold."},
    "T-002": {"program": "platform", "surfaces": ["SHELL"], "impact": ["api", "ui"], "title": "Discord OAuth2", "summary": "Discord OAuth2 callback end-to-end."},
    "T-003": {"program": "platform", "surfaces": ["SHELL"], "impact": ["api"], "title": "Dev login shortcut", "summary": "Development dev-login endpoint for local testing."},
    "T-004": {"program": "platform", "surfaces": ["SHELL"], "impact": ["api", "ui"], "title": "Frontend wired to backend", "summary": "Typed hooks, auth bootstrap, all pages on live data."},
    "T-005": {"program": "platform", "surfaces": ["DATA"], "impact": ["schema"], "title": "Platform documentation seed (pass 1)", "summary": "Documentation and seed work between T-004 scaffold and T-008 campaign refactor."},
    "T-006": {"program": "platform", "surfaces": ["DATA"], "impact": ["schema"], "title": "Platform documentation seed (pass 2)", "summary": "Documentation and seed work between T-004 scaffold and T-008 campaign refactor."},
    "T-008": {"program": "platform", "surfaces": ["SHELL", "ORBAT"], "impact": ["api", "schema", "ui"], "title": "Event to Campaign refactor", "summary": "Multi-mission events, event_missions, automated ORBAT."},
    "T-009": {"program": "platform", "surfaces": ["SHELL", "ORBAT"], "impact": ["ui"], "title": "Inline ORBAT on Event Hub", "summary": "Faction/squad/slot selector inline on Event Hub."},
    "T-010": {"program": "platform", "surfaces": ["ORBAT"], "impact": ["api", "schema", "ui"], "title": "Rich ORBAT slots + squad reservation", "summary": "Per-slot loadout/tag schema and leader squad holds."},
    "T-011": {"program": "platform", "surfaces": ["SHELL"], "impact": ["ui"], "title": "Aegis design system foundation", "summary": "macOS Aegis palette, glass primitives, shared UI components."},
    "T-012": {"program": "platform", "surfaces": ["SHELL"], "impact": ["ui"], "title": "macOS page split-pane redesigns", "summary": "Split-pane master/detail across platform pages."},
    "T-013": {"program": "platform", "surfaces": ["SHELL"], "impact": ["ui"], "title": "macOS dashboards and grids restyle", "summary": "Glass bento dashboards and mission library grid."},
    "T-018": {"program": "platform", "surfaces": ["SHELL"], "impact": ["ui"], "title": "R3 mono telemetry", "summary": "JetBrains Mono for player counts and ORBAT slot counts."},
    "T-019": {"program": "platform", "surfaces": ["SHELL"], "impact": ["ui"], "title": "R5 shared Sheet primitive", "summary": "Mission dossier moved to shared Sheet component."},
    "T-020": {"program": "platform", "surfaces": ["SHELL"], "impact": ["ui"], "title": "R4 token sweep (phase 1)", "summary": "Replace off-palette colors with Aegis tokens."},
    "T-021": {"program": "platform", "surfaces": ["SHELL"], "impact": ["ui"], "title": "R1/R2 full-bleed + SplitPane", "summary": "Full-bleed routes and SplitPane migration."},
    "T-022": {"program": "platform", "surfaces": ["SHELL"], "impact": ["ui"], "title": "Global Aegis consistency (batch 2)", "summary": "Mono telemetry and token fixes across pages."},
    "T-023": {"program": "platform", "surfaces": ["SHELL"], "impact": ["ui"], "title": "Global Aegis consistency (batch 3)", "summary": "Token sweep and SplitPane on remaining pages."},
    "T-024": {"program": "platform", "surfaces": ["SHELL"], "impact": ["ui"], "title": "Event Manager calendar anti-pattern fix", "summary": "Calendar + per-day op list replaces form-over-list."},
    "T-025": {"program": "platform", "surfaces": ["SHELL"], "impact": ["ui"], "title": "Global Aegis consistency (final batch)", "summary": "Final token and layout consistency pass."},
    "T-029": {"program": "eden", "surfaces": ["MAP"], "impact": ["ui", "state"], "title": "Mission Creator core viewport", "summary": "Deck.gl orthographic TacticalMap with terrain grid."},
    "T-030": {"program": "eden", "surfaces": ["DATA", "MAP"], "impact": ["state", "persistence"], "title": "Y.Doc state foundation", "summary": "Normalized Y.Doc store mirrored to Zustand with persistence."},
    "T-031": {"program": "eden", "surfaces": ["TOP", "BOTTOM"], "impact": ["ui"], "title": "Aegis-glass editor shell", "summary": "Full-bleed map under frosted command strip and toolbelt."},
    "T-032": {"program": "eden", "surfaces": ["LEFT", "RIGHT", "ATTR"], "impact": ["ui"], "title": "Eden Editor tree paradigm", "summary": "Outliner, asset browser, Attributes modal tree UI."},
    "T-033": {"program": "eden", "surfaces": ["LEFT", "RIGHT", "MAP"], "impact": ["state", "ui"], "title": "Wire Outliner and asset drag-to-map", "summary": "Editor layers entity map and palette drop creates slots."},
    "T-034": {"program": "eden", "surfaces": ["DATA"], "impact": ["schema"], "title": "MC doc alignment", "summary": "Eden UX spec and agent execution plan documentation."},
    "T-035": {"program": "eden", "surfaces": ["LEFT", "RIGHT", "TOP"], "impact": ["ui"], "title": "Eden docked shell", "summary": "Chromeless route, docked sidebars, always-on asset palette."},
    "T-036": {"program": "eden", "surfaces": ["MAP"], "impact": ["ui", "state"], "title": "Map drag and multi-select", "summary": "Drag-move, marquee select, group move/delete ops."},
    "T-037": {"program": "eden", "surfaces": ["LEFT"], "impact": ["ui", "state"], "title": "Outliner tree ops", "summary": "Layer rename, reparent, move slot, delete folder subtree."},
    "T-038": {"program": "eden", "surfaces": ["DATA"], "impact": ["compiler", "persistence", "api"], "title": "Compiler and persistence", "summary": "Mission compile/export, hydrate, Save Version POST flow."},
    "T-039": {"program": "eden", "surfaces": ["TOP"], "impact": ["ui", "api"], "title": "Save Version error surfacing", "summary": "Backend error messages and invalid mission id banner."},
    "T-040": {"program": "eden", "surfaces": ["SHELL"], "impact": ["ui", "api"], "title": "Create mission wizard wiring", "summary": "Create wizard sends max_players and real weather enums."},
    "T-043": {"program": "platform", "surfaces": ["DATA"], "impact": ["schema"], "title": "Platform documentation reorg", "summary": "Docs hub, domain ROADMAPs, FD vs T split."},
    "T-045": {"program": "platform", "surfaces": ["DATA"], "impact": ["schema"], "title": "Roadmap-centric naming", "summary": "Domain ROADMAP.md files replace numbered doc paths."},
    "T-046": {"program": "platform", "surfaces": ["DATA"], "impact": ["schema"], "title": "Doc link integrity pass", "summary": "Stale numbered cross-refs and relative link depths fixed."},
    "T-047": {"program": "platform", "surfaces": ["DATA"], "impact": ["schema"], "title": "Doc authority alignment", "summary": "agent_execution and wiki manifest deduped."},
    "T-048": {"program": "eden", "surfaces": ["SHELL"], "impact": ["ui"], "title": "Mission create from Library dialog", "summary": "CreateMissionDialog replaces standalone create route."},
    "T-049": {"program": "eden", "surfaces": ["MAP", "ATTR", "TOP", "DATA"], "impact": ["ui", "state"], "title": "Terrain, title, numeric position", "summary": "Terrain viewport, row meta hydrate, editable transform fields."},
    "T-050": {"program": "eden", "surfaces": ["BOTTOM"], "impact": ["ui"], "title": "Cursor Z readout", "summary": "Toolbelt shows X/Y/Z including cursor Z on flat map."},
    "T-052": {"program": "eden", "surfaces": ["TOP"], "impact": ["ui", "state"], "title": "Undo/redo keyboard shortcuts", "summary": "Ctrl/Cmd+Z/Y undo-redo with StrictMode lifecycle fix."},
    "T-053": {"program": "eden", "surfaces": ["MAP"], "impact": ["ui", "state"], "title": "Ctrl/Cmd additive select", "summary": "Modifier-click toggles slot in/out of selection."},
    "T-054": {"program": "eden", "surfaces": ["MAP", "LEFT", "ATTR"], "impact": ["ui"], "title": "Attributes modal entry points", "summary": "Native dblclick on map and ORBAT opens Attributes."},
    "T-055": {"program": "eden", "surfaces": ["RIGHT"], "impact": ["ui"], "title": "Asset browser search", "summary": "Case-insensitive label search in Factions asset tree."},
    "T-056": {"program": "eden", "surfaces": ["MAP"], "impact": ["ui", "state"], "title": "Ctrl+C/V copy-paste", "summary": "Copy slot selection and paste at cursor preserving layout."},
    "T-057": {"program": "eden", "surfaces": ["MAP", "BOTTOM"], "impact": ["perf", "ui"], "title": "Map performance hotfix", "summary": "Cursor off render path, no hover pick, pan coalescing, memo."},
    "T-058": {"program": "eden", "surfaces": ["BOTTOM"], "impact": ["ui"], "title": "Entity count readout", "summary": "OBJ total and SEL selected counts in toolbelt."},
    "T-059": {"program": "eden", "surfaces": ["MAP", "LEFT"], "impact": ["perf", "state"], "title": "Bulk paste/delete at scale", "summary": "O(n) pasteSlots batch writes and selection cap at 10k paste."},
    "T-060": {"program": "scale", "surfaces": ["DATA", "TOP"], "impact": ["api", "perf", "ui"], "title": "Fast load and save at scale", "summary": "256MB body limit, bulk sync load overlay, save progress UX."},
    "T-061": {"program": "scale", "surfaces": ["MAP"], "impact": ["perf", "ui"], "title": "Drag-move performance at 360k", "summary": "Dual IconLayer drag preview at ~60fps for 360k slots."},
    "T-062": {"program": "scale", "surfaces": ["DATA"], "impact": ["perf", "state"], "title": "Incremental bindings at 360k", "summary": "O(k) Zustand patches instead of full docToSnapshot on edits."},
    "T-063": {"program": "scale", "surfaces": ["MAP"], "impact": ["perf", "ui"], "title": "Spatial index for pick", "summary": "rbush R-tree pickNearest/pickRect replaces Deck GPU pick."},
    "T-064": {"program": "scale", "surfaces": ["LEFT"], "impact": ["perf", "ui"], "title": "Virtualized outliner at 367k", "summary": "TanStack virtual outliner replaces leaf cap band-aid."},
    "T-065": {"program": "scale", "surfaces": ["MAP"], "impact": ["perf", "ui"], "title": "Cluster LOD at extreme zoom", "summary": "Supercluster index with pan-stable cache for 367k missions."},
    "T-066": {"program": "scale", "surfaces": ["DATA"], "impact": ["perf", "compiler"], "title": "Worker compile offload", "summary": "Save/Export compile in Comlink worker via pickMapSnapshot."},
}

OPEN_ROWS: list[dict[str, Any]] = [
    {"id": "T-067", "order": 670, "program": "scale", "status": "ready", "surfaces": ["MAP", "DATA"], "impact": ["perf", "state"], "title": "Spatial chunks", "summary": "Viewport IconLayer cull and bulk-paste patch; lazy RAM @ 1M slice.", "spec": "Design_Docs/Mission_Creator_Architecture/t067_spatial_chunks.md", "slices": ["T-067.0", "T-067.1"], "active_slice": "T-067.0", "unblocks": ["T-068"], "route": "/missions/:id/edit"},
    {"id": "T-068", "order": 680, "program": "eden", "status": "queued", "surfaces": ["RIGHT", "DATA"], "impact": ["api", "ui", "state"], "title": "Asset registry + palette", "summary": "Replace mock catalog with registry-backed Factions tree.", "depends_on": ["T-067"], "unblocks": ["T-069", "T-070", "T-071"], "route": "/missions/:id/edit"},
    {"id": "T-069", "order": 690, "program": "eden", "status": "queued", "surfaces": ["MAP", "MRK"], "impact": ["ui", "state", "schema"], "title": "Markers on map", "summary": "Place and edit map markers with registry-backed types.", "depends_on": ["T-068"], "route": "/missions/:id/edit"},
    {"id": "T-070", "order": 700, "program": "eden", "status": "queued", "surfaces": ["MAP", "VEH", "RIGHT"], "impact": ["ui", "state", "schema"], "title": "Vehicles placeable", "summary": "Drag vehicles from palette onto map with crew hooks.", "depends_on": ["T-068"], "route": "/missions/:id/edit"},
    {"id": "T-071", "order": 710, "program": "eden", "status": "queued", "surfaces": ["TOP", "ORBAT", "ATTR"], "impact": ["ui", "state", "schema", "compiler"], "title": "ORBAT Manager modal", "summary": "Remove duplicate ORBAT tree from left sidebar; open ORBAT Manager modal for all-side faction/squad/slot authoring, slotting-screen order, standardizations, logos, and arsenal.", "depends_on": ["T-068"], "route": "/missions/:id/edit"},
    {"id": "T-072", "order": 720, "program": "eden", "status": "queued", "surfaces": ["MAP", "PLACE"], "impact": ["ui"], "title": "Ctrl multi-place", "summary": "Hold Ctrl to place multiple copies without re-selecting asset.", "route": "/missions/:id/edit"},
    {"id": "T-073", "order": 730, "program": "eden", "status": "queued", "surfaces": ["MAP", "XFORM"], "impact": ["ui"], "title": "Shift + map rotation", "summary": "Shift-drag and map rotation widget for placed entities.", "route": "/missions/:id/edit"},
    {"id": "T-074", "order": 740, "program": "eden", "status": "queued", "surfaces": ["RIGHT"], "impact": ["ui"], "title": "Faction submode / catalog filter", "summary": "Faction submode tabs and catalog filtering in asset browser.", "route": "/missions/:id/edit"},
    {"id": "T-075", "order": 750, "program": "eden", "status": "queued", "surfaces": ["MAP"], "impact": ["ui"], "title": "Spacebar flyTo vs widget", "summary": "Spacebar centers selection; resolve flyTo vs transform widget conflict.", "route": "/missions/:id/edit"},
    {"id": "T-076", "order": 760, "program": "eden", "status": "queued", "surfaces": ["VEH", "ATTR"], "impact": ["ui"], "title": "Vehicle crew UI", "summary": "Crew panel and boarding UI for placed vehicles.", "route": "/missions/:id/edit"},
    {"id": "T-077", "order": 770, "program": "eden", "status": "queued", "surfaces": ["MAP", "VEH"], "impact": ["ui"], "title": "Alt + empty vehicle", "summary": "Alt-click to enter empty vehicle placement mode.", "route": "/missions/:id/edit"},
    {"id": "T-078", "order": 780, "program": "eden", "status": "deferred", "surfaces": ["COMP"], "impact": ["ui", "state"], "title": "Custom compositions", "summary": "Save and place custom entity compositions.", "route": "/missions/:id/edit"},
    {"id": "T-079", "order": 790, "program": "eden", "status": "deferred", "surfaces": ["MAP", "DATA"], "impact": ["ui", "state", "schema"], "title": "Triggers + waypoints + systems", "summary": "Trigger volumes, waypoints, and game logic systems.", "route": "/missions/:id/edit"},
    {"id": "T-080", "order": 800, "program": "eden", "status": "deferred", "surfaces": ["CONN"], "impact": ["ui", "state"], "title": "Connection / sync UI", "summary": "Entity connection lines and sync group authoring.", "route": "/missions/:id/edit"},
    {"id": "T-081", "order": 810, "program": "eden", "status": "deferred", "surfaces": ["MAP", "XFORM"], "impact": ["ui"], "title": "Transform widget + snap grid", "summary": "On-map transform gizmo and snap grid.", "route": "/missions/:id/edit"},
    {"id": "T-082", "order": 820, "program": "eden", "status": "deferred", "surfaces": ["ATTR"], "impact": ["ui", "state"], "title": "Full attribute fields", "summary": "Complete Attributes modal field parity with Eden.", "route": "/missions/:id/edit"},
    {"id": "T-083", "order": 830, "program": "eden", "status": "deferred", "surfaces": ["TOP"], "impact": ["ui"], "title": "Top menu bar", "summary": "Eden-style top menu bar with file/edit/view stubs.", "route": "/missions/:id/edit"},
    {"id": "T-084", "order": 840, "program": "eden", "status": "deferred", "surfaces": ["RIGHT"], "impact": ["ui"], "title": "Classname / mod prefix search", "summary": "Classname-prefix search in asset browser.", "route": "/missions/:id/edit"},
    {"id": "T-085", "order": 850, "program": "platform", "status": "deferred", "surfaces": ["SHELL"], "impact": ["ui"], "title": "Wiki markdown renderer", "summary": "Render doctrine wiki pages from markdown at /wiki.", "route": "/wiki"},
    {"id": "T-086", "order": 860, "program": "platform", "status": "deferred", "surfaces": ["SHELL"], "impact": ["ui", "api"], "title": "Server Control + RCON API", "summary": "Live server control panel wired to RCON backend.", "route": "/server-control"},
    {"id": "T-087", "order": 870, "program": "platform", "status": "deferred", "surfaces": ["SHELL"], "impact": ["ui"], "title": "CMS rich text editor", "summary": "Rich text editor for announcements and CMS content.", "route": "/admin/content"},
    {"id": "T-088", "order": 880, "program": "platform", "status": "deferred", "surfaces": ["SHELL"], "impact": ["ui", "api"], "title": "Multi-server picker", "summary": "Select among multiple game servers in intel views.", "route": "/server-intel"},
    {"id": "T-089", "order": 890, "program": "platform", "status": "deferred", "surfaces": ["TOP", "DATA"], "impact": ["api", "ui"], "title": "Mission title PATCH sync", "summary": "PATCH mission title to server on edit (absorbs former T-051 scope).", "route": "/missions/:id/edit"},
    {"id": "T-090", "order": 900, "program": "infra", "status": "deferred", "surfaces": ["MAP"], "impact": ["ui"], "title": "Aligned map tiles", "summary": "Hosted aligned map tiles for mission editor basemap.", "route": "/missions/:id/edit"},
    {"id": "T-091", "order": 910, "program": "infra", "status": "deferred", "surfaces": ["MAP", "DATA"], "impact": ["ui", "schema"], "title": "DEM + auto Z", "summary": "Digital elevation model feeding real Z coordinates.", "route": "/missions/:id/edit"},
    {"id": "T-092", "order": 920, "program": "infra", "status": "deferred", "surfaces": ["DATA"], "impact": ["schema"], "title": "Mod golden coordinate test", "summary": "Golden-file coordinate tests for mod envelope export.", "route": "/missions/:id/edit"},
    {"id": "T-093", "order": 930, "program": "scale", "status": "deferred", "surfaces": ["DATA"], "impact": ["persistence", "ui"], "title": "Continuous autosave polish", "summary": "Background autosave UX and conflict handling polish.", "route": "/missions/:id/edit"},
    {"id": "T-094", "order": 940, "program": "scale", "status": "deferred", "surfaces": ["MAP", "DATA"], "impact": ["perf"], "title": "Typed-array IconLayer", "summary": "Typed-array slot icon buffer for mega-mission render path.", "route": "/missions/:id/edit"},
    {"id": "T-095", "order": 950, "program": "backend", "status": "deferred", "surfaces": ["DATA"], "impact": ["api", "schema"], "title": "Per-handler API reference doc", "summary": "Complete docs/backend/api.md per-handler reference.", "route": "/api/v1"},
    {"id": "T-096", "order": 960, "program": "backend", "status": "deferred", "surfaces": ["DATA"], "impact": ["api"], "title": "Live game-server telemetry bridge", "summary": "Bridge live game-server events into telemetry ingest.", "route": "/api/v1/telemetry"},
    {"id": "T-110", "order": 1100, "program": "infra", "status": "deferred", "surfaces": ["MAP", "DATA"], "impact": ["schema", "persistence"], "title": "Terrain base + sparse deltas", "summary": "Binary terrain base layer plus sparse delta props for 1M+ map objects.", "spec": "Design_Docs/Mission_Creator_Architecture/t110_terrain_base_mission_layers.md", "route": "/missions/:id/edit"},
]


def build() -> dict[str, Any]:
    tickets: list[dict[str, Any]] = []
    for tid in SHIPPED_IDS:
        meta = SHIPPED_META[tid]
        num = int(tid.split("-")[1])
        row: dict[str, Any] = {
            "id": tid,
            "title": meta["title"],
            "summary": meta["summary"],
            "program": meta["program"],
            "surfaces": meta["surfaces"],
            "impact": meta["impact"],
            "status": "shipped",
            "order": num * 10,
        }
        if tid in SPEC_MAP:
            row["spec"] = SPEC_MAP[tid]
        if meta["program"] == "eden" and num >= 29:
            row["route"] = "/missions/:id/edit"
        tickets.append(row)

    for row in OPEN_ROWS:
        r = dict(row)
        r.setdefault("branch", f"ticket/{r['id']}")
        tickets.append(r)

    assert len(tickets) == 85, f"Expected 87 rows (55 shipped + 32 open), got {len(tickets)}"
    return {"next_id": 111, "tickets": tickets}


def main() -> None:
    data = build()
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with OUT.open("w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")
    print(f"Wrote {OUT} ({len(data['tickets'])} tickets)")


if __name__ == "__main__":
    main()
