# T-151.4.1 — building wipe race + road joins (W4 corrective)

**Status:** **shipped** @ `552e68aa` (tag **T-151.4.1**, 2026-07-09) · verify log
[`t151_4_1_verify_log.md`](../../../.ai/artifacts/t151_4_1_verify_log.md) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` · **Baseline:** `723490a0` (tag **T-151.4**).

## In one sentence

Restore town building footprints wiped by a W4 residency race, and add miter joins + round caps
to road polyline strips so zoomed-in curves no longer tear.

## Shipped notes

- **P0 buildings:** empty `upload_world_buildings([])` no longer removes the lane mid-flight;
  abort clears `inflight`; skip empty push while unsettled. Operator confirmed ~8–9 buildings
  restored at town clusters.
- **P1 roads:** `expand_polyline_strip` miter joins (limit 4× half-width) + round end caps.
- **P2 forest:** unchanged — Deck-parity mega-hull / `DENSITY_ISO=1` overdraw deferred until
  **T-151.5** tree glyphs enable proper analysis.

## Documentation sync (Cursor)

Done with T-151.4 ship sync + T-151.5 setup.
