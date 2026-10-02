**Status:** archived

# API V2 Phase One Handoff

Status: relocation complete and committed on `main` as `6f562b02a`. Crate layout inside `src/` is still the legacy one; domain decomposition begins in Phase Two.

## Ownership and layout

- `apps/website/api_v2/` is the backend crate: `Cargo.toml` (package `website-api`, lib `website_api`, unchanged), `src/`, `tests/`, `migrations/`, `seeds/`, `.env.example`, `docker-compose.yml`, `rustfmt.toml`, `rust-toolchain.toml`, moved with git renames (155 paths).
- `missions/` (runtime `*.mission.json` staging drops, ignored by the crate's `.gitignore`) and the gitignored `.env` moved by plain `mv`; neither was ever tracked.
- The blueprint documents (`README.md`, `ARCHITECTURE_PLAN.md`, `ANALYSIS_AND_INVENTORY.md`, the `src/<domain>/README.md` placeholders) are now tracked alongside the legacy modules. No path collisions.
- `apps/website/api/` no longer exists.

## Pins retargeted

- Workspace member (`Cargo.toml`), `apps/website/Dockerfile`, `.github/workflows/{ci,editor-gates}.yml`, `.claude/launch.json`, three `.cursor/rules/*.mdc` (one live glob), `tools_v2/xtask/deploy/deploy.env.example`, `apps/website/docker-compose.staging.yml`, `.coding-standards-allowlist.yaml` (23 entries), `rust-toolchain.toml` comments.
- Frontend: `src/v2/core/test_support/fixtures.rs` `include_str!` of the router source, two test message strings, and the cross-reference comment in `apps/website/shared/is_http_url_cases.rs`.
- Tooling: 46 non-test and 16 test files under `tools_v2/xtask/src` and `tools_v2/ticket-engine/src` (constants, shell recipe strings, gate paths, fixture layouts). Four of them re-wrapped by rustfmt after the rename lengthened a line.
- Documentation: `CLAUDE.md`, `README.md`, 33 files under `docs/`, `documentation_v2/` (untracked), `contracts_v2/` (untracked), `contracts_v2/definitions/mission-editor-payload.schema.json` description string, `assets_v2/terrains/README.md`, `tools_v2/ARCHITECTURE_PLAN.md`, and comment-only citations in five Enfusion scripts plus `apps/mod/tbd-framework/README.md`. Generated ticket views (`docs/TICKET_*.md`) and `.ai/**` are untouched by design.
- Crate name strings (`-p website-api`), the image name `tbd-website-api`, and the systemd unit name are unchanged.

## Baseline repairs

- `apps/website/Dockerfile` copied the nonexistent `crates/map-engine-core`; it now copies `apps/website/map-engine` and `apps/website/graphics-engine` and lists both in the synthesized workspace manifest.
- `verify_no_select_star` (`tools_v2/xtask/src/verifications/database/sql_deserialization.rs`) scans `src` instead of `src/handlers` and `src/services`, so the coming decomposition cannot move SQL out of its reach.
- `staging_compose_paths` bans name the new directory, so the negative tests still bite on a path someone could type.

## Verification evidence

Logs under the session scratchpad `phase1/` (baseline under `phase0/`).

- `cargo check --workspace --locked`: pass.
- `cargo fmt --all --check`: pass.
- `cargo test -p xtask -p ticket-engine`: 840 passed, 0 failed.
- `cargo xtask verify route-tags`: PASS, 103 tags against 103 routes; route rows byte-identical to the Phase 0 golden.
- `cargo xtask verify file-length`: 2312 files, 0 violations.
- `cargo xtask ticket check`: OK.
- `cargo xtask db test-it`: 44 targets, 784 passed, 0 failed, 1 ignored (identical to baseline).
- `cargo xtask ci ci-local`: pass (rc 0).
- `podman build -f apps/website/Dockerfile`: see the addendum at the end of this file.

## Working-tree handling

- Another session left uncommitted work in the tree (ticket ledger, mod scripts, tooling READMEs, `verification-core`, frontend editor files, and ten tracked files deleted in the working tree). None of it is in the Phase 1 commit except files where the only hunk is this phase's path rename.
- `ci ci-local`'s language ban refuses to run over tracked-but-deleted paths. For the gate run those ten deletions were staged, then their index entries restored with `git reset -- <paths>`; the commit does not contain them.
- `target-container/` held graphics-engine test binaries linked against the host glibc; they were purged so the container toolchain rebuilt them. Host and container must not share a target directory.

## Follow-ups (not Phase 1 scope)

- `missions/` runtime drops live inside the crate directory; consider pointing `MISSIONS_DIR` outside the tree.
- `AGENTS.md`, `documentation_v2/`, `contracts_v2/`, `assets_v2/` are untracked and belong to other work.

## Addendum: container build

`podman build -f apps/website/Dockerfile -t tbd-website-api:local .` succeeds (builder stage compiles `website-api --release`, runtime stage on `debian:bookworm-slim`). This build did not work before Phase 1 because of the dead `crates/map-engine-core` COPY.
