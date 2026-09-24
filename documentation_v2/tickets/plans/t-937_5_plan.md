# T-937.5 — Plan

## Context
mission-editor-payload.schema.json:42-43 slots/editorLayers are bare arrays; mission.schema.json:6 pins
8388608 bytes but the editor ceiling (mission_library.rs:1452) is 64<<20; duplicate slot ids pass until the API.

## Approach
1. Verify on main: slot item {bogus: 1} validates → paste the red.
2. Payload schema: item schemas for slots and editorLayers (additionalProperties false); golden updated.
3. `state/operations/slot_ids.rs` (new, in operations.rs): duplicate_slot_ids(doc); save refuses with names.
4. mission_library.rs:1452: ceiling 8388608 from one constant; size readout before upload.
5. Perturbation: drop the callsign key from the check → test red; restore, touch, green.

## Risks
- Live payloads may carry unknown item keys — measure against committed fixtures before tightening.
- Ceiling drop from 64 MiB may refuse an existing large mission — the readout names the size.

## Verification
- `cargo xtask ci schema-validate`; `cargo xtask mk ci-local-leptos`
- `cargo xtask platform wave gate --slice T-937.5`
