# T-907 — Plan

## Context

Eleven map-engine files sit on the T-899 allowlist; `draw_order.rs` is already 487 lines (row drops only). `residency.rs` (3137 lines) and `slots_gpu.rs` carry Class-R byte-parity tests that scrub their own source, and `flatten.rs` is shared with T-674.1/T-675.1/T-682 — this ticket packs after them.

## Approach

1. Verify on main: line counts; note which files have byte-parity tests.
2. Per file: `<stem>.rs` keeps the public API and declares `<stem>/<responsibility>.rs` submodules (e.g. `world/residency/{load,evict,index}.rs`, `mission/flatten/{slots,vehicles,env}.rs`); no path or item renames.
3. Byte-parity tests: move the test with the code it scrubs so it still scrubs its own file; goldens unchanged.
4. Drop rows as files cross under 1000 lines.

## Risks

- A byte-parity test that scrubs the wrong file goes green for the wrong reason — perturbation-prove each moved test.

## Verification

- `cargo xtask verify file-length` · `cargo test -p map-engine-core --all-features` · `cargo test -p map-engine-render` · `cargo xtask platform wave gate --slice T-907`
