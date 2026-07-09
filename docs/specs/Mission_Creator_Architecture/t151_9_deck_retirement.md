# T-151.9 — Deck flip + retirement (W9)

**Status:** **ready** (executor queue) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `ec59d10e` (tag **T-151.8.1** — verify log
[`t151_8_verify_log.md`](../../../.ai/artifacts/t151_8_verify_log.md)).

## In one sentence

Make **wgpu the default Mission Creator map engine**, then **delete the Deck.gl runtime path**
(layers, workers, stores) while keeping deck.gl as a **devDependency camera oracle** only —
LANGUAGE GATE (D5) still binds: no new fat TS engine policy.

## Problem

W0–W8.1 shipped a production-capable wgpu mount behind `?engine=wgpu` /
`VITE_MC_ENGINE=wgpu`. Deck remains the default and still owns a large FE surface
(`TacticalMap.tsx` + `layers/*` + `worldmap/*Layer*` + workers). The program’s F4 analog is
to flip default → soak → delete Deck from the **runtime** bundle so the editor is one engine.

## Goal

1. **Default flip:** `VITE_MC_ENGINE=wgpu` (and/or remove the Deck default branch so
   Mission Creator mounts `WgpuTacticalMap` without a query flag). Keep `?engine=deck` (or
   equivalent) as an **escape hatch during soak only** if needed for A/B — document in verify
   log; remove escape hatch before tagging ship if soak is clean.
2. **Delete Deck runtime modules** listed in Locked (layers, worldmap Deck layers, workers,
   Deck-only stores). Retarget or delete vitests that imported those modules; camera/ortho
   oracles that use Deck math stay (devDependency).
3. **Bundle:** remove `deck.gl` / `@deck.gl/*` from **dependencies** (move to
   `devDependencies` if still required for oracle tests). Record before/after bundle sizes in
   the verify log.
4. **E2E gates:** editor load ~367k from IDB + server, edit, Save Version **201**, Export
   download, conflict path — on **wgpu default**.
5. Verify log + tag **T-151.9**. Cursor doc-sync after merge (registry program complete /
   W10 unlock note).

## Out of scope

- New features (markers **T-069**, vehicles **T-070**, ruler/LoS — **W10**).
- Forest Path B / T-149 polish.
- Growing `wgpu*Controller` / putting engine policy in TypeScript (D5).
- Re-opening W8 cull/ladder (shipped).
- Registry/docs (Cursor-owned) — except verify log + Ready for Cursor doc sync.

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | **LANGUAGE GATE:** no new cull/LOD/GPU sync policy in `.ts` | D5 |
| L2 | Default engine = **wgpu** after this slice | Hub W9 |
| L3 | Delete Deck **runtime** path; deck.gl may remain **devDependency** for camera oracle | Hub |
| L4 | Delete list (minimum): `useIconLayer`, `useDragIconLayer`, `useClusterIconLayer`, `useSelectionLayer`, `useDemLayer`, `useTerrainBasemapLayer`, `useBaseMapLayer`, `worldmap/useWorldMapLayers`, `worldmap/*Layer.ts` (Deck consumers), world worker trio (`worldObjects.worker` + related), Deck-only stores wired only from `TacticalMap` | Hub inventory |
| L5 | Keep pure math modules that wgpu already mirrors in Rust **only if** still imported by oracle tests or non-editor pages — otherwise delete with Deck | Avoid orphan dead code |
| L6 | `wgpuSlots.ts` stays ≤ **60** LOC; no new wgpu TS file > **80** LOC with policy | Drift control |
| L7 | Vitest green after retarget; floor ≥ **393** (may drop Deck-only tests that are deleted — record count) | Regression |
| L8 | Commit `T-151.9:` · tag **`T-151.9`** · verify log `.ai/artifacts/t151_9_verify_log.md` | House |
| L9 | **No silent deferrals** — do not ship “flip only, delete later” unless user explicitly defers | `.cursor/rules/no-silent-deferrals.mdc` |

## Pinned numbers

| Quantity | Value | Source |
|---|---|---|
| Baseline tag | **T-151.8.1** (`ec59d10e`) | git |
| Vitest @ W8 | **393** | T-151.8 |
| `wgpuSlots.ts` | ≤ **60** | LANGUAGE GATE |
| Escape hatch | document then remove if soak OK | L2 |

## Tasks

1. Flip default mount to `WgpuTacticalMap` / `VITE_MC_ENGINE=wgpu`.
2. Delete Deck runtime modules + workers; fix imports; retarget tests.
3. Move deck.gl packages to devDependencies (or remove if unused); record bundle delta.
4. Manual E2E checklist in verify log (367k load/save/export/conflict).
5. Tag **T-151.9**; Ready for Cursor doc sync.

## Verify

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
cargo build --workspace && make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint
! grep -l map_engine_wasm_bg dist/assets/index-*.js
# Deck must not be a runtime dependency of the editor entry
! grep -E '"deck\.gl"|"@deck\.gl/' package.json | grep -v devDependencies || true
# Prefer: deck.gl only under devDependencies after ship
wc -l src/features/tactical-map/wgpu/wgpuSlots.ts   # ≤ 60
# No leftover Deck imports from Mission Creator default path
rg -n "from '@deck\.gl|from 'deck\.gl|from \"@deck\.gl" src/features/mission-creator src/features/tactical-map --glob '!**/oracle*' || true
```

## Manual acceptance

- **S1:** Open `/missions/:id/edit` with **no** `?engine=` — wgpu mounts; map pans/zooms.
- **S2:** Place/select/drag slots; Save Version → **201**; Export downloads.
- **S3:** Load large mission (~367k) from IDB + server hydrate; conflict path still works.
- **S4:** Verify log has before/after bundle sizes + fps note (wgpu default).
- **S5:** Deck runtime modules gone; `npm run build` has no Deck in editor chunk (or only oracle).

## Documentation sync (Cursor, after merge)

Registry `T-151.9 → shipped`; program hub W9 shipped; CLAUDE §T-151 next → W10 / T-069;
`./scripts/ticket sync` + `check`.

## Claude Code prompt — T-151.9 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.9** — Deck flip + retirement (W9).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain
  git rev-parse HEAD   # expect ec59d10e+ (tag T-151.8.1)
  # Do NOT checkout branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  make wasm

═══ READ ═══
  1. .ai/artifacts/t151_9_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_9_deck_retirement.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md  (§T-151.9 + D5)
  4. .ai/tickets/CLAUDE_CODE_PROMPT.md  (§T-151 language gate)
  5. .cursor/rules/no-silent-deferrals.mdc
  6. apps/.../MissionCreatorPage.tsx  (engine flag)
  7. apps/.../TacticalMap.tsx + WgpuTacticalMap.tsx

═══ PROBLEM ═══
  wgpu is production-ready (W8.1) but Deck is still the default runtime path. Flip default
  to wgpu and delete Deck runtime modules/workers from the editor bundle.

═══ SHIPPED (do not reopen) ═══
  T-151.8 @ f4ffbfff · T-151.8.1 @ ec59d10e — cull, density ladder, damage, WebGPU compute cull.
  LANGUAGE GATE binding — do not recreate fat TS controllers.

═══ LANGUAGE GATE (MANDATORY — D5) ═══
  Rust OWNS: engine policy already shipped (cull, ladder, slots, camera).
  TypeScript ONLY: React, pointer, thin wasm, delete Deck glue — no new GPU/LOD policy in .ts.
  STOP IF: about to add a new wgpu*Controller with business logic → put it in crates/ instead.
  LOC: wgpuSlots.ts ≤ 60; any new wgpu TS ≤ 80 and wrappers only.

═══ LOCKED ═══
  - Default VITE_MC_ENGINE=wgpu / WgpuTacticalMap without query flag
  - Delete Deck runtime modules (L4 list); deck.gl → devDependency for oracle only
  - No silent deferrals: flip AND delete in this slice
  - No W10 features; no forest Path B
  - Commit T-151.9: · tag T-151.9 · .ai/artifacts/t151_9_verify_log.md

═══ DO ═══
  1. Flip Mission Creator default mount to wgpu
  2. Delete Deck runtime layers/workers/stores; fix imports; retarget vitest
  3. Move deck.gl to devDependencies (or remove); record bundle delta
  4. Manual E2E notes in verify log (S1–S5)
  5. Tag T-151.9; Ready for Cursor doc sync

═══ DO NOT ═══
  - Edit docs/registry/CLAUDE (except verify log)
  - Defer Deck deletion after flip without user saying so
  - Grow fat TS engine policy
  - Start T-069/T-070/W10
  - git checkout -b / ./scripts/ticket run

═══ VERIFY ═══
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
  cargo test -p map-engine-core --all-features
  cargo test -p map-engine-render
  cargo build --workspace && make wasm
  cd apps/website/frontend && npm test && npm run build && npm run lint
  ! grep -l map_engine_wasm_bg dist/assets/index-*.js
  wc -l src/features/tactical-map/wgpu/wgpuSlots.ts

═══ MANUAL ═══
  S1: default edit route = wgpu
  S2: place/select/drag + Save 201 + Export
  S3: ~367k load + conflict path
  S4: bundle delta in verify log
  S5: no Deck in editor runtime path

═══ RETURN ═══
  SHA + tag T-151.9
  .ai/artifacts/t151_9_verify_log.md
  Ready for Cursor doc sync.
```
