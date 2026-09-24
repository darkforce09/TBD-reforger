# T-305 — pak.rs seeks the wrong offset

Owner: command center. T-206 measured 300/300 compressed entries inflating at `entry.offset` and 3/300 at
`data_start + entry.offset`; uncompressed reads come back rotated by 56 bytes (data_start of every shipped pak).

## Claude Code prompt — T-305

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-305 && pwd && git branch --show-current   # must be slice/T-305
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
tools/tbd-tools/src/world/pak.rs (all), docs/plans/t-305_plan.md
═══ PROBLEM ═══
read_file (:249) and read_raw (:285) add data_start to an absolute entry offset; every read is shifted.
═══ SHIPPED ═══
Pak header/table parser (:73-:179) — keep it; only the two seeks change.
═══ LANGUAGE GATE ═══
Rust only; edition-2024 rustfmt for tools/*.
═══ LOCKED ═══
- No new dependencies; the synthetic pak is built in the test with std + flate2 already in the crate.
- data_start stays parsed and reported; it is no longer part of any seek.
═══ DO ═══
1. Write the synthetic-pak test first; run it on main; paste the red (rotated bytes).
2. Fix both seeks. 3. Perturb (re-add data_start on one seek) → red → restore → touch → green.
4. cargo xtask platform wave gate --slice T-305
═══ DO NOT ═══
No shell scripts, no git add -A, no git stash, no ci-local. Touch only pak.rs.
═══ VERIFY ═══
cargo test -p tbd-tools --lib world::pak ; cargo xtask platform wave gate --slice T-305
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
