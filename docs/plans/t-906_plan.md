# T-906 — Plan

## Context

Seven live website-api files exceed 1000 lines and sit on the T-899 allowlist: `app.rs`, `handlers/{admin/admin,events/events,missions/missions,auth/oauth,telemetry/telemetry}.rs`, `services/mission_compile.rs`. `contract/generated/faction_library.rs` is generated codegen (expires 2027-08-13, do-not-split) and stays.

## Approach

1. Verify on main: line counts per owned file; `cargo xtask verify file-length` green via rows only.
2. Per file: `<stem>.rs` keeps the router/handler surface and declares `<stem>/<responsibility>.rs` submodules (e.g. `handlers/missions/missions/{list,compile,versions}.rs`); `pub use` keeps handler paths stable for `app.rs`.
3. Drop each row from `.coding-standards-allowlist.yaml` as it crosses under 1000 lines; leave the generated row alone.

## Risks

- `app.rs` includes are referenced by frontend `CARGO_MANIFEST_DIR` guards (T-934.3 note) — grep the frontend before moving `app.rs` content.

## Verification

- `cargo xtask verify file-length` · `cargo xtask platform wave gate --slice T-906`
