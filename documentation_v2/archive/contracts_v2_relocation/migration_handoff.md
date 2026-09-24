**Status:** archived

# Contracts Relocation Handoff

`packages/tbd-schema` is gone. Its contents live in `contracts_v2/`, sorted into four kinds of data that a single flat directory had been mixing.

---

## 1. Where everything went

| Legacy path | New path | Count |
|:---|:---|:---:|
| `schema/*.schema.json` | `definitions/` | 24 |
| `bridge/bridge-messages.schema.json` | `definitions/` | 1 |
| `bridge/bridge-contract.md` | `definitions/bridge-messages.md` | 1 |
| `rules/prefab-classify.json` | `rules/` | 1 |
| `registry/kit-aliases.json` | `rules/kit-aliases.json` | 1 |
| `registry/registry-{items,compat}.workbench.json` | `catalogs/` | 2 |
| `registry/*.sample.json`, `registry.example.json`, `registry.vanilla-poc.json` | `fixtures/registry/` | 8 |
| `golden-missions/*.json` | `fixtures/missions/valid/` | 9 |
| `golden-missions-invalid/*.json` | `fixtures/missions/invalid/` | 6 |
| `golden/**` | `fixtures/map/` (keeping `density/`, `phased/`) | 20 |
| `enfusion/*.json` | `fixtures/enfusion_samples/` | 10 |
| `bridge/samples/*.json` | `fixtures/bridge_samples/` | 6 |

Every file kept its name and its bytes. No schema changed.

**Retired:** `spikes/` (four proof-of-concept notes, each superseded by shipped code), `VERSION` and `CHANGELOG.md` (a package version nothing read), the package `README.md` and `.gitignore` (an npm artifact from before the Node eradication), and `golden-missions-invalid/README.md` (replaced by the directory's new README).

## 2. Why `catalogs/` is separate

`registry-items.workbench.json` and `registry-compat.workbench.json` are the live arsenal: `cargo xtask db seed` imports them into Postgres and the Scenario Creator's loadout editor is limited to what they contain. Their sampled counterparts sit in `fixtures/registry/` and are what the ingest tests assert against.

Filed together, an ingest that fell back to a sample would fill the arsenal with sample data and pass every test, because the tests assert against the samples. `validate_all` now takes a `catalog_file` accessor distinct from `reg_file`, so the two cannot be confused in code either.

## 3. Consumers repointed

Compile-time `include_str!` embeds in the API (`missions/contract/schema_validators.rs`, `missions/handlers/mission_default_overrides.rs`), the map engine (`data/scenario/compiler/kit/aliases.rs`) and the frontend arsenal rules; roughly thirty test files across those crates; the `apps/website/Dockerfile` COPY set; the `schema.yml` and `contracts.yml` workflow path filters; the codegen input directory and the `// Source:` header it writes into every generated file.

In tooling, path literals were replaced by `developer_tools::repository_layout` in a prior commit, so the flip was one edit to that module. `SCAN_ROOTS` in the citation gate drops `packages` — the contract tree holds data, not code that declares a citation — and `verify-doc-layout` now walks `apps`, `contracts_v2` and `assets_v2`.

## 4. Validation

| Check | Result |
|:---|:---|
| `cargo xtask schema validate` | All contracts valid |
| `cargo xtask schema citations` | 98 citations across 5,262 files, all resolve |
| `cargo xtask schema codegen` + `ci verify-codegen-fresh` | Regenerated, no drift |
| `cargo xtask ci verify-doc-layout` | Pass |
| `cargo xtask verify file-length` | 2,518 files, 0 violations |
| `cargo xtask ticket check` | `check OK` |
| `cargo test -p xtask` | 631 passed |
| `cargo test -p developer-tools` | 246 passed |
| `cargo test -p website-map-engine --all-features` | 1,422 passed |
| `cargo test -p website-frontend` | 1,342 passed |
| `cargo test -p website-api --lib` | 284 passed |
| `cargo test -p website-api --test registry_compat --test factions` | 3 passed |

## 5. What still names the old paths

`.ai/tickets/` and `.ai/artifacts/` keep their original spellings: they are historical records of work done when the tree was at `packages/`, and rewriting them would falsify the record. The same applies to per-ticket plans under `docs/plans/` and the ticket-numbered specs under `docs/specs/`, and to the archival documents `SHIPPED_HISTORY.md`, `MONOREPO_MIGRATION.md`, `CODEBASE_AUDIT_2026.md`, `FACTORY_RUN_2026-09.md`, `GROK_WAVE_130_HANDOFF.md` and `engine_split_phase3_baseline.md`. Live runbooks, standards, agent rules and architecture specs were all rewritten.
