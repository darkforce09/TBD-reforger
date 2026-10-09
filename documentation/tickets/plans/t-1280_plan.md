**Status:** live

# T-1280 — Plan

## Context

Open tickets cite repository paths that the restructure and the earlier tool, API and frontend
moves retired. The scripted rewrite of 2026-10-04 (restructure S12) mapped 738 citations through
the relocation manifests and left 362 unmapped. A second pass followed each unmapped path
through the history of `main` (the commit that deleted it, then that commit's rename detection):
60 resolved to a current path (a rename keeping the file name or at least 75% of the
content, a folder at least half of whose files moved into one current folder, a mod script found
by the class it declares, or a hand check), and 218 were deleted with no successor, each
recorded in a "Paths with no successor" sentence at the end of its ticket's `notes`.

The 84 citations below, in 65 tickets, stay open: a folder whose files spread over
several crates (most of them context roots such as "paths under" phrases), a file git traces only
as a split (its largest part named, below 75% similar), or a path never on `main`. Each needs a
reader who knows what the ticket meant, not a rename table.

## Approach

1. Work the table ticket by ticket. For a context root (a whole old crate or app folder), rewrite
   the ticket's relative paths one by one under their new crates and drop the phrase.
2. For a split file, read the ticket's claim and cite the current file that holds the cited code
   (find it with `git grep` on the symbol the ticket names), or record in `notes` that the code is
   gone.
3. For a path never on `main` (the T-939.4 worktree's unmerged commits), leave the citation and
   point at T-1002, which records that worktree.
4. Edit ticket files in the canonical form (`documentation/runbooks/ticket_run_pipeline.md`,
   "Editing a ticket file by hand"), delete each row from this table as its ticket is fixed, then
   `cargo xtask ticket sync` and `cargo xtask wave repack` when an `owns` value changed.

## Risks

- A split file's largest part may not hold the code the ticket cites; check the symbol, never the
  file name alone.
- A ticket may be stale as a whole (its ask already met); then ship or cancel it under the
  registry's rules instead of fixing its paths.

## Verification

- `cargo xtask ticket check --strict` and `cargo test -p ticket_model corpus_roundtrip_real_tree_byte_identical`
  stay green.
- `cargo xtask refactor relocate --verify` holds.
- The table below is empty.

## Open citations (84)

| Ticket | Path as the ticket cites it | Historical path | Why it is open |
|---|---|---|---|
| T-090.4 | tools/tbd-tools/src/world | same | 5 of 14 files moved, spread over tools/map_assets/world_export_pipeline/src/ |
| T-090.6 | tools/tbd-tools/src/world | same | 5 of 14 files moved, spread over tools/map_assets/world_export_pipeline/src/ |
| T-090.10.2 | crates/map-engine-render/src | same | 7 of 19 files moved, spread over crates/ |
| T-090.12.7 | packages/map-assets | same | 4630 of 10093 files moved, spread over assets/ |
| T-131 | editor/tools | apps/website/frontend/src/editor/tools | 9 of 11 files moved, spread over crates/ |
| T-132 | crates/map-engine-core/src/doc | same | 1 of 7 files moved, spread over crates/mission/mission_crdt/src/ |
| T-132 | pages/public | apps/website/frontend/src/pages/public | 1 of 11 files moved, spread over crates/frontend/pages/account_pages/src/settings/ |
| T-135 | apps/website/api/src/handlers/content/modpacks.rs | same | split; its largest part (73% similar) is crates/api/api_community_content/src/handlers/modpack_admin.rs |
| T-135 | handlers/content/modpacks.rs | apps/website/api_v2/src/handlers/content/modpacks.rs | split; its largest part (73% similar) is crates/api/api_community_content/src/handlers/modpack_admin.rs |
| T-136 | handlers/telemetry/deployments.rs | apps/website/api_v2/src/handlers/telemetry/deployments.rs | split; its largest part (55% similar) is crates/api/api_operations/src/handlers/member_service_record.rs |
| T-136 | pages/public | apps/website/frontend/src/pages/public | 1 of 11 files moved, spread over crates/frontend/pages/account_pages/src/settings/ |
| T-141 | editor/panels | apps/website/frontend/src/editor/panels | 19 of 22 files moved, spread over crates/frontend/workspaces/ |
| T-157 | editor/library | apps/website/frontend/src/editor/library | 1 of 4 files moved, spread over crates/frontend/pages/mission_hub_pages/src/create_dialog/ |
| T-157 | editor/library/create_dialog.rs | apps/website/frontend/src/editor/library/create_dialog.rs | split; its largest part (64% similar) is crates/frontend/pages/mission_hub_pages/src/create_dialog/dialog.rs |
| T-170 | scripts/deploy | same | 7 of 13 files moved, spread over deploy/ |
| T-205 | contract/generated | apps/website/api_v2/src/contract/generated | 2 of 6 files moved, spread over crates/ |
| T-212 | packages/tbd-schema | same | 89 of 116 files moved, spread over the root/ |
| T-294 | xtask/src/gate_export_terrain.rs | same | split; its largest part (67% similar) is tools/map_assets/world_export_pipeline/src/export_terrain_driver.rs |
| T-404 | apps/website/api/src/handlers/deployments.rs | same | split; its largest part (55% similar) is crates/api/api_operations/src/handlers/member_service_record.rs |
| T-404 | apps/website/api/src/handlers/telemetry/deployments.rs | same | split; its largest part (55% similar) is crates/api/api_operations/src/handlers/member_service_record.rs |
| T-674 | packages/tbd-schema | same | 89 of 116 files moved, spread over the root/ |
| T-675 | packages/tbd-schema | same | 89 of 116 files moved, spread over the root/ |
| T-708 | tools/tbd-tools/src/capture.rs | same | split; its largest part (72% similar) is tools/browser_testing/browser_gate_suites/src/screen_capture/eval_js.rs |
| T-709 | tools/tbd-tools/src/capture.rs | same | split; its largest part (72% similar) is tools/browser_testing/browser_gate_suites/src/screen_capture/eval_js.rs |
| T-710 | tools/tbd-tools/src/capture.rs | same | split; its largest part (72% similar) is tools/browser_testing/browser_gate_suites/src/screen_capture/eval_js.rs |
| T-829 | tools/tbd-tools | same | 88 of 112 files moved, spread over tools/ |
| T-907 | crates/map-engine-core | same | 28 of 106 files moved, spread over crates/ |
| T-907 | crates/map-engine-render | same | 7 of 20 files moved, spread over crates/ |
| T-908 | tools/tbd-tools | same | 88 of 112 files moved, spread over tools/ |
| T-908 | tools/tbd-tools/src | same | 29 of 52 files moved, spread over tools/ |
| T-909 | xtask/src/verify_ci_shell.rs | same | split; its largest part (68% similar) is tools/commands/ci_task_catalog/src/workflow_checks/workflow_shell.rs |
| T-935.20 | map_blueprint/library.rs | tools_v2/xtask/src/map_blueprint/library.rs | renamed to tools_v2/developer-tools/src/blueprint/library.rs in c3c75fc33, which is gone without a traced successor |
| T-935.20 | tools/tbd-tools/src/world/reclassify.rs | same | split; its largest part (62% similar) is tools/map_assets/world_export_pipeline/src/reclassify/obj_get.rs |
| T-935.20 | xtask/src/map_world_los.rs | same | split; its largest part (61% similar) is tools/map_assets/map_asset_verification/src/world_line_of_sight.rs |
| T-935.20 | xtask/src/node_free.rs | same | split; its largest part (55% similar) is tools/commands/schema_tooling/src/generate/font_table.rs |
| T-939.4 | tools/tbd-tools/src/vsuite | same | never on HEAD's history (another branch, or never committed) |
| T-939.4 | tools/tbd-tools/src/vsuite.rs | same | split; its largest part (60% similar) is tools/browser_testing/browser_gate_suites/src/dom_oracle/routes.rs |
| T-939.5 | apps/website/api | same | 111 of 187 files moved, spread over the root/ |
| T-940 | apps/website/api | same | 111 of 187 files moved, spread over the root/ |
| T-940.13 | apps/website/api/src/models/telemetry.rs | same | split; its largest part (68% similar) is crates/api/api_match_telemetry/src/models/match_record.rs |
| T-940.13 | handlers/telemetry/deployments.rs | apps/website/api_v2/src/handlers/telemetry/deployments.rs | split; its largest part (55% similar) is crates/api/api_operations/src/handlers/member_service_record.rs |
| T-940.13 | models/telemetry.rs | apps/website/api_v2/src/models/telemetry.rs | split; its largest part (68% similar) is crates/api/api_match_telemetry/src/models/match_record.rs |
| T-947 | xtask/src/map_world_los_tests.rs | same | renamed to tools_v2/xtask/src/map_world_los_tests.rs in c2c11b7b6, which is gone without a traced successor |
| T-959 | wave/gate.rs | tools_v2/xtask/src/wave/gate.rs | split; its largest part (59% similar) is tools/commands/platform_execution/src/wave_execution/gate/gate_dispatch.rs |
| T-969 | wave/gate.rs | tools_v2/xtask/src/wave/gate.rs | split; its largest part (59% similar) is tools/commands/platform_execution/src/wave_execution/gate/gate_dispatch.rs |
| T-970 | packages/map-assets | same | 4630 of 10093 files moved, spread over assets/ |
| T-977 | xtask/src/label_gates.rs | same | split; its largest part (60% similar) is tools/map_assets/map_asset_verification/src/labels.rs |
| T-1004 | legacy/graphics_engine/src | same | 57 of 70 files moved, spread over crates/graphics/ |
| T-1004 | legacy/map_engine/src | same | 675 of 985 files moved, spread over the root/ |
| T-1004 | overlay/symbology/instances/bridge_3.rs | legacy/map_engine/src/overlay/symbology/instances/bridge_3.rs | split; its largest part (50% similar) is crates/map_rendering/symbology_layers_gpu/src/slot_symbology/mission_lanes.rs |
| T-1004 | tools/ticket_engine/src | same | 113 of 141 files moved, spread over tools/tickets/ |
| T-1005 | apps/frontend/src/foundation | same | 170 of 190 files moved, spread over crates/frontend/foundation/ |
| T-1049 | legacy/map_engine/src | same | 675 of 985 files moved, spread over the root/ |
| T-1054 | data/store | apps/website/map-engine/src/data/store | 135 of 148 files moved, spread over crates/mission/ |
| T-1056 | legacy/map_engine/src/data/scenario/ast | same | 8 of 11 files moved, spread over crates/mission/ |
| T-1056 | legacy/map_engine/src/data/store | same | 135 of 148 files moved, spread over crates/mission/ |
| T-1066 | crates/map-engine-core | same | 28 of 106 files moved, spread over crates/ |
| T-1067 | io/archives/models | legacy/map_engine/src/io/archives/models | 2 of 5 files moved, spread over crates/world_formats/world_file_formats/src/archives/tests/ |
| T-1067 | io/containers/headers | legacy/map_engine/src/io/containers/headers | 1 of 5 files moved, spread over crates/world_formats/world_file_formats/src/containers/tests/ |
| T-1067 | legacy/map_engine/src | same | 675 of 985 files moved, spread over the root/ |
| T-1068 | legacy/map_engine/src | same | 675 of 985 files moved, spread over the root/ |
| T-1077 | legacy/map_engine/src/editing | same | 101 of 111 files moved, spread over crates/ |
| T-1078 | legacy/graphics_engine/src | same | 57 of 70 files moved, spread over crates/graphics/ |
| T-1090 | api_v2/src/handlers | apps/website/api_v2/src/handlers | 7 of 28 files moved, spread over crates/api/ |
| T-1094 | apps/mod/tbd-framework/Scripts/WorkbenchGame | same | 4 of 120 files moved, spread over mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/ |
| T-1102 | tools/tbd-tools | same | 88 of 112 files moved, spread over tools/ |
| T-1130 | data/scenario | apps/website/map-engine/src/data/scenario | 162 of 202 files moved, spread over the root/ |
| T-1137 | tools/ticket_engine | same | 118 of 150 files moved, spread over tools/tickets/ |
| T-1138 | tools/ticket_engine/src | same | 113 of 141 files moved, spread over tools/tickets/ |
| T-1139 | tools/ticket_engine/src | same | 113 of 141 files moved, spread over tools/tickets/ |
| T-1140 | tools/ticket_engine/src | same | 113 of 141 files moved, spread over tools/tickets/ |
| T-1142 | tools/ticket_engine/src | same | 113 of 141 files moved, spread over tools/tickets/ |
| T-1151 | apps/website | same | 4217 of 6033 files moved, spread over the root/ |
| T-1152 | apps/website | same | 4217 of 6033 files moved, spread over the root/ |
| T-1154 | apps/website | same | 4217 of 6033 files moved, spread over the root/ |
| T-1171 | apps/frontend/src/pages | same | 341 of 352 files moved, spread over crates/frontend/ |
| T-1180 | apps/website | same | 4217 of 6033 files moved, spread over the root/ |
| T-1182 | apps/api/src/core | same | 74 of 86 files moved, spread over the root/ |
| T-1185 | apps/mod/tbd-framework/Scripts/WorkbenchGame | same | 4 of 120 files moved, spread over mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/ |
| T-1196 | tools/ticket_engine/src | same | 113 of 141 files moved, spread over tools/tickets/ |
| T-1198 | tools/ticket_engine/src | same | 113 of 141 files moved, spread over tools/tickets/ |
| T-1201 | packages/map-assets | same | 4630 of 10093 files moved, spread over assets/ |
| T-1211 | legacy/map_engine/src | same | 675 of 985 files moved, spread over the root/ |
| T-1234 | apps/frontend/src/v2 | same | 1029 of 1099 files moved, spread over the root/ |
