# T-673 — Plan
## Context
T-069 shipped the four schema-carried marker fields and T-706 shipped the `$defs/marker` widening (size, rotation, shape, brush, color, alpha). Nothing in `apps/mod` reads the six MRK keys, so Area markers authored in the editor never reach the game. The 2026-08-02 attempt sits unreviewed at `salvage/t853-dropped/T-673` (commit 113108a1, shared with T-674/T-675).

## Approach
1. Verify on main: `rg -n 'brush|alpha|shape' apps/mod/tbd-framework/Scripts/Game/TBD/Markers/` shows no reader.
2. `TBD_MarkerData.c`: add the six fields with defaults equal to today's icon marker; bind them from the marker payload.
3. `TBD_MarkerClient.c`: draw Area markers (shape × brush × color × alpha) and rotated/sized icons; icon-only markers render byte-for-byte as before.
4. Diff against the salvage commit; take only reader hunks that match this scope.

## Risks
- The Enfusion map widget API may not fill shapes; fallback is outline-only with a note in the report.
- Rendering cannot be seen headlessly — the human checklist covers in-game appearance.

## Verification
- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-673`
