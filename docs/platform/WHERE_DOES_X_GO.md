# Where does X go?

T-171 pin: canonical home for each artifact class after the `apps/website/{api,frontend}` nest. Lifted from [`.ai/artifacts/t171_inventory.md`](../../.ai/artifacts/t171_inventory.md) §9.

| X | Home (post-T-171) |
|---|---|
| SPA page module | `apps/website/frontend/src/<page>.rs` (one module per page; route in `src/router.rs`) |
| API handler | `apps/website/api/src/handlers/<resource>.rs` (models in `src/models/` = wire contract) |
| DB migration | `apps/website/api/migrations/NNNN_*.sql` (sqlx, embedded, runs on boot) |
| Data seed | `apps/website/api/seeds/*.sql` (applied by root `make seed`; mock_data.sql manual-psql only) |
| Editor/gate smoke | `tools/tbd-tools` (`gate` bin) wired through `make leptos-gates` |
| Test fixture | crate-local `tests/fixtures/` beside consumer; NEVER `.ai/artifacts/` |
| Cross-crate contract golden | `packages/tbd-schema/{schema,golden,golden-missions,registry}/` |
| Map asset | `packages/map-assets/<terrain>/` (LFS: dem png + sat .tbd-sat only; staging/tiles rebuildable local) |
| Ticket | `.ai/tickets/registry.json` + `./scripts/ticket sync` (generated TICKET_*.md never hand-edited) |
| Spec / doc | `docs/**` only — never `apps/**/docs` or `packages/**/docs` (verify-doc-layout enforces) |
| Ops script | `scripts/{website,mod,deploy}/` (mod scripts = tooling, distinct from OFF-LIMITS `apps/mod/`) |
| Shared engine code | `crates/map-engine-{core,render,wasm}` |
| Repo tooling | `xtask` (gates/codegen/ticket lib) · `tools/tbd-tools` (gate harness + asset pipelines) |
