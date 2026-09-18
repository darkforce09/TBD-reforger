# `website-api` — the TBD Reforger backend

The Axum REST API and Server-Sent Events hub behind the web platform. It serves `/api/v1` to the
Leptos single-page app and to the Enfusion game servers, exposes `/healthz` and `/metrics`, serves
the uploaded media under `/uploads` and the terrain assets under `/map-assets`, and owns the
Postgres schema through the SQL migrations in `migrations/`.

This document is the live atlas of the crate. The blueprint that the layout implements is
[`ARCHITECTURE_PLAN.md`](./ARCHITECTURE_PLAN.md); the pre-refactor catalog is
[`ANALYSIS_AND_INVENTORY.md`](./ANALYSIS_AND_INVENTORY.md).

---

## 1. `src/` layout

```text
src/
├── lib.rs                      Crate root: the module tree and the architecture-rule test hook.
├── bin/
│   ├── api.rs                  Server entrypoint: config → pool → migrations → workers → router → serve.
│   └── import_registry.rs      Offline ingest of registry envelopes into Postgres.
├── core/                       Cross-cutting foundations. Imports no domain.
├── background_workers/         The interval tasks the binary arms at boot. Imported only by `bin/api.rs`.
├── administration/             Member roster, moderation actions, and the audit log.
├── command_center/             Dashboard, leaderboards, and per-player statistics.
├── community_content/          Announcements, wiki, vehicle database, modpacks, media uploads.
├── identity_and_access/        Discord OAuth2, session tokens, the profile, the Arma link handshake.
├── match_telemetry/            Game-server ingest: live status heartbeat and finished-match reports.
├── missions/                   Scenario library, versions, armory, registries, approvals, export.
├── operations/                 Event calendar, ORBAT slotting, service records, fire missions.
├── server_infrastructure/      Dedicated-server registry, live status SSE, RCON console.
└── tests/
    └── architecture_rules.rs   Executable statements of the layout rules below, checked against `src/`.
```

Each of the ten module directories carries its own `README.md` with its responsibility, its public
surface, and a complete listing of its files. Each of the eight domains has the same shape:
`mod.rs`, `routes.rs`, `handlers/`, `services/`, `models/` (plus `contract/` and `validation/` in
`missions/`).

## 2. How the `/api/v1` table is composed

Each domain owns exactly one `routes.rs` with a single `pub fn routes`, listing its own
registrations. `core::http_router::api_v1_routes` merges the eight tables and nests the result
under `/api/v1`; it adds no prefix of its own. **A public URL is therefore the literal written in
the domain's `routes.rs`, with `/api/v1` in front of it** — `/missions/{id}` in `missions/routes.rs`
is `GET /api/v1/missions/{id}`.

Authorization tiers are enforced per handler by the extractor each one takes (`AuthUser`, the
role-gated newtypes, `ServiceAuth`), not by merge order or by path prefix.

`core::http_router::router` wraps that tree in the global middleware chain — outermost first:
request id → access logging → Prometheus observation → panic recovery → CORS → body limit → rate
limit. `/map-assets` is mounted *below* the rate-limit layer so cold terrain streaming is never
limiter-bound; that seam is commented at its call site and pinned by tests.

## 3. Placement rules

**Dependency direction.** `core` imports no domain, with two exceptions:
`core/application_state.rs` (it holds the Discord and webhook service instances) and
`core/http_router.rs` (it merges the route tables). A domain's handlers, services and models never
import another domain's **handlers** — cross-domain reuse goes through a service or a model.
`background_workers` is imported only by `src/bin/api.rs`. `src/tests/architecture_rules.rs`
enforces all of this against the source text, so a new file is covered the moment it is added.

**Where shared logic lives.** Logic that more than one domain needs and that names no domain
concept belongs on the shared floor in `core/`: offset pagination (`core::http::pagination`),
Postgres SQLSTATE predicates (`core::database::postgres_errors`), the JSON wire formats
(`core::wire_format`), HTML sanitation and the URL write-boundary guard (`core::text`), the outbound
`429` retry (`core::http_client::retry_on_429`), and the token primitives
(`core::authentication_primitives`).

Logic that *does* name a domain concept stays in that domain's `services/`, and other domains call
it there rather than re-deriving the query:

| Shared operation | Home |
|:---|:---|
| `load_user` | `identity_and_access/services/user_lookup.rs` |
| `load_mission`, `load_mission_or_404`, `mission_title_terrain` | `missions/services/mission_lookup.rs` |
| `load_cargo_phys_catalog` | `missions/services/cargo_catalog.rs` |
| `write_audit`, `actor_display_name` | `administration/services/audit_writer.rs` |
| `load_modpack`, `load_current_modpack` | `community_content/services/modpack_lookup.rs` |
| `publish_server_status`, `publish_all_server_statuses` | `server_infrastructure/services/status_broadcast.rs` |
| Event effective status and transitions | `operations/services/event_status_rules.rs` |
| Mortar ballistics (`solve_fire_mission`) | `apps/website/map-engine/src/data/scenario/ballistics/` |

**Adding an endpoint.** Write the handler in `src/<domain>/handlers/`, register it in that domain's
`routes.rs`, and put anything a second surface would need into that domain's `services/`.

## 4. Test conventions

- **Unit tests** live in a sibling file, declared from the production file as
  `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`. Inline `mod tests` blocks are rejected by
  `src/tests/architecture_rules.rs`.
- **Integration suites** are the top-level files under `tests/`. Cargo builds one test binary per
  top-level `tests/*.rs`; shared support lives in `tests/common/` (a subdirectory, so it produces no
  binary of its own) and is pulled in with `mod common;`.
- Production files stay under 500 lines, test files under 1000 (`cargo xtask verify file-length`).
- `cargo xtask verify route-tags` cross-checks the eight route tables against the tagged route
  inventory in both directions, so a route cannot be added, moved or dropped unnoticed.

## 5. The codegen contract

`src/missions/contract/generated/` is `typify` output produced from
`contracts_v2/definitions/*.json` by `cargo xtask ci schema-codegen`. Never hand-edit it: change
the schema and regenerate. `cargo xtask ci verify-codegen-fresh` diffs the directory to prove the
committed files match their schemas, and the directory is exempt from the prose rules in
`src/tests/architecture_rules.rs` because its wording belongs to the generator.

`src/missions/contract/loadout_projection.rs` is the one deliberate exception — `typify`'s output
for the loadout-export root `oneOf` is lossy, so that model is hand-maintained beside the generated
directory.

The snake_case models under `src/<domain>/models/` are the API contract's source of truth; the
frontend DTOs in `apps/website/frontend/src/v2/core/api/dto/` mirror them under golden-test parity.

## 6. Running it

Configuration is read from the environment at boot; `.env` holds the development values
(`APP_ENV=development`, Postgres on port **5434**), and `.env.example` documents every variable.

```bash
cargo xtask db up              # Start local Postgres container
cargo xtask db down            # Stop local Postgres container (keeps volume)
cargo xtask db seed            # Apply development SQL seeds
cargo xtask mk rust-api        # Axum API on :8080 (runs migrations on boot)
cargo xtask mk leptos          # Leptos SPA on :3000 (Trunk release build)
cargo xtask db test-it         # Rust backend integration tests (requires db up)
cargo xtask ci ci-local        # Replay the full CI check suite locally
```

With `APP_ENV=development`, `GET /api/v1/auth/dev-login?role=admin|mission_maker|enlisted` mints a
session without Discord — open it in a browser, or read `access_token` out of the `302` `Location`
fragment for API testing.

`SKIP_MIGRATE=1` keeps the binary from running migrations, for a harness that owns the schema of a
shared database itself.

## 7. Further reading

- [`ARCHITECTURE_PLAN.md`](./ARCHITECTURE_PLAN.md) — the domain-decomposition blueprint, the
  middleware hierarchy, and the rate-limit seam.
- [`ANALYSIS_AND_INVENTORY.md`](./ANALYSIS_AND_INVENTORY.md) — the pre-refactor inventory, kept for
  reference.
- `PHASE_1_HANDOFF.md` … `PHASE_5_HANDOFF.md` — the record of how the layout was reached and why
  each piece sits where it does.
- The ten module `README.md` files under `src/`.
