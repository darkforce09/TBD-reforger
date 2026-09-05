# Wave 1 planning agents — shared brief (read fully before touching anything)

Repo: /run/media/system/Disk_2/Projects/TBD-Reforger (branch main). You work DIRECTLY on main, but you may
ONLY create/edit files under `.ai/tickets/` and `docs/`. Never touch apps/, crates/, packages/, tools/,
xtask/. Never run `cargo xtask ticket mark-ready`, `wave repack`, `ticket sync`, `set-status`, git commit,
git add. The command center does those after you report. Read-only cargo commands are fine
(`cargo xtask ticket check`, `ticket show <id>`, `ticket list`).
Shell: `export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target` first. Two sibling agents are writing
other ticket ids at the same time — stay inside your id range / file list.

## Operator decisions (quote verbatim in tickets where a deferral or authorization is needed)
- Scope: TBD-Reforger only.
- "agents may edit the Enfusion mod scripts; gate = `cargo xtask mod compile`; in-game behaviour goes on a
  human checklist" → mod `.c` paths go in `owns` with `executor = "claude-code"`.
- Explicitly deferred by the operator (the ONLY legal deferrals): T-137 Discord platform rework;
  the RadioManagerEntity Workbench world edit (radio NO_BACKBONE); the in-editor "Play scenario" dry run.
- Tier-2 serialization = rkyv. flatbuffers rejected.
- No other deferrals, no "out of scope", no "follow-up later" (.cursor/rules/no-silent-deferrals.mdc).

## Ticket TOML (one file per ticket: .ai/tickets/T-<id>.toml). Copy the shape of an existing modern
ticket: read `.ai/tickets/T-934.toml` (program) and `.ai/tickets/T-934.3.toml` (work slice) first.
Required: id, title (≤10 words), summary (≤40 words), kind (program|work), status.
Ready-tier work tickets ALSO need nonempty: context[], requirement[], current_state[], approach[],
verify[] (commands), acceptance[] (outcomes, never command-shaped), main_goal, order (integer),
priority (0-3), class (bug|feature|chore|audit|docs), executor ("claude-code"), spec (path on disk),
plan (path on disk: docs/plans/<id lowercased, dots→underscores>_plan.md, e.g. T-935.3 → docs/plans/t-935_3_plan.md),
owns (exact file paths — never a bare directory; new files allowed, name them exactly), [scope] table
{domain = website|mod|schema|engine|repo, layer = ..., component = ...}, depends_on (ids that must be
shipped first — every listed dep MUST currently be shipped or cancelled, or be another ticket in THIS
program that packs earlier; check with `grep '^status' .ai/tickets/T-xxx.toml`), created_at RFC-3339 UTC.
Programs: kind = "program", children = [...], status = "queued". Slices: kind = "work", parent = "<program id>",
status = "queued" (command center promotes to ready). Body lines ≤ 30 words each.
Owns MUST be file-disjoint between slices that should run in the same wave; two slices that both need the
same file simply pack into different waves — that is fine, say so in approach.
Mod scripts: `apps/mod/tbd-framework/Scripts/...` exact paths. Files that will be NEW: still list them.

## Plan document: copy docs/plans/TEMPLATE.md → docs/plans/<id>_plan.md. Four sections: Context, Approach
(ordered steps naming files), Risks (+fallback), Verification (commands mirroring verify[]). Honest, specific.

## Spec document (one per program, or reuse an existing spec): docs/specs/<area>/<slug>.md containing a
fenced block `## Claude Code prompt — <slice id>` per slice, sections in this order:
PREFLIGHT · READ · PROBLEM (≤4 sentences) · SHIPPED · LANGUAGE GATE · LOCKED (≤8 bullets) · DO (3–12 steps)
· DO NOT · VERIFY · MANUAL · RETURN (ends "Ready for Cursor doc sync."). See .ai/tickets/CLAUDE_CODE_PROMPT.md.

## Slice-agent rules the tickets must encode (put in LOCKED/DO NOT):
verify defect on main before coding; perturbation proof (red pasted verbatim, `touch` after restore);
`cargo test -p map-engine-core --all-features` (never without the flag); no `git add -A`, no `git stash`,
no `cargo xtask ci ci-local`; `skip:` = FAIL; no .py/.sh/.mjs committed; file-length allowlists never
extended (new code → new files; `residency.rs`, `los_tool.rs`, `building_viewer.rs`, `mission_editor.rs`
are allowlisted SIZE-3 files); Class-R byte-parity tests scrub their own source; edition-2024 rustfmt
for tools/* and map-engine-render; agents never merge/push/change status; report schema:
pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns ·
found_not_fixed · deviations · commits.
Gates: slice `cargo xtask platform wave gate --slice T-xxx`; mod `cargo xtask mod compile`; editor
`cargo xtask mk leptos-gates`; schema `cargo xtask ci schema-validate` + `verify map-object-golden|
type-inventory|map-object-enums|terrain-manifest`; `verify blas-manifest`; `verify file-length`.

## Verified code facts (2026-09-04) — cite these anchors, do not re-derive from stale docs
- Ticket tooling: `cargo xtask ticket ...` (scripts/ticket and registry.json are GONE). Worktrees:
  `cargo xtask platform slice-worktree -- new T-xxx` → .ai/artifacts/worktrees/T-xxx, branch slice/T-xxx.
- Map loaders: apps/website/frontend/src/editor/world_assets/{world_host.rs (chunks :426, prefabs :133,
  roads :146, regions :156, manifest :99), forest_mass.rs:108 (TBDD .bin), mod.rs:619 (DEM PNG via
  fetch_bytes_streamed), satellite.rs (TBDS .tbd-sat via HTTP Range), labels.rs:70/76, occluder_host.rs
  :52/107/134 (blas-manifest.json, descriptors/*.json, *.bvh), fetch.rs (fetch_bytes/_streamed/_text/
  _range_outcome)}. Core parsers: crates/map-engine-core/src/world/{residency.rs:139 WorldResidency
  (3137 lines, allowlisted), chunk.rs:20 WorldChunk SoA (positions, prefab_idx u16, rotations, z, pitch,
  roll, scale, cls_codes u8, rows_by_class), store.rs:50/110/120, roads.rs:28, regions.rs:10, prefab.rs
  :15/33, manifest.rs:11/24, locations.rs:12, road_labels.rs:66, index.rs:35}, geometry/tbdd.rs:51
  decode_tbdd (byte loop), dem/png_decode.rs:58-79, bvh_sidecar.rs:219, world/occluder/descriptor.rs:52.
  GPU Pod structs only in crates/map-engine-render/src/scene.rs:32/47/72. rkyv: nowhere. bytemuck: only
  map-engine-render/Cargo.toml:21. flate2 + serde_json(float_roundtrip) are `world`-feature deps of
  map-engine-core (Cargo.toml:33,45,48).
- Compiler: tools/tbd-tools/src/world/build.rs (build-objects :135-520 writes chunks :503 gz9, prefabs
  :479, density :566, forest-regions :593, type-inventory :746; build-roads :1106-1183), world/aux.rs
  :1109 raw-u16-to-dem-png, map/unified.rs (TBDS), map/carto.rs, density.rs:24 (TBDD), bin/world.rs,
  bin/map.rs; xtask/src/map_blueprint/{library.rs:579-596, library_cli.rs, batch.rs, bvh.rs};
  xtask/src/gate_export_terrain.rs; xtask/src/mk_ci_tasks.rs:187-248 (map-water/cartographic lanes,
  lfs-dem/sat). Serving: apps/website/api/src/app.rs:1027 ServeDir "/map-assets".
- Assets: packages/map-assets/everon/{objects/chunks/*.json.gz (315, 22 MB), objects/density/*.bin
  (625), objects/{prefabs,roads,forest-regions}.json.gz, objects/type-inventory.json, dem/everon-dem-
  16bit.png (71.9 MB LFS), satellite/everon-sat.tbd-sat (152 MB LFS), prefabs/blas/*.bvh (1690 LFS),
  prefabs/descriptors/*.json (1623, 19 MB), prefabs/blas-manifest.json, prefabs/buildings/ (6 golden),
  locations.json, height-labels.json, road-names.json, manifest.json (objects.schemaVersion 1.1.0,
  transforms yaw+pitch+roll+scale)}; staging (gitignored): staging/water/TBD_InlandWaterExport_{mask,
  depth}.txt (328 MB each) + _vectors.json; staging/export/raw-entities.jsonl. .gitattributes:3-6 LFS
  rules; .gitignore:18,22 staging/tiles. Schema: packages/tbd-schema/schema/map-object-instance.schema.json
  :38-51 (chunk row 5 or 8 numbers), golden/map-objects/map-object-chunk-sample.json; gates
  xtask/src/schema_gates.rs:3366, golden_gate.rs:483.
- Audit anchors (verified TRUE unless marked): see /tmp/claude-1000/-var-home-Samuel/27d690e7-dbf3-45c4-8a58-16bfa7463c9d/scratchpad/audit-verified.md
