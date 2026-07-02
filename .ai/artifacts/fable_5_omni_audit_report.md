# Fable 5 Master Omni-Audit [2026-07-01]

**Living tracker** — update this file when a remediation ticket ships (do not treat as a frozen snapshot).  
**Program hub:** [`docs/platform/FABLE_5_AUDIT_PROGRAM.md`](../../docs/platform/FABLE_5_AUDIT_PROGRAM.md) · **T-122 audit log:** [`docs/platform/CODEBASE_AUDIT_2026.md`](../../docs/platform/CODEBASE_AUDIT_2026.md)

**Original audit baseline:** read-only pass @ `main` `a3efdf68` (2026-07-01). Severity: **CRITICAL** / **HIGH** / **MED** / **LOW**. EnforceScript (`.c`) plugins audited as Enfusion script, not C++.

**Last tracker update:** 2026-07-02 — **T-127** shipped @ `0515aabb` (tag **T-127**) · **T-128** shipped (tag **T-128**): doc link repair + staging honesty; log [`t128_doc_link_repair_log.md`](t128_doc_link_repair_log.md). **Fable program complete.**

---

## Remediation tracker (summary)

| Status | Count | Meaning |
|--------|------:|---------|
| **RESOLVED** | 24 | **T-126** S1–S6 @ `4a47688e` · **T-127** U1–U5 @ `0515aabb` · **T-128** (F1-05, F2B-10, F2C-02 docs-half, F2C-05, F5-01…F5-07) |
| **ACTIVE** | 0 | — (Fable program complete) |
| **QUEUED** | 0 | — |
| **PARTIAL** | 1 | **F4-03** — same-tab conflict loop fixed; new-tab cold boot still prompts (divergence tracking deferred) |
| **DEFERRED** | ~15 | Out of Fable program (**T-092**, **T-090.x**, future, T-122 carry-over) |
| **OPEN** | ~20 | Unassigned (Discord 429, archive/delete, CI scope, misc LOW, F2C-04 note) |
| **OK** | 12 | Verified clean at audit time — no action |

**Fable program order:** T-126 ✓ → T-127 ✓ → T-128 ✓ → resume T-090.1.2.8 / T-068.

### By ticket

| Ticket | Status | Scope |
|--------|--------|-------|
| **T-126** | **shipped** @ `4a47688e` | S1–S6 security + auth (verify [t126_verify_log.md](t126_verify_log.md)) |
| **T-127** | **shipped** @ `0515aabb` | U1–U5 MC UX (verify [t127_verify_log.md](t127_verify_log.md)) |
| **T-128** | **shipped** (tag **T-128**) | §5 doc rot, staging honesty, handoff link depths, orphans — log [`t128_doc_link_repair_log.md`](t128_doc_link_repair_log.md) |
| **T-092** | deferred | Mod REST `/compiled`, roster, mission canonical envelope, spawn/yaw |
| **T-090.1.1** | deferred | Map tile pyramid (pairs with U3 basemap coerce) |
| **T-090.3** | deferred | Building export `headingDeg` field names |
| **T-122** | partial | T15 addon GUID placeholder — deferred @ `f131770` |
| — | open | Discord 429, mission archive/delete, CI scope expansion, misc LOW |

### Full finding index

Status legend: **RESOLVED** · **ACTIVE** · **QUEUED** · **DEFERRED** · **OPEN** · **OK**

| ID | § | Sev | Finding (short) | Ticket | Status |
|----|---|-----|-----------------|--------|--------|
| F1-01 | 1 | HIGH | Mod `GET /api/missions/{id}/compiled` — no backend route | T-092 | DEFERRED |
| F1-02 | 1 | HIGH | Mod `Authorization: Bearer` vs backend `X-Service-Token` | T-092 | DEFERRED |
| F1-03 | 1 | HIGH | Mission list envelope/casing mismatch | T-092 | DEFERRED |
| F1-04 | 1 | HIGH | `GET /api/game/events/{id}/roster` — no route | T-092 | DEFERRED |
| F1-05 | 1 | MED | `rest-spike-0.1.md` links deleted handlers | T-128 | **RESOLVED** |
| F1-06 | 1 | MED | Spawn heading matrix sign vs compass semantics | T-092 | DEFERRED |
| F1-07 | 1 | MED | TerrainWorld export swapped yaw/pitch field names | T-090.3 | DEFERRED |
| F1-08 | 1 | HIGH | Everon manifest advertises `map` tiles; disk has satellite only | T-090.1.1 · U3 | DEFERRED · **RESOLVED** (coerce) |
| F1-09 | 1 | MED | Everon `metersPerPixel: 1` vs schema/Biki 2 m | — | OPEN |
| F1-10 | 1 | MED | Arland manifest tiles block; no files on disk | T-090 | DEFERRED |
| F1-11 | 1 | LOW | `terrainId` enum mismatch registry vs manifest schema | — | OPEN |
| F1-12 | 1 | MED | SpawnManager no disconnect cleanup | T-092 | DEFERRED |
| F1-13 | 1 | MED | Round-robin ignores roster-assigned slots | T-092 | DEFERRED |
| F1-14 | 1 | MED | ScenarioRouter placeholder addon GUID | T-122 T15 | DEFERRED |
| F1-15 | 1 | LOW | SpawnManager hardcoded faction presets | T-092 | DEFERRED |
| F1-16 | 1 | LOW | Profile mission read 8 MB cap silent truncate | — | OPEN |
| F1-17 | 1 | LOW | Mission list RPC unbounded / no admin gate | — | OPEN |
| F1-18 | 1 | LOW | Exporters ignore `FileHandle.Write` errors | — | OPEN |
| F1-19 | 1 | LOW | Registry export empty items violates minItems | — | OPEN |
| F1-20 | 1 | LOW | Satellite/ortho meta JSON unescaped strings; hardcoded Proton path | — | OPEN |
| F1-21 | 1 | — | Tile Y-flip centralized in `tileUrl.ts` | — | OK |
| F1-22 | 1 | — | DEM export grid math + anchor gate | — | OK |
| F1-23 | 1 | — | MCP handlers + `mcp-call.sh` hardened | — | OK |
| F1-24 | 1 | — | RadioBridge stubs intentional | — | OK |
| F2B-01 | 2 | HIGH | `ExportMission` skipped `canViewMission` | **T-126 S1** | **RESOLVED** |
| F2B-02 | 2 | HIGH | Refresh rotation not atomic; no reuse detection | **T-126 S2** | **RESOLVED** |
| F2B-03 | 2 | MED | ORBAT slot claim + capacity race | **T-126 S3** | **RESOLVED** |
| F2B-04 | 2 | MED | `Refresh` ignored `user.IsBanned` | **T-126 S4** | **RESOLVED** |
| F2B-05 | 2 | MED | No mission archive/delete handler or UI | — | OPEN |
| F2B-06 | 2 | MED | CI/`make ci-local` skips `internal/services` et al. | — | OPEN |
| F2B-07 | 2 | LOW | `missions.go` Count error ignored | — | OPEN |
| F2B-08 | 2 | LOW | `buildMissionDoc` silent empty export on load fail | — | OPEN |
| F2B-09 | 2 | LOW | Refresh token rows never purged | — | OPEN |
| F2B-10 | 2 | LOW | Empty `handlers/missions/` stray dir | T-128 P3 | **RESOLVED** (untracked dir — `rmdir` in main checkout; see T-128 log) |
| F2B-11 | 2 | LOW | In-memory ratelimit prefix match footgun | — | OPEN |
| F2B-12 | 2 | — | Handler envelopes, bodylimit, SSE, inject path | — | OK |
| F2F-01 | 2 | MED | 401-retry dropped rotated refresh token | **T-126 S5** | **RESOLVED** |
| F2F-02 | 2 | MED | Bootstrap/callback `clearSession` after rotation + `/me` blip | **T-126 S6** | **RESOLVED** |
| F2F-03 | 2 | MED | Conflict "load server" not persisted to IDB | **T-127 U1** | **RESOLVED** (partial: F4-03 new-tab) |
| F2F-04 | 2 | MED | `exportJson` fire-and-forget; no compile error UX | **T-127 U2** | **RESOLVED** |
| F2F-05 | 2 | MED | `basemapView==='map'` silent grid; no degrade toast | **T-127 U3** | **RESOLVED** |
| F2F-06 | 2 | LOW | `events.tsx` flattens 409 error strings | **T-127 U5** | **RESOLVED** |
| F2F-07 | 2 | LOW | `admin.tsx` uses `window.confirm` vs Aegis Dialog | — | OPEN |
| F2F-08 | 2 | — | TS strictness, compile order, tile math, mutations | — | OK |
| F2C-01 | 2 | — | Registry / loadout / editor-payload chains in sync | — | OK |
| F2C-02 | 2 | HIGH | Game-server chain: no canonical `mission.json` producer; staging docs assert live routes | T-092 · T-128 | DEFERRED · **RESOLVED** (docs now mark gates BLOCKED on T-092; producer itself stays T-092) |
| F2C-03 | 2 | MED | InjectMission path vs mod `$profile:` id/filename mismatch | T-092 | DEFERRED |
| F2C-04 | 2 | MED | `ticket brief` prints branch vs main-only policy | — | OPEN (policy now hybrid — parallel tickets use `ticket/T-0xx` worktrees; script text out of T-128 scope) |
| F2C-05 | 2 | LOW | Stale `apps/website/frontend/docs/` duplicate tree | T-128 P3 | **RESOLVED** (tree deleted) |
| F3-01 | 3 | MED | No Discord 429 / Retry-After handling | — | OPEN |
| F3-02 | 3 | MED | Webhook embed title not truncated (256 cap) | — | OPEN |
| F3-03 | 3 | LOW | OAuth redirect when client_id blank | — | OPEN |
| F3-04 | 3 | — | Webhook failure isolation, OAuth cookie, role sync | — | OK |
| F4-01 | 4 | MED | Folder delete subtree — no confirm | **T-127 U4** | **RESOLVED** |
| F4-02 | 4 | MED | Dual-view basemap logically unsound until map tiles | **T-127 U3** · T-090.1.1 | **RESOLVED** · DEFERRED |
| F4-03 | 4 | MED | Conflict-resolution loop (trust) | **T-127 U1** | **PARTIAL** (same-tab ✓; new-tab deferred) |
| F4-04 | 4 | MED | Mission library append-only (no archive/delete UX) | — | OPEN |
| F4-05 | 4 | LOW | Export download no success/fail toast; local export omits server fields | **T-127 U2** | **RESOLVED** |
| F4-06 | 4 | LOW | Registration 409 nuance lost in toast | **T-127 U5** | **RESOLVED** |
| F4-07 | 4 | LOW | Non-UUID mission id — interactive but unsavable editor | — | OPEN |
| F4-08 | 4 | LOW | No in-UI shortcut discoverability | — | OPEN |
| F5-01 | 5 | MED | CLAUDE.md stale T-090 ACTIVE SLICE contradictions | T-128 P4 | **RESOLVED** (registry + sync + narrative) |
| F5-02 | 5 | MED | Arland 10240 typo in CLAUDE + `MissionCreatorPage` comment | T-128 P4 | **RESOLVED** (both → 4096) |
| F5-03 | 5 | MED | `apps/website/CLAUDE.md` + frontend README wrong `CLAUDE.md` depth | T-128 P2 | **RESOLVED** |
| F5-04 | 5 | MED | Staging docs phantom `/compiled` + roster gates | T-128 P1 | **RESOLVED** (BLOCKED-on-T-092 callouts; deploy smoke skip-guarded) |
| F5-05 | 5 | MED | `apps/mod/README.md` pre-monorepo (25 broken links) | T-128 P2 | **RESOLVED** (monorepo rewrite) |
| F5-06 | 5 | MED | `apps/website/README.md` broken links | T-128 P2 | **RESOLVED** |
| F5-07 | 5 | MED | 155 broken relative markdown links (full list below) | T-128 P0–P4 | **RESOLVED** (worktree scan 158 → 2 benign: one untracked scaffold, one template ellipsis placeholder — see T-128 log) |
| F5-08 | 5 | LOW | `tileUrl.ts` variable named `tmsY` (XYZ row) | — | OPEN |
| F5-09 | 5 | LOW | Mermaid `\n` in t092 spec labels | — | OPEN |
| F5-10 | 5 | LOW | British/American spelling inconsistency | — | OPEN |
| F5-11 | 5 | — | Living docs misspellings above noise floor | — | OK |
| F5-12 | 5 | — | Eden-wiki scrape typos (verbatim external) | — | OK (no fix) |

---

## 1. MOD & ENGINE ARCHITECTURE

### REST loader chain (mod → backend) — dead end-to-end
- **DEFERRED (T-092)** — **HIGH** — `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c:239` fetches `GET /api/missions/{id}/compiled`. The backend registers **no such route** — everything lives under `/api/v1`, and no `compiled` endpoint exists anywhere in `apps/website/internal/handlers/handlers.go`. The mod's own doc comment (line 81) admits it is a "mod-expected backend route". The mission REST path can only ever exercise the `$profile:` file fallback.
- **DEFERRED (T-092)** — **HIGH** — `TBD_MissionLoader.c:237` (and `TBD_MissionListLoader.c:99`, `TBD_RosterLoader.c:94`) authenticates with `Authorization: Bearer <serverToken>`, but the backend's only service auth is the **`X-Service-Token`** header (`internal/middleware/auth.go:43`), scoped to `/api/v1/ingest/*`. Even if the routes existed, auth would 401. Also note the header string `"Authorization, Bearer %1,Accept,application/json"` — the first value arrives as `" Bearer <tok>"` with a leading space from the `", "` separator.
- **DEFERRED (T-092)** — **HIGH** — `TBD_MissionListLoader.c:11-16` parses `{"missions":[…],"count":N}` with camelCase `slotCount`; the backend list contract is `{data,total,limit,offset}` with snake_case rows (`handlers.go` / `missions.go:136`). Envelope + casing both mismatch.
- **DEFERRED (T-092)** — **HIGH** — `TBD_RosterLoader.c:96` fetches `GET /api/game/events/{eventId}/roster` — no such route exists. Roster enforcement silently degrades to round-robin forever.
- **RESOLVED (T-128)** — **MED** — `packages/tbd-schema/spikes/rest-spike-0.1.md` links `internal/handlers/gameserver.go`, `internal/middleware/servertoken.go`, `cmd/restspike` — all deleted. The game-server REST surface was a spike that never merged, yet the mod + staging tooling (see §2 cross-chain) still target it.

### Coordinate & transform logic
- **DEFERRED (T-092)** — **MED** — `TBD_SpawnManager.c:146-148`: heading matrix sets forward `Transform[2] = (-sin θ, 0, cos θ)`. For compass-style headings (90° = east = +X), forward at 90° becomes (−1,0,0) = **west** — the rotation sign is the CCW convention, unverified against `headingDeg`'s intended CW compass semantics. This is precisely the open T-092 "spawn Y/yaw" scope — flagging so the sign check is explicit in that program's gate.
- **DEFERRED (T-090.3)** — **MED** — `TBD_TerrainWorldExportPlugin.c:158-167`: JSONL deliberately emits **`yawDeg` = raw `GetAngles()[0]` (pitch)** and **`pitchDeg` = `[1]` (the real heading)** — documented at lines 142-145, but the field names lie. Any consumer that trusts the names silently mirrors every building. T-090.3 must emit `headingDeg = GetAngles()[1]` and this file should stop shipping swapped names even in a spike.
- **OK (verified)** — the tile Y-flip is correctly centralized: `tileUrl.ts` converts the editor's south-first row to the on-disk XYZ (north-first) row via `2**z − 1 − y`, with a unit test. (See §5 for its inverted `tmsY` variable naming.)
- **OK (verified)** — `TBD_TerrainExportPlugin.c` grid math (`px * WORLD/(W−1)`) matches the verify sampler's inverse by construction; anchor gate passed at 0.204 m max delta.

### Map-assets manifests vs disk
- **DEFERRED (T-090.1.1) · RESOLVED (T-127 U3 @ `0515aabb`)** — **HIGH** — Everon manifest advertises `map` pyramid; only satellite on disk. Frontend coerces persisted `'map'` → `'satellite'` until T-090.1.1.
- **OPEN** — **MED** — `packages/map-assets/everon/manifest.json:5`: `metersPerPixel: 1` contradicts the schema's own instruction ("must come from Info & Diags — Everon Biki: 2 m", `terrain-manifest.schema.json:29-32`) and the manifest's `precision.demNativeMetersPerPixel: 2`. One of the two numbers is wrong or the field is under-specified.
- **DEFERRED (T-090)** — **MED** — `packages/map-assets/arland/manifest.json:17-25` advertises a full `tiles` block (maxZoom 5) but `packages/map-assets/arland/` contains only `manifest.json` — no tiles, no DEM file (`widthPx: 0` stub is documented; the tiles block is not).
- **OPEN** — **LOW** — `terrain-manifest.schema.json:19` pins `terrainId` to `["everon","arland","custom"]` while `terrain-registry.schema.json` allows any id — adding a third real map requires a schema edit; the registry/manifest disagree on extensibility.

### Runtime robustness
- **DEFERRED (T-092)** — **MED** — `TBD_SpawnManager.c` has **no player-disconnect cleanup**: `m_mPlayerSlot`, `m_sDeployRequested` key on `playerId`, which Reforger reuses after disconnect. A rejoining (or new) player inheriting a used id gets a stale slot or is never deployed (`DeployPlayer` early-returns on `m_sDeployRequested.Contains`).
- **DEFERRED (T-092)** — **MED** — `TBD_SpawnManager.c:54-62`: round-robin assignment never skips roster-assigned or already-assigned slots — a walk-in player and a rostered player can both be assigned the same slot.
- **DEFERRED (T-122 T15)** — **MED** — `TBD_ScenarioRouter.c:13`: `TBD_ADDON_GUID = "B2C3D4E5F6A78901"` is a placeholder (own TODO, T-122 T15). Cross-terrain `RequestScenarioChangeTransition` ships a bogus addon list in production.
- **DEFERRED (T-092)** — **LOW** — `TBD_SpawnManager.c:95-103`: `EngineFactionKey` hardcodes `blufor→US`, `opfor→USSR`; `factions[].presetId` from the mission document is ignored — any third faction silently gets no spawn points (`continue` at line 134).
- **OPEN** — **LOW** — `TBD_MissionLoader.c:306`: profile mission read caps at 8 MB (`handle.Read(data, 8*1024*1024)`); a larger file silently truncates and surfaces as a misleading "JSON parse failed".
- **OPEN** — **LOW** — `TBD_MissionBrowser.c:88` (`TBD_RpcAsk_MissionList`): the list RPC has no admin gate (only select does) and returns the whole list as one RPC string — unbounded payload for large libraries.
- **OPEN** — **LOW** — file exporters (`TBD_TerrainExportPlugin.c`, `TBD_TerrainWorldExportPlugin.c`, `TBD_SatelliteExportPlugin.c`, `TBD_RegistryItemsExportPlugin.c`) never check `FileHandle.Write` results — a full disk yields a silently truncated raster/JSON.
- **OPEN** — **LOW** — `TBD_RegistryItemsExportPlugin.c:150-157`: if every row fails to resolve, it still writes `"items": []`, violating its own schema (`minItems: 1`) with no guard.
- **OPEN** — **LOW** — `TBD_SatelliteExportPlugin.c:163` / `TBD_EngineOrthoExportPlugin.c:137`: `attempts`/`rmsg` strings are embedded in hand-built JSON without escaping — a quote in `GetReportMessage` output corrupts the meta JSON. Hardcoded Proton path `C:/Users/steamuser/…` (line 48) is env-fragile (documented, but unconfigurable).
- **OK (verified)** — `EMCP_WB_*` handlers use engine `JsonApiStruct` (no hand-built JSON), cap var dumps at 50, and hold no cross-call entity refs; `scripts/mod/mcp-call.sh` is genuinely hardened (daemon-first, bounded retries, PIPESTATUS-checked, tmpfile cleanup trap).
- **OK (verified)** — `TBD_RadioBridgeStub` empty bodies are documented intentional Phase-3 stubs, not defects.

---

## 2. WEB FULL-STACK ARCHITECTURE

### Backend (Go)
- **RESOLVED (T-126 S1 @ `4a47688e`)** — **HIGH** — `internal/handlers/missions.go:758-765`: **`ExportMission` never calls `canViewMission`** — any `mission_maker` can export any other author's *draft/pending/rejected* mission, payload included. This is exactly the leak T-122 T2 closed for `GetMission`/`GetVersion`/`GetArmory`; export was missed. `InjectMission` is safe only incidentally (requires `live` status). **Proof:** `TestExportMissionVisibility`.
- **RESOLVED (T-126 S2 @ `4a47688e`)** — **HIGH** — `internal/handlers/auth.go:180-202`: refresh rotation is **not atomic** — plain `First` then unconditional `Update revoked_at`. Two concurrent presentations of the same token both pass the revoked check and both mint fresh pairs (token-family fork), and there is **no reuse detection** (the standard response to a spent token is revoking the whole family). Fix: `UPDATE … WHERE id = ? AND revoked_at IS NULL` + `RowsAffected == 1` gate; treat 0 rows as reuse. **Proof:** `TestRefreshReuseRevokesFamily`.
- **RESOLVED (T-126 S3 @ `4a47688e`)** — **MED** — `internal/handlers/events.go:731-753`: slot claim is check-then-set with no row lock or conditional update — two users claiming the same slot both get 200; last write wins and the loser's registration row points at a slot assigned to someone else. Fix: `UPDATE orbat_slots SET assigned_to=? WHERE id=? AND (assigned_to IS NULL OR assigned_to=?)` + RowsAffected. Same class: the `registered >= capacity` waitlist check (line 754) races registrations past capacity. **Proof:** `TestSlotClaimRace` (2 goroutines → one 200 / one 409).
- **RESOLVED (T-126 S4 @ `4a47688e`)** — **MED** — `internal/handlers/auth.go:204-215`: `Refresh` never checks `user.IsBanned`. Mitigated because `BanUser` (`admin.go:160-162`) revokes refresh tokens — but that revocation's error is silently discarded; if it fails, a banned user refreshes for up to 30 days. Belt-and-braces: check the flag in `Refresh`. **Proof:** `TestRefreshBannedRejected`.
- **OPEN** — **MED** — mission lifecycle dead-end: `models.MissionArchived` and `Mission.DeletedAt` exist (`models/mission.go:20,78`) but **no handler ever archives or deletes a mission** — no `DELETE /missions/:id`, no archive action, no UI. The library only grows.
- **OPEN** — **MED** — CI test scope: `.github/workflows/ci.yml:69` and `Makefile:42` run only `go test ./internal/handlers/...`. Unit tests in `internal/services` (`discord_test.go`, `webhook_test.go`, `mission_payload_test.go`, `mortar_test.go`), `internal/middleware`, `internal/realtime` **never run in CI or `make ci-local`** (only the unwired `make test` covers them).
- **OPEN** — **LOW** — `missions.go:126`: `q.Count(&total)` error ignored — a failed count silently reports `total: 0` alongside real rows. Same best-effort reads in `decorateMissions` (authors/bookmarks).
- **OPEN** — **LOW** — `missions.go:713-721` (`buildMissionDoc`): a failed current-version load silently exports `payload: {}` / `version: "0.0.0"` — a broken export looks like an empty mission instead of an error.
- **OPEN** — **LOW** — refresh-token rows accumulate forever (revoked rows are never purged); no cleanup job.
- **RESOLVED (T-128 P3)** — **LOW** — `apps/website/internal/handlers/missions/` is an empty stray directory. *(Untracked — not in git; removed via `rmdir` in the main checkout, see T-128 log.)*
- **OPEN** — **LOW** — `middleware/ratelimit.go` is in-memory single-instance (documented); `strings.Contains(path, p)` prefix matching would misfire on any future route containing `/auth/` mid-path.
- **OK (verified)** — error envelope `{"error": …}` is consistent across all 8 handler files; list envelope matches CLAUDE.md; `bodylimit.go` mission-version skip is correct with a concrete-path fallback; `realtime/hub.go` delete-before-close under lock is race-safe; `RequireServiceToken` is constant-time and disabled-when-empty; `InjectMission` path is traversal-safe (UUID-derived); SSE + graceful shutdown wiring is sound.

### Frontend (React/TS)
- **RESOLVED (T-126 S5 @ `4a47688e`)** — **MED** — `src/api/client.ts:44-46`: on 401-retry with no `user` in the store, only `setAccessToken` is called — **the rotated `refresh_token` is dropped** while the presented one was already revoked server-side. Next refresh 401s → forced logout. `setAccessToken` should persist the whole rotated pair. **Fix:** `setTokens()` on full rotated pair.
- **RESOLVED (T-126 S6 @ `4a47688e`)** — **MED** — `src/hooks/useAuthBootstrap.ts:43-45`: a transient `/me` failure *after* a successful rotation runs `clearSession()`, discarding the freshly rotated (only valid) refresh token — a network blip at boot = forced re-login. Same pattern in `pages/auth.tsx:114-117` (callback page). Rotation success and profile-fetch failure need distinct handling. **Fix:** retain rotated pair on `/me` blip; only clear on rotation failure.
- **RESOLVED (T-127 U1 @ `0515aabb`)** — **MED** — `features/mission-creator/hooks/useMissionEditor.ts:521-536` (`resolveConflict('server')`): … **Proof:** [t127_verify_log.md](t127_verify_log.md). **Partial:** new-tab cold boot still prompts (F4-03 residual).
- **RESOLVED (T-127 U2 @ `0515aabb`)** — **MED** — `useMissionEditor.ts:508-519`: `exportJson` … **Proof:** verify log.
- **RESOLVED (T-127 U3 @ `0515aabb`)** — **MED** — `features/tactical-map/layers/useTerrainBasemapLayer.ts` … coerce in `basemapView.ts`. **Proof:** verify log.
- **RESOLVED (T-127 U5 @ `0515aabb`)** — **LOW** — `pages/events.tsx:395`: … **Proof:** verify log.
- **OPEN** — **LOW** — `pages/admin.tsx:175` uses native `window.confirm` for a destructive delete while the rest of the app uses the Aegis `Dialog` — inconsistent affordance.
- **OK (verified)** — zero `: any` in `src/`; refresh single-flight is correct (StrictMode-safe via module-level promise); `buildVersionBlob`'s hand-rolled JSON brace/comma assembly is correct; `compileMission` faction/squad/slot ordering matches Go `deriveOrbatFromEditor` exactly; `hydrateMissionDoc` clears entity maps before applying (replace, not merge); tile LOD zoom math (`ceil(log2(w/256)+zoom)`) is correct; `terrainManifest.ts` is fully defensive (`null`/`false` on any fetch failure); mutations surface errors at call sites throughout.

### Cross-system contract chain (Pass 5)
- **OK (verified)** — the registry chain: `TBD_RegistryItemsExportPlugin.c` (snake_case emit) → `registry-items.schema.json` → generated `internal/contract/registryitems` + `types/contract/registryItems.ts` (byte-identical embedded schema; `diff` clean) → `registry_items` model tags → `GET /registry`. The loadout chain (`loadout-export.schema.json` → `TBD_LoadoutGearStruct`) and the editor-payload chain (compile.ts → schema → `validate.go` → `ParseOrbatTemplate`) also line up field-for-field, including the deliberate integer-vs-string `schemaVersion` namespace split. `packages/tbd-schema` `validate.mjs`: **all contracts valid**.
- **DEFERRED (T-092) · RESOLVED (T-128 P1, docs half)** — **HIGH (chain break)** — the **game-server consumption chain has no producer**: `mission.schema.json` (canonical, `x/z/headingDeg`, `meta.id: msn_*`) is what `TBD_MissionDocumentStruct` parses, but nothing in the backend emits it. `ExportMission`/`InjectMission` emit the *editor superset wrapped in the camelCase envelope* (`missionJSON`), which the mod cannot parse (no top-level `meta`/`factions`/`zones`). This is known deferred work (T-092), but three artifacts treat it as live: `docs/mod/STAGING-SERVER.md:180-181` (gates V2/V3 expect 200 from `/api/missions/:id/compiled` + `/api/game/...roster`), `scripts/mod/deploy-staging.sh:189-197` (curls the same phantom route), and `docs/mod/tbd-reforger-platform-build-plan.md:290`. Those verification gates cannot pass against the current backend. T-128 marks gates **BLOCKED on T-092**.
- **DEFERRED (T-092)** — **MED (chain break)** — id + filename namespace mismatch: `InjectMission` stages `missions/<uuid>.mission.json` relative to the **API process cwd** (`field_tools.go:19,140`), while the mod fallback reads `$profile:missions/<missionId>.json` with `missionId` like `msn_8f3a2c`. Different directory root, different filename pattern, different id namespace; no bridge script maps one to the other.
- **OPEN (was T-128 P4)** — **MED** — `./scripts/ticket brief` prints `BRANCH: ticket/T-090` and `Makefile`/docs mention `ticket/T-0xx` branches, while CLAUDE.md policy (and memory) is **commit directly to `main`, never branch**. The generator contradicts the process it drives.
- **RESOLVED (T-128 P3)** — **LOW** — `apps/website/frontend/docs/pages/` is a stale duplicate of the moved `docs/website/frontend/pages/` tree (CLAUDE.md: surface specs are "not under `apps/`") — 2 orphaned files carrying 28 broken links.

---

## 3. DISCORD INTEGRATION

- **OPEN** — **MED** — **no 429 handling anywhere**: `services/discord.go:180-193` (`do`) and `webhook.go:107-117` treat 429 as a generic non-2xx failure — no `Retry-After` respect, no backoff, no retry. A rate-limited webhook push simply fails; a rate-limited OAuth exchange fails the login. For a community-scale app this is survivable, but announcement pushes and login bursts should honor `Retry-After` (Discord ToS expectation).
- **OPEN** — **MED** — embed limits unenforced: `webhook.go:80-89` truncates the description to 500 runes (safe against the 4096 cap) but **`a.Title` is passed through unbounded** — Discord rejects embeds with titles > 256 chars, so a long CMS title turns into a 400 "webhook push failed" with no hint. Pre-validate/truncate title (256) and footer (2048).
- **OPEN** — **LOW** — `handlers.go:42-48`: `DiscordService`/`WebhookService` are constructed even when unconfigured (empty client id / URL) — correct behavior is preserved by `Enabled()` checks, but `AuthorizeURL` with an empty `client_id` still redirects users to a broken Discord consent URL rather than failing fast when OAuth env is blank (known-blank in dev per CLAUDE.md; a guard + clear error would prevent a confusing prod misconfig).
- **OK (verified)** — failure isolation is correct: webhook push failure maps to a 502 with an audit trail and never crashes the request path (`cms.go:243-269`); `FetchGuildMember` tolerates non-members (`nil, nil` on 404) so login works for non-guild users; OAuth `state` cookie is httpOnly/SameSite=Lax/Secure-outside-dev with constant-time compare; tokens ride the URL *fragment* (not query) to keep them out of server logs; Discord user access tokens are used once and never stored; both HTTP clients carry a 10 s timeout; role sync (`role_sync.go`) is transactional with priority-based resolution (its `ResyncAllRoles` full-table N+1 loop is fine at community scale, noted only for growth).
- **OK (verified)** — payload schema matches the current webhook API: `embeds[]` with `title/description/color/timestamp/footer`, `?wait=true` for the message id, RFC3339 timestamp.

---

## 4. UX & UI DESIGN THEORY

### Strengths worth keeping (verified against live code)
The Mission Creator shell (`MissionCreatorPage.tsx`) is genuinely strong UX engineering: a determinate load gate with phase-labelled progress and an indeterminate-sweep fallback only where no total exists; a blocking, *actionable* restore-failure overlay (C3) instead of a silently empty editor; a forced-choice conflict dialog that can't be dismissed by outside-click; a degraded-basemap toast; field-focus-guarded shortcuts; and an Eden-faithful docked shell. The Save dialog's phase/progress/debug-report pipeline (T-060.1.x) is exemplary failure UX.

### Gaps and anti-patterns
- **RESOLVED (T-127 U4 @ `0515aabb`)** — **MED** — **destructive folder delete with no confirmation**: Aegis Dialog with subtree counts before `removeEditorLayer`.
- **RESOLVED (T-127 U3 @ `0515aabb`)** · **DEFERRED (T-090.1.1)** — **MED** — **dual-view basemap**: `'map'` coerced to `'satellite'` until map tiles ship; degraded toast when tiles absent.
- **PARTIAL (T-127 U1 @ `0515aabb`)** — **MED** — **conflict-resolution loop**: same-tab reload fixed (IDB + warm marker); new-tab cold boot still prompts — divergence tracking deferred.
- **OPEN** — **MED** — mission lifecycle has no exit: users can create, edit, submit, get rejected, resubmit — but never archive or delete (§2). The library becomes an append-only pile; "My Missions" grows forever, and mission_makers will ask admins to clean up via SQL.
- **RESOLVED (T-127 U2 @ `0515aabb`)** — **LOW** — export feedback: success/error toasts on Export (local export still omits server fields by design).
- **RESOLVED (T-127 U5 @ `0515aabb`)** — **LOW** — registration 409 nuance: distinct backend error strings surfaced in toasts.
- **OPEN** — **LOW** — journey seam on `/missions/:id/edit` for non-UUID ids: the editor stays fully interactive under a small yellow banner ("needs a real mission id", `MissionCreatorPage.tsx:245-251`) — work done there persists only to IndexedDB and can never be saved to the server; an interactive-but-unsavable editor is a data-loss trap dressed as a warning.
- **OPEN** — **LOW** — `window.confirm` in the admin Event Manager (§2) breaks the otherwise consistent frosted-Dialog language; keyboard model is strong overall (undo/redo/copy/paste/space/delete) but none of it is discoverable in-UI (no shortcut cheat-sheet, tooltips carry no keybinds).

---

## 5. MICROSCOPIC SEMANTIC ERRORS

**Method:** hunspell + custom sweeps over all 486 repo markdown files (node_modules excluded), a resolver that checked every relative link target on disk (155 broken), table/format lint against `DOCUMENTATION_STANDARDS.md`/`CODING_STANDARDS.md`, and close-reads of CLAUDE.md, ticket docs, and every code comment read in Passes 1–3. Living docs are notably clean of misspellings — the true typos concentrate in the `eden-wiki` scrape (verbatim external content; flagged but arguably faithful-as-scraped).

### Factual/consistency errors in living docs
- **RESOLVED (T-128 P4)** — `CLAUDE.md` §Status "### ACTIVE SLICE — T-090" block: says "active slice **T-090.3.0** (Workbench export spike); **T-090.1** … **queued**" and "**.3.0** Workbench spike **active** · **.1** basemap tiles (queued)" — **contradicts its own header** ("ACTIVE NOW: T-090.1.2.4") and the registry (`activeSlice: T-090.1.2.4`; brief lists T-090.3.0, T-090.1, T-090.1.2.x as shipped/DO-NOT-REOPEN). Stale block, two places.
- **RESOLVED (T-128 P4)** — `CLAUDE.md` T-049 bullet: "the camera + base grid resize to Everon 12800 vs **Arland 10240**" — Arland is **4096** everywhere authoritative (`coords/terrains.ts:59`, `terrain-registry.json:18`, `arland/manifest.json:4`).
- **RESOLVED (T-128 P4)** — `MissionCreatorPage.tsx:44` code comment repeats it: "Everon 12.8km vs **Arland 10.24km**" — same wrong number in live code.
- **RESOLVED (T-128 P2)** — `apps/website/CLAUDE.md` redirect: linked `../CLAUDE.md` (one level short) = `apps/CLAUDE.md` — **did not exist** (root is two levels up); the canonical-context pointer was broken, same bug in `apps/website/frontend/README.md → ../../CLAUDE.md`. Both fixed to the correct depth.
- **OPEN (was T-128 P4; policy now hybrid — worktree branches in use)** — `scripts/ticket brief` output: `BRANCH: ticket/T-090` vs the repo-wide "never branch, commit to main" rule — generator text contradicts policy (also §2).
- **OPEN** — `packages/map-assets/everon/manifest.json:5` `metersPerPixel: 1` vs schema instruction + `demNativeMetersPerPixel: 2` (also §1) — a numeric self-contradiction inside one file.
- **OPEN** — `layers/tileUrl.ts:24`: the computed value is the on-disk **XYZ** (north-first) row, but the variable is named `tmsY` — inverted terminology (TMS is south-first); comment and name disagree with each other.
- **RESOLVED (T-128 P1)** — `docs/mod/STAGING-SERVER.md` V2/V3 gate rows + `docs/mod/MILESTONES.md:21` + `docs/mod/tbd-reforger-platform-build-plan.md:44,168,290` describe `GET /api/missions/{id}/compiled` and `GET /api/game/events/{id}/roster` as live, expected-200 endpoints — no longer true of the current backend (also §2 chain break).
- **RESOLVED (T-128 P2)** — `apps/mod/README.md` is the **pre-monorepo README** (dated 2026-06-14, "Repo: github.com/darkforce09/tbd-reforger-platform", pre-move paths) — 25 broken links; the monorepo migration's doc-link repair missed it. Same class: `apps/website/README.md` (8 broken links, pre-move relative paths).
- **RESOLVED (T-128 P3)** — `apps/website/frontend/docs/pages/` — stale duplicate doc tree left under `apps/` (CLAUDE.md: surface specs live at `docs/website/frontend/pages/`, "not under apps/"); its `mission-editor.md` alone carries 28 dead links.

### Broken relative markdown links — 155 total, by source file (target that does not resolve)

**RESOLVED (T-128 P0–P4)** — batch repaired 2026-07-02; before/after counts in [`t128_doc_link_repair_log.md`](t128_doc_link_repair_log.md) (worktree scan 158 → 3 benign).

- `.ai/tickets/AI_PLAYBOOK.md` (8): `../docs/AGENT_COMMIT_CHECKLIST.md`, `../docs/TICKET_MOD_QUEUE.md` (×2), `../docs/TICKET_REGISTRY.md`, `../docs/TICKET_LEAD.md`, `../docs/TICKET_DEV_QUEUE.md`, `../docs/MILESTONES.md`, `../docs/TICKET_BRAINSTORM.md` — systematic one-level-short `../docs/` (needs `../../docs/`); several targets also live elsewhere (`AGENT_COMMIT_CHECKLIST` is under `docs/website/`).
- `.ai/tickets/README.md` (2): `../docs/TICKET_LEAD.md`, `../docs/AGENT_COMMIT_CHECKLIST.md` — same depth bug.
- `.ai/tickets/SPEC_TEMPLATE.md` (2): `../docs/TICKET_LEAD.md`, `../docs/AGENT_COMMIT_CHECKLIST.md`.
- `.ai/tickets/CLAUDE_CODE_PROMPT.md` (1): `../docs/specs/...t090_1_2_2_sap_cell_seam_repair.md` — one level short.
- `.ai/artifacts/README.md` (2): `../scripts/tools/scrape-eden-wiki.mjs`, `../docs/specs/.../eden/wiki_manifest.yaml` — one level short.
- `.ai/artifacts/t090_*_claude_code_handoff.md` + `t090_*_SEND_TO_CLAUDE.md` + `t090_1_2_operator_resume.md` (17 across 11 files): every `../docs/...` link — the whole handoff family shares the one-level-short pattern (needs `../../docs/...`); `t090_1_claude_code_handoff.md` additionally targets never-created specs `t090_1_aligned_basemap.md`, `t090_basemap_dual_view.md`.
- `.ai/artifacts/t122|t123|t124|t125_claude_code_handoff.md` (7): `../docs/platform/...` — same pattern; `t122` also targets `docs/platform/audit/t122_codebase_audit_hotfix.md` which never existed.
- `apps/mod/README.md` (25): `docs/STAGING-SERVER.md` (×3), `MILESTONES.md` (×2), `CLAUDE-CODE-START.md`, `CLAUDE-CONTINUATION.md`, `tbd-reforger-platform-build-plan.md`, `../shared/tbd-schema/`, `website/`, `Tbd_framework/`, `scripts/`, `docs/`, `scripts/mcp-call.sh`, `scripts/mcp-wb-logs.sh`, `scripts/tbd-spawn-verify.sh`, `scripts/tbd-dev-bootstrap.sh`, `scripts/setup-mcp-game-root.sh`, `scripts/deploy-staging.sh`, `scripts/debug-direct-join.sh`, `scripts/setup-client-addons.sh`, `scripts/remote-log-grep.sh`, `scripts/bootstrap-staging-server.sh`, `scripts/setup-server-profile.sh`, `scripts/run-dev-server.sh` — all pre-monorepo paths.
- `apps/mod/tbd-framework/README.md` (3): `../Tbd_framework/REFERENCE-ONLY.md`, `../docs/STAGING-SERVER.md`, `../../shared/tbd-schema/spikes/registry-poc-0.4.md`.
- `apps/website/README.md` (8): `DEV_RUNBOOK.md`, `docs/README.md`, `docs/TICKET_LEAD.md`, `docs/TICKET_REGISTRY.md`, `docs/website/frontend/ROADMAP.md`, `docs/backend/ROADMAP.md`, `docs/specs/Mission_Creator_Architecture/ROADMAP.md`, `docs/archive/README.md`.
- `apps/website/CLAUDE.md` (1): `../CLAUDE.md` (see above).
- `apps/website/frontend/README.md` (1): `../../CLAUDE.md`.
- `apps/website/frontend/docs/pages/mission-editor.md` (28): every `../../../specs/...` and `../../../TICKET_*` link — stale duplicate tree (full list in the link-audit; all 404 from that location).
- `apps/website/frontend/public/map-assets/README.md` (1): `../../docs/specs/.../t090_091_map_terrain_program.md` — two levels short (needs `../../../../docs/...`).
- `apps/website/frontend/src/stitch-exports/README.md` (1): `../../../docs/archive/README.md` — `docs/archive/` doesn't exist (archive is `docs/website/archive/`).
- `apps/mod/crf_framework/!Docs/.../VEHICLE_DEPOT_USER_GUIDE.md` (1): `images/vehicle_depot_example.png` (vendored reference content — archive-tier).
- `docs/mod/CLAUDE-CODE-START.md` (1): `../../.cursor/mcp.json` — `.cursor/` is gitignored/absent.
- `docs/specs/Mission_Creator_Architecture/ROADMAP.md` (3): `../../../website/frontend/pages/{mission-library,mission-editor,mission-creator}.md` — one level too many (resolves above repo root; needs `../../website/frontend/pages/`); `mission-creator.md` doesn't exist under the correct path either.
- `docs/specs/Mission_Creator_Architecture/t048_library_create_dialog.md` (6): `../../../apps/website/frontend/src/pages/not-found.tsx` (file is `utility.tsx`), `../../../website/frontend/pages/mission-library.md` (×2), `../../../website/frontend/shell/sidebar.md`, `../../../website/frontend/TRACKING.md`, `../../.cursor/rules/tbd-documentation.mdc`.
- `docs/specs/Mission_Creator_Architecture/t049…t060_1` specs (7 files, 1 each): `../../../website/frontend/pages/mission-editor.md` — same off-by-one depth (`t049_terrain_title_position.md`, `t050_cursor_z_readout.md`, `t052_undo_shortcuts.md`, `t056_copy_paste.md`, `t057_map_performance_hotfix.md`, `t058_entity_count_readout.md`, `t059_bulk_paste_operations.md`, `t060_1_scale_load_save_completion.md`).
- `docs/specs/Mission_Creator_Architecture/t068_3_palette_wire.md` (1): link to deleted `assetCatalogMock.ts` (deletion is the ticket's own point — link should be plain text).
- `docs/specs/Mission_Creator_Architecture/t090_0_map_program_hub.md` (2): `../../../scripts/website/verify-terrain-{manifest,alignment}.ts` — scripts live under `packages/tbd-schema/scripts/` as `.mjs`.
- `docs/website/AGENT_COMMIT_CHECKLIST.md` (6): `../../website/frontend/INDEX.md` (×2), `../../website/frontend/pages`, `../../website/frontend/shell/sidebar.md`, `../../website/frontend/pages/mission-editor.md` (×2) — one level too many (needs `../frontend/...`). This is the doc the commit process itself points agents at.
- `docs/website/CURSOR_SETUP.md` (1): `../../mod/CLAUDE-CODE-START.md` — needs `../mod/`.
- `docs/website/DEV_RUNBOOK.md` (1): `../tbd-schema/schema/terrain-manifest.schema.json` — needs `../../packages/tbd-schema/...`.
- `docs/website/frontend/_template.md` (2): `../../../../apps/website/frontend/src/stitch-exports/README.md` (one level too many), `../../../website/platform/context_handoff.md` (path never existed post-reorg).
- `docs/website/platform/macos_ux_architecture.md` (3): `../../../website/frontend/pages/{event-schedule,event-manager,mission-library}.md` — same class.
- `packages/tbd-schema/spikes/rest-spike-0.1.md` (7): `../../website`, `../../website/internal/middleware/servertoken.go`, `../../website/internal/handlers/gameserver.go`, `../../website/internal/server/server.go`, `../../website/cmd/restspike`, `../../website/scripts/rest-spike.sh`, `../../website/internal/handlers/gameserver_test.go` — archive-tier spike; the referenced code was removed (see §1/§2).

### Misspellings & grammar
Living docs (fix-worthy):
- **OK** — None found above the noise floor — the hand-written corpus (CLAUDE.md, specs, runbooks, standards) survived hunspell + manual close-reads with no genuine misspellings; flagged candidates all resolved to identifiers, domain vocabulary (ORBAT, supertexture, hillshade…), deliberate British/American mixing ("honour", "neighbourhood" — inconsistent but not wrong; CODING/DOCUMENTATION standards don't mandate a dialect), or quoted historical names (`REORG_CHANGELOG.md:45`'s "Mission ccreator" documents the *old* directory's actual name).

`.ai/artifacts/eden-wiki/` scrape (verbatim Bohemia-wiki copies — external prose, listed for completeness; "fixing" them would diverge from the source):
- **OK (no fix)** — `Eden_Editor__Composition.md:27` + `Eden_Editor__Custom_Composition.md:27` — "overwite" → overwrite.
- **OK (no fix)** — `Eden_Editor__Composition.md:143` + `Eden_Editor__Custom_Composition.md:143` — "extremly" → extremely.
- **OK (no fix)** — `Eden_Editor__Object_Categorization.md:183,267` — "aircrafts" → aircraft.
- **OK (no fix)** — `Eden_Editor__Object_Categorization.md:419` — "unamaged" → undamaged.
- **OK (no fix)** — `Eden_Editor__Object_Categorization.md:497` — "deigner" → designer.
- **OK (no fix)** — `Eden_Editor__Object_Categorization.md:585` — "immediatelly" → immediately.
- **OK (no fix)** — `Eden_Editor__Trigger.md:19` — "civillian" → civilian.
- **OK (no fix)** — `Eden_Editor__Trigger.md:286` — "interuptable" → interruptible.
- **OK (no fix)** — `Eden_Editor__Scenario_Attributes.md:645` — "respawn_guerrila" (sic — Arma engine class name; correct as quoted).

### Formatting / standards drift
- **RESOLVED (T-128 P4)** — `CLAUDE.md` "### ACTIVE SLICE — T-090" stale block (above) sits inside the `<!-- ticket-sync:status -->`-adjacent §Status that the docs say must be regenerated, not hand-drifted — the contradiction indicates a missed `./scripts/ticket sync` after the registry advanced to T-090.1.2.4.
- **OPEN** — `docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md:29,32`: mermaid labels use literal `\n` inside quoted strings ("payload\neditor", "string\nnative") — renders as `\neditor`/`\nnative` in some mermaid versions; use `<br/>`.
- **RESOLVED (T-128 P3)** — `apps/website/frontend/docs/` + `apps/website/internal/handlers/missions/` (empty dir) — two orphans that violate the reorg's own layout contract.
- **OPEN** — Domain dialect inconsistency (minor, pervasive): "artefact/artifact", "honour/honor", "modelled/modeled", "visualised/visualized" both appear across living docs; standards don't pick a side.

---

*Original audit: 6 passes @ `a3efdf68` (2026-07-01). **Living tracker** — T-126 ✓ · T-128 ✓ (this branch) · T-127 flips its rows on its own merge; update **Remediation tracker** + inline status prefixes when further tickets ship. Mirror shipped security items in [`CODEBASE_AUDIT_2026.md`](../../docs/platform/CODEBASE_AUDIT_2026.md) §Fable.*
