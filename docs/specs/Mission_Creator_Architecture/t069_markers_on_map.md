# T-069 — Markers on map (post–T-151.10)

**Status:** **queued** · **Program:** Eden (unlocked after **T-151.10** Fable audit +
remediations) · **Executor:** claude-code · **Worktree:** `tbd-reforger-wgpu-spike/`
(absolute: `/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not**
touch `main`) · **Baseline:** tag **T-151.9** (`c4831451` ship / tip `58c8fcc3`) · verify
[`.ai/artifacts/t151_9_verify_log.md`](../../../.ai/artifacts/t151_9_verify_log.md) · **Gate:**
[`t151_10_fable_program_audit.md`](t151_10_fable_program_audit.md).

## In one sentence

Ship **mission map markers** (`MapMarker` in the yrs doc): place, render on **wgpu**, select,
move, delete — LANGUAGE GATE (D5) still binds (geometry/GPU in Rust; TS = thin UI).

## Problem

`MapMarker` exists in `state/schema.ts` (`kind: line | arrow | phase | icon | polygon`) and the
doc entity map `markers` is reserved, but there is no `addMarker` / GPU draw / pick path on the
wgpu Mission Creator. Gap analysis **RIGHT-MODE-006** / **RIGHT-STUB-002** and ROADMAP T-069.

## Goal

1. **Doc actions:** `addMarker` / update / remove (one undo step per gesture) on the yrs
   `markers` map; hydrate/compile round-trip preserves markers.
2. **wgpu render:** draw marker kinds using existing engine pipelines where possible
   (polyline/polygon for line/arrow/phase/polygon; icon atlas for `icon`).
3. **Interaction:** select / multi-select / drag-move / Delete — same toolbelt select tool
   patterns as slots (extend, do not fork a second gesture machine).
4. **UI entry:** Asset palette / Markers stub becomes a real place path (or tool mode) so a
   mission maker can drop at least **icon** + **line** in v1; remaining kinds if cheap.
5. Verify log + tag **T-069**. Cursor doc-sync after merge.

## Out of scope

- Vehicles (**T-070**), ruler/LoS, ORBAT Manager (**T-071**).
- Registry-backed marker catalogs (use schema kinds + colors; no new registry API).
- Growing fat TS controllers / reintroducing Deck.
- Perfect Eden parity for every marker subtype in one slice — ship **icon + line** minimum;
  arrow/phase/polygon if they fit without scope blow-up (document any deferral **explicitly**
  in verify log — no silent deferrals).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | **LANGUAGE GATE:** marker GPU compose / pick policy in Rust; TS thin | D5 |
| L2 | Work on `tbd-reforger-wgpu-spike/` only; tag `T-069` | Standing worktree |
| L3 | Persist via existing `markers` entity map + compiler `editor` / export path | Schema already has `MapMarker` |
| L4 | Reuse `useSelectTool` + selection `kind: 'marker'` | No second gesture stack |
| L5 | `wgpuSlots.ts` stays ≤ **60** LOC; new thin `wgpuMarkers.ts` ≤ **80** LOC if needed | Drift control |
| L6 | Vitest green; add Class R/S tests for any new pure/Rust geometry | Regression |
| L7 | Commit `T-069:` · tag **`T-069`** · verify `.ai/artifacts/t069_verify_log.md` | House |
| L8 | **No silent deferrals** — if a kind slips, name it in verify log with operator OK | `.cursor/rules/no-silent-deferrals.mdc` |

## Pinned references

| Item | Path |
|------|------|
| Schema | `apps/.../tactical-map/state/schema.ts` `MapMarker` |
| Engineering model | `engineering_plan.md` §2.1 `MapMarker` |
| Engine hub | `t151_wgpu_engine_program.md` §W10 |
| Gap | `eden/gap_analysis.md` RIGHT-MODE-006 |

## Tasks

1. Yrs `addMarker` / update / remove + bindings → `markersById`.
2. Rust/wgpu draw + pick for shipped kinds.
3. Wire select/move/delete + palette/tool entry for place.
4. Compiler/hydrate preserve markers.
5. Verify + tag **T-069**; Ready for Cursor doc sync.

## Verify

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
cargo build --workspace && make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint
wc -l src/features/tactical-map/wgpu/wgpuSlots.ts   # ≤ 60
# No Deck imports on production MC/tactical-map paths (allowlist unchanged from T-151.9)
```

## Manual acceptance

- **S1:** Place an icon marker; it draws on wgpu; survives Save Version + reload.
- **S2:** Place a line marker (≥2 points); select + drag + Delete work.
- **S3:** Undo/redo one place/move/delete step.
- **S4:** Export / compile still succeeds with markers present.
- **S5:** Verify log lists shipped kinds + any explicitly deferred kinds.

## Documentation sync (Cursor, after merge)

Registry `T-069 → shipped`; ROADMAP row; CLAUDE § next → **T-070**; `./scripts/ticket sync`.

## Claude Code prompt — T-069 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-069** — Markers on map (W10).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain
  git rev-parse HEAD   # expect T-151.9 tip (58c8fcc3+)
  git lfs pull && make map-assets-link
  make wasm

═══ READ ═══
  1. .ai/artifacts/t069_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t069_markers_on_map.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md  (§W10 + D5)
  4. .ai/tickets/CLAUDE_CODE_PROMPT.md  (§T-151 language gate)
  5. .cursor/rules/no-silent-deferrals.mdc
  6. apps/.../tactical-map/state/schema.ts  (MapMarker)
  7. apps/.../tactical-map/WgpuTacticalMap.tsx + wgpu/*

═══ PROBLEM ═══
  MapMarker schema + markers entity map exist, but no place/render/select/move/delete on wgpu.

═══ DO ═══
  - Yrs add/update/remove markers; store bindings
  - Rust/wgpu draw + pick (icon + line minimum)
  - Select/move/delete via existing select tool (kind marker)
  - Palette/tool entry to place
  - Hydrate/compile preserve markers
  - .ai/artifacts/t069_verify_log.md
  - Commit T-069: · tag T-069

═══ DO NOT ═══
  - Edit docs/registry/CLAUDE (verify log OK)
  - Reintroduce Deck / fat TS controllers
  - Grow wgpuSlots.ts past 60
  - Silently skip a kind — name deferrals in verify log
  - Start T-070 / ruler / LoS

═══ VERIFY ═══
  (commands in spec §Verify — all exit 0)
  Manual S1–S5

═══ RETURN ═══
  SHA + tag T-069
  .ai/artifacts/t069_verify_log.md
  Ready for Cursor doc sync.
```
