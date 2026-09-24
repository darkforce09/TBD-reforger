# T-932 — Plan

## Context

Wave-211 F1 MAJOR (T-826 found_not_fixed): `pendingBriefingMarkers` is session/meta state — the local yrs `encode_state` keeps it, but a server JSON save/reload drops it before the first faction mint (`store.rs:905` `promote_pending_briefing_markers`, `:3998`); compile ignores it and hydrate clears it.

## Approach

1. Verify on main: core round-trip test save → reload of a marker-only mission loses the parked markers (red).
2. `compile.rs` + `store.rs`: emit `pendingBriefingMarkers` on the compile/hydrate wire (or an equivalent durable park) and restore it on hydrate; promotion on first faction mint unchanged.
3. Perturbation proof; V1 path intact.

## Risks

- Wire shape must stay schema-valid; if the schema lacks the key, park under meta rather than widen.

## Verification

- `cargo test -p map-engine-core --all-features` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-932`
