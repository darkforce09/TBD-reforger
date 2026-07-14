# T-152.21 — Landmark early visibility (`importanceZoom` wired)

**Ticket:** T-152 · **Slice:** T-152.21 (remediation ladder #10)
**Status:** `shipped` · **Tag:** `T-152.21` @ `d5c746df`
**Executor:** **claude-code** (Claude Code)
**Authority:** T-152 program hub · audit [`t152_11_fidelity_audit_report.md`](../../../.ai/artifacts/t152_11_fidelity_audit_report.md) §10 A4/A5 (P1–P3) · [`t090_render_lod_contract.md`](t090_render_lod_contract.md) (importanceZoom contract)
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152` · tag **`T-152.21`**
**Depends on:** T-152.18 preferred (Reforger icon art) but not required — works with current atlas

## In one sentence

Wire the `importanceZoom` prefab override — defined in schema, populated in classify rules (−4 for landmarks), specified in the LOD contract, parsed into `PrefabInfo`, and then read by **nothing** — so lighthouses/castles/military landmarks surface at island zoom instead of z ≥ +1, closing the original "white rectangles at default zoom" complaint.

---

## Problem

Audit A4/A5. Buildings draw as OBB fills from z ≥ −2.5 and get glyphs only at `BUILDING_BADGE_MIN_ZOOM = 1.0` (`lod_gates.rs:14,16`) — at the Mission Creator's default zoom (−2) every landmark is a rectangle, which is P1's original symptom. The designed remedy exists in four layers and is dead in all of them: schema `render.importanceZoom`; `prefab-classify.json` landmark rules set −4; `t090_render_lod_contract.md` specifies "visible when `deckZoom ≥ importanceZoom` even if class gate is higher"; Rust parses it (`prefab.rs:28,88`) — and no render path consults it. TS helper `landmarkVisible()` (`lodGates.ts:112-115`) has zero callers.

---

## Goal

1. **Wire it (Rust):** badge/glyph emission (`residency.rs` badge path, `:868` gate) treats a prefab with `importance_zoom = Some(v)` as visible when `deck_zoom ≥ v`, overriding the `buildingBadge` class gate (per the T-090 contract semantics).
2. **Glyph size floor at coarse zoom:** early landmarks render at `max(displayPx, BADGE_SIZE_MIN_PX)` (existing `glyph_math.rs:8` floor) so a lighthouse at z=−4 is a readable badge, not a sub-pixel dot.
3. **Fill de-emphasis:** landmark classes with an active early glyph drop the bright OBB tint at z < badge band (glyph replaces rectangle, no double-draw shout) — lighthouse `[235,235,235,220]` white fill (P1) no longer the landmark's coarse-zoom face.
4. **Oracle cleanup:** either wire the TS `landmarkVisible` oracle into parity tests or delete it — no dead policy mirrors.
5. Verify log `.ai/artifacts/t152_21_verify_log.md`.

---

## Out of scope

- Changing `BUILDING_BADGE_MIN_ZOOM` for non-landmark classes.
- Icon art (T-152.18); label lanes.
- Adding new importanceZoom values to classify rules beyond what data already carries (data change only if a landmark class verifiably lacks the field — record it).

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | Override semantics exactly per `t090_render_lod_contract.md`: `deckZoom ≥ importanceZoom` ⇒ glyph visible, else class gate applies | Contract already written |
| L2 | Landmark early-glyph set = prefabs whose classify rules carry `importanceZoom` (today −4) — data-driven, no hardcoded class list in Rust | Single source |
| L3 | Coarse-zoom fill de-emphasis only when the early glyph actually drew (state-conditioned like .14's mass handoff) | No invisible landmarks |
| L4 | Class R test: synthetic prefab with `importance_zoom=−4` → badge instance at z=−4, none at z=−4.1; Everon fixture: lighthouse chunk emits early badge | Proof |
| L5 | TS `landmarkVisible` becomes parity-test-only or deleted (pick in-slice; record) | No dead mirrors |
| L6 | Commit `T-152.21:` · tag `T-152.21` · verify log | House convention |

---

## Pinned numbers

| Quantity | Value | Source |
|----------|-------|--------|
| `BUILDING_BADGE_MIN_ZOOM` | 1.0 (unchanged for non-landmarks) | `lod_gates.rs:16` |
| Landmark importanceZoom in data | −4 | `prefab-classify.json` rules |
| `BADGE_SIZE_MIN_PX` | 8.0 | `glyph_math.rs:8` |
| Parsed-but-dead field | `PrefabInfo.importance_zoom` | `prefab.rs:28,88` |

---

## Tasks

1. `residency.rs` badge emission: per-prefab override check (L1/L2) + size floor application.
2. Fill de-emphasis conditioned on early-glyph state (L3).
3. Tests: Class R override edges, lighthouse fixture, fill-deemphasis state; oracle decision (L5).
4. Verify suite + verify log + commit + tag.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | Synthetic prefab importance_zoom=−4: badge at z=−4.0, absent at z=−4.1; non-landmark unchanged (badge only z ≥ 1) | Class R |
| **G2** | Everon fixture chunk containing lighthouse: `badge_glyph_count > 0` at z=−2 (default editor zoom) | Fixture |
| **G3** | Early badge display size ≥ `BADGE_SIZE_MIN_PX` at z=−4 | Size floor |
| **G4** | Lighthouse OBB bright fill suppressed at z < 1 when early glyph drew; restored if glyphs empty | Handoff |
| **G5** | `landmarkVisible` TS: wired into a parity test **or** deleted (grep gate: no un-called export remains) | Hygiene |
| **G6** | cargo/wasm/FE suites exit 0 | Regression |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
cargo fmt --check && cargo clippy --all-targets -- -D warnings
cargo test -p map-engine-core     # G1–G4
make wasm
cd apps/website/frontend && npm test && npm run build && npm run lint
rg -n "landmarkVisible" apps/website/frontend/src   # G5 evidence
```

---

## Manual acceptance

- **M1:** Everon @ z=−2 (default) — lighthouses/castles/military read as icon badges, not white rectangles.
- **M2:** Zoom out to −4 — landmark badges persist, ordinary buildings gone.

---

## Documentation sync (Cursor, after merge)

Registry `T-152.21 → shipped`; hub row (P1–P3 closure note); `./scripts/ticket sync`.

---

## Claude Code prompt — T-152.21 (copy-paste)

Authority: this spec. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the T-152 worktree:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.21** — landmark early visibility (importanceZoom wired).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  make wasm && cargo test -p map-engine-core 2>&1 | tail -3

═══ READ (in order — spec wins) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_21_landmark_early_visibility.md
  2. .ai/artifacts/t152_11_fidelity_audit_report.md §10 A4/A5
  3. docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md (importanceZoom semantics)
  4. crates/map-engine-core/src/world/{prefab.rs,residency.rs,glyph_math.rs,lod_gates.rs}
  5. packages/tbd-schema/rules/prefab-classify.json (importanceZoom rows)
  6. apps/website/frontend/src/features/tactical-map/worldmap/lodGates.ts (:112-115 dead helper)

═══ PROBLEM ═══
  importanceZoom designed in 4 layers (schema/rules/contract/parse) and read by nothing —
  landmarks are white rectangles at default zoom −2 (original P1 complaint).

═══ SHIPPED (do not reopen) ═══
  Badge atlas plumbing; class gates for non-landmarks; .15 strip lanes.

═══ LANGUAGE GATE ═══
  Rust OWNS override policy. TS: only the L5 oracle wire-or-delete.

═══ LOCKED ═══
  - deckZoom ≥ importanceZoom overrides buildingBadge gate (contract semantics)
  - Data-driven landmark set (no hardcoded class list)
  - BADGE_SIZE_MIN_PX floor at coarse zoom; fill de-emphasis state-conditioned
  - landmarkVisible wired-or-deleted (record)

═══ DO ═══
  1. Badge-path override + size floor
  2. Fill de-emphasis handoff
  3. Class R + lighthouse fixture tests
  4. Oracle decision
  5. Verify; .ai/artifacts/t152_21_verify_log.md; commit "T-152.21: ..."; tag T-152.21

═══ DO NOT ═══
  - Lower BUILDING_BADGE_MIN_ZOOM globally
  - Edit docs/**, .ai/tickets/**
  - Invent importanceZoom values absent from committed rules

═══ VERIFY (all exit 0) ═══
  (bash block from spec §Verify)

═══ MANUAL ═══
  M1–M2 per spec

═══ RETURN ═══
  - Commit SHA + tag; verify log path
  - Early-landmark census at z=−2 (badge count) + oracle decision
```
