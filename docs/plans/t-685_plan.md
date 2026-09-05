# T-685 — Plan

## Context

TBD zones are 2D circles/polygons with no height; WOG's `WMT_Task_Point` evidence (inferred — the addon is absent from the corpus) adds min/max height, capture/defender counts and starting owner. Schema keys shipped in T-706; `zoneRules` needs loader fields and `TBD_ObjectiveRegistry.c` a resolve branch.

## Approach

1. Verify on main: no height or count fields in `TBD_MissionLoader.c` zone binding.
2. `TBD_MissionLoader.c`: bind the new keys; new `Zones/TBD_ZoneVolume.c`: volume test (min/max height) and count/owner semantics; `TBD_ObjectiveRegistry.c`: resolve branch consuming them.
3. Zone inspector: three numeric fields and a side picker in `zones_panel.rs` (if the inspector lives in `attributes_modal.rs`, report it under `files_outside_owns`).

## Risks

- Copying inferred WOG semantics as fact; design them as a TBD question and keep the caveat in comments.

## Verification

- `cargo xtask mod compile` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-685`
