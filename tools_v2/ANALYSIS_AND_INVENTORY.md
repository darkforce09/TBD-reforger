# Tooling inventory

What lives under `tools_v2/`, module by module, and what each part is responsible for. The
architecture, its invariants and the dependency direction are in
[ARCHITECTURE_PLAN.md](ARCHITECTURE_PLAN.md); this document is the map.

---

## 1. The four crates

| Crate | Rust files | Lines | Responsibility |
|---|---:|---:|---|
| `verification-core` | 18 | 2,282 | Fail-closed assertion primitives: `Verdict`, `Report`, `Pattern`, the repository lock, the file walk, and child-process isolation. |
| `ticket-engine` | 127 | 18,779 | The ticket domain: typed storage, canonical TOML, validated operations, generated views, wave scheduling and accounting. |
| `developer-tools` | 242 | 48,663 | The heavy services: headless browser harness, Enfusion oracle and archives, map raster pipeline, world export, blueprint compilation, map verification. |
| `xtask` | 374 | 65,160 | The command router: every `cargo xtask ...` verb, and every repository verification. |

A fifth directory, `enfusion_mcp_node_package/`, is not a crate: it holds the npm manifest,
lockfile and node version that pin the `enfusion-mcp` server, and it sits outside every crate root
so the dependency tree `npm ci` installs beside it is never walked by a crate-scoped file scan.

---

## 2. `verification-core`

The foundation every check is built on. It depends on no workspace crate.

| Module | Responsibility |
|---|---|
| `verdict` | The outcomes a check can reach — pass, fail, did-not-run — and their exit codes. A check that cannot run never reports clean. |
| `report` | Accumulated findings and the operator-facing rendering of a run. |
| `pattern` | Compiled literal and regular-expression matchers, so a missing external search tool cannot read as a clean result. |
| `scan` | The fail-closed file walk: a declared-but-absent root is an error, and an empty input is never a green. |
| `lock` | The repository verification lock, shared across worktrees so two gates cannot run unserialised against one build cache. |
| `proc` | Child processes: `runner` spawns into its own process group and enforces a deadline, `stream` drains the pipes on dedicated threads, `lookup` resolves programs on `PATH` and waits on a condition. |

---

## 3. `ticket-engine`

The ticket corpus and everything that reads or writes it. It depends on no workspace crate.

| Module | Responsibility |
|---|---|
| `model` | The typed tickets: program and work shapes, scope, status, class. |
| `encoding` | Canonical TOML: one rendering per ticket, byte-stable, with serde aliases for the spellings older blobs carry. |
| `store` | Fail-closed corpus loading over every `.ai/tickets/T-*.toml`, parents and children alike. |
| `ops` | Validated mutations: each produces a post-image the validation rules accept, or refuses before any write. |
| `registry` | The ticket files on disk, the read-only value projection the Markdown views read, and the ticket statuses at a past revision. |
| `validation` | Schema, vocabulary, ownership, body caps, readiness tiers, the ship gate, hierarchy, accounting, debt and repository references. |
| `cli` | Briefs, queries, mutations, shipping, batch selection and configuration, as command services the host calls. |
| `sync` | The generated Markdown views, the dispatch queue JSON, roadmap markers and the gap-analysis ticket column. |
| `wave_lock` | Dependency packing, file-disjoint collision selection, the deterministic lock file, drift checks, reservations, and the archived wave plans at the revisions that still carry them. |
| `metrics` | Measured run receipts, elapsed time, token accounting and the derived estimates. |
| `vocab` | The four-level scope vocabulary, resolved at every corpus load. |
| `corpus_pins` | The corpus facts no ticket file states: the programme tickets, the ids that must never be minted, and the editor gap rows no ticket claims. |
| `repository` | Every repository path the crate reads or writes, spelled once, with a `documentation` submodule for the ones under the documentation tree, plus checkout-root discovery. |
| `timestamp` | RFC 3339 UTC lifecycle stamps and their strict parse. |

---

## 4. `developer-tools`

The heavy services, behind six executables: `enf`, `gate`, `mcpd`, `world`, `map`, `capture`.

| Module | Responsibility |
|---|---|
| `browser_testing` | The headless Chrome harness: the CDP client, the static SPA server and `/api` proxy, the DOM oracle, the editor smoke scenarios, the fontconfig and liveness diagnostics, and screen capture. |
| `enfusion_tooling` | The Enfusion oracle: the symbol index, the Script API parse, vanilla source carve, citations, the capability matrix, and the MCP broker. |
| `enfusion_pak` | The `.pak` archive reader shared by the oracle and the blueprint lane. |
| `map_raster_pipeline` | Map imagery: the SAP aerial ortho, the cartographic render and tile pyramid, the glyph atlas, the inland-water lane, the satellite container and the label archives. |
| `world_export_pipeline` | The world export: topo decode, texture decode, prefab classification, chunk partitioning, density grids, forest contours, roads, the DEM, the binary twins and the mathematical phase gate. |
| `blueprint` | Building blueprints from voxel dumps: mesh decode, architectural analysis, BVH construction, the prefab archive and the parity report. |
| `map_verification` | Engine-backed checks over the committed map assets: labels, object goldens, the terrain manifest, the BLAS manifest and world line-of-sight. |
| `repository_layout` | Every repository path this crate reads or writes, spelled once. |
| `repository_paths` | Checkout-root discovery, independent of `ticket-engine` so neither foundational crate depends on the other. |

---

## 5. `xtask`

`cargo xtask <domain> <verb>` is the one entry point to repository operations.

### 5.1 Commands

| Domain | What it drives |
|---|---|
| `agent_context` | The context budget an agent session is handed. |
| `build` | The build recipes: the workspace, the SPA, the engines, the API. |
| `ci` | The task index every CI job runs, and the composite local replay. |
| `db` | The development Postgres lifecycle and the integration-test database lane. |
| `debug` | Operator probes: direct join, remote logs. |
| `deploy` | The website and staging deployments, the host agent install, and the guarded database backup, restore and drill. |
| `fetch` | Vanilla Enfusion API and source retrieval. |
| `generate` | Generated source: the bitmap font table. |
| `map` | The map lane, delegated to `developer-tools`. |
| `mcp` | The Enfusion MCP bridge: calls, the daemon, the smoke and the selftest. |
| `mod_ops` | The game mod: compile, playtest, world boot, mission and API tests, and its own wave driver. |
| `platform` | The platform wave lifecycle, slice worktrees, preflight and receipts. |
| `reproduction` | Reproductions of reported defects, end to end. |
| `schema` | Contract validation, object enums, type inventory, specification consistency and the glyph manifest. |
| `setup` | Machine setup: client addons, the MCP game root, the server profile, the Workbench. |
| `ticket` and `wave` | Adapters that delegate to `ticket-engine`. |
| `verify` | Every repository verification, by name. |

### 5.2 Verifications

| Group | What it holds |
|---|---|
| `architecture` | Editor and ORBAT coherency, route tags, engine layer boundaries. |
| `ci` | The parity between the verification surface and the CI task index, and the workflow shell rules. |
| `database` | SQL shape rules and the seed checks. |
| `deployment` | The staging compose path pin. |
| `language_bans` | Zero tracked shell, Python or Node sources, and the file-length limits. |
| `licensing` | No upstream reference source reaches the shipping mod. |
| `map_assets` | Label, manifest and alignment checks, delegated to `developer-tools`. |
| `mod_scripts` | Comment contracts, spawn determinism, UI layouts and the mission REST limits over the EnfScript sources. |
| `registry` | Object alias against spawn registry parity. |
| `schemas` | Contract citations, content budgets, object enums, type inventory, specification consistency, kit references, wire-field readers, glyphs and mission validation. |

### 5.3 Shared

| Module | Responsibility |
|---|---|
| `cli` | The top-level parser, argument preprocessing and routing. |
| `core` | Checkout-root discovery, the container-to-host bridge, Cargo target-directory handling, and `repository_layout`, which spells every repository path this crate reads. |
| `tests` | The structural rules: dependency direction, file limits, test placement, path ownership and prose rules. |

---

## 6. Data beside the crates

| Directory | Read by |
|---|---|
| `xtask/deploy/` | `cargo xtask deploy website` and `deploy staging`: the environment example, the Caddy site file and the systemd units. |
| `xtask/dedicated_server_profiles/` | `cargo xtask mod playtest` and `mod world-boot`. |
| `xtask/fixtures/mcp/` | `cargo xtask mcp selftest`. |
| `developer-tools/fixtures/dom_oracle/` | The DOM oracle's frozen route captures. |
| `developer-tools/test_fixtures/` | The blueprint and contract fixtures the unit tests read. |
| `developer-tools/gate-env.json` | `gate doctor`: the pinned chromium, toolchain and resource limits. |
| `ticket-engine/tests/fixtures/` | The receipt, estimate and corpus fixtures, shared with `xtask`. |
| `enfusion_mcp_node_package/` | Every agent session and every `cargo xtask mcp` verb. |
