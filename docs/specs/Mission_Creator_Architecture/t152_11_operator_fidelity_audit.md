# T-152.11 — Operator fidelity audit (analysis only)

**Ticket:** T-152 · **Slice:** T-152.11  
**Status:** `ready`  
**Executor:** **claude-code** (analysis report only — **no application code**)  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152`  
**Depends on:** T-152.0–.10 shipped (automated); operator O1–O12 may still be open  
**Blocks:** Remediation slices **T-152.12+** (filed after this audit returns)

## In one sentence

Honest **code + data + provenance audit** of everything T-152 shipped, driven by operator symptoms (fences, text, towns, **missing trees on zoom-in**, Workbench one-button extract gap) — produce a fix matrix with file:line evidence; **do not patch code in this slice**.

---

## Program deliverable ledger (AUTHORITATIVE — audit every row)

Claude **must** reproduce this table in the report with columns: **Intended (plan/hub)** · **Shipped (evidence)** · **Status** (`DONE` / `PARTIAL` / `MISSING` / `WRONG_PATH`) · **Operator-visible?** · **Follow-up slice**.

Source of “Intended”: approved plan locked approaches + hub problem table P1–P8 + slice specs `.1`–`.10`. Do **not** invent new product scope; do **not** treat automated Gn PASS as operator-visible.

### Icons (operator escalation — do not soft-pedal)

| | |
|--|--|
| **Intended** | **Extract** Reforger map icons **where possible**; redraw **only gaps**; rebuild atlas; wire (.2→.3) |
| **Shipped** | MCP spike **TIMEOUT** → **all 21** `LANDMARK_SET` keys `source:redraw` with `reforgerRef: null` — [`.ai/artifacts/t152_2_icon_discovery.json`](../../../.ai/artifacts/t152_2_icon_discovery.json) |
| **Status** | **`WRONG_PATH`** — redraw-all without operator permission to abandon extract; extract never completed |
| **Audit must** | Quote discovery JSON; list every key as redraw; score Workbench extract readiness; propose remediation that **retries extract with operator Workbench warm** (not another silent redraw) |

### Full intended → shipped matrix

| ID | Intended deliverable | Slice | What shipped (honest) | Status |
|----|----------------------|-------|----------------------|--------|
| **D1** | wgpu text lane + declutter | .1 | Path B baked ASCII atlas + declutter; GPU draw deferred to consumers | `PARTIAL` (lane exists; fidelity low) |
| **D2** | Extract Reforger icons + redraw gaps only; atlas | .2 | **0 extracted / 21 redraw**; MCP FAIL | **`WRONG_PATH`** |
| **D3** | Wire landmarks → non-OBB glyphs @ locked zoom | .3 | Wired @ `BUILDING_BADGE_MIN_ZOOM (+1)` — still OBB rectangles at default −2 | `PARTIAL` |
| **D4** | Fence thin strips (P5) | .4 | Strips ship; LOD `PROP_MIN_ZOOM=3` (too late); rotation suspect | `PARTIAL` |
| **D5** | Pier/dock thin strips | .4 | **0 strips** (aspect vacuous); fills suppressed → **invisible** | **`MISSING`** |
| **D6** | Bridge deck + glyph + railings | .4 | Deck+glyph; railings weak (39/144 proximity) | `PARTIAL` |
| **D7** | Airfield runway polish + apron + hangar/tower | .5 | Shipped; **no taxiways** (Path B) | `PARTIAL` |
| **D8** | `locations.json` from World/Locations (Workbench spike) | .6 | Path B staged JSONL + CfgWorlds; Path A plugin unproven one-button | `PARTIAL` |
| **D9** | **Height / elevation markers** on map (+ optional contour labels) | .7 | Data: **10** DEM peak labels in `height-labels.json`; contour labels **waived**; operator reports **missing on map** (likely text orientation / visibility / zoom / toggle — **must verify**) | `PARTIAL` / possibly **`MISSING` visually** |
| **D10** | Town names from locations + declutter | .8 | Drawn from all 60 rows (incl. hills); ASCII unreadable; orientation broken; hide @ z>+2 | `PARTIAL` |
| **D11** | Road names (spike → extract or curated fallback) | .9 | **Curated 6** majors only — no extract | `PARTIAL` / Path B stub |
| **D12** | E2E + operator cartographic sign-off | .10 | Automated PASS; **O1–O12 PENDING** | `PARTIAL` |
| **D13** | Tree glyphs readable when zoomed in (T-151 contract, not T-152 slice but operator S7) | T-151.5/8 | Glyphs exist; heatmap/budget + LOD handoff can clear them | `PARTIAL` (regression vs expectation) |

### Hub problem table P1–P8 — closure check

Audit must mark each hub **P#** as `CLOSED` / `OPEN` / `PAPERED` (gates pass, operator still sees original symptom):

| P# | Original symptom | Expected close |
|----|------------------|----------------|
| P1–P3 | Landmark white squares / missing glyphs | .2+.3 — still PAPERED at island zoom |
| P4 | Placeholder SVGs | .2 — PAPERED if redraw≠extract |
| P5 | No text lane | .1 — CLOSED as infrastructure; OPEN as cartographic font |
| P6 | Pier/bridge fat; fences missing | .4 — fences PARTIAL; **piers OPEN/MISSING** |
| P7 | Airfield white runway only | .5 — mostly CLOSED minus taxiways |
| P8 | No town/road/height labels | .6–.9 — PARTIAL; heights may be **visually OPEN** |

### Explicit “meant to add” checklist (tick during audit)

```text
[ ] Text GPU lane usable for labels
[ ] LANDMARK_SET icons — EXTRACTED from Reforger (not redraw-only)
[ ] Landmark glyphs visible (not white OBB-only) at agreed zoom
[ ] Fences as thin strips, correct yaw, visible at sensible zoom
[ ] Piers/docks as thin strips (harbor readable)
[ ] Bridges: deck + icon + rails
[ ] Airfield: runway + apron + hangar/tower (+ taxiway if data exists)
[ ] locations.json complete + Workbench one-button path proven
[ ] Elevation/height markers VISIBLE on map (peaks; contour labels if not waived with operator quote)
[ ] Town names readable + oriented correctly + town-only (not hills)
[ ] Road names (extract preferred; curated only with operator-visible honesty)
[ ] Tree glyphs remain when zooming into forest
[ ] Operator O1–O12 signed
```

Also note: T-152 text-uniform 16 B hotfix may be on tip — confirm S3 is remaining orientation bug vs pre-hotfix panic.

---

## Problem (operator report — 2026-07-13)

Operator visual pass after T-152.10 automated gate:

| ID | Symptom |
|----|---------|
| **S1** | Fences require zooming in **far too close** (near “100% magnification”) before they appear — should be readable earlier |
| **S2** | Some fence strips **wrong rotation / placement** (looks like **90°** errors; other instances also misaligned) |
| **S3** | Map **text is upside-down and/or mirrored** (back-to-front) — town/road/height labels |
| **S4** | **Not all towns** show names; remaining labels are **hard to read** even if orientation were correct (atlas/font fidelity) |
| **S5** | Overall cartographic symbology still feels incomplete vs Reforger Map view |
| **S6** | **Name provenance glossed over** — north star is: open Workbench → press export button → perfect extract. Interim Path B curated / staged-jsonl paths must be called out as **not** that end state |
| **S7** | When **zooming in**, **tree glyphs disappear / are missing** — detail zoom should show trees, not lose them |
| **S8** | **Elevation / height markers missing** on the map (operator cannot see peak ASL labels) — treat as visual MISSING even if `height-labels.json` exists |
| **S9** | **Icons were redrawn, not extracted** — operator did **not** approve abandoning Reforger extract; silent redraw-all is unacceptable |

### Mandatory analysis addenda (beyond S1–S9 — do not skip)

These are high-confidence ship/process gaps from verify logs + LOD contract. Audit must cover each with file:line + fix-matrix row even if operator did not name them aloud.

| ID | Topic | Why |
|----|-------|-----|
| **A1** | **Piers/docks invisible** | T-152.4 G4 **vacuous**: max pier OBB aspect **2.57 < 4.0** → **0** thin strips; pier/dock OBB fills suppressed → **~2,299** harbor instances draw **nothing** (O3 will fail) |
| **A2** | **Tree heatmap vs glyphs** | `INSTANCE_BUDGET = 150_000`: when viewport tree count exceeds budget, density heatmap **clears** individual tree glyphs — prime S7 root cause |
| **A3** | **Forest mass ↔ tree LOD handoff @ z=0** | Forest fill/outline **off** when trees **on** (`TREE_GLYPH_MIN_ZOOM=0`); if heatmap/cull empty → green mass becomes blank |
| **A4** | **Buildings still rectangles at default zoom (−2)** | Footprints from z≥−2.5; landmark **glyphs only @ z≥+1** — original “white squares / rectangles” complaint still true at island view |
| **A5** | **`importanceZoom` unwired** | Prefab early-landmark override exists in lodGates docs but T-152.3 did not wire — lighthouses stay late |
| **A6** | **Contour elevation labels waived** | T-152.7 G-contour **operator waived** — hub locked contour labels; only 10 peak ASL labels shipped |
| **A7** | **Icons = redraw, not Reforger extract** | T-152.2 MCP **TIMEOUT**; all LANDMARK_SET `source:redraw` — same as **S9** / ledger **D2 `WRONG_PATH`**; remediation = extract with warm Workbench, **not** more redraw |
| **A8** | **`locations.json` kind pollution** | 60 rows include hills/peaks/airport; town lane may label non-towns; peaks duplicate height markers |
| **A9** | **Road names = 6 curated only** | Vast majority of 888 segments unnamed — Path B curated is a stub vs Reforger map literacy |
| **A10** | **Taxiways absent** | T-152.5 Path B — runway+apron only |
| **A11** | **ASCII 8×8 text atlas** | Path B baked atlas — readability ceiling for towns/roads even if orientation fixed |
| **A12** | **Town labels hide above z=+2** | Can feel like “names vanish when I zoom in” (distinct from S4) |
| **A13** | **Bridge railings weak** | Path B census; only **39/144** bridges have fence within 8 m |
| **A14** | **Vacuous/waived gate honesty** | List every automated PASS that is vacuous or waived (G4 pier, G-contour, taxiway B, railing B) — do not treat as visual PASS |
| **A15** | **Mission Settings toggles incomplete vs O10** | Dialog may only expose T-152 prefs; trees/buildings/props not all operator-toggleable |
| **A16** | **All Mn + O1–O12 still PENDING** | Automated ship ≠ operator cartographic sign-off |

Also note: T-152.7 hotfix for `TextUniforms` 16 B vs 32 B (`vec3` pad) may already be on the worktree tip — audit must confirm whether S3 is **remaining orientation bug** vs residual pre-hotfix panic.

---

## Goal

1. Fill the **Program deliverable ledger** (D1–D13 + P1–P8 + meant-to-add checklist) — every row Status + Operator-visible?
2. Read **every** T-152 slice verify log + spike JSON + key Rust/TS/WGSL paths.
3. Write [`.ai/artifacts/t152_11_fidelity_audit_report.md`](../../../.ai/artifacts/t152_11_fidelity_audit_report.md) with:
   - **§Program deliverable ledger** (copy of D1–D13 filled in) — **first section after summary**
   - Per-symptom root-cause hypotheses with **file:line** (S1–S9 + A1–A16)
   - **Provenance ledger** for towns / roads / heights / icons (Path A vs B vs C; one-button Workbench gap)
   - **LOD / zoom table** for fences **and tree glyphs** vs operator expectation
   - Height-marker visibility trace (data exists vs drawn vs readable)
   - **Fix matrix** → proposed **T-152.12+** slices (no silent deferral; icon extract retry explicit)
4. Update program hub “open remediation” pointer (Cursor after report lands — Claude does **not** edit registry).

---

## Out of scope

- Any edit under `apps/`, `crates/`, `packages/map-assets/` (except **reading**).
- Merging `ticket/T-152` → `main`.
- Signing operator O1–O12 as PASS.
- Implementing remediation (that is **T-152.12+**).

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | **Analysis only** — zero application code commits in this slice | User: don’t edit code now |
| L2 | Every finding needs **file:line** or artifact path + quote | No vibes |
| L3 | Provenance section must answer: “If I press the Workbench button tomorrow, what still fails?” | North star |
| L4 | Fence LOD: document current `PROP_MIN_ZOOM` / strip width vs S1; propose locked new zoom/width in fix matrix | Operator S1 |
| L5 | Text orientation: trace `vs_text` / yaw / UV / pack path for mirror+flip | Operator S3 |
| L6 | Town coverage: reconcile `locations.json` (60 rows) vs drawn set vs operator “missing towns” | Operator S4 |
| L7 | Tag **T-152.11** on the **report commit only** (docs/artifacts) | Convention |
| L8 | Remediation stays sequential: do **not** start code fixes until Cursor files T-152.12+ from the matrix | Gate |
| L9 | Tree glyphs: audit LOD / density ladder / cull / residency so **zoom-in does not drop trees** (S7) — cite `lodGates`, tree stream, INSTANCE_BUDGET, compute cull | Operator S7 |

---

## Known provenance (seed — audit must deepen, not paper over)

| Data | Shipped path | Honest gap |
|------|--------------|------------|
| **Towns** | T-152.6 **Path B**: `raw-entities.jsonl` Location compositions + CfgWorlds name crosswalk → `locations.json` (60). Path A plugin `TBD_LocationsExportPlugin.c` authored; MCP blocked in agent CI | Not yet “press button in Workbench → commit artifact” E2E |
| **Roads** | T-152.9 **Path B**: curated `road-names.json` (6 MAJOR_EVERON_ROADS). Spike: no `name` on `roads.json.gz`; `.topo` attrs not UTF-8 names | **Fully curated** — furthest from one-button extract |
| **Heights** | DEM local maxima + `sample_elevation` → `height-labels.json` | Algorithmic — not Eden location names |
| **Icons** | T-152.2 redraw + atlas; MCP icon discovery timed out → redraw path | Not extracted Reforger UI assets end-to-end |
| **Fences** | T-152.4 P5_props export + thin strips; LOD **`deckZoom ≥ 3`** (prop band); width **0.35 m** | Matches S1 “must zoom way in”; orientation from OBB long axis — S2 suspects yaw/axis bug |

Spike artifacts: [`.ai/artifacts/t152_6_locations_spike.json`](../../../.ai/artifacts/t152_6_locations_spike.json), [`.ai/artifacts/t152_9_road_name_spike.json`](../../../.ai/artifacts/t152_9_road_name_spike.json).

---

## Tasks

1. Inventory tip SHAs / tags T-152.0–.10 (+ any text-uniform hotfix).
2. Trace fence strip orientation + LOD gates.
3. Trace text pack / `vs_text` / UV / yaw for mirror+flip.
4. Census towns: schema list vs UI readability (atlas 16×6 ASCII).
5. Provenance ledger + Workbench one-button gap list.
6. Write audit report + proposed slice ladder T-152.12+.
7. Commit docs/artifacts only; tag **T-152.11**.

---

## Mathematical acceptance matrix

| Gate | Predicate |
|------|-----------|
| **G1** | Report exists at `.ai/artifacts/t152_11_fidelity_audit_report.md` |
| **G2** | Report opens with **§Program deliverable ledger** (D1–D13 filled) + P1–P8 closure + meant-to-add checklist ticked; then Symptoms · Provenance · LOD · Text · Trees · Piers · Heights visibility · Vacuous gates · Fix matrix (≥1 row per **S1–S9**, **A1–A16**, and every **MISSING/WRONG_PATH/PARTIAL** D-row) |
| **G3** | ≥ **12** distinct `path:line` citations across crates / FE / scripts / artifacts |
| **G4** | Provenance ledger states Path **A/B/C** for towns **and** roads with spike JSON quotes |
| **G5** | Fence LOD row quotes current min zoom constant + file:line |
| **G6** | Text orientation section names the suspect transform (yaw / UV / pack / MVP) with file:line |
| **G7** | Zero application-code file diffs in the ship commit (`git show --stat` apps/crates/packages empty of logic) |
| **G8** | `./scripts/ticket check` still OK after Cursor registry sync (Cursor may sync after Claude returns) |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
test -f .ai/artifacts/t152_11_fidelity_audit_report.md
rg -n 'Provenance|Fix matrix|PROP_MIN_ZOOM|vs_text|Path B|tree|TREE' .ai/artifacts/t152_11_fidelity_audit_report.md
# G7: ship commit must not touch engine/app source
```

---

## Manual

Operator may attach screenshots under `.ai/artifacts/t152_11_shots/` (optional). Claude must not invent screenshot claims.

---

## Documentation sync (Cursor, after report)

- Registry: T-152.11 → shipped; file T-152.12+ from fix matrix (separate Mode B pass).
- Hub: point Active → remediation ladder.
- Do **not** mark T-152 program `done` until remediations + O1–O12.

---

## Claude Code prompt — T-152.11 (copy-paste)

Authority: this spec + handoff. **Do not edit application code. Do not edit registry.**

```
Read CLAUDE.md first.

CWD: /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
Branch: ticket/T-152

Implement **T-152.11** — Operator fidelity audit (ANALYSIS ONLY — no app code).

═══ PREFLIGHT ═══
  pwd; git status -sb; git log --oneline -15; git tag -l 'T-152*'
  test -f docs/specs/Mission_Creator_Architecture/t152_11_operator_fidelity_audit.md

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t152_11_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t152_11_operator_fidelity_audit.md
  3. docs/specs/Mission_Creator_Architecture/t152_map_cartographic_fidelity_program.md
  4. ALL .ai/artifacts/t152_*_verify_log.md and t152_*_spike.json / t152_*_icon_discovery.json
  5. Fence: crates/... residency / strip / lodGates; T-152.4 spec
  6. Text: crates/map-engine-render/src/shader.wgsl vs_text; text_layout.rs; engine.rs TEXT_UNIFORM
  7. Towns/roads: packages/map-assets/everon/locations.json; road-names.json; export-locations.mjs; verify-town-labels.mjs
  8. Provenance spikes: .ai/artifacts/t152_6_locations_spike.json; t152_9_road_name_spike.json
  9. Trees: lodGates / tree glyph stream / density ladder / INSTANCE_BUDGET / compute cull / WorldResidency tree path (T-151.5 / T-151.8)

═══ PROBLEM ═══
  Automated T-152.0–.10 shipped, but operator says cartography still broken:
  fences only at extreme zoom + wrong rotation; text upside-down/mirrored; towns missing/unreadable;
  name sourcing glossed vs Workbench one-button extract; **tree glyphs vanish when zooming in**;
  **elevation markers missing on map**; **icons were redrawn not extracted (WRONG_PATH)**.
  ALSO mandatory: Program deliverable ledger D1–D13; piers invisible; INSTANCE_BUDGET heatmap;
  buildings still rectangles at default zoom; ASCII text; waived contours; curated roads; incomplete Settings.
  Need a ruthless audit report + fix matrix — NOT patches.

═══ SHIPPED (do not reopen as code in this slice) ═══
  T-152.0 docs · .1 text lane · .2 icons · .3 landmark wire · .4 fence/pier/bridge ·
  .5 airfield · .6 locations Path B · .7 heights · .8 towns · .9 roads Path B curated ·
  .10 E2E automated · optional TextUniforms 16B hotfix on tip

═══ LANGUAGE GATE ═══
  N/A for code — you write MARKDOWN ONLY.
  If you find a bug, document file:line + proposed fix; do not apply it.

═══ LOCKED ═══
  - Analysis only (L1)
  - file:line evidence (L2)
  - Honest Workbench one-button gap (L3)
  - Fence LOD + orientation called out (L4–L5)
  - Town coverage vs locations.json (L6)
  - Tree glyphs must not disappear on zoom-in (L9 / S7) — check heatmap + LOD handoff
  - Cover mandatory addenda A1–A16 (piers, heatmap, rectangles, provenance, waived gates)
  - **Program deliverable ledger D1–D13 is mandatory** — Intended vs Shipped vs Status
  - Icon row D2/S9 = WRONG_PATH (redraw-all); remediation = extract w/ operator Workbench — do NOT propose more silent redraw
  - Height markers D9/S8 — verify VISIBLE on map, not just JSON on disk
  - Propose T-152.12+ slices; do not implement (L8)

═══ DO ═══
  1. Write .ai/artifacts/t152_11_fidelity_audit_report.md — FIRST section = filled Program deliverable ledger (D1–D13) + P1–P8 + meant-to-add checklist
  2. Include a “Workbench one-button extract readiness” scorecard (towns/roads/icons/fences)
  3. Fix matrix: for each S1–S9 AND A1–A16 AND every non-DONE D-row → proposed slice id, files, acceptance gates
  3b. Dedicated sections: tree glyph vs zoom (heatmap/budget); pier/dock invisibility; height-marker visibility; vacuous/waived gate ledger; icon extract failure (quote t152_2_icon_discovery.json)
  3c. Full LOD zoom table reproduced from lod_gates.rs / lodGates.ts
  4. Commit ONLY docs/artifacts (and this report). Message: T-152.11: Operator fidelity audit report
  5. Tag T-152.11
  6. STOP — do not start remediation code

═══ DO NOT ═══
  - Edit apps/**, crates/**, packages/map-assets/** (except reading)
  - Edit .ai/tickets/registry.json or generated docs/TICKET_*
  - Mark findings DEFERRED without proposing a remediation slice
  - Claim Path A Workbench extract works if spike says blocked
  - Soft-pedal curated road names — call Path B curated what it is
  - Soft-pedal icon redraw — call WRONG_PATH; do not invent “redraw was fine”
  - Claim height markers “shipped” if operator cannot see them on the map

═══ VERIFY ═══
  test -f .ai/artifacts/t152_11_fidelity_audit_report.md
  rg -n 'Provenance|Fix matrix|PROP_MIN_ZOOM|vs_text|Path B|tree|TREE' .ai/artifacts/t152_11_fidelity_audit_report.md
  git show --stat HEAD | head -40   # must not list engine/app source logic files

═══ RETURN ═══
  - Path to audit report
  - Top 5 root causes (one line each) — include tree zoom-in if confirmed
  - Proposed T-152.12+ slice titles
  - Workbench one-button gap summary (3–5 bullets)
  - Commit SHA + tag
```
