**Status:** live

# T-1281 — Plan

## Context

`apps/` holds five Rust packages that sit outside the judged crate layout: the API app, the
single-page app, the offline service worker, the game-server host agent and the ticket registry
viewer. The workspace laws exempt anything under `apps/`
(`tools/foundation/repository_laws/src/workspace_laws/crate_layout.rs`), so their dependency edges,
crate shape and tiers go unjudged. The operator wants every Rust app in the tiered layout under a
descriptive name (CLAUDE.md law 4). After the move `apps/` keeps only the Enfusion mod, and the only
members outside the layout are the two tool binaries `tools/xtask` and `tools/developer_tools`.

This is stage S13 of the workspace restructure. Its run folder (orchestration prompts, agent
reports, gate logs) is outside the repository, in the detached worktree's
`target/restructure-s13/`.

### Names (operator-approved)

| Old package | New package | Folder | Binaries |
|---|---|---|---|
| api | api_server | crates/api/api_server | api → api-server; import-registry → import-item-registry |
| frontend | frontend_application | crates/frontend/shell/frontend_application | frontend → frontend_application |
| offline_service_worker | unchanged | crates/frontend/shell/offline_service_worker | unchanged |
| fleet_host_agent | game_server_host_agent | crates/fleet/game_server_host_agent | → game_server_host_agent |
| ticketboard | ticketboard_desktop | tools/tickets/ticketboard_desktop | → ticketboard_desktop |

Kept on purpose:
- the compose project `api` and its volume `api_tbd_pgdata`;
- `tbd-website-api.service` and its `StateDirectory`;
- the image tag `tbd-website-api:local`;
- `COMPILER_PACKAGE_VERSION`;
- the public worker URL `/offline_service_worker.js`.

## Approach

### Decisions

- O1–O5, operator:
  - the names above;
  - the host agent's rename is a host migration that the operator runs as a `deploy staging` step;
  - the glossary term becomes "game server host agent";
  - the ticketboard eframe app id is renamed, so saved preferences reset;
  - the kept list above.
- K1: the worker sits in the shell layer beside the app. The two are peers. A firewall clause keeps
  GPU and rendering crates out of the worker, and `ci-local-leptos` is its one lane.
- K2: the dependency deny-lists retire into the laws:
  - rule 7 generalised (no member depends on an application package, in any table);
  - the shell peer order;
  - the worker clause.

  A table proves each entry is still caught.
- K3: a fail-closed remote preflight in `deploy website` and `deploy staging` refuses the rsync when
  the server's new `.env` is missing.
- K4: anatomy fixes in the moved libraries; development asset defaults anchored on the repository
  root; the documentation mirrors move with the code.
- K5: every multi-root scan reads each crate once, with a test.
- A1: the host agent's HTTP user agent follows the new binary name, because nothing consumes it.
- A2: after the stage, the old spellings survive only as migration inputs and listed kept literals.
- A3: the operator steps below.

### Waves

| Wave | Agents | Content |
|---|---|---|
| W0 | orchestrator | this ticket, baselines |
| W1 | M1–M4, serial | one relocation manifest per app (s13_w01_1 … s13_w01_4), with the doc mirrors; a WIP commit after each |
| W2 | L1, A1, A2, F1, H1, T1, T2, D1, parallel | laws; api_server shape and scans; shell crates; host agent crate; lanes; database, readiness, staging and tickets; deploy and migration |
| W3 | R1, R2 | repository docs; glossary, runbooks, mirrors |
| W4 | closing-fix agents, orchestrator | routed findings, perturbation proofs, stage gate, landing |

### Execution record

| Wave | Result |
|---|---|
| W0 | ticket opened; baselines: workspace laws 5/5 PASS, relocate --verify 2127 checks, documentation gates OK, ticket check --strict OK, 161 members |
| W1 M1 | API moved to crates/api/api_server (4 path, 4 rust_path rows); 386 files rewritten; api_server lib tests 32/32 |
| W1 M2 | frontend and worker moved to crates/frontend/shell (5 path rows); APP_BUNDLE_PREFIX follows; Trunk release build gives frontend_application-<hash>.js |
| W1 M3 | host agent moved and renamed to crates/fleet/game_server_host_agent (3 path, 2 rust_path rows); crates/fleet glob added; 148 tests twice |
| W1 M4 | viewer moved to tools/tickets/ticketboard_desktop (2 path, 4 scoped text rows); app id renamed |
| W1 gate | no member under apps/; fmt OK; relocate --verify 2147 checks; workspace laws red with 28 findings (unknown categories crates/fleet and crates/frontend/shell), handed to W2 |
| W2 L1, L1b | judged set narrowed to the tool binaries; fleet class; shell layer and peer order; rule 7 over every table by application package; rendering-stack firewall clause for the worker and the API crates; the deny-lists deleted after a 20-row proof table; route tags read once; 12 law perturbations red then restored |
| W2 A1 | api_server prelude and thiserror error type (no anyhow); development asset folders resolved from the repository root, production requires absolute paths |
| W2 A2 | one deduplicated API package list for every api_server source scan, with read-once tests |
| W2 F1 | one frontend source-root helper; documentation audit and keymap census read each package once; the worker passes the audit |
| W2 H1 | host agent prelude, error type and typed ids; user agent renamed (A1) |
| W2 T1 | lanes renamed; the worker is gated only by ci-local-leptos; 19 base reds in ci_task_catalog fixed |
| W2 T2 | database lanes deduplicated; the API environment file is one constant; readiness register repointed (a new dependency-boundary check); ticket sparse sets |
| W2 D1 | the .env preflight in both deploys; one host-owned exclude list; unit files; Docker; CI; the host-agent rename migration with dry run and a deploy guard that refuses while an old name remains |
| W2 gate | fmt, workspace clippy with --locked and -D warnings, workspace laws 5/5 PASS, file-length, relocate --verify 2147 checks, ticket check --strict, repository_checks 99/99 |
| W3 R1, R2 | CLAUDE.md, architecture, standards and READMEs; glossary entry "game server host agent"; runbooks with the migration step; mirrors; documentation gates green |
| W4 G1 | closing fixes: command names in comments, a hollow failpoints build-flag check made real, the fresh-host refusal text, ten open tickets, the old-spelling audit |
| Stage gate | `cargo xtask ci ci-local`: every step green. The API integration suite ran 154 binaries (1508 tests, 0 failed), and ci-local-leptos ran 2046 tests plus the Trunk release build from crates/frontend/shell/frontend_application. Workspace-member tests were first red in five packages: four read Git LFS files that were pointers in the worktree, and one citation-scope fixture had lost its `apps/` folder in the move. All five were fixed and rerun green. `cargo xtask mk leptos-gates`: doctor OK, 22 of 22 editor smokes, 26 of 26 DOM routes. Docker build OK (entrypoint api-server). Both deploy dry runs and the migration dry run show the new excludes, the `.env` probe and the retired-name check. `cargo xtask mod compile` clean |
| Perturbations | the six M, L1b, A, F, H, T and D agents' proofs. L1b ran twelve law perturbations: rule 7 normal and dev, frontend to fleet, both peer edges, worker to gpu_frame in both tables, three crates under apps/, the fleet glob removed, a duplicate tag-sweep root. D1 ran six preflight and migration perturbations, and G1 five build-flag and refusal-text ones. The orchestrator re-ran worker to gpu_frame: rules 6 and 4 red, restored byte-equal |

### Amendments

| Id | Source | Change |
|---|---|---|
| A1 | coordinator, plan approval | the user agent renamed (D4 reversed) |
| A2 | coordinator, plan approval | the audit of old spellings |
| A3 | coordinator, plan approval | the numbered operator steps, the order and the migration dry run |

### Findings

| Id | Where | Triage | Action |
|---|---|---|---|
| F-S13-M1-01 | relocation tool | FIX (done in W1); NOTE for the tool | a one-segment rust_path row also rewrites the bare word in prose and strings; rows narrowed to two-segment prefixes |
| F-S13-M1-02 | `../api` fixtures | NOTE | held with placeholders during the apply and restored byte-equal |
| F-S13-M1-05 | map_raster_pipeline prose | FIX (W2, T1) | the history-prose test is red at the base commit |
| F-S13-M1-06 | rust-toolchain.toml comments | FIX (W3, R1) | names a file that does not exist |
| F-S13-M2-01 | crate_dependencies.rs deny-lists | FIX (W2, L1) | name the old package; retired by K2 |
| F-S13-M2-02 | frontend documentation audit | FIX (W2, F1) | reads the app twice |
| F-S13-M2-04 | offline_cache_policy fixture name | CLOSE | the classification does not depend on the bundle prefix |
| F-S13-M3-01 | host agent manifest | FIX (done in W1) | the rust_path scope is narrowed so the user agent is renamed by hand |
| F-S13-M4-01 | viewer README | FIX (done in W1) | the ignored-tests command names the model crate |
| F-S13-L1-01 | readiness register | FIX (W2, T2) | the engineering-laws check names its six real cases; a new check names the nine law tests that carry the retired deny-list coverage |
| F-S13-A1-01 | API shutdown test harness | FIX (W2, A2) | sets both asset folders now that the development default refuses outside a checkout |
| F-S13-A2-02 | engineering_laws failpoints count | CLOSE | the manifest list reads each package once; a read-once test is perturbation-proved |
| F-S13-F1-01 | frontend documentation audit | NOTE | the audit does not see past an attribute that spans several lines; the worker's attributes are one line each |
| F-S13-T1-02 | rust-fmt task | NOTE | runs a package fmt check before the workspace one, which already covers it |
| F-S13-T2-01 | mod dev-bootstrap | FIX (W2, T2) | its API step ran a dead npm command; it now starts the database and the API recipe |
| F-S13-T2-02 | milestone announcement | NOTE | the retired database fallback stays open in T-1104 |
| F-S13-T2-08 | readiness progress checkpoint | NOTE | a dated run log keeps its old command spellings |
| F-S13-D1-04 | deploy | FIX (W2, D1) | a deploy refuses while a host still holds an old host-agent name, so no second agent starts beside an old one |

### Old spellings that remain (amendment A2)

After the stage, the old names (`fleet_host_agent`, the `ticketboard` package or binary, `apps/api`,
`apps/frontend`, `apps/offline_service_worker`, `apps/ticketboard`) survive only where a reason
holds:

| Reason | Where |
|---|---|
| archive or relocation manifest | `documentation/archive/`, `documentation/relocation_manifests/`, the retired-path map of T-1280's plan |
| dated run log | `.ai/artifacts/`, the readiness progress checkpoint |
| migration "from" spelling | the host-agent rename migration, its tests and the READMEs and runbook lines that describe it; the earlier single-instance migration |
| closed ticket record | closed `.ai/tickets/*.toml` |
| synthetic relocation-tool fixture | the relocation tool's own scenario tests |
| recorded transcript | the recorded journal lines of the staging fleet procedure's tests |

## Operator steps

The order is: migrate the hosts, move the `.env`, then deploy. The stage itself ran no deploy and
no migration against a real host. `<repo>` is the server checkout the deploys rsync into.

1. On the operator workstation, from the main checkout: move the local API environment file with
   `mv apps/api/.env crates/api/api_server/.env`. Delete the leftover untracked folders
   `apps/api`, `apps/frontend` (its `dist/` and empty `src/v2/` folders) and
   `apps/offline_service_worker`, if present.
2. Preview the host-agent rename migration on the staging host:
   `cargo xtask deploy staging --migrate-host-agent-name --dry-run`. Read the printed script.
3. Run it: `cargo xtask deploy staging --migrate-host-agent-name`.
   - It stops and disables every `fleet_host_agent@N.service`.
   - It moves `~/.local/bin/fleet_host_agent` to `~/.local/bin/game_server_host_agent`, and
     `~/.config/fleet_host_agent/` to `~/.config/game_server_host_agent/`, keeping each credential
     file's mode and owner.
   - It replaces the unit template, then enables and starts `game_server_host_agent@N.service` for
     the same instances.
   - A second run changes nothing.
   - If both the old and the new folder or binary exist, it exits 3 and changes nothing; decide
     which to keep, remove the other by hand, and run it again.
4. On each host (website and staging share the checkout layout): `mkdir -p <repo>/crates/api/api_server`,
   then `mv <repo>/apps/api/.env <repo>/crates/api/api_server/.env`. If `<repo>/apps/api/.tools/`
   exists, move it there too. Check that no relative `SPA_DIST_DIR`, `MAP_ASSETS_DIR`, `UPLOAD_DIR`
   or `EQUIPMENT_DATA_DIR` remains in that `.env`: production needs absolute paths.
5. Deploy: `cargo xtask deploy website` without `TBD_SKIP_SPA_BUILD`, then `cargo xtask deploy staging`.
   - Both refuse before the rsync while the new `.env` is missing.
   - The staging deploy also refuses while any old host-agent name remains.
6. Reinstall the API unit with the install command the website deploy prints, run
   `systemctl --user daemon-reload`, then `systemctl --user restart tbd-website-api.service`. The
   unit now runs `target/release/api-server` from `crates/api/api_server`. Its name and
   `StateDirectory` are unchanged, so uploads and equipment data stay where they are.
7. On each host, remove the stale build and leftovers: `<repo>/target/release/api`, `<repo>/apps/api`,
   and `<repo>/apps/frontend/{dist,dist-debug,target-*}`.
8. Recreate Caddy on its new mount:
   `docker compose -f deploy/compose.staging.yml up -d --force-recreate caddy` (or `podman compose`).
9. GitHub branch protection: the required check "wasm-ci (offline service worker, wasm32 lint)" is
   now "wasm-ci (wasm32 lint)". The api and frontend job names are unchanged.

## Risks

- A text rewrite of a common word (`api`, `frontend`, `ticketboard`) touches unrelated code. Rows
  are scoped tightly and proved with `git grep`, and hand edits cover the rest.
- The relocation tool rewrites synthetic fixture paths. Each dry run is reviewed and the fixtures
  are restored.
- A skipped server step deletes a host's `.env` on the next deploy. The K3 preflight refuses that
  deploy.
- Two agents polling with one credential after a half-done host migration. The migration is
  idempotent and fail-closed.

## Verification

- `cargo xtask ci ci-local`, with the API integration binaries counted;
- `cargo xtask mk leptos-gates`;
- the Docker build;
- `cargo xtask deploy website --dry-run` and `cargo xtask deploy staging --dry-run`, with the new
  exclude and the preflight in the plan;
- `cargo xtask mod compile`;
- `cargo xtask ci verify-workspace-laws`, `cargo xtask refactor relocate --verify`,
  `cargo xtask ci verify-documentation`, `cargo xtask ticket check --strict`;
- perturbation proofs of every changed law, recorded in the execution record.
