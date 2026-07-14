# T-152.1 — wgpu text lane (SDF / canvas-to-texture)

**Ticket:** T-152 · **Slice:** T-152.1  
**Status:** **queued**  
**Executor:** claude-code *(implementing agent: **Grok 4.5 in Cursor**)*  
**Authority:** [`t152_map_cartographic_fidelity_program.md`](t152_map_cartographic_fidelity_program.md)  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · **Branch:** `ticket/T-152`  
**Depends on:** T-152.0 shipped · **Blocks:** T-152.7, T-152.8, T-152.9, T-152.10 (label consumers)

---

## In one sentence

Add a **Rust-owned text draw lane** in `map-engine-render` (SDF font atlas **or** deterministic canvas→texture bake) plus an **importance-distance declutter helper** stub (T-144 G8) — TypeScript stays a thin wasm bridge.

---

## Problem

- **P5:** No text/font pipeline exists under `crates/map-engine-render/` — contour elevations, town names, and road labels cannot ship on wgpu.
- T-144 **G8** recommends precomputed `_nearestMoreImportant` distance for label ordering — no module exists.
- HTML/CSS overlays would break D5 (camera anchor, zoom scaling, pick parity).

---

## Goal

1. **Text pipeline** in Rust: load a font (bundled TTF/OTF or build-time atlas PNG), layout UTF-8 strings in world space, draw with a dedicated `PipelineKind::Text` (or textured quads from baked glyphs).
2. **Wasm API**: `TextLane::set_labels(Vec<LabelSpec>)`, `compose(viewport, zoom)` → GPU instance buffer; stats key `text_labels_drawn`.
3. **Declutter helper** (`label_declutter.rs`): given `LabelSpec { x, y, importance, text }`, compute draw-set such that ∀ pair drawn (i,j): `distance(i,j) ≥ MIN_LABEL_PX · 2^−zoom` **or** `importance(i) > importance(j)` (T-144 G8 analogue).
4. **Vitest / Rust tests**: declutter invariant, zero labels → zero draw calls, wasm size regression logged.

---

## Out of scope

- Town/road **data** import (T-152.6 / T-152.9)
- Height-marker **placement** / peak detect (T-152.7 — consumes this lane)
- Landmark **icons** (T-152.2 / T-152.3)
- HTML/DOM text overlay

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | **Rust owns** font load, layout, glyph quad placement, declutter, GPU upload | T-151 D5 |
| L2 | Choose **one** path: (A) SDF in shader **or** (B) offline/bake-time RGBA atlas — document in verify log | Either meets perf |
| L3 | `LabelSpec` world coords in **meters** (same as icon instances); screen min distance uses `deckZoom` | N1 zoom authority |
| L4 | `MIN_LABEL_PX = 48` at reference; scales as `MIN_LABEL_PX · 2^−deckZoom` world meters via viewport | G8 analogue |
| L5 | `importance: u16` — higher wins ties; stable sort by `(−importance, id)` | A3 declutter |
| L6 | TS files: `wgpuTextLane.ts` **≤ 80 LOC** — init + `setLabels` wasm calls only | D5 budget |
| L7 | No draw if string empty or `class_visible` gate false (hook for future contour class) | Regression guard |
| L8 | Commit `T-152.1:` · tag **`T-152.1`** · verify `.ai/artifacts/t152_1_verify_log.md` | House |

---

## Tasks

| # | Path | Action |
|---|------|--------|
| 1 | `crates/map-engine-core/src/label/` | New: `label_spec.rs`, `declutter.rs` (importance-distance) |
| 2 | `crates/map-engine-render/src/text/` | Font/atlas load, `TextLane`, WGSL shader |
| 3 | `crates/map-engine-render/src/engine.rs` | Register text pipeline + draw pass (after badges, before grid) |
| 4 | `crates/map-engine-wasm/src/` | Export `set_text_labels`, `text_label_count` |
| 5 | `apps/website/frontend/src/features/tactical-map/wgpu/wgpuTextLane.ts` | Thin bridge (≤80 LOC) |
| 6 | `crates/map-engine-core/src/label/declutter.rs` tests | G4 invariant proofs |
| 7 | `.ai/artifacts/t152_1_verify_log.md` | G1–G8 + wasm bytes |

---

## Mathematical acceptance matrix

| ID | Predicate | Pass condition |
|----|-----------|----------------|
| **G1** | Rust tests | `cargo test -p map-engine-core --all-features` exit 0 |
| **G2** | Render tests | `cargo test -p map-engine-render` exit 0 |
| **G3** | Wasm build | `make wasm` exit 0; merged `map_engine_wasm_bg.wasm` size recorded |
| **G4** | Declutter invariant | ∀ test fixture set S, ∀ (i,j) in draw(S): `dist(i,j) ≥ d_min(z)` ∨ `imp(i)>imp(j)` — unit test |
| **G5** | Empty input | `labels=[]` ⇒ `text_label_count()==0` and no text pipeline bind (mock or stats) |
| **G6** | TS budget | `wc -l wgpuTextLane.ts ≤ 80` |
| **G7** | Frontend gates | `cd apps/website/frontend && npm test && npm run build && npm run lint` exit 0 |
| **G8** | No TS layout | `rg 'declutter\|importance.*label\|MIN_LABEL' apps/website/frontend/src/features/tactical-map/wgpu --glob '*.ts'` → only `wgpuTextLane.ts` matches |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
cargo build --workspace
make wasm
wc -l apps/website/frontend/src/features/tactical-map/wgpu/wgpuTextLane.ts
cd apps/website/frontend && npm test && npm run build && npm run lint
```

---

## Manual checklist

| ID | Check | Pass |
|----|-------|------|
| M1 | Dev server: inject 3 test labels via console/wasm — visible, pan/zoom anchored | ☐ |
| M2 | Zoom out: declutter drops lower-importance colliding labels first | ☐ |

---

## Documentation sync (Cursor, after merge)

Registry `T-152.1 → shipped`; hub active **T-152.2**; `./scripts/ticket sync`.

---

## §Grok Code prompt — T-152.1 (copy-paste)

Authority: this spec + hub. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE (NOT main).

Implement **T-152.1** — wgpu text lane (SDF or canvas-to-texture) + declutter helper.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git branch --show-current    # expect ticket/T-152
  git status --porcelain       # empty @ T-152.0 shipped
  # Do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  make wasm

═══ READ (in order — spec wins on conflict) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_map_cartographic_fidelity_program.md
  2. docs/specs/Mission_Creator_Architecture/t152_1_wgpu_text_lane.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md  (D5)
  4. .ai/artifacts/t144_arma3_map_architecture_report.md  (§9 G8)
  5. crates/map-engine-render/src/{engine.rs,lanes.rs,shader.wgsl}
  6. crates/map-engine-core/src/world/lod_gates.rs
  7. apps/website/frontend/src/features/tactical-map/wgpu/WgpuTacticalMap.tsx

═══ PROBLEM ═══
  wgpu map has vectors + icon glyphs but NO text lane. Contour elevations and town/road names
  (T-152.4+) need Rust-owned labels with T-144 G8 importance-distance declutter. TS must stay thin.

═══ SHIPPED (do not reopen) ═══
  T-151 program @ 8237cda6 — wgpu-only engine, icon atlas, residency glyphs.
  T-152.0 — docs hub lock.

═══ LANGUAGE GATE (MANDATORY) ═══
  Rust OWNS: font/SDF, label layout, declutter policy, SoA text instances, shaders, wasm exports.
  TypeScript ONLY: thin wasm calls in wgpuTextLane.ts (≤80 LOC), React mount hooks.
  STOP IF: label layout, declutter, or zoom scaling logic lands in .ts/.tsx → move to crates/.

═══ LOCKED ═══
  - One of SDF shader OR baked glyph atlas — document choice in verify log
  - LabelSpec in world meters; MIN_LABEL_PX=48 scaled by 2^−deckZoom
  - Declutter: draw i,j only if dist≥d_min OR imp(i)>imp(j)
  - Draw order: after badge glyphs, before grid
  - stats(): additive text_labels_drawn
  - wgpuTextLane.ts ≤80 LOC

═══ DO ═══
  1. Add map-engine-core label/declutter module + unit tests (G4)
  2. Add map-engine-render TextLane pipeline + font load
  3. Wasm exports set_text_labels / text_label_count
  4. Thin wgpuTextLane.ts + hook from WgpuTacticalMap (test labels OK for M1)
  5. Write .ai/artifacts/t152_1_verify_log.md; commit T-152.1: · tag T-152.1

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, CLAUDE.md sync markers
  - HTML/CSS text overlays; DOM label layers
  - Town/road data import; contour placement along lines (later slices)
  - ./scripts/ticket run; grow fat wgpu*Controller in TS

═══ VERIFY (all exit 0) ═══
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
  cargo test -p map-engine-core --all-features
  cargo test -p map-engine-render
  cargo build --workspace
  make wasm
  wc -l apps/website/frontend/src/features/tactical-map/wgpu/wgpuTextLane.ts
  cd apps/website/frontend && npm test && npm run build && npm run lint

═══ MANUAL ═══
  M1: 3 wasm-injected test labels visible, pan/zoom anchored
  M2: zoom out drops lower-importance colliding labels first

═══ RETURN ═══
  - Commit SHA + tag T-152.1
  - .ai/artifacts/t152_1_verify_log.md (G1–G8 table, wasm bytes, SDF vs bake decision)
  - **Ready for Cursor doc sync → T-152.2**
```
