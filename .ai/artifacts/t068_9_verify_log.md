# T-068.9 verify log — Registry worker + compat ingest (T-150 data)

**Date:** 2026-07-10 · **Executor:** Claude Code (Fable 5) · **Branch:** `main` @ repo root ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t068_9_registry_worker_ingest.md` ·
**Baseline:** T-150 @ `9107bf4e`

## Result

**PASS** — both T-150 envelopes ingest idempotently into Postgres, the compat graph is served
at `GET /api/v1/registry/compat`, and a Comlink registry worker answers
`canEquip` / `canAttach` / `hasEdge` / `itemsFor` / `hostsFor` off an IDB-cached adjacency
index. Every gate below is an exact predicate (set equality / oracle parity), not a spot check.

Commits: codegen `84f7fa23` · backend `798f8d4e` · gate repair `6bb7079e` · frontend
`36c47c63` · this log = the **T-068.9**-tagged commit.

**Note (spec drift):** the spec's Go/GORM/`cmd/import-registry-items` wording predates the
T-145 Rust cutover — implemented as sqlx migration `0003_registry_compat.sql`, model in
`src/models/registry.rs`, importer `src/services/registry_import.rs`, bin
`src/bin/import_registry.rs`, handler in `src/handlers/registry.rs`.

## Import CLI

```bash
# Committed T-150 envelopes into the dev DB (uses .env DATABASE_URL; migrates on boot):
make registry-import
# equivalent to:
cd apps/website && cargo run --bin import-registry -- \
  --items  ../../packages/tbd-schema/registry/registry-items.workbench.json \
  --compat ../../packages/tbd-schema/registry/registry-compat.workbench.json
# flags: --modpack <uuid> (override envelope modpackId) · --prune (delete modpack rows
#        absent from the envelope — full-scan-set semantics; default OFF)
```

First run against the seeded dev DB: `items total=1880 unique=1880 inserted=1859 updated=21`
(the 21 Phase-1 seed rows converged onto the envelope values — all 21 resource_names are in
the export) · `compat total=4012 unique=4012 inserted=4012 updated=0`. Second run:
**inserted=0 updated=0 pruned=0** on both. Dev DB counts:

```
registry_items = 1880 · registry_compat = 4012   (modpack 00000000-0000-4000-a000-000000000001)
edge histogram: attachment_on_weapon 241 · character_default_loadout 2746 ·
                mag_in_vehicle_weapon 118 · mag_in_weapon 545 · optic_on_weapon 362
```

Histograms are byte-identical to the T-150 verify log.

## Proof ledger

Normalization for set comparisons: `evidence` NULL ≡ `''` ≡ absent; items project to
`(resource_name, display_name, category, kind)`, edges to `(from_node, to_node, edge_type,
evidence)`.

| ID | Predicate | Result | Proven in |
|----|-----------|--------|-----------|
| **G1** Ingest bijection | DB row-set for the modpack set-equals the envelope, items **and** edges (both directions via size + membership) | **PASS** (1,880 / 4,012) | `tests/registry_compat.rs` |
| **G2** Idempotency | Re-run ⇒ inserted=updated=pruned=0 ∧ full row snapshot (ids, `created_at`, `updated_at`) identical ∧ API ETag byte-identical. Mechanism: `IS DISTINCT FROM` guards on both `DO UPDATE SET` clauses | **PASS** | IT + live dev-DB re-run |
| **G3** API fidelity | `GET /registry/compat` data (projected, normalized) set-equals the envelope; `GET /registry` serves all 1,880 items | **PASS** | IT |
| **G4** Filter correctness | `?edge_type=mag_in_weapon` set-equals the oracle filter (545) ∧ carries a distinct ETag (query discriminator) that does **not** satisfy the unfiltered resource | **PASS** | IT |
| **G5** Referential integrity | `count(edges with an endpoint ∉ registry_items) = 0` for the modpack, post-ingest SQL | **PASS** (0) | IT |
| **G6** Index losslessness | `edgesOf(buildIndex(S))` from the **byFrom** side = projected `S` = from the **byTo** side (round-trip bijection) | **PASS** (4,012) | `registryGraph.test.ts` |
| **G7** Query ≡ oracle | `hasEdge` true for **all 4,012** envelope edges (typed + untyped, exhaustive) ∧ false for **1,000** seeded-LCG complement tuples (oracle-filtered, deterministic seed `0x12345678`) ∧ `itemsFor`/`hostsFor` equal the oracle groupings for **every** distinct host and item ∧ `canAttach`/`canEquip` equal the oracle family unions for every edge pair | **PASS** | `registryGraph.test.ts` |
| **G8** Cache round-trip | IDB put→get returns a structurally identical graph (incl. absent evidence) ∧ etag + `'last'` bookkeeping exact ∧ two modpacks cache side-by-side without cross-talk ∧ miss → null | **PASS** | `registryCompatCache.test.ts` (fake-indexeddb) |
| **G9** Any-mod synthetic round-trip | Test-constructed second-modpack envelopes (all **16 kinds**, all **7 edge_types** incl. `ammo_in_mag` + `ammo_in_vehicle_weapon`, names with spaces/parens/apostrophe, one evidence-less edge) ingest via the envelope-modpackId path ⇒ G1/G3 hold; **vanilla modpack rows + ETag unchanged** (isolation); `prune` with a 2-edge subset ⇒ DB set-equals the subset (5 pruned); schema-invalid envelope rejected before SQL | **PASS** | IT |
| **G10** Histograms | Importer per-kind + per-edge_type histograms = envelope histograms = T-150 verify-log numbers | **PASS** | CLI output + IT |

Named sample edges (spec §Verify): STANAG M855 `{2EBF60EF24B108FC}…` → `Rifle_M16A2.et`
(`mag_in_weapon`, `canAttach` true) · `Box_762x51_M60_100rnd_4AP_1Tracer.et` →
`MG_M60_Mounted.et` (`mag_in_vehicle_weapon`; `itemsFor` lists all 6 boxes) · cross-well
negative: `Magazine_545x39_AK_30rnd_Base.et` → M16A2 **false** · `Helmet_PASGT_01_cover.et`
→ `Character_US_GL_Guard.et` (`canEquip` true; `canEquip(helmet, rifle)` false).

## Scales to any mod, any modset (zero code edits — T-150 invariant continued)

- **DB:** `kind` / `edge_type` are plain text (no Postgres enum/CHECK); nodes are arbitrary
  `resource_name` strings; everything keyed `(modpack_id, …)` — unlimited modpacks coexist
  (G9 proves two side-by-side with isolation).
- **Ingest:** single-statement UNNEST upserts in 10k-row chunks inside one transaction —
  envelope size unbounded; last-wins key dedupe guards arbitrary exports; schema validation
  (embedded draft-2020-12) is the only vocabulary gate.
- **API:** `?modpack=` selects any modpack; `?edge_type=` is a bind param (no enum match).
- **Worker:** index maps are keyed by plain strings — an unknown future edge family flows
  through `hasEdge`/`itemsFor`/`hostsFor` untouched (explicit vitest case `gear_in_slot`);
  IDB caches per modpack.
- **Extension recipe:** a new edge family or item kind = schema enum bump +
  `make schema-codegen` (+ widen the hand-written TS union, T-150 precedent). No importer,
  DDL, API, or worker-logic change.
- **Scale envelope (measured):** 308 B/edge, 261 B/item raw JSON → vanilla graph 1.2 MB;
  a 20× modset ≈ 80k edges ≈ 25 MB one-time GET (T-060 precedent: 142 MB bodies ship), then
  ETag/304 + IDB warm-start amortize to zero transfer; index build O(E) in the worker,
  queries O(1). No HTTP compression layer exists platform-wide (pre-existing; noted, not
  added — would interact with the SSE routes).

## Commands + outputs

```bash
make registry-import        # run 1: items 1859+21upd / compat 4012 · run 2: all zeros
make test-it                # 74 passed / 0 failed (registry_compat gates 1.2 s)
# fresh-DB migration gate (0003 applies; 30 base tables):
MIGRATE_TEST_DATABASE_URL=postgres://tbd:tbd@localhost:5434/migrate_it?sslmode=disable \
  cargo test --test db_migrate    # ok
cd apps/website/frontend && npm test        # 300 passed (41 files; +15 this slice)
npm run build                                # clean (1.40 s)
npm run lint                                 # 1 pre-existing error: router.tsx react-refresh
                                             #   (T-150 log: reproduced on clean HEAD; untouched)
cd packages/tbd-schema && npm run validate   # All contracts valid.
make verify-citations       # 26 @contract + 39 TS-6 exports resolve; GO-7 skipped (Go retired @ T-145)
make verify-coding-standards # 3 pre-existing SIZE-1 warnings, 0 violations; no-select-star clean
make schema-codegen && git diff --exit-code apps/website/src/contract/generated \
  apps/website/frontend/src/types/contract   # codegen-drift clean
make rust-ci                # fmt + clippy -D warnings + build + wasm + test-it: 256 passed / 0 failed
editorconfig-checker <ticket files>          # clean (repo-wide run: 4,910 pre-existing errors,
                                             #   byte-identical on clean HEAD — local artifacts)
```

## Gate repairs shipped (pre-existing rot, proven on clean HEAD via stash)

Both broke silently at the T-145 Go→Rust cutover and blocked this slice's verify:

- `verify-contract-citations.mjs` GO-7 crashed reading the deleted
  `internal/handlers/handlers.go` — now skips with a note when the Go tree is absent (axum
  wires routes through typed handler fns; a rename is a compile error, not doc rot).
- `verify-file-length.mjs` SIZE-3 flagged the generated, gitignored wasm-bindgen pkg
  (`src/wasm/pkg/…d.ts`, 1,206 lines) — generated output now excluded (SIZE gates target
  hand-written code).

## Known limitations (explicit, not silent)

- `ammo_in_mag` and `ammo_in_vehicle_weapon` remain **empty in the committed data** (T-150
  OPEN: vanilla links ammo via AmmoConfig `.conf`, no engine-readable prefab edge). The
  pipeline fully supports both families the moment an export ships them — G9 proves the
  end-to-end path with synthetic test fixtures (test-only; no edges invented in the data).
- `canEquip` semantics = "appears in that character's **default loadout**"
  (`character_default_loadout` is the only equip-family evidence T-150 exports); documented
  on `EQUIP_EDGE_TYPES`.
- Worker client ships **unwired** (nothing imports `registryCompatClient.ts` yet) — T-068.10
  Smart Forge is the first consumer, per spec §Out of scope. Precedent: T-145 F2 `yrsPersist`.
- Pre-existing and untouched: `router.tsx` react-refresh lint error, 3 SIZE-1 warnings,
  24-file prettier drift (none of this slice's files), repo-wide editorconfig noise from
  gitignored local artifacts — all reproduced byte-identically on clean HEAD.

## Ready for Cursor

- **T-068.10** Smart Forge UI can build on `initRegistryCompat` / `canAttach` / `itemsFor`
  (Forge dropdown feed is `itemsFor(host, 'mag_in_weapon' | …)`).
- **T-146** Asset Browser wiring unblocked (items ingest now serves the full 1,880 catalog).
- Doc-sync: CLAUDE §Status bullet, `t068_virtual_arsenal_program.md` hub row, DEV_RUNBOOK
  §Registry catalog (`make registry-import`), ticket registry `T-068.9 → shipped`.
