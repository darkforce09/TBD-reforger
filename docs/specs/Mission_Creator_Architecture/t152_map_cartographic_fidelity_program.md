# T-152 — Map cartographic fidelity program

**Status:** **ready for merge** · **Active slice:** **T-152.10** (E2E gate — automated PASS; operator O1–O12 pending)  
**Ticket:** T-152 · **Registry:** [`.ai/tickets/registry.json`](../../../.ai/tickets/registry.json)  
**Worktree:** `.ai/artifacts/worktrees/TBD-T-152` (absolute: `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`) · **Branch:** `ticket/T-152`  
**Parallel lane:** runs **in parallel** with **T-068** arsenal work on `main` — no file overlap with arsenal compile until merge.

**Authority:** approved plan T-152 Map Fidelity · A3 reference [`.ai/artifacts/t144_arma3_map_architecture_report.md`](../../../.ai/artifacts/t144_arma3_map_architecture_report.md)

---

## In one sentence

Close the **cartographic readability gap** after T-090 export + T-151 wgpu: **Reforger-familiar landmark icons**, **fence/pier/bridge geometry**, **airfield symbology**, **height markers**, **town names**, and **road names** — sequential slices, every Gn PASS before advance.

---

## Agent split (LOCKED)

| Agent | Owns | Must NOT |
|-------|------|----------|
| **Cursor (Composer 2.5)** | Program hub, **all** slice specs `.0`–`.10`, registry, handoffs, `./scripts/ticket sync` | Application code |
| **Grok 4.5 in Cursor** | **All code** for T-152.1–.9 + verify logs; assist T-152.10 | Registry/docs as SoT; `./scripts/ticket run` |

**Registry note:** code slices keep `executor: claude-code` (house enum). **Implementing agent = Grok 4.5** — every code slice ends with **§Grok Code prompt**.

**Forbidden:** `./scripts/ticket run`, silent `PARTIAL` / `DEFERRED` verify rows, starting N+1 with open FAIL.

---

## Advance gate contract (SEQUENTIAL — binding)

```text
T-152.0 → T-152.1 → T-152.2 → T-152.3 → T-152.4 → T-152.5
       → T-152.6 → T-152.7 → T-152.8 → T-152.9 → T-152.10
```

1. **No code slice starts** until prior verify log records **every Gn = PASS**.
2. Operator manual rows (Mn) may be `PENDING` only when the slice marks them operator-only **and** all automated Gn are PASS — **T-152.10** requires all Mn PASS.
3. Cursor doc sync after each shipped slice.
4. **Merge to `main`** only after **T-152.10** PASS.

---

## Why T-152 (not more T-090.x)

| T-090 / T-151 (done) | T-152 (this program) |
|----------------------|----------------------|
| Basemap, DEM, roads, OBB buildings, forest mass, tree glyphs | Landmark **icons that look like Reforger**, not placeholder SVGs |
| Contour **lines** | Spot **height markers** + contour labels via text lane |
| No map text | wgpu **text lane** + town/road names |
| Pier/bridge = fat OBB | Thin strips + bridge icon |
| Runway polyline only | Airfield apron + airport icons |
| P2 export (no fences) | P5 fences as thin strips |

Sibling pattern: same as [`t092_spawn_transform_program.md`](t092_spawn_transform_program.md).

---

## Problem table (evidence)

| # | Symptom | Evidence | Slice |
|---|---------|----------|-------|
| P1 | Landmarks = tinted OBB squares (lighthouse white) | `fill_color` lighthouse `[235,235,235,220]` — [`residency.rs`](../../../crates/map-engine-core/src/world/residency.rs) | .3 |
| P2 | Building glyphs skipped in lookup | `else { continue }` in `rebuild_glyph_lookup_from_prefabs` — same file | .3 |
| P3 | Badges only military/tower/bunker | [`glyph_math.rs`](../../../crates/map-engine-core/src/world/glyph_math.rs) `badge_icon_key` | .3 |
| P4 | Glyph SVGs are **placeholders** | [`packages/map-assets/glyphs/svg/`](../../../packages/map-assets/glyphs/svg/) | .2 |
| P5 | No wgpu text/font lane | No SDF/text pipeline in `crates/map-engine-render/` | .1 |
| P6 | Pier/bridge fat rects; fences missing | pier→building OBB; P2 `importPhaseMax` (no P5 props) | .4 |
| P7 | Airfield = white runway only | [`polyline_strip.rs`](../../../crates/map-engine-core/src/geometry/polyline_strip.rs) `runway` | .5 |
| P8 | No town / road / height labels on map | No `locations` artifact; toolbelt Z only | .6–.9 |

---

## Locked ship approaches

| Feature | Ship path |
|---------|-----------|
| Map icons | Extract Reforger icons where possible; redraw gaps to Reforger-familiar; rebuild atlas (.2); wire (.3) |
| Pier / dock | Oriented **thin strips** from OBB long-axis (.4) |
| Fence / wall | P5 catalog + thin OBB strips (.4) |
| Bridge | Deck OBB + `building-bridge` glyph; railings with fences (.4) |
| Airfield | Runway polish + DEM-flat apron + hangar/tower icons (.5) |
| Height markers | wgpu text + DEM peaks + declutter (+ contour labels) (.7) |
| Town names | `locations.json` + text lane (.6 → .8) |
| Road names | Spike + curated Everon fallback + polyline labels (.9) |

---

## Workbench matrix

| Feature | Workbench? |
|---------|------------|
| Height markers / airfield runways / pier recompose / atlas bake | **No** |
| P5 fences | Only if raw stale |
| Icon texture discovery | **Spike yes** (.2) |
| Town / road name sources | **Spike yes** (.6 / .9) |

---

## Slice ladder (.0–.10) — PLAN AUTHORITATIVE

| Slice | Spec | Executor | Delivers |
|-------|------|----------|----------|
| **T-152.0** | [`t152_0_program_hub_lock.md`](t152_0_program_hub_lock.md) | cursor-docs | Hub + **all** specs + registry |
| **T-152.1** | [`t152_1_wgpu_text_lane.md`](t152_1_wgpu_text_lane.md) | claude-code→**Grok** | wgpu text lane + declutter helper |
| **T-152.2** | [`t152_2_reforger_icon_art.md`](t152_2_reforger_icon_art.md) | claude-code→**Grok** | Reforger-familiar atlas (replace placeholders) |
| **T-152.3** | [`t152_3_wire_landmark_glyphs.md`](t152_3_wire_landmark_glyphs.md) | claude-code→**Grok** | Wire landmark glyphs/badges |
| **T-152.4** | [`t152_4_fence_pier_bridge.md`](t152_4_fence_pier_bridge.md) | claude-code→**Grok** | Fence/pier/bridge geometry |
| **T-152.5** | [`t152_5_airfield_symbology.md`](t152_5_airfield_symbology.md) | claude-code→**Grok** | Airfield apron + runway + icons |
| **T-152.6** | [`t152_6_locations_export.md`](t152_6_locations_export.md) | claude-code→**Grok** (+ workbench spike) | `locations.json` + schema |
| **T-152.7** | [`t152_7_height_markers.md`](t152_7_height_markers.md) | claude-code→**Grok** | Height markers on map |
| **T-152.8** | [`t152_8_town_labels.md`](t152_8_town_labels.md) | claude-code→**Grok** | Town labels |
| **T-152.9** | [`t152_9_road_names.md`](t152_9_road_names.md) | claude-code→**Grok** (+ workbench spike) | Road names |
| **T-152.10** | [`t152_10_e2e_cartographic_gate.md`](t152_10_e2e_cartographic_gate.md) | human + Grok | E2E gate + merge readiness |

All eleven slice specs are **authored in the T-152.0 docs pass** (not deferred).

---

## `LANDMARK_SET` (locked — T-152.2 / .3)

```text
building-residential, building-civic, building-agricultural, building-industrial,
building-commercial, building-hangar, building-bunker, building-tower, building-military,
building-bridge, building-castle, building-lighthouse, building-shed, building-container,
building-tent, building-ruin, building-garage, building-generic,
building-badge-military, building-badge-bunker, building-badge-tower
```

After **.2**: no placeholder art remains for these keys. After **.3**: each draws as glyph/badge at locked zoom (not white-square-only).

---

## Verify commands (program overview)

```bash
# CWD must be worktree TBD-T-152
git rev-parse --show-toplevel   # …/TBD-T-152
cargo fmt --check
cargo clippy -p map-engine-core -p map-engine-render -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
make wasm
make map-glyphs-verify          # after .2
cd apps/website/frontend && npm test && npm run build && npm run lint
./scripts/ticket check
```

Per-slice **Gn** matrices live in each slice spec. Verify logs: `.ai/artifacts/t152_{n}_verify_log.md`. Program close-out: [`.ai/artifacts/t152_10_verify_log.md`](../../../.ai/artifacts/t152_10_verify_log.md) · merge: [`.ai/artifacts/t152_merge_readiness.md`](../../../.ai/artifacts/t152_merge_readiness.md).

---

## Out of scope (program)

- **T-069** markers · **T-070** vehicles · **T-143** perfect water · **T-090.10.2** raster retirement
- Named summit folklore beyond numeric ASL
- True mesh footprint polygons (OBB/strips ship)
- Mission slot icons / ORBAT / arsenal

---

## Related

| Doc | Role |
|-----|------|
| [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) | Export + basemap (sibling) |
| [`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) | D5 language gate |
| [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md) | `iconKey` contract |
| [`t090_render_lod_contract.md`](t090_render_lod_contract.md) | Zoom bands |
| [`t144_arma3_map_architecture_study.md`](t144_arma3_map_architecture_study.md) | G8 declutter |
| [`ROADMAP.md`](ROADMAP.md) | MC planning view |
| [`.ai/artifacts/t152_10_verify_log.md`](../../../.ai/artifacts/t152_10_verify_log.md) | E2E gate (T-152.10) |
| [`.ai/artifacts/t152_merge_readiness.md`](../../../.ai/artifacts/t152_merge_readiness.md) | Merge promotion |

---

## Documentation sync (Cursor, after each shipped slice)

1. Registry `slice_plan.T-152.n → shipped` + `shipped_at`  
2. Hub **Status** / **Active slice**  
3. `./scripts/ticket sync` && `./scripts/ticket check`  
4. CLAUDE.md status only via ticket sync markers
