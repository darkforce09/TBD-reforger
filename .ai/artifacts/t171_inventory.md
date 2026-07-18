# T-171 — Class-R monorepo/website hygiene inventory

Recon base: `47ec9fb9` (tag T-169 at HEAD). Method: 3-agent full-tree sweep (API crate · SPA/fixtures/map-assets · root wiring/docs) + first-hand reads of every load-bearing config (root Cargo.toml/Makefile, ci.yml/contracts.yml/schema.yml, Trunk.toml, ignore/attr files, xtask/tbd-tools path constants, deploy scripts). All file:line cites verified at recon time. No deletes executed before this document existed (C1 gate).

Phase→commit map: C1 = this inventory · C2 = layout+renames (+`fixtures/api` move — see §5 note) · C3 = dead code/dual-SoT · C4 = fixture homes remainder + map-assets story · C5 = tooling polish + Cursor T-171.docs return list.

---

## 1. Layout debt

| Item | Today | End state (LOCKED) |
|---|---|---|
| API crate root | `apps/website/` (src, tests, migrations, internal/, Makefile, compose, .env*) | `apps/website/api/` |
| SPA | `apps/website-leptos/` | `apps/website/frontend/` |
| API pkg / lib | `reforger-backend` / `reforger_backend` | `website-api` / `website_api` |
| SPA pkg | `website-leptos` | `website-frontend` |
| App-level docs | `apps/website/{CLAUDE.md,README.md}` inside crate root | stay at `apps/website/` (app level); content = Cursor FULL-REWRITE rows |
| **api/ ownership (LOCKED)** | — | `docker-compose.yml`, `.env(.example)`, `rust-toolchain.toml`, `rustfmt.toml`, `migrations/`, `seeds/` (new), `tests/`, runtime `missions/` + `uploads/` all live under `apps/website/api/`; `make api` CWD = `api/` so CWD-relative serving (`uploads`, map-assets default) keeps one story |
| rust-toolchain.toml | `apps/website/` only (root has none) | moves with crate → `api/` (hoisting to root would change toolchain for frontend/crates — behavior preserved) |
| Workspace members | `apps/website`, `apps/website-leptos` | `apps/website/api`, `apps/website/frontend` |
| Untracked riders on move | `apps/website/.env`, `apps/website/missions/` (25 runtime JSON), ignored empty `internal/handlers/missions/`; `uploads/` not on disk | whole-dir `git mv` carries them; snapshot in scratchpad `untracked-manifest.txt` for abort recovery |

## 2. Dead code — SAFE / UNSAFE / ASK

| Row | Class | Evidence |
|---|---|---|
| `apps/website/internal/db/migrations/` (6 SQL: 00_extensions…05_registry_items) | **SAFE-delete (C3)** | Rust boot uses `sqlx::migrate!("./migrations")` (`src/db.rs:51`) against `apps/website/migrations/` (0001…0006). No Makefile/CI/script reads internal/db/migrations. Go binary deleted at T-145. |
| `apps/website/internal/db/seeds/{discord_roles,registry_dev}.sql` | **LIVE — relocate first (C3)** | Root `Makefile:24-26` `make seed` pipes both. Relocate → `apps/website/api/seeds/` BEFORE purging `internal/` (handoff DO-NOT). |
| `apps/website/internal/db/seeds/mock_data.sql` | **LIVE-ish — relocate (C3)** | Applier (`go run ./cmd/seed`) deleted at T-145; file still the only source of the four fixed-UUID mock missions; `docs/website/DEV_RUNBOOK.md:171-173` documents it (path fix → Cursor). Manual `psql <` remains the mechanism. |
| `apps/website/Makefile` (nested) | **SAFE-delete (C3)** | All recipes Go/Vite lies: `go run ./cmd/api` (:24), `cd frontend && npm run dev` (:27), `go test` (:30,:33), `go build` + `npm run build` (:36-37), `go mod tidy` (:40); its `seed` (:21) is a subset of root's. `ticket-*` targets use relpath broken from that dir. CLAUDE.md redirect already says "run from repo root". |
| `scripts/__pycache__/`, `scripts/lib/__pycache__/`, `scripts/mod/lib/__pycache__/` | **SAFE-rm (C3, untracked)** | T-162 eradication debris; `__pycache__/` gitignored; contradiction with `make verify-no-python` optics. |
| `apps/website/.gitignore` Go/React rows (`/bin/`, `/tmp/`, `*.test`, `*.out`, `frontend/node_modules/`, `frontend/dist/`, `frontend/.env.local`, `.stitch-backup-exports/`) | **SAFE-rewrite (C3)** | Go build outputs + deleted React app. NOTE: `frontend/*` rows would half-apply to the NEW Leptos frontend after nesting — must be rewritten, with `frontend/.gitignore` (`/dist`) staying authoritative for the SPA. |
| Root `Makefile` `.PHONY` entry `ci-local-backend` | **SAFE-drop (C3)** | Declared (:9) with no recipe anywhere. |
| Root `Makefile` ci-local comment "run `make db-up` + `nvm use` first" (:221) | **SAFE-fix (C3)** | `nvm` = Node-era lie (T-165 eradicated Node from gates). |
| `tools/tbd-tools/src/map/carto.rs:651-665` React-wasm skip block (`apps/website/frontend/src/wasm/pkg/map_engine_wasm_bg.wasm`) | **SAFE-delete (C5)** | Path never exists post-React-deletion; after the move it would point INSIDE the Leptos tree at a file that never exists there — dead either way. |
| `tools/tbd-tools/src/inject.rs` driver-parity test (`t159_gates/driver/` @ :126) | **SAFE-delete (C5)** | `driver/` deleted on disk at T-165.6 (JS inlined as consts); the test is a no-op skip today. |
| `xtask/src/schema_gates.rs:1010-1011` Gate-8 entries `apps/website/frontend/docs/{INDEX.md,pages/mission-editor.md}` | **SAFE-drop (C5)** | Old-React doc paths; never existed post-deletion; Leptos ships no `docs/` (verify-doc-layout forbids md under `apps/**/docs`) — entries are dead and would be misleading once `apps/website/frontend` exists again. |
| `.editorconfig-checker.json` excludes `node_modules`, `public/map-assets`, `stitch-exports/`, `package-lock\.json` | **SAFE-prune (C3, after checker-green proof)** | Node/React-era paths absent from tree. |
| `scripts/mod/manual-test.sh:55-58` `go test ./internal/handlers/` block | **SAFE-strip (C3)** | Go tests deleted at T-145; breaks louder once `internal/` gone. |
| `docker-compose.staging.yml` referenced by `scripts/mod/deploy-staging.sh:175` | **ASK — pre-existing rot, no stub invented** | File exists nowhere in repo (`find` = only `apps/website/docker-compose.yml`). Deploy script's own comment says its verify curls are BLOCKED on removed REST spike routes. T-171 fixes the path strings (`apps/website` → `apps/website/api`) but does NOT author a staging compose — that is new deploy infra, not hygiene. **Operator decision needed on whether/when to build it.** |
| CI branch-protection required-check names (`rust-backend`, `website-leptos`) | **ASK/ship-note — never blocks tag** | Job ids rename to `website-api`/`website-frontend` (spec Phase 4). If GitHub branch protection pins the old names, operator must update settings; read-only `gh api` attempted at C5, outcome reported either way. |

## 3. Dual sources of truth

| Duplication | Resolution |
|---|---|
| `internal/db/migrations` (Go-era) vs `migrations/` (sqlx, live) | Delete Go-era set (C3); `migrations/` is sole SoT (embedded via `migrate!`). |
| `internal/db/seeds` vs root `make seed` recipe | Seeds → `api/seeds/`; root Makefile recipe = sole applier; nested Makefile seed deleted with the Makefile. |
| Nested `apps/website/Makefile` vs root `Makefile` | Nested deleted (C3); root is sole task entrypoint (CLAUDE.md redirect already points there). |
| `context_handoff.md` twins: `docs/platform/context_handoff.md` (8-line redirect stub) → `docs/website/platform/context_handoff.md` (156-line canonical; L15-16 still claim React/Go) | Cursor row: refresh canonical claims; keep or retire stub (Cursor's call — both listed). |
| ROADMAP claims vs reality: `docs/website/backend/ROADMAP.md` (:3 "Go API", :16 `make web`, :39 internal/db, :53 golangci, :107 GORM), `docs/website/frontend/ROADMAP.md` (:7,:30,:112 old paths), MC `ROADMAP.md` (React paths, Deck) | Cursor rows (§7). |
| `apps/website/README.md` (:9-10 `make api` "Go API :8080" / `make web` "Vite :5173"; :30-31 "Go (Gin+GORM)" / "React 19+Vite") vs root CLAUDE.md reality | Cursor FULL-REWRITE row. |
| gate oracle "react dist" default (`gate.rs:29` `apps/website/frontend/dist`) vs frozen oracle actually consumed from `oracle-freeze/` | Freeze mode retired with hard error (C4); `v-suite verify` reads `gold_dir()` only. |

## 4. Rename blast radius

**`reforger-backend` → `website-api`** (cheap — measured):
- External `reforger_backend::` importers: **ZERO** (no workspace crate depends on the API lib).
- In-crate: 57 refs / 20 files (2 bins + integration tests) — mechanical sed in C2.
- Config: `apps/website/Cargo.toml:2` (+ `[lib] name`), `Cargo.lock` (2 stanzas regen, offline), root `CLAUDE.md:23` (Cursor), `docs/platform/t171_monorepo_hygiene_program.md:30` (Cursor's own spec, allows it).
- CI job id/name `rust-backend` → `website-api` (ci.yml:26-27) + `working-directory` (:46) + rust-cache `workspaces:` (:55 — NOTE pre-existing misconfig: points at `apps/website` which has no Cargo.lock since the T-145 workspace fold; fix → `.`).

**`website-leptos` → `website-frontend` + path `apps/website-leptos` → `apps/website/frontend`:**
- Root `Cargo.toml:11`; `apps/website-leptos/Cargo.toml:5`; `Cargo.lock` stanza.
- Root `Makefile:37,40,231-234` (`-p website-leptos` ×3 + two `cd`).
- `ci.yml:95-96,110,112,114,116` (job id/name, `-p` ×3, working-directory) + comments :7,:91.
- tbd-tools: `bin/gate.rs:31`, `smokes.rs:22` (dist defaults), `sroutes.rs:3,17` (router.rs path).
- `apps/website/.env.example:17` (`# SPA_DIST_DIR=apps/website-leptos/dist` → `../frontend/dist`, T-170 consumes this string).
- Frontend `README.md:21,34,38` self-paths (mechanical — Claude), prose (Cursor).
- Docs/registry hits (69 in docs/ + registry.json:5318 + generated TICKET_REGISTRY.md:132) → Cursor list §7 (generated files via `./scripts/ticket sync` after registry edit).
- Trunk wasm artifact basename changes (`website-leptos*_bg.wasm` → `website-frontend*`): zero by-name consumers (`index.html` uses `data-trunk rel="rust"` auto-injection).

**Relative-path breakage classes on the api/ move (+1 `../`):** `src/contract/validate.rs:18-26` (5× include_str), `src/contract/generated/loadout.rs:141,143` (hand-maintained include_str), `tests/factions.rs:82` + `tests/registry_compat.rs:30-35` (CARGO_MANIFEST_DIR joins), `src/app.rs:292` map-assets CWD default, `apps/website/Cargo.toml` map-engine-core path dep, root `Makefile:29-31` registry-import `../../packages` (CWD = $(WEB)). Crate-relative survivors (no edit): `sqlx::migrate!("./migrations")`, `app.rs:287` uploads ServeDir, rustfmt/rust-toolchain discovery.

**Frontend move breakage:** `frontend/Cargo.toml` crate paths `../../crates/*` → `../../../crates/*`; `src/dto.rs:648-653` fixture include_str (superseded by §5 move → crate-relative `../tests/fixtures/api/`, move-proof).

**`apps/website/frontend` collision (the big hazard):** the path is TODAY'S dead-React meaning in `gate.rs:29` (oracle default), `vsuite.rs:490` label, `carto.rs:652`, `schema_gates.rs:993,1010-1011`, `.coding-standards-allowlist.yaml:7,12,16`, `.cursor/rules/{application-code-forbidden.mdc:3,20, cursor-agent-workflow.mdc:92-94, tbd-platform.mdc:12}`, ~344 doc hits. After C2 the SAME string means the live Leptos SPA. Every code/tooling site is retargeted or retired in C2/C4/C5; rules/docs sites are Cursor rows flagged "collision, not just rot".

## 5. Golden/fixture homes → ONE convention

**Census:**
| Home | Contents | Consumers |
|---|---|---|
| `.ai/artifacts/t159_gates/fixtures/api/` | 21 R-api response goldens + `_index.tsv` (byte-pinned; `GET__registry.json` r_api byte-exact) | `apps/website-leptos/src/dto.rs:649,652` (include_str), `tools/tbd-tools/{vsuite.rs:91, smokes.rs:339,383,410}` |
| `.ai/artifacts/t159_gates/manifests/` | 5 frozen React S-gate CSVs (routes.csv oracle) | `tools/tbd-tools/sroutes.rs:18` |
| `.ai/artifacts/t159_gates/v/oracle-freeze/` | ~40 frozen React DOM/pixel oracles — **NON-REGENERABLE** (React app deleted) | `tools/tbd-tools/vsuite.rs:87` |
| `packages/tbd-schema/golden/` + `golden-missions/` | schema/map-object/mission contract goldens | map-engine-core `world/{chunk,regions}.rs`, tbd-tools `density.rs:88` (CARGO_MANIFEST_DIR — verified fine), xtask `golden_gate.rs` + `schema_gates.rs` |
| `crates/map-engine-core/tests/fixtures/` | `deckgl_ortho_goldens.json` (1 file) | `tests/deckgl_ortho_parity.rs:47` include_str |
| `scripts/mod/fixtures/` | 5 enfusion-mcp JSONL | enfusion-mcp (the sanctioned Node floor; not in spec's consolidation enumeration — untouched, recorded here) |

**PINNED CONVENTION (the "one convention"):**
1. **Fixtures live crate-local** in `tests/fixtures/` beside their primary consumer.
2. **Cross-crate contract data** lives in `packages/tbd-schema` (it IS the contract package).
3. **`.ai/artifacts/` is pipeline OUTPUT only — never a load-bearing input** (no code may `include_str!`/read fixtures from it).

**Moves under the convention:** `fixtures/api/` → `apps/website/frontend/tests/fixtures/api/` (primary consumer = the SPA's R-api golden tests; tbd-tools joins updated). Executed **in C2**, not C4, so `dto.rs` gets a single edit to a crate-relative move-proof path — decision recorded here before execution. `manifests/` + `v/oracle-freeze/` → `tools/tbd-tools/fixtures/t159/{manifests,oracle-freeze}/` (sole consumer = gate harness; byte-preserving `git mv`; frozen internals — including the stale `"frozenFrom": "react dist"` string inside the frozen `manifest.json` — are artifact bytes and stay untouched). `t159_gates/` remainder reconciles to output-only (logs, .gitignore shrink). `tbd-schema/golden*` + `map-engine-core/tests/fixtures` already comply. Guard: `.editorconfig-checker.json` excludes for both new fixture homes land in the SAME commit as each move (byte-pinned goldens must never be whitespace-"fixed").

## 6. map-assets consumption story

**Corpus:** `packages/map-assets/` — everon 1.3 GB on disk, of which **tracked-in-LFS = exactly 2 objects**: `everon/dem/everon-dem-16bit.png` (72 MB) + `everon/satellite/everon-sat.tbd-sat` (153 MB). `**/staging/` + `**/tiles/` gitignored (785 MB + 246 MB local-only, rebuildable via `make map-*`). `.gitattributes` LFS patterns = `packages/map-assets/**/*.{png,r16,tbd-sat}` only — unaffected by any T-171 move.

**Who needs what:**
| Consumer | Needs | Mechanism |
|---|---|---|
| CI `map-engine` job | DEM only | `git lfs pull --include packages/map-assets/everon/dem/everon-dem-16bit.png` (ci.yml:83); map-engine-core tests + `dem/peaks.rs:375` read it |
| CI other jobs | none | no LFS pull (sat 153 MB deliberately never dragged) |
| Local dev editor | DEM + sat | backend serves `/map-assets` via `ServeDir` (app.rs:292-297; `MAP_ASSETS_DIR` env, default `../../../packages/map-assets` from api/ CWD post-move) ← Trunk `[[proxy]]` `/map-assets` → :8080 ← SPA same-origin `fetch("/map-assets/...")` |
| Gate harness | dist + optional map-assets | `gate serve --map-assets` (serve.rs passthrough); smokes join `repo_root()/packages/map-assets` |
| Clone without LFS objects | degraded | manifest.json + JSON/chunks are plain git → editor boots; DEM/sat requests 404/pointer-file → satellite/hillshade absent. **Convenience targets shipped in C4: `make lfs-dem` (72 MB, enough for map-engine tests + hillshade) and `make lfs-sat` (153 MB full satellite).** |
| Prod | same ServeDir chain | `MAP_ASSETS_DIR` explicit in env (T-170 operator lane) |

Prose home for this story in `docs/**` = Cursor row (§7); the mechanics (targets + defaults + this matrix) are Claude-shipped in T-171.

## 7. Doc / ADR / rule rot (→ Cursor T-171.docs; counts at recon)

Pattern hit-counts across `docs/`: old-React `apps/website/frontend/src` **204** (`apps/website/frontend` any-form **344**) · `npm run` **227** + `npm ci` **23** · Go word-boundary **319** · `React` **144** · yjs/Y.Doc **117** · `Vite` **81** · `website-leptos` **69** · deck.gl **64** · `y-indexeddb` **18** · `ticket/T-0` branch lore **12** · `internal/db` **10** · `:5173` **10**. Generated `docs/TICKET_*.md` carry AUTO-GENERATED banners — fix via registry edits + `./scripts/ticket sync`, never by hand. **No `*adr*` file exists**; de-facto ADRs live in `docs/specs/Mission_Creator_Architecture/engineering_plan.md` (ADR-1 Deck.gl · ADR-2 Vite+React 19 · ADR-3 Yjs — all three superseded by T-151 wgpu / T-159 Leptos / T-145 yrs). Full per-file rows with line numbers = C5 return list (root CLAUDE.md broken `docs/backend/architecture.md` link @ L5 included).

## 8. Ticket hygiene

Registry: 146 tickets — 82 shipped · 28 idea · 17 queued · 16 deferred · 3 ready. `T-171` row: queued, executor claude-code, branch main, spec path valid. `./scripts/ticket check` = registry structure + spec-file existence (no website-path assumptions → survives the moves; verified in xtask/src/check.rs). Rot: `.ai/tickets/README.md` (:4,:16,:27,:59) + `scripts/ticket` usage text (:33-36) still narrate the superseded `ticket/T-0xx` branch/worktree model (root CLAUDE.md declares main-only). Script usage text = code (Claude, C5); README = Cursor row. Old registry summaries embed pre-move paths (e.g. :5318) — Cursor may batch-fix wording or leave as history; TICKET_*.md regenerate via sync either way.

## 9. Conventions gaps — "Where does X go?" (pin content for Cursor to land in docs/platform/)

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

---

## Cursor T-171.docs return list (COMPLETE — apply then flip T-171 → shipped + `./scripts/ticket sync`)

Row format: file → wrong claim → correct claim. "PATH" = mechanical swap set: `apps/website-leptos` → `apps/website/frontend` · `apps/website/{src,migrations,tests,internal,.env,docker-compose.yml,Makefile}` → `apps/website/api/...` (internal/ is DELETED — seeds now `apps/website/api/seeds/`) · pkg `website-leptos` → `website-frontend` · crate `reforger-backend` → `website-api` (lib `website_api`) · fixture home `.ai/artifacts/t159_gates/fixtures/api` → `apps/website/frontend/tests/fixtures/api` · oracle/manifests → `tools/tbd-tools/fixtures/t159/{oracle-freeze,manifests}`.

### A. Root CLAUDE.md (leave `<!-- ticket-sync -->` markers to `./scripts/ticket sync`)
- :5 → links `docs/backend/architecture.md` (nonexistent dir) → `docs/website/backend/architecture.md`.
- :23 → "crate `reforger-backend` in `apps/website/`" → "crate `website-api` in `apps/website/api/`".
- :24, :29, :40, :48, :73, :95-97, :103, :756 → PATH (`apps/website-leptos/*`; `cargo test -p website-leptos` → `-p website-frontend`).
- :35-39 → `apps/website/src/...` rows (entrypoint/handlers/models/migrations/services) → `apps/website/api/src/...`; migrations row → `apps/website/api/migrations/`.
- :62 → registry-import bullet path; :66-67 → `apps/website/src/contract/generated` → `apps/website/api/src/contract/generated`.
- §Run it locally → "configured in `apps/website/.env`" → `apps/website/api/.env`.
- §Monorepo layout → new nest line: `apps/website/` = `api/` (Axum) + `frontend/` (Leptos, Trunk); seeds `api/seeds/`; fixture convention line (crate-local `tests/fixtures/`; `.ai/artifacts` output-only).
- §Status → T-171 shipped bullet (Cursor authors); note CI job renames `rust-backend`→`website-api`, `website-leptos`→`website-frontend`.
- Any `internal/db/seeds` mention (mock-data para near the end) → `apps/website/api/seeds/`.

### B. apps/website/CLAUDE.md — FULL REWRITE
- Redirect stub cites `make web` (target gone). → redirect to root CLAUDE.md; run from repo root: `make api` / `make leptos`; one line on the `api/` + `frontend/` nest.

### C. apps/website/README.md — FULL REWRITE
- :9 "make api — Go API on :8080" → Rust Axum API (`api/`); :10 "make web — Vite dev server on :5173" → `make leptos` — Trunk dev on :3000; :30 "Go (Gin + GORM) … `internal/`, `cmd/api/`" → Rust (Axum + sqlx), `api/src/`, migrations embedded; :31 "React 19 + TypeScript + Vite — `frontend/`" → Leptos 0.8 CSR (Rust→wasm, Trunk) — `frontend/`.

### D. .cursor/rules/
- `application-code-forbidden.mdc` :3 globs `apps/website/**/*.go, apps/website/frontend/src/**/*.{ts,tsx}` → dead languages; retarget guard to `apps/website/api/src/**/*.rs`, `apps/website/frontend/src/**/*.rs` (+ keep mod globs); :20 `apps/website/internal/` (deleted) + old React-path wording → same retarget. **Collision note:** `apps/website/frontend/src` now = the LIVE Leptos SPA, no longer a deleted-React ghost.
- `cursor-agent-workflow.mdc` :16 path list; :61 "Do not patch `.go`/`.ts`/`.c`" → `.rs`/`.c` reality; :92-94 STOP list `apps/website/internal/**` (deleted) + `apps/website/frontend/src/**` (reword: live Rust app code, Claude-owned).
- `tbd-platform.mdc` :11 repo root `/home/Samuel/Projects/TBD-Reforger/` (checkout also mounts at `/var/home/...` — state $HOME-relative); :12 "never `apps/website/frontend/docs/`" → keep intent, reword "specs live in `docs/website/frontend/` — never under `apps/**/docs` (verify-doc-layout enforces)"; :18 `T-0xx` branch lore → main-only.
- `claude-prompt-delivery.mdc` :42, :50 example headers using `worktree`/`ticket/T-092` branch flow → main-only examples.
- `class-r-plans.mdc` :62/:76/:126-133 React-porting framing → mark historical (rule mechanics stay).

### E. docs/website/
- `AGENT_COMMIT_CHECKLIST.md` :33 `npm run format:check` → `cargo fmt --check` (or `make ci-local-leptos`); :46 `frontend/src/router.tsx` → `apps/website/frontend/src/router.rs`; :48 `frontend/src/config/navigation.ts` → the Leptos nav module (`apps/website/frontend/src/nav.rs`); :76/:97 `cd frontend && npm run build && npm run lint` → `make ci-local-leptos`; :84/:90 `apps/website/frontend/src/*.tsx` → `.rs` PATH; :108 `ticket/T-0xx` branch flow → main-only.
- `DEV_RUNBOOK.md` :3 "DB + API + Vite" → Trunk; :41 `http://localhost:5173` → `http://127.0.0.1:3000`; :50 `cd packages/tbd-schema && npm run validate` → `make schema-validate`; :72 "API + Vite: kill" → API + trunk; :171-173 `apps/website/internal/db/seeds/mock_data.sql` → `apps/website/api/seeds/mock_data.sql` (mechanism = manual psql; `go run ./cmd/seed` gone); §Registry catalog paths → api/. ADD: map-assets pull story (see K2).
- `backend/architecture.md` — wholesale Go-era (:5 "Go REST API", :23 "Go (chi or gin router), pgx + sqlc, golang-migrate", :500 "## 2. Go REST API", :9/:511 `apps/website/internal/*`) → rewrite to Axum/sqlx reality OR stamp ARCHIVE banner pointing at live code + root CLAUDE.md (Cursor's call; banner minimum).
- `backend/ROADMAP.md` :3 "Go API" → Rust; :16 `make web` → `make leptos`; :39 `internal/db/{migrations,seeds}` → `api/migrations` + `api/seeds`; :53 `internal/contract/` + golangci → `api/src/contract/` + clippy; :107 GORM tags → sqlx/serde.
- `frontend/README.md` :14 · `frontend/ROADMAP.md` :7,:30,:112 · `frontend/THEME.md` :5 · `frontend/_template.md` :12 → PATH.
- `frontend/pages/*.md` "Live source:" lines (≥10 files incl. audit-logs:12, vehicle-database:12, auth/auth-callback:12, event-hub:43, mission-editor:12) → `apps/website/frontend/src/<page>.rs`.
- `frontend/pages/mission-editor.md` :9/:21 Deck.gl → wgpu `map-engine-render`; :34 y-indexeddb/Vite → yrs-IDB/Trunk; :86 React.memo → Leptos reality.
- `platform/context_handoff.md` (canonical) :15 "Frontend: React" / :16 "Backend: Go" → Leptos / Rust (Axum + sqlx); resolve twin: `docs/platform/context_handoff.md` stub — keep as redirect or delete (dual-SoT row).
- `README.md` + `frontend/INDEX.md` — verified CLEAN at recon; only touch if pages rows change.
- `CURSOR_SETUP.md` :5173 + branch-lore hits (1 each) → current reality.

### F. docs/platform/
- `DOCUMENTATION_STANDARDS.md` :145 `gorm:` example, :157 `*gin.Context`, :160/:220-221 `apps/website/frontend/src/types/...`, :329 "`apps/website/frontend/docs/`" (reword per D), :334 `router.tsx`, :395 `apps/website/frontend/src/types/contract/` → generated home is `apps/website/api/src/contract/generated` (Rust-only), :419-420 Go/TS lint rows → clippy analogs. ADD (K4): fixture-home convention section.
- `CODING_STANDARDS.md` — RETIRED banners exist (:7-11, :154-157) but live-looking rows remain: :38,:82,:85,:132,:139,:164-178,:278,:360-374 golangci/`npm run`/tsc → mark historical or swap cargo/xtask equivalents; :157 `apps/website-leptos/src/dto.rs` → PATH; :443-456 website-leptos CI notes → website-frontend job names.
- `t171_monorepo_hygiene_program.md` :30-31 name table "(was …)" → post-ship truth (shipped as `website-api`/`website-frontend`, no exception needed).
- `HOME_SERVER.md` npm/:5173 rows → current stack.
- Historical `t159_*.md` slice specs (t159_16:41,:67 · t159_23:52-55 · t159_15_2:46 · t159_leptos_ui_program:23 · macos_ux_architecture:5173 · t125_coding_standards_enforcement npm/golangci rows) → RECOMMEND one-line "paths pre-date the T-171 `apps/website/{api,frontend}` nest" banner on `t159_leptos_ui_program.md` + leave bodies as history (do NOT mass-rewrite verify logs/specs).
- `t144_arma3_map_architecture_report.md` / other .ai-artifact-linked docs — no action (artifacts are history).

### G. docs/specs/Mission_Creator_Architecture/
- `engineering_plan.md` :22-24 ADR-1 "Renderer = Deck.gl" / ADR-2 "Vite + React 19 SPA" / ADR-3 "Yjs" (+ :28-29 dep list, :61-166 React module tree) → SUPERSEDED banner: ADR-1 → T-151 wgpu (`map-engine-render`); ADR-2 → T-159 Leptos CSR + Trunk (`apps/website/frontend`); ADR-3 → T-145 yrs doc core. Body stays as history.
- `ROADMAP.md` :5 `frontend/src/features/mission-creator/` → `apps/website/frontend/src/mission_editor.rs` lane; :60/:192/:238/:333 Deck.gl/React.memo, :205 y-indexeddb, :84/:512 Vite → superseded-tech notes.
- `agent_execution.md` :163/:187/:225 Deck.gl; :333 React.memo/useMapStore; :335/:346/:348 `internal/middleware/bodylimit.go` (Go, deleted)/Vite; :373/:511+ `npm run build && npm run lint` → `make ci-local-leptos`.
- `feature_inventory.md` (35 yjs / 6 y-indexeddb / 3 deck hits) → superseded-tech sweep or history banner.
- t068_*/t09x_* specs citing `apps/website/frontend/src/*.tsx` (the 204-hit set; top: t048:16, t091_2:11, t049:11, t067:8, t090_1_2_6:7, t062_2:7) → batch: one banner per actively-cited spec ("React-era paths; live surface = `apps/website/frontend/src/*.rs`"); full file list = `grep -rl 'apps/website/frontend/src' docs/specs`.

### H. .ai/tickets/
- `README.md` :4 branch-merge lore; :16 `ticket done` wording; :27 parallel-run branch wording; :59 `artifacts/ticket-pipeline/...` → `.ai/artifacts/ticket-pipeline/...` + main-only pipeline description (match `scripts/ticket` usage text updated in C5).
- `registry.json` — historical summaries embed pre-move paths (e.g. T-167 :5318 `apps/website-leptos/src/arsenal.rs`); optional batch wording fix; ANY registry edit → `./scripts/ticket sync` regenerates `docs/TICKET_*.md` (never hand-edit those; current TICKET_REGISTRY.md:132 hit clears with sync).
- T-171 row: status → `shipped` ONLY after this docs pass lands; then sync.

### I. .ai/tickets/CLAUDE_CODE_PROMPT.md + AI_PLAYBOOK.md
- Sweep for `ticket/T-0xx` branch lore + old paths (registry-adjacent prose Cursor owns).

### J. Root README.md (Claude already fixed the two hard lies; Cursor prose pass)
- Layout row + quick start updated in C5 — review wording; add `apps/website/{api,frontend}` nest mention in §Layout if richer phrasing wanted.

### K. NEW content Cursor lands (not fixes)
1. **"Where does X go?" conventions pin** — new doc under `docs/platform/` (content ready: §9 above, lift verbatim; link from CLAUDE.md §Conventions + AGENT_COMMIT_CHECKLIST).
2. **map-assets consumption story prose** — §6 matrix verbatim: 2 LFS objects (DEM 72 MB / sat 153 MB), CI pulls DEM only (map-engine job), `make lfs-dem` / `make lfs-sat`, serving chain Axum ServeDir (`MAP_ASSETS_DIR`, default `../../../packages/map-assets` from api/) ← Trunk proxy ← same-origin fetch, clone-without-LFS degradation. Home: DEV_RUNBOOK §Map assets (+ pointer from packages/map-assets README if desired — packages README = code-adjacent, Cursor may edit).
3. **CI required-check rename note** — jobs `rust-backend`→`website-api`, `website-leptos`→`website-frontend`: update any doc citing check names; **operator**: update branch-protection pinned checks if configured.
4. **Fixture-home convention** section in DOCUMENTATION_STANDARDS (crate-local `tests/fixtures/`; contract data in `packages/tbd-schema`; `.ai/artifacts` output-only; byte-pinned fixtures excluded from editorconfig-checker).
5. **v-suite freeze retirement** note wherever the V-suite is documented (gate docs cite `freeze|verify|accept` → now `verify|accept`; oracle home `tools/tbd-tools/fixtures/t159/oracle-freeze`).

### ASK rows (operator)
1. `docker-compose.staging.yml` — referenced by `scripts/mod/deploy-staging.sh:175`, never existed in-repo. Path strings fixed to `apps/website/api/`; authoring a staging compose = new deploy infra, not T-171 hygiene. Decide owner/ticket.
2. Branch-protection pinned check names — **RESOLVED at ship**: `gh api repos/.../branches/main/protection` → 404 "Branch not protected" (checked pre-push). The CI job renames (`rust-backend`→`website-api`, `website-leptos`→`website-frontend`) therefore orphan no required checks; if protection is enabled later, pin the new names.
