# T-937 — Plan (program)

## Context
Audit S2 (2026-09-04), verified on main: store.rs:5162-5191 Any::Array id lists; :350-371 ZeroClock undo
with no cap; :740-820 materialize per-slot resolution; entity.rs:1885 existence via materialize;
persist.rs:827-832 silent save errors, :952-957 fire-and-forget pagehide, :815-826 lockout;
payload schema :42-43 bare arrays; editor ceiling 64 MiB vs 8 MiB API.

## Approach
1. T-937.1 YArray id lists (priority 0) → T-937.2 undo grouping → T-937.3 materialize cache (store.rs chain).
2. T-937.4 persist error surface + hidden flush (priority 0, independent files).
3. T-937.5 payload item schemas, duplicate guard, 8 MB ceiling (after .3).

## Risks
- store.rs is 14 728 lines and shared by three slices — strictly one per wave; parity fixtures guard bytes.
- Undo grouping changes a deliberate decision (T-159.22.1) — record it in the code comment.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask mk ci-local-leptos`
- `cargo xtask platform wave gate --slice T-937.N`
