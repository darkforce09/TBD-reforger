# T-305 — Plan

## Context
pak.rs:249 and :285 seek `data_start + entry.offset`; T-206 measured that entry offsets are absolute (300/300
entries inflate at `entry.offset`). Everything built on `enf extract` reads bytes shifted by 56.

## Approach
1. `tools/tbd-tools/src/world/pak.rs`: add a test that builds a synthetic pak (header, data_start = 56, two entries,
   one deflated) in memory and reads both via read_file/read_raw — red on main with rotated bytes.
2. Change both seeks to `SeekFrom::Start(u64::from(r.entry.offset))`; keep `data_start` parsed for diagnostics.
3. Perturbation: restore the `data_start +` term on one seek → red; restore, `touch`, green. Edition-2024 rustfmt.

## Risks
- A shipped pak variant with relative offsets would regress; the T-206 sample (300 entries, three paks) says none exists.
- Fallback: header flag to select the mode, default absolute.

## Verification
- `cargo test -p tbd-tools --lib world::pak`
- `cargo xtask platform wave gate --slice T-305`
