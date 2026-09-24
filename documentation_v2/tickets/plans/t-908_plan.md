# T-908 — Plan

## Context

Eight xtask/tools files exceed 1000 lines on the T-899 allowlist: `xtask/src/{cmds,main,schema_gates}.rs`, `tools/tbd-tools/src/{map/sap,smokes,world/aux,world/build,world/gates}.rs`. T-902/T-903/T-904 own overlapping xtask paths, and T-090.4/T-090.6 own `world/mod.rs`/`bin/world.rs` — this ticket packs after them.

## Approach

1. Verify on main: line counts; `cargo xtask verify file-length` green via rows only.
2. `xtask/src/main.rs` + `cmds.rs`: move subcommand groups into `cmds/<group>.rs` declared from `cmds.rs`; `schema_gates.rs` → `schema_gates/<gate>.rs`; tools files → `<stem>/<responsibility>.rs` (e.g. `world/build/{objects,roads,density}.rs`).
3. Drop rows as files cross under 1000 lines; xtask subcommand names and tbd-tools CLI flags unchanged.

## Risks

- `mk_ci_tasks.rs` lanes reference xtask function paths by name in generated tasks — grep before moving.

## Verification

- `cargo xtask verify file-length` · `cargo test -p xtask` · `cargo xtask platform wave gate --slice T-908`
