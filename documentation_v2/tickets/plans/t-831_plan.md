# T-831 — Plan

## Context

Operator requirement (Arma 3 model): markers authored per side, visible only to that side. The data model is already side-scoped (markers on per-side briefings, T-069) and `dock_right.rs:3799` says "Map markers for the active side's briefing" — but how the active side switches and whether OPFOR/INDFOR authoring is reachable is unknown. Game enforcement is T-673.

## Approach

1. Audit first: how the editor switches the active side, what each side's briefing exports; record it in the report.
2. `dock_right.rs`: explicit side selector on the Markers surface; the map shows the selected side's set (or all with side badges).
3. Export test: BLUFOR and OPFOR markers on their own side briefings.

## Risks

- T-838 also edits marker surfaces; owns are disjoint (dock vs outliner/map) but coordinate wording.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-831`
