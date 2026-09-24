# T-935.7 — Plan

## Context
labels.rs:70/76 fetches three JSON files (locations, height-labels, road-names); locations.rs:12
and road_labels.rs:66 parse them. One MapLabelsArchive replaces the three fetches. bin/map.rs is
edited here; T-935.9 packs later and registers through map/mod.rs.

## Approach
1. `tools/tbd-tools/src/map/labels_emit.rs` + `map/mod.rs`: read the three files through the core
   types, build MapLabelsArchive {towns, height_labels, road_names}, write rkyv.
2. `bin/map.rs`: `labels-rkyv --terrain <dir>` subcommand.
3. `locations.rs`, `road_labels.rs`: `from_archive`; `labels.rs`: single fetch when
   `manifest.labels` is Some.
4. Parity test on everon; perturbation drops road_names → red; restore, touch, green.

## Risks
- Label strings with non-ASCII names: rkyv `String` is UTF-8 safe; test with a town containing
  an accent.

## Verification
- `cargo test -p map-engine-core --all-features labels`; `cargo test -p tbd-tools labels_emit`
- `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-935.7`
