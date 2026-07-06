# T-145 — Go → Rust backend rewrite: complete engineering record

**Outcome:** the entire `apps/website` backend was rewritten from Go (Gin + GORM) to
Rust (Axum + sqlx), verified equivalent under 9 gates, and the Go code deleted. Rust
is now the sole backend.

- **Branch / worktree:** `t-145-rust-rewrite` / `tbd-reforger-t145`
- **Commits:** `9cc7a161` (Rust port, added alongside Go) → `c05ddbc9` (cutover, −13,099 lines of Go)
- **Final shape:** 64 Rust source files · 15 integration test files · 76 tests (0 failed) · 2 migrations · **0 Go files**
- **Toolchain:** Rust 1.95 (edition 2024) at `~/.cargo/bin`; Node 26 (nvm); Postgres 18 (podman, :5434)

---

## 1. Scope & invariants

Only the Go backend changed. Four things were **invariants** the rewrite had to preserve
byte/semantically-identical, each with a proof:

| Invariant | Proof |
|---|---|
| Frontend wire API (`apps/website/frontend`) | differential `≡` (G5) |
| PostgreSQL schema (29 tables, 12 enums, indexes, MV) | `pg_dump --schema-only` diff empty (G2) |
| Mod compiled-doc + export envelope | `≡` + schema-valid + field assertions (G6) |
| Cross-boundary codegen (`packages/tbd-schema`) | `make schema-codegen` → `git diff` empty (G4) |

## 2. Stack decisions

| Concern | Go | Rust (locked) |
|---|---|---|
| Web framework | Gin | **Axum** (tokio + tower-http) |
| DB layer | GORM + pgx | **sqlx** — runtime-checked queries, plain-SQL migrations (no `query!` macros → no `.sqlx` cache) |
| JWT | golang-jwt | jsonwebtoken (HS256, `rust_crypto`) |
| HTML sanitize | bluemonday | ammonia |
| JSON schema | santhosh-tekuri | jsonschema |
| Rate limit | x/time/rate | governor |
| HTTP client | net/http | reqwest + rustls-ring |
| Fidelity | — | **faithful 1:1 first**, cleanups deferred |

## 3. The verifiability model — 9 gates

Completeness is a **scripted bijection** (count-equality Go↔Rust); behavior is
**differential** (`≡` compared Go-vs-Rust). The `≡` relation: identical status; identical
values for the present header subset {content-type, etag, location, content-disposition,
cache-control}; bodies canonically-equal — **numbers by value** (`0`≡`0.0`), **strings
decoded** (`<`≡`<`), **objects order-insensitive but key-presence-sensitive** (absent ≠
null ⇒ omitempty parity), **arrays order-sensitive**, **temporal/string leaves exact**.

| Gate | Assertion | Result |
|---|---|---|
| **G1** Route bijection | Rust exposes exactly the 84 routes; `@route` tags = 84; verbs match | ✅ 84=84 (9 DEL / 38 GET / 6 PATCH / 28 POST / 3 PUT) |
| **G2** Schema | `pg_dump --schema-only` Go vs Rust → normalized diff empty | ✅ 0 lines |
| **G3** Tests | all 68 Go tests represented (mapping table) + `cargo test` green | ✅ 68→76, 0 failed |
| **G4** Codegen | `make schema-codegen` → `git diff --exit-code` empty | ✅ deterministic |
| **G5** Differential | Go & Rust behind seeded DBs; replay corpus → every response `≡` | ✅ 18/18 |
| **G6** Compiled doc | ModDoc(Rust) `≡` locked contract + schema-valid + field assertions | ✅ |
| **G7** Concurrency | (a) refresh single-winner, (b) ORBAT slot single-winner | ✅ G7a + G7b |
| **G8** Sanitizer | ammonia output vs golden; no-XSS property | ✅ |
| **G9** Soft-delete | delete a row → hidden across every endpoint | ✅ |

## 4. Encoder & type-mapping contract (the anti-regression core)

Go `encoding/json` ≠ serde. Equivalence is **canonical JSON equality**, not byte-identity.
Each hazard has an exact resolution:

| # | Hazard | Resolution |
|---|---|---|
| 5 | Timestamps (44 fields) — Go RFC3339Nano trims trailing-zero nanos (`.5` not `.500`) | custom `go_time` serializer reproducing `time.Time.MarshalJSON` |
| 5b | 2 `date` columns (`starts_on`,`ends_on`) — Go emits full `…T00:00:00Z` | decode as `DateTime<Utc>` (midnight), same serializer |
| 6 | omitempty (~60 fields) — Go omits nil ptrs and empty strings | `Option<T>` + `skip_serializing_if`; string-empty → `str::is_empty` |
| 7 | numeric (6 cols) — sqlx decodes `numeric`→Decimal, never f64 | `CAST … AS float8` → f64 (attendance_rate, 2×server_fps, azimuth_deg, kd_ratio, command_win_rate) |
| 8 | jsonb (2 cols) — Go passes Postgres-normalized bytes verbatim | decode as `Json<Box<RawValue>>` — passthrough |
| 9 | `time_of_day` — `time` column, Go field `string` | `SELECT time_of_day::text` |
| inet | `ip` — Go emits bare host, `::text` keeps `/32` | `host(ip)` (found by G5) |

**Non-bit-exact surfaces (documented, bounded):** HTML sanitizer (different engines),
JWT token string (opaque, self-issued), rate-limit timing (behavioral only). Everything
else is `≡`-gated.

---

## 5. Phase-by-phase

| Phase | Title | Gate |
|---|---|---|
| 0 | Scaffold + CI swap | — |
| 1 | DB schema & migrations | G2 |
| 2 | Models + encoder contract | — |
| 3 | Config + boot | — |
| 4 | Middleware | — |
| 5 | Realtime SSE hub | — |
| 6 | Auth vertical | G7a |
| 7 | Contract & codegen | G4 |
| 8 | Services | G6, G8 |
| 9 | Handlers (84 routes) | G1, G5 |
| 10 | Tests (68 → 76) | G3 |
| 11 | Standards + differential + cutover | G5 |

### Phase 0 — Scaffold + CI swap
`Cargo.toml` (axum 0.8.9, sqlx 0.9, tokio, serde, jsonwebtoken 10, jsonschema 0.46,
reqwest 0.13 + rustls-ring, ammonia, governor, rand 0.10), `rust-toolchain.toml` (1.95),
`rustfmt.toml`, `src/bin/api.rs` (config→pool→migrate→`/healthz`). CI: additive
`rust-backend` job; Makefile `rust-*` targets; PATH prepends `~/.cargo/bin`.
**Fixes:** rand 0.10 trait reorg (`RngExt`); reqwest feature names (`rustls-no-provider` +
ring install, no cmake/aws-lc); jsonwebtoken `rust_crypto` backend.

### Phase 1 — DB schema & migrations (G2)
`migrations/0001_initial_schema.sql` = 911-line schema **generated from `pg_dump
--schema-only`** of a Go-migrated reference DB (boilerplate stripped), iterated until
`pg_dump(Go) diff pg_dump(Rust)` was empty. `src/db.rs`: `connect` (10-attempt backoff,
pool 25/idle 5m/lifetime 30m), `migrate` (`sqlx::migrate!`), `refresh_leaderboard`
(CONCURRENTLY + fallback). **Later (Phase 10) addendum `0002_populate_leaderboard_mv.sql`** —
`pg_dump --schema-only` emits the matview `WITH NO DATA` (unpopulated → errors on SELECT);
Go created it populated. Invisible to G2 (both dumps show WITH NO DATA); caught by a test.

### Phase 2 — Models + encoder contract
29 model structs / 12 enums / 47 values, field order = Go. DB structs `rename_all="snake_case"`;
compiled-doc + export structs `camelCase`. The **§4 encoder contract** applied mechanically:
`go_time`/`go_time_opt`/`go_date` serializers, `skip_serializing_if` on omitempty, `float8`
casts, `RawJson = Json<Box<RawValue>>` for jsonb, `time_of_day::text`. 4 soft-delete tables
route reads through helpers appending `deleted_at IS NULL`.
**Fix:** GORM writes `''` (never NULL) for non-pointer strings + sets timestamps app-side —
Rust INSERTs must mirror (`''` + `now()`).

### Phase 3 — Config + boot
`config.rs` (dotenvy, 16 env vars, hard-fail on DATABASE_URL/JWT_SECRET,
`mission_version_body_limit()` = 256 MB). `bin/api.rs` boot order mirrors Go; graceful
shutdown on SIGINT/SIGTERM; `into_make_service_with_connect_info` (real client IP for
rate-limit). A `SKIP_MIGRATE` guard was added later for the G5 harness.

### Phase 4 — Middleware (7 concerns)
request-id · logging (tracing) · recovery (`CatchPanic`) · CORS (exact allow-list
reflection, never `*`) · rate-limit (per-IP 20/40 global + strict on `/api/v1/auth/` +
`/api/v1/ingest/`) · body-limit (1 MB global, mission-version POST overridden to 256 MB) ·
auth tier extractors. `role_rank` preserves the quirk **mission_maker(3) > leader(2)**.

### Phase 5 — Realtime SSE hub
`tokio::sync::broadcast` per topic. Two SSE endpoints via axum `Sse` (server-status = hub;
audit = DB-poll @ 2s), preserving `text/event-stream`, `X-Accel-Buffering: no`, `data: …\n\n`.

### Phase 6 — Auth vertical (G7a)
HS256 JWT, rotating single-use refresh tokens, Discord OAuth2, axum tier extractors.
**Refresh invariant (verbatim):** hash-lookup → reuse-of-revoked ⇒ revoke family + 401 →
expiry/ban → **atomic `UPDATE … WHERE id=$1 AND revoked_at IS NULL`**, `rows_affected != 1`
⇒ reuse ⇒ revoke family + 401 → mint new pair. **G7a** proven E2E via tower `oneshot`.

### Phase 7 — Contract & codegen (G4)
`codegen.mjs` gained a Rust target (quicktype serde + rustfmt → `src/contract/generated`).
2 runtime validators re-implemented with `jsonschema` (`ValidateMissionEditorPayload` → 400
+ details); schemas reached via `include_str!` from `packages/tbd-schema` directly.

### Phase 8 — Services (G6, G8)
`flatten_to_mod_document` (the crown jewel — compiled mod doc; `#[serde(rename_all="camelCase")]`,
orbat as sorted `BTreeMap`, coord map x→x/y→z/z→y/rotation→heading, `NoSlots`, string
schemaVersion) — **G6** vs the locked contract + schema. Plus ORBAT parse/derive, mortar,
webhook, discord OAuth+retry, role sync, audit, token purge, and the sanitizer (**G8**).

### Phase 9 — Handlers (84 routes, G1/G5)
Every route ported preserving envelopes (`{data,total,limit,offset}`, `{data,next_cursor}`),
error `{error[,details]}`, and every status code. By domain: auth/identity 11 · content 11 ·
reads 12 · missions 14 · events 15 · telemetry 2 · admin 7 · approvals 3 · field_tools 4 ·
cms 5. Heaviest: **events** (ORBAT materialize + the **slot-claim race G7b**: `FOR UPDATE`
+ conditional `UPDATE … WHERE assigned_to IS NULL` + `rows_affected==1` + waitlist promotion),
**missions** (413/409, per-route body cap, `/compiled` wiring G6 live), **telemetry**
(idempotent ingest + MV refresh + SSE publish — closes the live loop). **G1 PASS** (84=84).

### Phase 10 — Tests (68 → 76, G3)
All 68 Go tests represented (mapping in `t145_phase10_test_mapping.md`) + 8 Rust-only.
Highlights: mock-HTTP webhook/discord suites (spawned axum server), the concurrency race
(seeded second user), archive/soft-delete lifecycle, body-limit override, token purge.
**G3 PASS** — 76 tests, 0 failed.

### Phase 11 — Standards + differential + cutover (G5)
- **G5 harness** (`scripts/website/{differential.mjs,differential_seed.sql,run-differential.sh}`):
  boots Go + Rust on separate seeded DBs (fixed timestamps with varied fracs), replays a
  read+error corpus, compares canonical-`≡`. **18/18.**
- **`make rust-ci` green** (fmt + clippy + build + integration on a fresh dedicated DB).
- **Cutover:** deleted 85 `.go` files + `go.mod`/`go.sum`/`.golangci.yml` + Go-syntax verify
  scripts; rewired ci.yml/contracts.yml/codegen.mjs/Makefile to Rust. Preserved the
  language-agnostic seed/migration SQL.

---

## 6. Bugs the process caught (and fixed)

| Where | Bug | Caught by |
|---|---|---|
| leaderboard MV | matview `WITH NO DATA` → SELECT errors | dashboard test |
| event_missions / matches / fire_missions | nullable timestamp, no default (GORM set app-side) | events/telemetry tests |
| server status history | GORM-pluralized table `server_status_histories` | telemetry test |
| numeric writes | `f64→numeric` needs `::float8::numeric` cast | telemetry test |
| user directory | NULL `avatar_url` crashed `String` decode → `COALESCE` | events test |
| **inet ip** | `ip::text` kept `/32` netmask → `host(ip)` | **G5 differential** |
| **timezone** | Go emits local offset; force UTC | **G5 differential** |
| export tier | `/export` is mission_maker-tier (403 at extractor, not 404) | lifecycle test |
| body limit | over-cap body on a JSON route → 400 (only CreateVersion → 413) | lifecycle test |

## 7. Operational notes (also in memory)

1. **Reset the dev DB before `make api`.** `make api` is now `cargo run --bin api` and
   migrates on boot; the dev `tbd_reforger` DB is Go-migrated (no `_sqlx_migrations`) → sqlx
   re-runs `0001` → "type already exists" → boot fails. Drop+recreate it. `make rust-test-it`
   resets a dedicated `rust_it` DB for this reason.
2. **No `.sqlx` cache** — runtime queries, not `query!` macros. `SQLX_OFFLINE` / `cargo sqlx
   prepare` are moot; builds never need a live DB.
3. **Not ported:** `cmd/seed` + `cmd/import-registry-items` (Go tools). Seed roles/registry
   via `make seed` (psql on the preserved `internal/db/seeds/*.sql`).
4. **Auth impact:** in-flight 15-min access tokens won't validate cross-runtime (claim
   serialization differs) — at most one re-auth; refresh tokens (opaque hex, SHA-256-hashed) survive.

## 8. Artifacts & references

- `.ai/artifacts/t145_phase10_test_mapping.md` — the 68→76 test mapping table
- `scripts/website/differential.mjs` · `differential_seed.sql` · `run-differential.sh` — G5 harness
- `apps/website/migrations/000{1,2}_*.sql` — the frozen schema + MV populate
- Commits `9cc7a161` (port) · `c05ddbc9` (cutover)
- Approved plan: `~/.claude/plans/okay-so-here-s-the-mighty-babbage.md`
