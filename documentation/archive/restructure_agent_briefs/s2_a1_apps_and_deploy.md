**Status:** archived

# S2 agent A1: apps and deploy

You are agent A1, who runs the stage S2 relocations of the workspace restructure program with the
relocation tool: the website folder dissolves into `apps/`, `legacy/` and `crates/`, every package
takes its snake_case folder name, and one root `deploy/` folder is born.

Read [the shared brief](/documentation/archive/restructure/agent_briefs/shared_brief.md) first (Rules,
Efficiency, Spec, Report format), where `<scratch>` = `<scratch>`.
Stage S1 is committed; the tree is clean when you start. You run alone: nobody else edits the tree
until you report. Agents A2–A6 start from your report.

## Goal

- `apps/website/api_v2` → `apps/api`, `apps/website/frontend` → `apps/frontend`,
  `apps/website/offline-service-worker` → `apps/offline_service_worker`.
- `apps/website/map-engine` → `legacy/map_engine`, `apps/website/graphics-engine` →
  `legacy/graphics_engine` (D12, D13).
- Every package is snake_case with no `website-` prefix and equals its folder name (D3):
  `website-api` → `api` (library and binary both `api`, accepted), `website-frontend` →
  `frontend`, `website-map-engine` → `map_engine`, `website-graphics-engine` → `graphics_engine`,
  `website-offline-service-worker` → `offline_service_worker`, `fleet-host-agent` →
  `fleet_host_agent`. `ticketboard`, `xtask`, `verification_core`, `developer_tools` and
  `ticket_engine` already comply.
- The fleet host agent takes its snake_case name everywhere (D17): binary, systemd unit template
  `fleet_host_agent@.service`, `~/.config/fleet_host_agent/`, `~/.local/bin/fleet_host_agent`
  and the HTTP user agent `fleet_host_agent/<version>` (`apps/fleet_host_agent/src/ledger_client/ledger_api.rs:87`;
  no API code reads it). Your rows cover every non-Markdown site; A2 adds the host migration and
  A6 the prose.
- One root `deploy/` folder (D6): `Dockerfile`, `compose.dev.yml`, `compose.staging.yml`,
  `Caddyfile`, `systemd/`, `deploy.env.example` and the deploy README.
- The API's nested `rust-toolchain.toml` and `rustfmt.toml` deleted (format-neutral: the
  `rustfmt.toml` holds `edition = "2024"` and `max_width = 100`, the manifest's edition and the
  rustfmt default; the toolchain file names the root file's `1.95.0` channel minus wasm32).
- The API goldens (`apps/website/frontend/tests/fixtures/api/`, 122 files) at
  `contracts/fixtures/api_goldens/`.
- The shared URL case table at `crates/foundation/http_url_guard/src/cases.rs` and its README at
  the crate root, where A5 builds the crate.
- The documentation mirror moves with the code: `documentation/apps/{api,frontend}` (the frontend
  mirror keeps its inner `apps/` and `pages/` layout until S3), `documentation/legacy/{map_engine,graphics_engine}`,
  `documentation/apps/README.md` (the website docs index; A6 rewrites its body), and the app
  mirrors `documentation/apps/{fleet_host_agent,ticketboard}`. The mod mirror moves in M1.

## Steps

1. Copy the manifest draft at the end of this document to
   `documentation/restructure/manifests/s2_apps_and_deploy.tsv`. Read
   `tools/xtask/src/commands/refactor/relocate/README.md` and `path_references/README.md` once.
   Keep the row order: the `tools/xtask/deploy` row stays above every same-depth row whose `to`
   lies in `deploy/` (moves run by `from` depth, then row line), so
   `git mv tools/xtask/deploy deploy` runs while `deploy/` does not exist and the gitignored
   `deploy.env` rides along.
2. `cargo xtask refactor relocate --manifest documentation/restructure/manifests/s2_apps_and_deploy.tsv --dry-run`
   (log). The dry run verifies its own plan. Unresolved literals must be zero; see "If the tool
   reports" below. Never hand-edit a spelling the tool can rewrite.
3. Before the apply, prove the library-name `text` rows cannot shadow a binding: `git grep -n -w -E
   "website_(api|frontend|map_engine|graphics_engine|offline_service_worker)"` and confirm every hit
   is a crate or library reference (paths, `[lib]` names, `extern crate`, doc and regex text). A
   local identifier that would become `api`, `frontend` or another bare name beside an existing
   binding of that name is a semantic change the compiler does not catch: narrow that row's scope
   (a glob) to exclude the file and rename the identifier by hand to a clear name, listing it in
   the report.
4. `--apply` (log). The tool verifies after applying.
5. After the apply, by hand (the tool moves, it never deletes):
   - `rm apps/api/rust-toolchain.toml apps/api/rustfmt.toml` (they moved with the folder);
   - `rm apps/website/README.md` (the folder has no tracked child left), then
     `find apps/website tools/xtask/deploy -depth -type d -empty -delete` (git leaves empty
     source folders);
   - `git status --ignored --short apps deploy tools/xtask` shows `apps/api/.env`,
     `apps/frontend/dist/` and `deploy/deploy.env` at their new homes (ignored files ride with a
     whole-folder `git mv`, proven on a throwaway repository) and nothing under `apps/website/`.
6. `cargo metadata --format-version 1 > /dev/null` once without `--locked` (renamed packages
   re-sort `Cargo.lock`; report the diff: only the six renamed packages and their dependents'
   references), then `cargo metadata --format-version 1 --locked > /dev/null` and
   `cargo check --workspace --all-targets --locked` (logs).
7. Fix what the tool cannot see, split literals only:
   - `tools/verification_core/src/repository_laws/engine_layers/rules.rs:98`: the text row skips
     `\bwebsite_graphics_engine\s*::` (the `b` of `\b` is a letter) but rewrites
     `extern\s+crate\s+website_graphics_engine\b`; make both `graphics_engine`. Then
     `cargo test -p verification_core engine_layers` (log).
   - `git grep -n -E "website-(api|frontend|map-engine|graphics-engine|offline-service-worker)|website_(api|frontend|map_engine|graphics_engine|offline_service_worker)|apps/website|fleet-host-agent|Caddyfile\.website"`
     outside `documentation/archive/`, `documentation/tickets/` and `.ai/tickets/`: list every hit
     with a verdict and owner. Expected leftovers: Markdown prose naming the host agent (A6, or
     A2/A4 in their folders), the app bundle prefix `website-frontend-` (A5), glossary anchors
     `#fleet-host-agent` (legitimate: the heading "Fleet host agent" keeps its slug).
8. `cargo xtask refactor relocate --verify` (log); `cargo xtask verify crate-tiers` and
   `cargo xtask verify strangler` (logs: every member's package name equals its folder name; only
   apps and `tools/developer_tools` depend on `legacy/`).

### If the tool reports

- **A CWD-relative literal** (`apps/website/api_v2/src/core/http_router.rs:110,115`,
  `core/configuration/mod.rs:32,152`, `.env.example`): it resolves from the crate anchor to a
  tracked leading part (`assets`) and should be rewritten one level shallower; if reported, add
  nothing by hand and hand the list to A4.
- **The frontend fixture folder literal** (`apps/website/frontend/src/v2/core/test_support/fixtures.rs:11,21`,
  `"/tests/fixtures/api/"` beside `CARGO_MANIFEST_DIR`; `core/api/dto/tests/r_api.rs:252` checks
  its suffix): if left or reported, rewrite it by hand to the goldens' new home and keep the
  suffix check true; it is a split literal.
- **A synthetic manifest fixture** (`path = \"../graphics-engine\"` inside verification_core and
  xtask tests): if two crate anchors read it differently, report the row; the fixtures describe
  throwaway trees and need no rewrite.

## Run-time names that change with package names

| Name | Evidence (S1-base spelling) | Who rewrites it |
|---|---|---|
| Trunk app bundle `website-frontend-<hash>.js` / `_bg.wasm` becomes `frontend-<hash>…` (no `data-bin`, `apps/website/frontend/index.html:12`) | `frontend/src/v2/core/offline/service_worker_registration.rs:18-20` (`APP_BUNDLE_PREFIX`), tests `core/offline/tests/offline_manifest.rs:23-40`, `offline_pack.rs:53`, `service_worker_registration.rs:12-35`, `offline-service-worker/src/tests/request_classification.rs:31`, `core/offline/README.md:55` | Hand: A5 (a `-` neighbour blocks the text row) |
| Worker glue `offline_service_worker.js` / `_bg.wasm` | `offline-service-worker/Cargo.toml:18-20`, `index.html:17`, `service_worker.js:7-8` | Unchanged; A5 keeps the binary name |
| Worker manifest link | `index.html:16` `../offline-service-worker/Cargo.toml` | Tool |
| `Trunk.toml`, `manifest.webmanifest` | no package or path names | Nothing |
| Dockerfile build stage | `apps/website/Dockerfile:22-30,32-36,39` (`-p website-api --bin api`) | Tool for paths and `-p api`; A4 repairs the trimmed workspace (D18) |
| API unit | `tbd-website-api.service:48,51` (`WorkingDirectory`, `EnvironmentFile`), `:66` `target/release/api` | Tool; the `api` binary keeps its name |
| Host agent unit, binary, config folder | file `systemd/fleet-host-agent@.service` (lines 1, 6, 16), `deploy/staging/host_agent.rs:55-58,63`, `fleet_instances.rs:23,348-350`, `fleet_units.rs:3,28,52,59,65`, `staging/staging_settings.rs:181`, `environment_identity/build_identity.rs:30`, `fleet_procedure/operator_lists.rs:191`, `fleet_procedure/waves/mod.rs:44`, ten test files | Tool (`path` row for the unit file, `text` rows over `.rs`, unit files, manifests, lock, `deploy.env.example:53`); A2 adds the host migration |
| Host agent user agent and usage | `ledger_api.rs:87`, `main.rs:1,19`, test path `agent_configuration/tests/agent_configuration.rs:65` | Tool |
| rsync excludes, remote builds | `deploy/website/rsync_argv.rs:39-42`, `deploy/staging/remote/ssh_argv.rs:35-39,50`, `deploy/website/remote_steps.rs:115,133,166` | Tool |
| Caddy config in the container | `docker-compose.staging.yml:56`, `deploy/tests/website/tests.rs:397` | Tool (`Caddyfile.website` row); A4 narrows the mount |
| ci.yml job ids | `.github/workflows/ci.yml:141-142` and the API job | Tool; the operator renames required checks |

## Owned files

The whole tree for the mechanical apply; afterwards only the manifest, the deletions of step 5 and
the split-literal fixes of step 7.

## Report additions

Section 2 includes the dry-run and apply numbers per kind, the ignored-state listing of step 5, the
lock diff, and the step-7 list (path:line, verdict, owner).

Budget: M (250k tokens). Stop there and report done and not done.

## Manifest

The stage manifest is committed as
[s2_apps_and_deploy.tsv](/documentation/relocation_manifests/s2_apps_and_deploy.tsv).

## Decisions taken on the draft's questions

1. **`--migrate-single-instance` stays.** The staging host still runs the single instance
   (checked 2026-10-02: `fleet-host-agent.service` and `tbd-reforger.service` enabled,
   `~/.config/fleet-host-agent/` present, no `fleet-host-agent@N` units), so the single-instance
   migration is the one migration D17 needs: it reads the host's kebab-case names and installs the
   snake_case ones. The `.rs` text row rewrites its kebab literals; A2 restores the names that
   describe what the host has today in that module only.
2. **A5's births run from their own manifest** (`s2_crate_births.tsv`), as A5's body describes.
