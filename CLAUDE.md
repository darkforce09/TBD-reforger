# TBD Reforger Platform

Platform suite for the "TBD" Arma Reforger milsim community: Discord auth, event / ORBAT scheduling, mission library, the Mission Creator (a top-down 2D mission editor), game-server fleet control and telemetry, leaderboards, doctrine wiki, the TBD game mod, and Enfusion mod tooling.

---

## 1. Core Project Laws

1. **No silent deferrals.** Do the whole ask. Never invent "out of scope", "deferred", "fold forward" or ship an MVP and call the task done. Only the operator's explicit words ("defer X", "skip X", "not this pass") defer a piece; quote them in the commit body and record the piece as a `deferred` ticket in `ttm`. Soft plan prose ("optional", "if feasible", "P1 later") is not authorization, even when you wrote it. Truly blocked (secrets, Workbench down, no GPU): stop and ask.
2. **Git: direct to `main`.** Never create a branch by hand. The only branches are the `slice/<slug>` branches the ticket manager's runner (`ttm worktree`, `ttm wave`) creates, merges and deletes itself, in worktrees under `.worktrees/`. Commit only when asked; a commit that lands a ticket names its slug in the subject.
3. **Clean architecture over hacks.** No ad-hoc code to "just make it work"; when the clean fix needs a structural refactor, plan it and do it.
4. **Self-describing names.** Every folder, file, module and symbol name says what it is without project history or jargon; no cryptic abbreviations.
5. **Group variants.** No flat dumps of dozens of files or variants in one folder; group them into well-named subfolders.
6. **Strict boundary layers.** Product crates sit at `crates/<category>/<crate>` (frontend at `crates/frontend/<layer>/<crate>`), tool crates at `tools/<category>/<crate>`, each declaring `category`, `tier` and `targets`; tiers point down and no member depends on an application crate (`api_server`, `frontend_application`, `offline_service_worker`, `game_server_host_agent`). Graphics crates know no map concept; `wgpu` lives only in `crates/map_rendering/` and the GPU crates; Leptos only in `crates/frontend/`; sqlx and axum only in `crates/api/` (plus the named tool harnesses); the `xtask` closure holds no tokio, axum, reqwest, resvg or image. `mod/` holds no Rust crate. `cargo xtask verify crate-tiers` and `cargo xtask verify crate-anatomy` enforce it; every rule is in [crate boundary rules](/documentation/standards/crate_boundary_rules.md).
7. **File size and tests (guidance).** Keep production files at or under 500 lines; split by responsibility past that (`cargo xtask verify file-length` warns). No inline test modules: tests live in sibling files declared via `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
8. **In-code documentation.** Comments describe what the code does now and why (invariants, models, constraints, failure modes); never history, ticket ids or retired codebases. Rust: non-trivial modules carry a `//!` header (`**Role:**`, `**Position:**`, `**Signals & state:**`, `**Invariants:**`); public items carry `///` docs with valid intra-doc links. Enfusion mod (mandatory): `//! @authority server|client|owner` on context-dependent methods, `//! @rpc <Reliable|Unreliable> <Server|Owner|Broadcast>` above every `[RplRpc]`, `//! @replicated <prop>` above every `[RplProp]`, `//! @contract <schema>#<pointer>` on hand-written JSON DTOs, `//! @route <METHOD> <path>` on REST call sites. Full style: [documentation standards](/documentation/standards/documentation_standards.md).
9. **API and contract parity.** Backend models (`crates/api/api_<domain>/src/models/`) are the snake_case source of truth; contract types are generated from `contracts/definitions/*.json` (`cargo xtask ci schema-codegen`); frontend DTOs (`crates/frontend/foundation/frontend_api_dtos/src/`) mirror the models under golden-test parity.
10. **Keep documentation truthful.** When a change alters a folder's surface, commands or boundaries, update its README.md and the feature docs it changed. [documentation/README.md](/documentation/README.md) is the entry; terms follow the [glossary](/documentation/glossary/README.md): the editor is the **Mission Creator**, the authored document a **mission** (never "scenario" in prose), Enfusion's world plus game-mode config the **mission header**, a scheduled session an **event**, its domain **operations**.
11. **Pre-alpha test policy.** Test core logic only: math and geometry, file and wire formats, CRDT merge and undo, auth and permissions, mission compile and validation, data integrity, guards on destructive operations. Never pin source text, prose, CSS classes, constants, file layout or `Debug`/`Display` output. One API integration binary per domain. No failpoint or property-evidence suites. Slow suites (browser gates, mod world boot) run nightly or on demand. Before committing: `cargo xtask mk rust-fmt`, `cargo xtask mk rust-clippy`, and the tests of the crates you touched.
12. **Gates are reproducible and fail fast.** A gate's external tools are pinned in committed files (browser pins in `tools/browser_testing/browser_gate_suites/gate-env.json`, the toolchain in `rust-toolchain.toml`) and bumped deliberately in the commit that needs them. A gate that cannot run is an infrastructure bug to fix, never a silent skip or "commit anyway"; record environment breakage in `documentation/known_bugs/`.
13. **Everything stays in the repository.** Nothing project-related goes to `~`, `~/.cache` or `/tmp` (a session scratchpad aside): machine-local state lives in the gitignored `.workstation/`, logs in `.workstation/logs/`, worktrees in `.worktrees/`, build output in `target/`.

---

## 2. Agent Environment and Workflow

- **Two glibcs.** Agent sessions run in a Debian container (`claude-desktop`, glibc 2.36); the host is Fedora with a newer glibc. Builds never share a target folder across them: in-container cargo builds into `target/container/` (the session sets `CARGO_TARGET_DIR`), host cargo into `target/host/` through `hcargo` (`.workstation/bin/hcargo`, on `PATH`). `cargo xtask` recipes pick the folder themselves and refuse a folder another glibc stamped.
- **Host-only tools.** `podman`, Steam, Workbench and `ArmaReforgerServer` run on the host; tooling reaches them through `distrobox-host-exec` (`process_runner::host_execution`). `git push` runs in the container (git-lfs).
- **Machine-local state** (`.workstation/`, gitignored): `bin/` (hcargo, hrustfmt), `logs/`, `enfusion_mcp_game_root/` (the MCP pak farm, `cargo xtask setup mcp-game-root`), `playtest_server/`, `reforger_extract/`, `enfusion_unpacker/`, run records.
- **Tickets** live in the external ticket manager `ttm` (`~/.local/bin/ttm`, project `reforger`, repository `/run/media/system/Disk_2/Projects/tbd_ticketmanager`); every command takes a slug or a legacy `T-<n>` id. A ticket's `executor` says who may take it: `claude-code` an AI agent, `documentation` a docs pass; `workbench`, `human` and `ci` mean stop and wait for that party.
- **Orchestration is opt-in.** Work in the chat by default. Use sub-agents only for broad searches or genuinely parallel, independent pieces. A multi-agent program runs only when the operator asks for one, and each sub-agent's plan is approved before it starts. [Sub-agent orchestration](/documentation/runbooks/sub_agent_orchestration.md) is the procedure.
- **Bash discipline.** One build, test or gate per Bash call, each with its own log under `.workstation/logs/`; read the log's tail instead of streaming output. The PreToolUse guard (`cargo xtask ai guard`) refuses uncapped `grep`/`rg` and whole-file re-reads.

---

## 3. Repository Map

[Workspace layout](/documentation/architecture/workspace_layout.md) describes every folder and member; [Where does X go?](/documentation/standards/where_does_x_go.md) places a new file.

| Folder | Holds |
|---|---|
| `crates/` | Product crates by category: foundation, contracts, geometry, world formats, terrain, world objects, line of sight, map overlay, streaming, map rendering, paper doll, graphics, mission, mission editing, ballistics, api (`api_server` on :8080), fleet (`game_server_host_agent`), frontend (Leptos SPA `frontend_application` on :3000, `offline_service_worker`) |
| `tools/` | Tool crates by category (foundation, commands, checks, enfusion, browser testing, staging, map assets), the binaries `xtask` and `developer_tools`, the pinned enfusion-mcp npm package |
| `mod/` | Enfusion addons `tbd-framework` (the game mod), `tbd-export`, `tbd-emcp`; `References/` (licensed upstream lanes, gitignored); `reference_symbol_index/` (their committed symbol tables) |
| `contracts/` | JSON Schemas (`definitions/`), rules, catalogs, golden fixtures |
| `assets/` | Terrain datasets (served at `/map-assets`), the world-object glyph set |
| `deploy/` | Dockerfile, compose files, Caddy, systemd units, `deploy.env.example` and `api.env.example` (the gitignored `deploy.env` and `api.env` sit beside them) |
| `documentation/` | All documentation; entry and authority ladder in `documentation/README.md` |
| `.repository_root` | The checkout-root marker every tool's root walk looks for |
| `.workstation/`, `.worktrees/`, `target/` | Gitignored machine state, linked worktrees, build output (`target/host`, `target/container`) |

---

## 4. Canonical Commands

Configuration: copy `deploy/api.env.example` to `deploy/api.env` (`APP_ENV=development`, Postgres on 5434); the API reads it from the checkout root or `TBD_API_ENV_FILE`. Step-by-step: [local development](/documentation/runbooks/local_development.md).

```bash
# Local stack, in this order
cargo xtask db up              # Start local Postgres container
cargo xtask mk rust-api        # Axum API on :8080 (applies pending migrations on boot)
cargo xtask db seed            # Apply the five development SQL seeds (needs the migrated tables)
cargo xtask mk leptos          # Leptos SPA on :3000 (trunk serve --release; proxies /api and /map-assets to :8080)
cargo xtask db down            # Stop local Postgres container (keeps volume)

# Quality gates and tests
cargo xtask mk rust-fmt        # Format check (pre-commit)
cargo xtask mk rust-clippy     # Clippy with warnings denied (pre-commit)
cargo xtask ci ci-local        # Replay the CI check suite locally
cargo xtask mk ci-local-leptos # Frontend checks: fmt, clippy wasm32, test, trunk release build
cargo xtask db test-it         # API integration tests (requires db up)
cargo xtask mod compile        # Compile check of the Enfusion mod scripts
cargo xtask mk leptos-gates    # Headless Chrome editor gates (nightly / on demand)
cargo xtask verify link-check  # Links, anchors, backticked paths and cited commands resolve
cargo xtask help               # Every ci / mk / db task with its help line

# Tickets (ttm, project reforger)
ttm -p reforger next           # Running tickets and the next ones to take
ttm -p reforger show <ref>     # One ticket (slug or legacy T-<n>)
ttm -p reforger check          # Validate the project's tickets

# Deployment (deploy/deploy.env)
cargo xtask deploy website --dry-run  # Print the plan
cargo xtask deploy website     # Rsync, build the API + SPA on the server, restart the unit
cargo xtask deploy staging     # Game-server instances, host agents and relay on the staging host
cargo xtask staging preflight  # Read-only preconditions of the staging procedures
```

### Dev Login (No Discord Required)
`APP_ENV=development` exposes `GET /api/v1/auth/dev-login?role=guest|enlisted|leader|mission_maker|admin` (any other or missing role signs in as `admin`). Open in browser or read `access_token` from the 302 `Location` fragment (`/auth/callback#…`) for API testing.
