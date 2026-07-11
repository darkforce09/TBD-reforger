# Arsenal polish findings — operator review of T-068.10/.10.1 (for Cursor re-scope)

**Date:** 2026-07-11 · **From:** Claude Code session (T-068.10 / T-068.10.1 shipped) ·
**Operator verdict:** *"generally, the idea is getting there"* — but **"a lot more finer
adjustments … on the arsenal. Let's make the arsenal proper before we make the compiler for
it."*

## Program directive (operator)

**Hold T-068.11 (compiler loadout block) until the Arsenal is "proper."** Cursor: insert one
or more Arsenal-polish slices between T-068.10.1 and T-068.11 in the registry, and re-order
`slices[]` accordingly. Note: the T-068.11 *mission-embed* half already shipped inside
T-068.10 (per-slot `Slot.loadout` rides Save Version/Export; orbat summary wired) — the
remaining T-068.11 value is the compiled **mod** document block for T-068.12, which is exactly
the part the operator wants to wait.

## What the operator hit (verified against the dev DB, 1,880-item T-150 export)

1. **Kind misclassification — the headline bug.** `gear_primary` (125 rows) is polluted with
   non-rifles; the Primary dropdown literally opens with `MG UK59 4x8 · Smoke M18 Red ·
   Rifle M16A2 carbine AP2k · Pod UB1657 · SmokeGrenade Base…`. Confirmed in `gear_primary`:
   - grenades/smokes: `Grenade RGD5`, `FragGrenade Base`, `Smoke M18 *` (7), `Smoke RDG2`,
     `Smoke ANM8HC`, `M18`
   - flares: whole `ArmaReforger/Weapons/Flares` category (8)
   - abstract bases: `SmokeGrenade Base`, `FragGrenade Base`, `Launcher Base`, `Rifle AK74
     base`, … — **349 items repo-wide end in "Base"** (non-placeable template prefabs)
   - misc: vehicle weapon pods (`Pod UB1657`) classified as infantry primaries.
   Root cause is the **T-150 Workbench exporter classify heuristics** (kind is derived at
   export time), not the ingest/UI — fix belongs in the exporter rules + re-export + re-import
   (claude-code + Workbench MCP lane), with `grenade`/`throwable` probably becoming its own
   kind (schema enum bump + `make schema-codegen`, T-150 extension recipe).
2. **No grouping/sorting inside the pickers.** Flat `<select>` ordered by `sort_order ASC,
   display_name ASC` — and `sort_order` values are effectively arbitrary (1798, 1489, 652…),
   so the list reads as random. `category` data is good (`ArmaReforger/Weapons/Rifles/M16`,
   `…/MachineGuns/PKM`, …) and unused by the UI. Candidate UI work (claude-code):
   `<optgroup>` by category (or curated group map), alpha sort within group, hide `* Base`
   abstracts, search/filter box (T-055 pattern), maybe show edge counts.
3. **Operator's general note:** categorization + sorting "and everything" need a pass — treat
   the above as the observed floor, not the ceiling; a triage pass over all 16 kinds is worth
   a slice task (e.g. `crate` 276 / `other` 252 rows likely hide more misfiled gear).

## Suggested slice split (Cursor to spec/number)

- **A — registry data quality (exporter):** classify-rule fixes in the T-150 Workbench export
  (grenade/flare/pod/abstract-base handling; possibly new `gear_grenade` kind + schema bump),
  re-export, `make registry-import`, G-gate re-run. Executor: claude-code (+ Workbench MCP;
  plugin recompile needs operator per memory).
- **B — Arsenal UX (frontend):** grouped + sorted pickers, base-prefab suppression, search;
  builds on the existing `LOADOUT_ROWS` config (rows are declarative — this is render-layer
  only). Executor: claude-code.
- Then unpause **T-068.11**.

## Current state (unchanged facts)

Shipped + tagged: **T-068.10** @ `3bc0bd24`, **T-068.10.1** @ `9a86ce9b` (clothing
mix-and-match; compat = weapon families only). 19/19 browser gates PASS post-.10.1; vitest
316; verify log `.ai/artifacts/t068_10_verify_log.md` (with .10.1 addendum).
