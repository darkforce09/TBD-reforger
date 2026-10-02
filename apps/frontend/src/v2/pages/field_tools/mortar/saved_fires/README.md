# Mortar calculator save area

The signed-in half of the mortar calculator: the event picker, saving the solved fire mission
against the chosen event, and the event's saved fire missions, any of which loads back into the
inputs.

## Contents

```text
apps/frontend/src/v2/pages/field_tools/mortar/saved_fires/
├── connection_gate.rs  the save area behind `AuthGate`, or the needs-a-connection notice while the catalog comes from the offline copy
├── list.rs          the selected event's saved fire missions, newest first, each a button that loads it
├── mod.rs           the event picker and its remembered choice, the fetched batch, the hydration rule
├── restore.rs       a stored row back into drafts: a catalog-model row with its guns, or a legacy row
├── save_area.rs     the save area: fetches, the save button, loading a row, the newest row once per event
└── save_request.rs  the `FireMissionSave` body from the solved mission, the post and its answer
```

## How it works

The save area renders inside `AuthGate` while the platform answers. While the page solves from
the catalog saved on this device, `connection_gate.rs` shows "Saving fire missions and the saved
fire missions need a connection to the platform; the calculator above keeps working offline."
instead (`data-mortar-save-area="needs-connection"`), with no sign-in link. The chosen event is kept in `localStorage` under
`tbd-mortar-event`. The save posts the solve's own inputs and the page's solution; the API
re-solves them and answers `solution_mismatch` when the two differ. A fetched batch carries the
event it answers for, and hydration fills the drafts from the newest row only for a batch of the
selected event, at most once per event.

## Boundaries

- Depends on: `crate::v2::core::api` (`FireMissionSave`, `SavedFire`, `SavedFireMissionAnswer`,
  `DataEnvelope`, `Paginated`); `map_coordinates::grid_reference`; the mortar inputs and
  the solve bridge.
- Used by: the mortar page (`page.rs`).
- Rules: hydration refuses a batch fetched for another event; the save body carries the solved
  inputs and the client solution unchanged; a legacy row restores its target and one gun.

## Related documentation

- [Mortar calculator page](/documentation/apps/frontend/pages/field_tools/mortar/mortar_calculator_page.md)
  — the page's behaviour, its data and its decisions.
