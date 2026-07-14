# T-152.17 — Town label correctness (settlement-only lane + kind hygiene)

**Ticket:** T-152 · **Slice:** T-152.17 (remediation ladder #6)
**Status:** `shipped` · **Tag:** `T-152.17` @ `45e4d247`
**Executor:** **claude-code** (Claude Code)
**Authority:** T-152 program hub · audit [`t152_11_fidelity_audit_report.md`](../../../.ai/artifacts/t152_11_fidelity_audit_report.md) §10 A8/A12 (S4, D10)
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152` · tag **`T-152.17`**
**Depends on:** **T-152.12** (lane) · T-152.13 (font) · plays nicely with T-152.16 (heights get the hills/peaks)

## In one sentence

Make the town lane draw settlements only — correct the polluted `kind` data (sawmills/farms tagged `town`), stop drawing 33 hills/peaks as towns, and replace the hard hide-above-z=+2 with an operator-decided fade-or-keep.

---

## Problem

Audit A8/A12: `locations.json` = 60 rows (**town 23 · peak 17 · hill 16 · village 2 · natural 1 · airport 1**) and **all 60** draw on the town-label lane (`t152_8_verify_log.md:16` "60/60 drawn") — a third of "town names" are hills. At least four `kind:"town"` rows are sub-features (`everon-le-moule-sawmill-01`, `everon-montignac-farm-01`, `everon-montignac-sawmil-01` [sic], `everon-north-east-farm-01`) from the heuristic in `lib/locations-export.mjs:147`. The .6 verify log's prose contradicts its own data (Morton "village" vs `town`). And `TOWN_LABEL_MAX_ZOOM = 2.0` hides every name past z=+2 (`importance_declutter.rs:12-13`, `should_draw_town_label:69`) — "names vanish when I zoom in".

**Operator evidence (2026-07-13, post-.12) — two new requirements for this slice:**
1. **Band misses island view entirely.** At island-fit zoom (z≈−3.68 on a 1080p viewport) the map shows **zero** town names (`.ai/artifacts/t152_12_operator/island_z-3.68_no_labels.png`) because `TOWN_LABEL_MIN_ZOOM = −3.0`. Reforger's own in-game map shows names at wide zoom. Widen the min bound (propose **−4.5**, importance-gated so only high-importance towns draw at the widest band) — lock the value in-slice with the operator.
2. **Band not enforced live on zoom-in.** A location label ("MOUNTAINS WEST HILL …", a `hill` row) is visible at **z=4.48** (`upright_z4.48_font_quality.png`) — above `TOWN_LABEL_MAX_ZOOM = 2.0`. Suspect the town lane uploads once and is not re-decluttered/re-uploaded on zoom change (check `useWgpuTownLabels` subscription + wasm `declutter_town_labels_json` call site). Diagnose and fix as part of the band work; add a gate asserting drawn-count at z=3 is 0 (or fade-band behavior per L4).

---

## Goal

1. **Lane filter (Rust):** `should_draw_town_label` additionally requires `kind ∈ {town, village}`; airport keeps its label (styled via importance) — hills/peaks/natural excluded (they move to heights, T-152.16).
2. **Data hygiene:** fix the four sub-feature rows (`kind: "locality"` new enum value, importance ≤ 0.45) + the `sawmil` typo id note; reconcile Morton/Gorey kinds with the .6 importance table; regenerate via `lib/locations-export.mjs` classifier fix (not hand-editing the artifact where avoidable).
3. **Zoom-in behavior:** replace the hard `> TOWN_LABEL_MAX_ZOOM` cut with alpha fade over z ∈ [2.0, 3.0] (drawn but fading), unless the operator M-row picks "keep the hard hide" — decision recorded.
4. Verify log `.ai/artifacts/t152_17_verify_log.md`.

---

## Out of scope

- New name extraction (Path A — T-152.19).
- Heights-lane merge (T-152.16).
- Font/orientation (.12/.13).
- Declutter formula / importance scale retune.

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | Kind filter in Rust `importance_declutter.rs` (lane policy), not in the TS fetch | LANGUAGE GATE |
| L2 | `locality` added to the location kind vocabulary (schema additive); sub-features become `locality`, drawn small at z ≥ 0 only | Sawmills are map-real, just not towns |
| L3 | Classifier fix in `lib/locations-export.mjs` + regenerated artifact committed together (artifact stays reproducible) | No hand-edited artifacts |
| L4 | Fade band **[2.0, 3.0]** default; hard-hide fallback selectable by operator (M2) — constant `TOWN_LABEL_FADE_END = 3.0` | A12 without clutter regression |
| L5 | Required-towns gate (8 towns @ z=−2) must stay green after filtering | No regression on T-152.8 G2 |
| L6 | Commit `T-152.17:` · tag `T-152.17` · verify log | House convention |

---

## Pinned numbers

| Quantity | Value | Source |
|----------|-------|--------|
| Rows drawn today | 60/60 | `t152_8_verify_log.md:16` |
| Settlement rows (post-fix target) | 23 town + 2 village − 4 reclassified + airport = **22** on town lane | audit A8 |
| Band today | [−3, +2] hard | `importance_declutter.rs:11-13` |
| Fade band (new) | [2.0, 3.0] | This slice |

---

## Tasks

1. `importance_declutter.rs`: kind filter + fade math (alpha into pack tint) + tests.
2. `lib/locations-export.mjs`: classifier rules for sub-features → `locality`; regen `locations.json`; schema enum additive.
3. Update `verify-town-labels.mjs` gates (drawn-set = settlements; required-towns unchanged).
4. Tests: filter counts, fade endpoints, required-towns regression.
5. Verify suite + verify log + commit + tag.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | Drawn set @ z=−2 ⊆ kinds {town, village, airport, locality}; **0** rows kind ∈ {peak, hill, natural} drawn on town lane | Filter |
| **G2** | All 8 `REQUIRED_EVERON_TOWNS` still drawn @ z=−2 | Regression (T-152.8 G2) |
| **G3** | 4 sub-feature rows reclassified `locality`; regenerated artifact diff shows classifier provenance (no hand edit) | Data |
| **G4** | Fade: alpha(z=2.0)=1.0, alpha(z=2.5)≈0.5, alpha(z=3.0)=0.0 — or hard-hide + operator quote (M2) | Behavior |
| **G5** | `make schema-validate` green with `locality` enum | Schema |
| **G6** | cargo/wasm/FE suites exit 0 | Regression |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
node scripts/map-assets/export-locations.mjs --terrain everon    # regen (flags per actual CLI)
node scripts/map-assets/verify-town-labels.mjs
cargo test -p map-engine-core
make wasm && make schema-validate
cd apps/website/frontend && npm test && npm run build && npm run lint
```

---

## Manual acceptance

- **M1:** Island view — town names only over settlements; hills carry height labels instead.
- **M2:** Zoom past +2 — fade (or operator-chosen hard hide); decision recorded.

---

## Documentation sync (Cursor, after merge)

Registry `T-152.17 → shipped`; hub row; `./scripts/ticket sync`.

---

## Claude Code prompt — T-152.17 (copy-paste)

Authority: this spec. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the T-152 worktree:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.17** — town label correctness (settlement-only lane + kind hygiene).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  git log --oneline -6   # expect .12/.13 (and ideally .16) shipped
  make wasm

═══ READ (in order — spec wins) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_17_town_label_correctness.md
  2. .ai/artifacts/t152_11_fidelity_audit_report.md §10 A8/A12
  3. crates/map-engine-core/src/world/importance_declutter.rs
  4. scripts/map-assets/lib/locations-export.mjs (+ export-locations.mjs, verify-town-labels.mjs)
  5. packages/map-assets/everon/locations.json
  6. packages/tbd-schema (locations schema, if present — additive enum)

═══ PROBLEM ═══
  60/60 rows draw as towns (33 are hills/peaks); sawmills/farms tagged kind:"town";
  hard hide above z=+2 reads as "names vanish when zooming in".

═══ SHIPPED (do not reopen) ═══
  .12/.13 text lane; .16 heights merge (its lane owns hills/peaks).

═══ LANGUAGE GATE ═══
  Rust OWNS lane policy (filter/fade). Node script = classifier/data. Zero TS.

═══ LOCKED ═══
  - Filter kinds {town,village,airport,locality} in should_draw_town_label
  - locality enum additive; 4 sub-features reclassified via classifier fix + regen
  - Fade [2.0,3.0] default; operator may pick hard-hide (record M2)
  - 8 required towns must stay drawn @ z=−2

═══ DO ═══
  1. Rust filter + fade + tests
  2. Classifier fix + regen locations.json + schema enum
  3. verify-town-labels gate update
  4. Verify; .ai/artifacts/t152_17_verify_log.md; commit "T-152.17: ..."; tag T-152.17

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/**
  - Hand-edit locations.json without matching classifier change
  - Touch heights lane internals (T-152.16)

═══ VERIFY (all exit 0) ═══
  (bash block from spec §Verify)

═══ MANUAL ═══
  M1–M2 per spec

═══ RETURN ═══
  - Commit SHA + tag; verify log path
  - Drawn-set census by kind (before/after)
  - Fade-vs-hide decision record
```
