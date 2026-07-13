# T-152.15 — Fence / pier / bridge visibility + orientation remediation

**Ticket:** T-152 · **Slice:** T-152.15 (remediation ladder #4)
**Status:** `queued`
**Executor:** **claude-code** (Claude Code)
**Authority:** T-152 program hub · audit [`t152_11_fidelity_audit_report.md`](../../../.ai/artifacts/t152_11_fidelity_audit_report.md) §5, §6.2 (S1, S2, A1, A13, D4–D6)
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152` · tag **`T-152.15`**
**Depends on:** T-152.11 audit

## In one sentence

Make fences appear at a sensible zoom with correct yaw, make all 2,299 piers/docks actually draw as thin quays decoupled from the fence gate, resolve bridge railings (implement proximity or delete the dead constant), and de-vacuous the pier gate — census > 0 or FAIL.

---

## Problem

Audit §5/§6.2:

- **S1:** `fences_visible = toggle_fences && class_visible("prop", z)` → z ≥ **3** (`residency.rs:325-327`; `lod_gates.rs:20`) with 0.35 m strips and no pixel floor — invisible until near-max zoom.
- **S2:** strip yaw = prefab rotation + a long-axis pick by `half_x >= half_y` (else +90°, `cartographic_strip.rs:37-50`); transposed/near-square measured OBBs flip 90°; the strip local frame (`forward = [cos·L, −sin·L]`) is constructed differently from the fill path (`obb_corners`, `obb.rs:16-25`) with no parity test.
- **A1/D5:** pier/dock fills unconditionally skipped (`residency.rs:697-700`); strips require aspect ≥ 4.0 — **0 of 2,299** instances qualify (max 2.57; T-152.4 G4 vacuous); the pier-strip loop additionally sits inside the fence-gated builder (`residency.rs:752,762`). Harbors are blank; O3 cannot pass.
- **A13/D6:** `BRIDGE_RAILING_RADIUS_M = 8.0` has **zero consumers** — railings were never implemented; bridge "deck" is a generic building fill tint (`residency.rs:83`).

---

## Goal

1. **Fence zoom + width:** fences visible from `FENCE_MIN_ZOOM = 1.5` (new dedicated gate class — no longer the `prop` band) with a **`STRIP_MIN_PX = 1.5`** screen-width clamp so strips stay hairline-visible at the gate boundary.
2. **Fence yaw correctness:** strip endpoints derived from the **same** corner math as the fill (`obb_corners` frame) — long axis = the OBB edge pair with greater length in the *rotated* frame; parity test across all 255 fence prefabs (strip ≡ fill long axis within 0.5°).
3. **Piers draw:** dedicated pier lane — aspect ≥ 4 → thin strip (unchanged); aspect < 4 → **capped-width strip** along the long axis (width = min(hy,hx)×2 clamped to `PIER_STRIP_MAX_WIDTH_M = 6.0`) so every pier/dock renders a quay. Pier visibility = `toggle_buildings && z ≥ −1.0` (`PIER_MIN_ZOOM`) — decoupled from the Fences toggle and prop band.
4. **Railings decision:** implement proximity railings (fence-strip styling for fence props within `BRIDGE_RAILING_RADIUS_M` of a bridge OBB) **or** delete the constant + spec language — operator picks via M-row; default path = implement (data shows 39/144 have candidates; remainder get short synthetic rail strips along deck edges).
5. **Bridge deck styling:** distinct deck treatment (outline + slightly widened casing along the crossing axis) so bridges read as bridges, not gray buildings.
6. **De-vacuous gates:** pier census gate `pier_strips_drawn ≥ 2,299 × 0.99`; fence orientation parity gate; both FAIL on empty sets.
7. Verify log `.ai/artifacts/t152_15_verify_log.md`.

---

## Out of scope

- Re-measuring prefab OBBs in Workbench (data as committed; note `map_export_everon.json` census drift is uncommitted in the worktree — leave it).
- Icon/badge art (T-152.18), landmark zoom (T-152.21).
- Prop-class glyphs beyond fences.
- TS beyond (at most) a thin new toggle-plumb if a pier pref is split out.

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | New gate classes in `lod_gates.rs`: `fence` (≥ **1.5**), `pier` (≥ **−1.0**) — `prop` band untouched for real props | Fences/piers are cartography, not clutter props |
| L2 | `STRIP_MIN_PX = 1.5` screen clamp for fence/pier strip width (`max(width_m·2^z, STRIP_MIN_PX px)` at compose or shader) | 0.35 m ≈ invisible below z≈2 |
| L3 | Pier fallback strip: width = `min(hx,hy)×2` clamped ≤ **6.0 m**; length = long axis; **every** pier/dock instance emits exactly one strip | Kill A1; O3 passable |
| L4 | Pier lane keyed to `toggle_buildings` only; fence lane to `toggle_fences` only — no cross-coupling | Gate hygiene |
| L5 | Strip frame derived via `obb_corners` midpoints (long-edge midpoint pair) — single source of truth with fill; parity test ≤ 0.5° over all fence + pier prefabs | S2 fix by construction |
| L6 | Railings default = implement path A (proximity ≤ 8 m) + synthetic deck-edge rails when no candidate; M-row lets operator downgrade to delete | Data says 39/144 only |
| L7 | Draw order preserved: deck → strips → glyphs (T-152.4 L7) | No z-fights |
| L8 | Commit `T-152.15:` · tag `T-152.15` · verify log | House convention |

---

## Pinned numbers

| Quantity | Value | Source |
|----------|-------|--------|
| Fence census | 255 prefabs / 36,204 instances | `t152_4_verify_log.md:36-38` |
| Pier/dock census | 2,299 instances, max aspect 2.57 | same |
| Bridge census | 9 prefabs / 144 instances | same |
| `FENCE_MIN_ZOOM` | **1.5** (new) | This slice |
| `PIER_MIN_ZOOM` | **−1.0** (new) | This slice |
| `STRIP_MIN_PX` | **1.5** | This slice |
| `PIER_STRIP_MAX_WIDTH_M` | **6.0** | This slice |
| `BRIDGE_RAILING_RADIUS_M` | 8.0 (finally consumed or deleted) | `cartographic_strip.rs:14` |

---

## Tasks

1. `lod_gates.rs`: add `fence`/`pier` classes + tests; rewire `fences_visible`/new `piers_visible` in `residency.rs` (kill cross-coupling at `:752,762`).
2. `cartographic_strip.rs`: rebuild strip frame from `obb_corners` long-edge midpoints; pier fallback path (L3); px clamp (L2 — likely in compose given ortho zoom known per rebuild).
3. Railings: proximity association pass over bridge OBBs + synthetic deck-edge rails; style hook.
4. Bridge deck styling (outline/casing) in the fill/outline lane.
5. Tests: orientation parity (all fence+pier prefabs), pier census ≥ 99 %, zoom gates, hysteresis-free rebuild correctness; update the vacuous T-152.4-era tests.
6. Verify suite + verify log + commit + tag.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | `class_visible("fence", 1.5)=true`, `(1.49)=false`; `class_visible("pier", −1.0)=true`, `(−1.01)=false` | LOD |
| **G2** | Orientation parity: ∀ fence/pier prefab × sample yaws {0°,37°,90°,123°}: strip axis ≡ fill OBB long axis within 0.5° | Class R |
| **G3** | Pier census: strips emitted ≥ **0.99 × 2,299** on Everon; **gate FAILS if 0** (anti-vacuous clause asserted in test) | Census |
| **G4** | Fence strip screen width ≥ 1.5 px at z=1.5 (compose output check) | Pixel floor |
| **G5** | Railings: every bridge instance has ≥ 2 rail strips (proximity or synthetic) — or verify log records operator delete-decision + constant removed | Railing |
| **G6** | Fences toggle off → 0 fence strips, piers unaffected; Buildings toggle off → 0 pier strips, fences unaffected | Decoupling |
| **G7** | cargo fmt/clippy (native+wasm)/tests, `make wasm`, FE test/build/lint exit 0 | Regression |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
cargo fmt --check && cargo clippy --all-targets -- -D warnings
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
cargo test -p map-engine-core      # G1–G6
make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint
```

---

## Manual acceptance

- **M1:** Harbor (e.g. Saint-Philippe docks) @ z=0 — quays visible as thin strips.
- **M2:** Rural fence lines readable from z≈1.5; orientation follows field boundaries (no 90° combs).
- **M3:** Bridge crossing — deck + rails distinct from buildings.
- **M4:** Railing decision recorded (keep A / delete) — operator.

---

## Documentation sync (Cursor, after merge)

Registry `T-152.15 → shipped`; hub row; `./scripts/ticket sync`.

---

## Claude Code prompt — T-152.15 (copy-paste)

Authority: this spec. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the T-152 worktree:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.15** — fence/pier/bridge visibility + orientation remediation.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  git lfs pull && make map-assets-link && make wasm

═══ READ (in order — spec wins) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_15_fence_pier_bridge_visibility.md
  2. .ai/artifacts/t152_11_fidelity_audit_report.md §5 + §6.2
  3. crates/map-engine-core/src/world/cartographic_strip.rs
  4. crates/map-engine-core/src/world/residency.rs (:325-327,:697-700,:752-810)
  5. crates/map-engine-core/src/world/{obb.rs,lod_gates.rs}
  6. .ai/artifacts/t152_4_verify_log.md (census + vacuous G4 history)

═══ PROBLEM ═══
  Fences gated z≥3 @0.35 m (invisible); yaw 90° suspects; piers: 0/2,299 draw (fill skipped,
  aspect gate vacuous, cross-coupled to fence gate); railings = dead constant; bridge deck generic.

═══ SHIPPED (do not reopen) ═══
  T-152.4 strip infrastructure (reuse expand_polyline_strip); .12–.14 lanes.

═══ LANGUAGE GATE ═══
  Rust OWNS: gates, strip math, parity, census. TS: at most thin pref plumb.
  STOP IF: geometry/policy creeping into apps/**.

═══ LOCKED ═══
  - New lod classes fence ≥1.5 / pier ≥−1.0; prop band untouched
  - STRIP_MIN_PX=1.5; PIER_STRIP_MAX_WIDTH_M=6.0; every pier draws
  - Strip frame from obb_corners long-edge midpoints (parity ≤0.5° gate)
  - Pier↔buildings toggle, fence↔fences toggle (no cross-coupling)
  - Railings: implement proximity+synthetic OR operator-approved delete (record in log)
  - Anti-vacuous: census gates FAIL on empty sets

═══ DO ═══
  1. lod_gates fence/pier classes + residency rewire
  2. Strip frame rebuild + pier fallback + px clamp
  3. Railings + bridge deck styling
  4. Parity/census/decoupling tests (G1–G6)
  5. Verify suite; .ai/artifacts/t152_15_verify_log.md; commit "T-152.15: ..."; tag T-152.15

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/**
  - Commit .ai/artifacts/map_export_everon.json drift (pre-existing; leave)
  - Re-export from Workbench (data as committed)

═══ VERIFY (all exit 0) ═══
  (bash block from spec §Verify)

═══ MANUAL ═══
  M1–M4 per spec

═══ RETURN ═══
  - Commit SHA + tag; verify log path
  - Pier strip census (expect ≈2,299) + parity worst-case angle
  - Railing path decision
```
