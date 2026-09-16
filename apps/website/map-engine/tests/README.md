# apps/website/map-engine/tests

Crate-root integration suites: rendering contracts, camera parity, and — since T-0xx Phase 2A —
the three headless document suites that came over with `website-mission-core`.

## Contents

- `camera_props.rs`
- `deckgl_ortho_parity.rs`
- `operation_boundaries.rs`
- `paste_keeps_authored_z.rs`
- `zone_round_trip.rs`

The last three exercise editor operations, paste, and zone save/reload against the document
directly, through the same headless boundary the editor and the API use. They are gated on
`store`.

## Boundaries

Tests keep the original assertions and fixtures; run the crate suite with `--all-features`.
