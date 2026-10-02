**Status:** archived

# API V2 Phase Seven Handoff

Status: the completion audit of 2026-09-21 found leftovers the Phase 6 gates could not see, and this
phase closes every one of them. Commits: `8a75ebc2b` (7.1 wire-format names and the `src/` prose
sweep), `1d3b8d263` (7.2 the integration suites), `134b6901c` (7.5 runtime storage and the deploy
steps), `2a640e88f` (7.4 the environment template and blueprint prose), `46d0dc1cb` (7.6 the
migrations and seeds), `08dd32d8b` (7.3 the prose rules), `1beffbd9f` (7.7 the live documents),
`77b7c98f3` (formatting of a ticket-engine file the workspace format gate reported), and the commit
carrying this handoff (7.8, with one clippy simplification in `prose_rules.rs`).

## What the audit found, and what closed it

- **The sweep blind spot.** Every Phase 1–6 old-path check was `grep -rn 'apps/website/api\b' . | grep -v api_v2`, and the `-v` also dropped every hit *inside* `apps/website/api_v2/` because the file-name prefix contains `api_v2`. The content-filtered recipe is `git grep -nP 'apps/website/api(?!_v2)' -- . ':(exclude).ai'`; it found 20 references inside the crate, including two runtime operator hints and a test `expect` message. All are gone (7.1, 7.2, 7.4, 7.6). This session's shell `grep` is also a `ugrep` wrapper; the sweeps use `command grep` or `git grep`.
- **`.env.example` had never been scrubbed** (ticket ids, the old crate path, the retired implementation's pool defaults, eight dead module paths). Rewritten in present tense with the module that reads each variable; the OAuth host-agreement pin still reads it (7.4).
- **The suites carried the delivery process**: ~250 ticket-shaped fixture literals (`T235 Life Alpha`, `t405-…`, `t336-arma-…`), the vocabulary of waves, slices, gates and perturbations in ~40 files, citations of the deleted `Makefile` and `wave.sh`, and the merged suites' old names. Fixtures are named by subject, prose says what each assertion proves, six suites are named for what they cover, and the refresh-token purge test lives with the refresh-token suite (7.2, 7.3).
- **`src/` leftovers the rule phrases did not match**: `go_time` / `go_date` / `go_compatible_time.rs` named the retired implementation; eleven comments said `Go's`, `differential`, `Separate ticket`, `placeholder echo`, or cited `handlers/auth/oauth.rs`. The wire-format modules are `core::wire_format::{rfc3339_utc, rfc3339_utc_opt, rfc3339_utc_date}` in `rfc3339_timestamps.rs`, and the approvals note states why no review-comment thread exists instead of specifying one (7.1).
- **Runtime storage lived inside the crate**: 455 ignored `*.mission.json` drops, CWD-relative `uploads` / `missions` constants, and a production unit whose working directory is the crate. `UPLOAD_DIR` and `MISSION_STAGE_DIR` are configuration — development defaults to the gitignored `assets_v2/scratch/website-api/{uploads,missions}`, outside development both are required, absolute and whitespace-free, test configurations use a per-process temporary directory, and the router creates both at build. The unit declares `StateDirectory=tbd-website-api` and points both under it; the staging compose service mounts a named volume the image pre-creates for the API's uid; the crate `.gitignore` no longer reserves either directory (7.5, 7.4).
- **Migrations and seeds carried ~150 ticket ids, wave narrative and dead Rust paths**, one ticket in a filename, and one dead path inside a `COMMENT ON` statement. All 22 comment blocks and the seeds are rewritten with every statement byte-identical (proved by diffing with comment lines stripped); `0016_backfill_linked_match_stats.sql` is named for what it does; `0026_refresh_catalog_comments.sql` re-issues the two `COMMENT ON` statements. `tests/migrations_are_immutable.rs` pins the new hashes (7.6).
- **The rules had blind spots**: they walked `src/` only and matched five phrases. `src/tests/prose_rules.rs` walks `src/`, `tests/`, `.env.example`, the seeds and the migration comment lines for ticket ids, other-implementation narrative, delivery-process vocabulary and retired paths; `architecture_rules.rs` keeps the dependency and shape rules (7.3).
- **Live documents were stale**: the backend roadmap was the retired implementation's file table, the website README linked `api/`, the documentation_v2 API page described the layer layout, a CI comment cited `tests/common/mod.rs:87`, the README listed five handoffs, and the blueprint documents carried eight ticket-id lines (7.7, 7.4).

## Migration edits, from here on

A shipped migration's statements are never edited; the schema moves with a new migration. A comments-only edit is allowed and has a fixed procedure: rewrite, recompute the pin in `tests/migrations_are_immutable.rs` (`sha384sum migrations/<file>.sql`), **commit** (a renamed file must be in history before the repair, because `cargo xtask db repair-migration-checksum` recovers the applied bytes through `git log --follow`), run the repair locally in proof mode, and let `cargo xtask deploy website` run it on the server — it is a remote step after the builds and before the restart, against the staging Postgres container, so the new binary never meets an unrepointed row. The development database was repointed this way (22 of 22), which also closed the `0021` drift that had required `SKIP_MIGRATE=1`.

## Verification evidence

Logs under the session scratchpad as `p7_<package>_*.log`; every run below was executed on the committed tree.

- `cargo fmt --all --check` pass (after `77b7c98f3`); `cargo check --workspace --locked` pass (245 warnings, all `website-frontend`, the Phase 1 baseline); `cargo clippy -p website-api -p website-map-engine -p xtask --all-targets --all-features -- -D warnings` clean.
- `cargo xtask verify route-tags` PASS (103 tags against 103 routes in 8 route files — unchanged since Phase 0); `cargo xtask verify file-length` 2529 files, 0 violations; `cargo xtask ticket check` OK; `cargo xtask ci verify-codegen-fresh` pass.
- `cargo test -p website-api --lib --bins` 294 passed (284 before this phase: six configuration tests and four prose rules added); `cargo test -p website-api --doc` 0 failed (1 ignored); `cargo test -p xtask -p ticket-engine` 864 passed (845 before: the deploy plan and remote-step pins); `cargo test -p website-map-engine --all-features` 1444 passed.
- `cargo xtask db test-it` 61 targets, 512 passed, 0 failed, 1 ignored (502 at the audit's start plus the ten new unit tests, which the runner also builds; every renamed suite keeps its count).
- `cargo xtask mk ci-local-leptos` pass (1342 frontend tests, trunk release build; private target dir).
- `cargo xtask ci ci-local` pass (rc 0, 20 steps, run alone after `db test-it`; its integration step included).
- `podman build -f apps/website/Dockerfile -t tbd-website-api:local .` succeeds (image 113 MB; the runtime stage pre-creates `/srv/state`).
- Boot smoke against the development database **with migrations enabled** (no `SKIP_MIGRATE`): `0026` applied, `/healthz` 200, `/metrics` 401, `/api/v1/servers` 401, 120 requests to `/map-assets/...` all 404 and never 429 while the same burst against `/api/v1/servers` throttles after the budget (45 × 401, then 429). A boot from the crate directory creates `assets_v2/scratch/website-api/{uploads,missions}` and nothing under `apps/website/api_v2/`.
- `cargo xtask deploy website --dry-run` prints the two new remote steps after the builds and before the restart.
- Sweeps (content-filtered): 0 references to the retired crate path in `src/`, `tests/`, `.env.example`, seed comments and migration comments (the one remaining string is `0010`'s immutable `COMMENT ON FUNCTION` statement, superseded in the catalog by `0026`); 0 ticket ids in the same scopes; 0 inline test modules; 0 `api_v2` allowlist entries; the crate directory holds no `uploads/` or `missions/`.

## Environment notes

- `cargo xtask mk …` recipes run in `CARGO_TARGET_DIR=…/target-container-api-v2`; db and ci-local runs need the `podman` shim on PATH; `db test-it` and `ci ci-local` run one after the other (see the Phase 5 and 6 handoffs).
- `AGENTS.md` at the repository root is a local, git-excluded mirror of `CLAUDE.md`; the command-list edit was applied to both, and only `CLAUDE.md` is committed.

## Outside this crate, observed and left as is

- `.github/workflows/ci.yml` and several xtask module headers carry ticket-narrative comments in the repository's tooling style; only the one stale path was in this scope.
- Dated documents — `docs/plans/**`, `docs/specs/**`, `docs/platform/t1*`, `SHIPPED_HISTORY.md`, `CODEBASE_AUDIT_2026.md`, the factory documents — describe the tree at the time they were written and keep their paths; `.ai/**` and the generated `docs/TICKET_*.md` are the ticket registry.
- 45 local branches (`slice/T-*`, `probe/*`, `scratch/*`, five checked out in worktrees) predate this work.
