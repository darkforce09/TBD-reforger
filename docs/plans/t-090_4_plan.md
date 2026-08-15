# T-090.4 — Z placement audit (buried / floating objects)

## Context

Enfusion map objects export with pivots at ground, roof, or model center, so the
T-090.5 basemap can render props at wrong heights across ~1M Eden objects — far
beyond manual verification. Phase A is the cheap full-catalog screen: one DEM
sample per object, detect only, no auto-fix (geometry-aware Phase B is T-090.6).

## Approach

Offline tool in `tools/tbd-tools/src/world`: for each catalog instance under
`packages/map-assets/everon/objects`, sample the T-091 DEM at `(x, y)`, compare
`demZ` against the exported pivot `z`, and classify with per-kind warn/fail
thresholds. Missing `z` is a warn, never a fabricated value. Emit a machine- and
human-readable report keyed by object id/kind so T-090.6 can consume the deltas.

## Risks

Blocked on the T-090.3 export and the T-091 DEM actually being present; 16-bit DEM
quantization on slopes produces false positives, and tilted/large props produce
false negatives by design — both documented, deferred to the T-090.6 OBB pass
rather than tuned away here. Threshold choices per kind may need operator review.

## Verification

Run the audit over the full exported catalog; every instance reports `demZ` vs `z`
with its classification; missing `z` counts surface as warns; spot-check known
bridges/trees against the report. No auto-fix writes anywhere
(spec: `docs/specs/Mission_Creator_Architecture/t090_4_z_placement_audit.md`).
