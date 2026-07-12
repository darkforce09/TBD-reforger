# T-152.2 — Reforger map icon art + atlas rebuild

**Ticket:** T-152 · **Slice:** T-152.2  
**Status:** **queued**  
**Executor:** claude-code *(implementing agent: **Grok 4.5 in Cursor**)*  
**Authority:** [`t152_map_cartographic_fidelity_program.md`](t152_map_cartographic_fidelity_program.md)  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · **Branch:** `ticket/T-152`  
**Depends on:** T-152.1 shipped · **Blocks:** T-152.3

---

## In one sentence

Run a **Workbench/MCP icon discovery spike**, replace **placeholder** `LANDMARK_SET` SVGs with Reforger-familiar art, rebuild `world-glyphs.webp` + manifest, and prove **∀ iconKey ∈ LANDMARK_SET** coverage with `make map-glyphs-build` + `make map-glyphs-verify`.

---

## Problem

- **P4:** [`packages/map-assets/glyphs/svg/`](../../../packages/map-assets/glyphs/svg/) contains hand-drawn **placeholder** icons (generic building shapes), not Arma Reforger map iconography.
- Operators cannot distinguish lighthouse vs castle vs hangar at zoom ≥ `BUILDING_BADGE_MIN_ZOOM` — only fill tint differs ([`residency.rs`](../../../crates/map-engine-core/src/world/residency.rs) `fill_color`).
- [`manifest.json`](../../../packages/map-assets/glyphs/manifest.json) lists 28 keys but art quality is pre-T-152 scaffold (T-090.5 / T-151.5).

---

## Goal

1. **Discovery artifact** [`.ai/artifacts/t152_2_icon_discovery_spike.json`](../../../.ai/artifacts/t152_2_icon_discovery_spike.json): Reforger pak/UI paths, reference PNG dimensions, mapped `iconKey` rows, gaps list.
2. **Redraw** every `LANDMARK_SET` SVG (hub list) to match Reforger-familiar silhouette + palette (document deviations).
3. **`make map-glyphs-build`** → updated [`atlas/world-glyphs.webp`](../../../packages/map-assets/glyphs/atlas/world-glyphs.webp) + [`world-glyphs.json`](../../../packages/map-assets/glyphs/atlas/world-glyphs.json).
4. **`make map-glyphs-verify`** + golden prefab `render.iconKey` scan PASS.
5. **No placeholder SVG** remains for `LANDMARK_SET`: predicate `∀ k ∈ LANDMARK_SET: svg[k]` has `source:reforger` or `source:redraw` in discovery JSON (not `placeholder`).

---

## Out of scope

- **Residency / GPU wire** (T-152.3)
- Tree/veg/prop art refresh unless discovery marks **mismatch** with Reforger (then include in spike JSON only)
- Workbench height/airfield/pier **recompose**
- New `iconKey` enum values without schema/taxonomy row (use existing `building-*` keys)

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | Discovery via **Workbench MCP** (`scripts/mod/mcp-call.sh`, `api_search` for map icon / UI atlas) | Operator toolchain |
| L2 | `LANDMARK_SET` = hub §LANDMARK_SET (21 keys) | Locked program |
| L3 | SVG rules per [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md): north-up, simple fills, 24×24 viewBox | Atlas builder |
| L4 | Atlas build = existing [`scripts/map-assets/build-glyph-atlas.mjs`](../../../scripts/map-assets/build-glyph-atlas.mjs) — **no** new bake pipeline | T-090.5.2 |
| L5 | Placeholder detection: SHA256 of each pre-slice SVG recorded in verify log; post-slice hash **must differ** for every `LANDMARK_SET` key | Objective gate |
| L6 | Commit art + atlas + discovery JSON; tag **`T-152.2`** | LFS for webp if needed |
| L7 | Operator sign-off on 3 pinned landmarks (lighthouse, military, church/civic) advisory Mn only | Not blocking G gates |

---

## Tasks

| # | Path | Action |
|---|------|--------|
| 1 | Workbench MCP | Discovery spike → `t152_2_icon_discovery_spike.json` |
| 2 | `packages/map-assets/glyphs/svg/building-*.svg` | Replace `LANDMARK_SET` sources |
| 3 | `packages/map-assets/glyphs/manifest.json` | Update `defaultColor` / `baseSizePx` only if discovery requires |
| 4 | `packages/map-assets/glyphs/atlas/` | Rebuild webp+json |
| 5 | `packages/tbd-schema/scripts/verify-map-glyphs*.mjs` | Must pass unchanged contract |
| 6 | `.ai/artifacts/t152_2_verify_log.md` | G1–G7 + before/after hashes |

---

## Mathematical acceptance matrix

| ID | Predicate | Pass condition |
|----|-----------|----------------|
| **G1** | Discovery artifact | File exists; `keys.length ≥ |LANDMARK_SET|`; each landmark has `reforgerRef` or `redrawRationale` |
| **G2** | Coverage | `∀ k ∈ LANDMARK_SET: manifest.glyphs[k]` defined |
| **G3** | SVG on disk | `∀ k ∈ LANDMARK_SET: test -f packages/map-assets/glyphs/svg/${k}.svg` |
| **G4** | Placeholder evicted | `∀ k ∈ LANDMARK_SET: sha256(svg[k]) ≠ sha256_pre[k]` (pre recorded in verify log) |
| **G5** | Atlas build | `make map-glyphs-build` exit 0; `icons` count ≥ 28 |
| **G6** | Verify script | `make map-glyphs-verify` exit 0 |
| **G7** | Golden prefabs | `cd packages/tbd-schema && npm run verify-map-glyphs` exit 0 |

Let **LANDMARK_SET** be:

```text
{ building-residential, building-civic, building-agricultural, building-industrial,
  building-commercial, building-hangar, building-bunker, building-tower, building-military,
  building-bridge, building-castle, building-lighthouse, building-shed, building-container,
  building-tent, building-ruin, building-garage, building-generic,
  building-badge-military, building-badge-bunker, building-badge-tower }
```

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
test -f .ai/artifacts/t152_2_icon_discovery_spike.json
make map-glyphs-build
make map-glyphs-verify
cd packages/tbd-schema && npm ci --silent && npm run verify-map-glyphs
# G3/G4: per-key sha256 checks scripted in verify log appendix
```

---

## Manual checklist

| ID | Check | Pass |
|----|-------|------|
| M1 | Operator: lighthouse + castle + military @ zoom +1.5 — recognizable vs Reforger map | ☐ |
| M2 | Atlas preview PNG in verify log — no magenta/empty cells for LANDMARK_SET | ☐ |

---

## Documentation sync (Cursor, after merge)

Registry `T-152.2 → shipped`; hub active **T-152.3**; `./scripts/ticket sync`.

---

## §Grok Code prompt — T-152.2 (copy-paste)

Authority: this spec + hub. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE (NOT main).

Implement **T-152.2** — Reforger map icon discovery + LANDMARK_SET art + atlas rebuild.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git branch --show-current    # ticket/T-152
  git status --porcelain
  # Workbench warm: bash scripts/mod/tbd-dev-bootstrap.sh (operator)
  git lfs pull
  sha256sum packages/map-assets/glyphs/svg/building-*.svg > /tmp/t152_2_pre_sha.txt

═══ READ (in order — spec wins on conflict) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_2_reforger_icon_art.md
  2. docs/specs/Mission_Creator_Architecture/t152_map_cartographic_fidelity_program.md  (LANDMARK_SET)
  3. docs/specs/Mission_Creator_Architecture/t090_world_object_glyphs.md
  4. packages/map-assets/glyphs/manifest.json
  5. scripts/map-assets/build-glyph-atlas.mjs
  6. docs/mod/MCP_TOOLING.md

═══ PROBLEM ═══
  packages/map-assets/glyphs/svg/* are PLACEHOLDERS. Operators need Reforger-familiar building
  landmark icons in world-glyphs.webp before T-152.3 wires them on the map.

═══ SHIPPED (do not reopen) ═══
  T-152.1 — text lane (if shipped; else stop — sequential gate)
  T-151.5 — 28-key atlas scaffold + IconInstanced pipeline

═══ LANGUAGE GATE ═══
  Rust/wgpu untouched this slice except if atlas JSON contract requires constant sync (prefer not).
  This slice is ART + build scripts + discovery JSON. No residency.rs edits (T-152.3).

═══ LOCKED ═══
  - LANDMARK_SET = 21 keys in hub (building-* + building-badge-*)
  - MCP discovery → .ai/artifacts/t152_2_icon_discovery_spike.json
  - make map-glyphs-build + make map-glyphs-verify must PASS
  - ∀ k ∈ LANDMARK_SET: sha256(svg[k]) ≠ pre-slice hash
  - No new iconKey strings without taxonomy alignment

═══ DO ═══
  1. Workbench MCP icon discovery spike → JSON artifact (G1)
  2. Redraw/replace SVGs for LANDMARK_SET; update manifest if needed
  3. make map-glyphs-build; commit atlas webp+json (G5)
  4. make map-glyphs-verify + npm run verify-map-glyphs (G6–G7)
  5. .ai/artifacts/t152_2_verify_log.md with G1–G7 + sha256 table; tag T-152.2

═══ DO NOT ═══
  - Edit crates/map-engine-core/src/world/residency.rs (T-152.3)
  - Edit docs/**, registry.json
  - Workbench height/airfield/pier recompose
  - Leave any LANDMARK_SET SVG at placeholder hash
  - ./scripts/ticket run

═══ VERIFY (all exit 0) ═══
  test -f .ai/artifacts/t152_2_icon_discovery_spike.json
  make map-glyphs-build
  make map-glyphs-verify
  cd packages/tbd-schema && npm ci --silent && npm run verify-map-glyphs

═══ MANUAL ═══
  M1: lighthouse, castle, military recognizable @ zoom ≥ +1.5
  M2: atlas preview — no empty cells for LANDMARK_SET

═══ RETURN ═══
  - Commit SHA + tag T-152.2
  - .ai/artifacts/t152_2_verify_log.md (G1–G7, sha256 pre/post)
  - **Ready for Cursor doc sync → T-152.3**
```
