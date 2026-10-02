**Status:** archived

# API V2 Phase Six Handoff

Status: the refactor is complete. `apps/website/api_v2/` is the backend crate (`website-api`, lib `website_api`); `apps/website/api/` does not exist. Commits: `939db54e9` (6.1 test support split), `2cec5c57e` (6.2 telemetry and events suites), `948da1cdf` (6.3 missions, misc, null-tolerance suites), `4374fa8b2` (6.4 suite renames), `dd139dbdf` (6.5 schema descriptions and regenerated projections), `9fa3f4d35` (6.6 present-tense sweep of the suites and the crate description), `16598a095` and `e700fa80d` (6.7 docs), `d5a77ffe5` (6.8 gate fix-up: the prefab schema keeps the sentence the n6 gate pins across the spec documents).

## Final shape

- `src/`: `lib.rs`, `bin/{api,import_registry}.rs`, `core/`, `background_workers/`, eight domains (`administration`, `command_center`, `community_content`, `identity_and_access`, `match_telemetry`, `missions`, `operations`, `server_infrastructure`) each with `routes.rs`, `handlers/`, `services/`, `models/` as needed, and `tests/architecture_rules.rs`. One README per top-level module, generated from the tree. No file over 500 lines; unit tests live in sibling `tests/<file>.rs` modules.
- `tests/`: 56 integration suite files, all under 1000 lines, subject-named, over `common/{database,http,fixtures,source_text}.rs` and per-family support modules (`telemetry_support`, `events_support`, `missions_support`, `router_boot_support`, `null_tolerance_support`); the support module's own checks run once in `test_support_self_checks.rs`.
- `.coding-standards-allowlist.yaml`: zero `apps/website/api_v2` entries.
- `contracts_v2/definitions/*.json` descriptions are present tense; `src/missions/contract/generated/` is regenerated from them and carries no doctest-shaped blocks.
- Documentation: `apps/website/api_v2/README.md` is the live atlas; `ARCHITECTURE_PLAN.md` opens with the current layout and the merge design; `ANALYSIS_AND_INVENTORY.md` is marked as the pre-refactor inventory; the repo `CLAUDE.md` atlas, the coding and documentation standards, the placement guide, the runbooks, the commit checklist, the backend roadmap, and the context handoff name the domain tree.

## Verification evidence

Logs under the session scratchpad `phase6/` (`6_1_*` to `6_8_*`).

- Structural: `apps/website/api` absent; 0 `T-NNN` references under `src/` outside the codegen directory; 20 under `tests/`, all fixture literals asserted by value; 0 inline test modules; 0 production files over 500 lines; 0 test files over 1000; 0 `api_v2` allowlist entries.
- `cargo fmt --all --check` pass; `cargo check --workspace --locked` pass (245 warnings, all `website-frontend`, the Phase 1 baseline); `cargo clippy -p website-api -p website-map-engine --all-targets --all-features -- -D warnings` clean.
- `cargo xtask verify route-tags` PASS (103 tags against 103 routes in 8 route files); `cargo xtask verify file-length` 2516 files, 0 violations; `cargo xtask ticket check` OK; `cargo xtask ci verify-codegen-fresh` pass.
- `cargo test -p website-api --lib --bins` 282 passed, 0 failed (274 crate tests plus the eight architecture rules); `cargo test -p website-api --doc` 0 failed (1 ignored); `cargo test -p xtask -p ticket-engine` 845 passed; `cargo test -p website-map-engine --all-features` 1444 passed, 0 failed.
- `cargo xtask db test-it` 60 targets, 497 passed, 0 failed, 1 ignored. The count is the Phase 5 figure (785) minus the eight test-support self-checks that used to compile into 37 binaries and now run once (296 fewer runs, 8 kept). Every suite keeps its pre-split test count.
- `cargo xtask mk ci-local-leptos` pass (1342 frontend tests, trunk release build; private target dir).
- `cargo xtask ci ci-local` pass (rc 0, run alone after `db test-it`; its integration step included).
- `podman build -f apps/website/Dockerfile -t tbd-website-api:local .` succeeds (image 113 MB).
- Boot smoke against the development database (`SKIP_MIGRATE=1`): `/healthz` 200, `/metrics` 401 without a token, `/api/v1/servers` 401; 120 requests to `/map-assets/...` all 404 and never 429 (the exempt mount sits below the rate-limit layer) while the same burst against `/api/v1/servers` throttles after the configured budget (49 × 401, then 429).
- One `db test-it` run that overlapped a concurrent `ci ci-local` in the same target directory aborted at its 30th binary because ci-local's integration step recreates the shared base database; the two runs above were executed one after the other. The earlier gate logs were lost when the session's scratchpad was cleared on resume, so every run above was executed again from the committed tree.

## Environment notes

- `cargo xtask mk …` recipes run in `CARGO_TARGET_DIR=…/target-container-api-v2` because the shared `target-container/` is stamped by another session's glibc; db and ci-local runs need the `podman` shim on PATH (see the Phase 5 handoff).
- The other session's ten tracked-but-deleted paths are staged only for `ci ci-local` and restored afterwards; its unrelated working-tree edits are not in any refactor commit.

## Follow-ups (outside this refactor's scope)

Every item below was closed by Phase 7 — see `PHASE_7_HANDOFF.md`. The list stays as the record of what this phase handed on.

- `apps/website/api_v2/migrations/0021_rate_limit_buckets.sql:7` names the former `t578_ratelimit` suite; migration files are checksummed once applied, so the comment stays until a migration policy allows editing it.
- The 433 runtime `missions/*.mission.json` drops live inside the crate directory (ignored by git); point `MISSIONS_DIR` outside the tree and remove them.
- `tests/` fixture literals shaped like `t529-seed-arma-529001` are data, not names; no guard matches them.
- `docs/specs/**`, `docs/plans/**`, and `.ai/**` keep historical paths by design.
- The local development database still carries the migration `0021` checksum drift noted since Phase 2; a bare boot against it needs `SKIP_MIGRATE=1`.
