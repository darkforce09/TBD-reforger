# T-090.6 — Geometry-aware placement audit (simplified 3D bounds)

## Context

The T-090.4 point audit (pivot Z vs DEM) misses tilted, large and spanning props
by design. This slice is Phase B: for every exported map object, use center +
rotation + simplified 3D bounds — never full meshes — to classify parts as above
terrain, buried, or inside another object, fully automated at the 1M-object scale.

## Approach

Extend the audit tool in `tools/tbd-tools/src/world`: build an OBB per instance
from `spatial.halfExtentsM` + `rotationDeg` (the normative shipped geometry;
footprint polygon rings supersede rectangles only where the T-090.3.0 spike proved
export), apply the measured `localUp → world Z` axis remap, sample DEM at OBB
corners/edges, and classify above/buried/inside with per-kind thresholds. Overlap
detection against neighbor OBBs via a spatial grid over
`packages/map-assets/everon/objects`.

Files: new `tools/tbd-tools/src/world/placement_audit.rs` (registered in `world/mod.rs`, subcommand in `bin/world.rs`); rewrites `objects/z-audit.json` with OBB classes and adds `z-audit-workbench.json` for the spot-check set. Packs after T-090.4 (shared `mod.rs`/`bin/world.rs`).

## Risks

The axis remap and rotation handedness come from the T-090.3.0 spike — applying
them wrongly inverts the buried/floating classes; OBB-only geometry over-reports
on concave props (bridges, arches), so classifications carry a confidence note
rather than pretending mesh precision. Full-catalog runtime must stay batch-friendly.

## Verification

Full-catalog run completes without manual eyeballing; OBB classes reproduce the
T-090.4 findings on the point-audit subset and additionally flag known tilted
props; spot-check against Workbench ground truth where available
(spec: `docs/specs/Mission_Creator_Architecture/t090_6_geometry_placement_audit.md`).
