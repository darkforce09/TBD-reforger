# Fable 5 Master Omni-Audit [2026-07-01]

**Remediation:** **T-126** (security) → **T-127** (MC UX) → **T-128** (docs) · hub [`docs/platform/FABLE_5_AUDIT_PROGRAM.md`](../../docs/platform/FABLE_5_AUDIT_PROGRAM.md)

Read-only audit of the full monorepo at working-tree state (`main` @ `a3efdf68` + uncommitted T-090.1.2.4 work). Preflight: `git pull` — already up to date; `./scripts/ticket brief T-090` — active slice **T-090.1.2.4**. Severity: **CRITICAL** (breaks a shipped flow), **HIGH** (breaks an intended contract or will bite soon), **MED** (real defect, bounded blast radius), **LOW** (hygiene). Every finding was re-verified against the cited file before inclusion. Terminology note used once: the "C++ Workbench plugins" are EnforceScript (`.c`) — audited as such.

---

## 1. MOD & ENGINE ARCHITECTURE

### REST loader chain (mod → backend) — dead end-to-end
- **HIGH** — `apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c:239` fetches `GET /api/missions/{id}/compiled`. The backend registers **no such route** — everything lives under `/api/v1`, and no `compiled` endpoint exists anywhere in `apps/website/internal/handlers/handlers.go`. The mod's own doc comment (line 81) admits it is a "mod-expected backend route". The mission REST path can only ever exercise the `$profile:` file fallback.
- **HIGH** — `TBD_MissionLoader.c:237` (and `TBD_MissionListLoader.c:99`, `TBD_RosterLoader.c:94`) authenticates with `Authorization: Bearer <serverToken>`, but the backend's only service auth is the **`X-Service-Token`** header (`internal/middleware/auth.go:43`), scoped to `/api/v1/ingest/*`. Even if the routes existed, auth would 401. Also note the header string `"Authorization, Bearer %1,Accept,application/json"` — the first value arrives as `" Bearer <tok>"` with a leading space from the `", "` separator.
- **HIGH** — `TBD_MissionListLoader.c:11-16` parses `{"missions":[…],"count":N}` with camelCase `slotCount`; the backend list contract is `{data,total,limit,offset}` with snake_case rows (`handlers.go` / `missions.go:136`). Envelope + casing both mismatch.
- **HIGH** — `TBD_RosterLoader.c:96` fetches `GET /api/game/events/{eventId}/roster` — no such route exists. Roster enforcement silently degrades to round-robin forever.
- **MED** — `packages/tbd-schema/spikes/rest-spike-0.1.md` links `internal/handlers/gameserver.go`, `internal/middleware/servertoken.go`, `cmd/restspike` — all deleted. The game-server REST surface was a spike that never merged, yet the mod + staging tooling (see §2 cross-chain) still target it.

### Coordinate & transform logic
- **MED** — `TBD_SpawnManager.c:146-148`: heading matrix sets forward `Transform[2] = (-sin θ, 0, cos θ)`. For compass-style headings (90° = east = +X), forward at 90° becomes (−1,0,0) = **west** — the rotation sign is the CCW convention, unverified against `headingDeg`'s intended CW compass semantics. This is precisely the open T-092 "spawn Y/yaw" scope — flagging so the sign check is explicit in that program's gate.
- **MED** — `TBD_TerrainWorldExportPlugin.c:158-167`: JSONL deliberately emits **`yawDeg` = raw `GetAngles()[0]` (pitch)** and **`pitchDeg` = `[1]` (the real heading)** — documented at lines 142-145, but the field names lie. Any consumer that trusts the names silently mirrors every building. T-090.3 must emit `headingDeg = GetAngles()[1]` and this file should stop shipping swapped names even in a spike.
- **OK (verified)** — the tile Y-flip is correctly centralized: `tileUrl.ts` converts the editor's south-first row to the on-disk XYZ (north-first) row via `2**z − 1 − y`, with a unit test. (See §5 for its inverted `tmsY` variable naming.)
- **OK (verified)** — `TBD_TerrainExportPlugin.c` grid math (`px * WORLD/(W−1)`) matches the verify sampler's inverse by construction; anchor gate passed at 0.204 m max delta.

### Map-assets manifests vs disk
- **HIGH** — `packages/map-assets/everon/manifest.json:35-39` advertises a `map` pyramid at `tiles/map/{z}/{x}/{y}.webp`; **only `tiles/satellite/` exists on disk**. Combined with the frontend behavior (§2/§4), selecting the Map view renders nothing.
- **MED** — `packages/map-assets/everon/manifest.json:5`: `metersPerPixel: 1` contradicts the schema's own instruction ("must come from Info & Diags — Everon Biki: 2 m", `terrain-manifest.schema.json:29-32`) and the manifest's `precision.demNativeMetersPerPixel: 2`. One of the two numbers is wrong or the field is under-specified.
- **MED** — `packages/map-assets/arland/manifest.json:17-25` advertises a full `tiles` block (maxZoom 5) but `packages/map-assets/arland/` contains only `manifest.json` — no tiles, no DEM file (`widthPx: 0` stub is documented; the tiles block is not).
- **LOW** — `terrain-manifest.schema.json:19` pins `terrainId` to `["everon","arland","custom"]` while `terrain-registry.schema.json` allows any id — adding a third real map requires a schema edit; the registry/manifest disagree on extensibility.

### Runtime robustness
- **MED** — `TBD_SpawnManager.c` has **no player-disconnect cleanup**: `m_mPlayerSlot`, `m_sDeployRequested` key on `playerId`, which Reforger reuses after disconnect. A rejoining (or new) player inheriting a used id gets a stale slot or is never deployed (`DeployPlayer` early-returns on `m_sDeployRequested.Contains`).
- **MED** — `TBD_SpawnManager.c:54-62`: round-robin assignment never skips roster-assigned or already-assigned slots — a walk-in player and a rostered player can both be assigned the same slot.
- **MED** — `TBD_ScenarioRouter.c:13`: `TBD_ADDON_GUID = "B2C3D4E5F6A78901"` is a placeholder (own TODO, T-122 T15). Cross-terrain `RequestScenarioChangeTransition` ships a bogus addon list in production.
- **LOW** — `TBD_SpawnManager.c:95-103`: `EngineFactionKey` hardcodes `blufor→US`, `opfor→USSR`; `factions[].presetId` from the mission document is ignored — any third faction silently gets no spawn points (`continue` at line 134).
- **LOW** — `TBD_MissionLoader.c:306`: profile mission read caps at 8 MB (`handle.Read(data, 8*1024*1024)`); a larger file silently truncates and surfaces as a misleading "JSON parse failed".
- **LOW** — `TBD_MissionBrowser.c:88` (`TBD_RpcAsk_MissionList`): the list RPC has no admin gate (only select does) and returns the whole list as one RPC string — unbounded payload for large libraries.
- **LOW** — file exporters (`TBD_TerrainExportPlugin.c`, `TBD_TerrainWorldExportPlugin.c`, `TBD_SatelliteExportPlugin.c`, `TBD_RegistryItemsExportPlugin.c`) never check `FileHandle.Write` results — a full disk yields a silently truncated raster/JSON.
- **LOW** — `TBD_RegistryItemsExportPlugin.c:150-157`: if every row fails to resolve, it still writes `"items": []`, violating its own schema (`minItems: 1`) with no guard.
- **LOW** — `TBD_SatelliteExportPlugin.c:163` / `TBD_EngineOrthoExportPlugin.c:137`: `attempts`/`rmsg` strings are embedded in hand-built JSON without escaping — a quote in `GetReportMessage` output corrupts the meta JSON. Hardcoded Proton path `C:/Users/steamuser/…` (line 48) is env-fragile (documented, but unconfigurable).
- **OK (verified)** — `EMCP_WB_*` handlers use engine `JsonApiStruct` (no hand-built JSON), cap var dumps at 50, and hold no cross-call entity refs; `scripts/mod/mcp-call.sh` is genuinely hardened (daemon-first, bounded retries, PIPESTATUS-checked, tmpfile cleanup trap).
- **OK (verified)** — `TBD_RadioBridgeStub` empty bodies are documented intentional Phase-3 stubs, not defects.

---

## 2. WEB FULL-STACK ARCHITECTURE

### Backend (Go)
- **HIGH** — `internal/handlers/missions.go:758-765`: **`ExportMission` never calls `canViewMission`** — any `mission_maker` can export any other author's *draft/pending/rejected* mission, payload included. This is exactly the leak T-122 T2 closed for `GetMission`/`GetVersion`/`GetArmory`; export was missed. `InjectMission` is safe only incidentally (requires `live` status).
- **HIGH** — `internal/handlers/auth.go:180-202`: refresh rotation is **not atomic** — plain `First` then unconditional `Update revoked_at`. Two concurrent presentations of the same token both pass the revoked check and both mint fresh pairs (token-family fork), and there is **no reuse detection** (the standard response to a spent token is revoking the whole family). Fix: `UPDATE … WHERE id = ? AND revoked_at IS NULL` + `RowsAffected == 1` gate; treat 0 rows as reuse.
- **MED** — `internal/handlers/events.go:731-753`: slot claim is check-then-set with no row lock or conditional update — two users claiming the same slot both get 200; last write wins and the loser's registration row points at a slot assigned to someone else. Fix: `UPDATE orbat_slots SET assigned_to=? WHERE id=? AND (assigned_to IS NULL OR assigned_to=?)` + RowsAffected. Same class: the `registered >= capacity` waitlist check (line 754) races registrations past capacity.
- **MED** — `internal/handlers/auth.go:204-215`: `Refresh` never checks `user.IsBanned`. Mitigated because `BanUser` (`admin.go:160-162`) revokes refresh tokens — but that revocation's error is silently discarded; if it fails, a banned user refreshes for up to 30 days. Belt-and-braces: check the flag in `Refresh`.
- **MED** — mission lifecycle dead-end: `models.MissionArchived` and `Mission.DeletedAt` exist (`models/mission.go:20,78`) but **no handler ever archives or deletes a mission** — no `DELETE /missions/:id`, no archive action, no UI. The library only grows.
- **MED** — CI test scope: `.github/workflows/ci.yml:69` and `Makefile:42` run only `go test ./internal/handlers/...`. Unit tests in `internal/services` (`discord_test.go`, `webhook_test.go`, `mission_payload_test.go`, `mortar_test.go`), `internal/middleware`, `internal/realtime` **never run in CI or `make ci-local`** (only the unwired `make test` covers them).
- **LOW** — `missions.go:126`: `q.Count(&total)` error ignored — a failed count silently reports `total: 0` alongside real rows. Same best-effort reads in `decorateMissions` (authors/bookmarks).
- **LOW** — `missions.go:713-721` (`buildMissionDoc`): a failed current-version load silently exports `payload: {}` / `version: "0.0.0"` — a broken export looks like an empty mission instead of an error.
- **LOW** — refresh-token rows accumulate forever (revoked rows are never purged); no cleanup job.
- **LOW** — `apps/website/internal/handlers/missions/` is an empty stray directory.
- **LOW** — `middleware/ratelimit.go` is in-memory single-instance (documented); `strings.Contains(path, p)` prefix matching would misfire on any future route containing `/auth/` mid-path.
- **OK (verified)** — error envelope `{"error": …}` is consistent across all 8 handler files; list envelope matches CLAUDE.md; `bodylimit.go` mission-version skip is correct with a concrete-path fallback; `realtime/hub.go` delete-before-close under lock is race-safe; `RequireServiceToken` is constant-time and disabled-when-empty; `InjectMission` path is traversal-safe (UUID-derived); SSE + graceful shutdown wiring is sound.

### Frontend (React/TS)
- **MED** — `src/api/client.ts:44-46`: on 401-retry with no `user` in the store, only `setAccessToken` is called — **the rotated `refresh_token` is dropped** while the presented one was already revoked server-side. Next refresh 401s → forced logout. `setAccessToken` should persist the whole rotated pair.
- **MED** — `src/hooks/useAuthBootstrap.ts:43-45`: a transient `/me` failure *after* a successful rotation runs `clearSession()`, discarding the freshly rotated (only valid) refresh token — a network blip at boot = forced re-login. Same pattern in `pages/auth.tsx:114-117` (callback page). Rotation success and profile-fetch failure need distinct handling.
- **MED** — `features/mission-creator/hooks/useMissionEditor.ts:521-536` (`resolveConflict('server')`): (a) uses the **blocking** `hydrateMissionDoc` — at the 300k+ scale this codebase explicitly targets, that's a minutes-long freeze (the load path already has the chunked variant); (b) hydrate runs under `INIT_ORIGIN`, and v2 IDB persistence only triggers on `LOCAL_ORIGIN` (`useMissionEditor.ts:283`) — the adopted server state is **never written to IndexedDB**, so the next cold boot restores the old local content and re-prompts the same conflict until the user happens to make an edit.
- **MED** — `useMissionEditor.ts:508-519`: `exportJson` is `async`, typed `() => void`, invoked fire-and-forget from `TopCommandStrip` — a worker-compile failure is an unhandled rejection with **zero user feedback** (no toast, no error state). Pass 2's one true floating promise.
- **MED** — `features/tactical-map/layers/useTerrainBasemapLayer.ts:89-95,144-157`: with `basemapView === 'map'`, `computeLod` returns `none` **and** the resolve effect early-returns, so `onDegraded` never fires — Map view renders a silent grid-only canvas. `basemapView.ts` happily reads a persisted `'map'` from localStorage. Until T-090.1.1 ships map tiles, `'map'` should either be coerced to satellite or show the degraded toast.
- **LOW** — `pages/events.tsx:395`: mutation errors are flattened to generic strings ("Could not claim that slot") — the backend's distinct 409 reasons (`slot already taken` vs `squad is reserved by a leader`) are discarded rather than surfaced.
- **LOW** — `pages/admin.tsx:175` uses native `window.confirm` for a destructive delete while the rest of the app uses the Aegis `Dialog` — inconsistent affordance.
- **OK (verified)** — zero `: any` in `src/`; refresh single-flight is correct (StrictMode-safe via module-level promise); `buildVersionBlob`'s hand-rolled JSON brace/comma assembly is correct; `compileMission` faction/squad/slot ordering matches Go `deriveOrbatFromEditor` exactly; `hydrateMissionDoc` clears entity maps before applying (replace, not merge); tile LOD zoom math (`ceil(log2(w/256)+zoom)`) is correct; `terrainManifest.ts` is fully defensive (`null`/`false` on any fetch failure); mutations surface errors at call sites throughout.

### Cross-system contract chain (Pass 5)
- **VERIFIED IN SYNC** — the registry chain: `TBD_RegistryItemsExportPlugin.c` (snake_case emit) → `registry-items.schema.json` → generated `internal/contract/registryitems` + `types/contract/registryItems.ts` (byte-identical embedded schema; `diff` clean) → `registry_items` model tags → `GET /registry`. The loadout chain (`loadout-export.schema.json` → `TBD_LoadoutGearStruct`) and the editor-payload chain (compile.ts → schema → `validate.go` → `ParseOrbatTemplate`) also line up field-for-field, including the deliberate integer-vs-string `schemaVersion` namespace split. `packages/tbd-schema` `validate.mjs`: **all contracts valid**.
- **HIGH (chain break)** — the **game-server consumption chain has no producer**: `mission.schema.json` (canonical, `x/z/headingDeg`, `meta.id: msn_*`) is what `TBD_MissionDocumentStruct` parses, but nothing in the backend emits it. `ExportMission`/`InjectMission` emit the *editor superset wrapped in the camelCase envelope* (`missionJSON`), which the mod cannot parse (no top-level `meta`/`factions`/`zones`). This is known deferred work (T-092), but three artifacts treat it as live: `docs/mod/STAGING-SERVER.md:180-181` (gates V2/V3 expect 200 from `/api/missions/:id/compiled` + `/api/game/...roster`), `scripts/mod/deploy-staging.sh:189-197` (curls the same phantom route), and `docs/mod/tbd-reforger-platform-build-plan.md:290`. Those verification gates cannot pass against the current backend.
- **MED (chain break)** — id + filename namespace mismatch: `InjectMission` stages `missions/<uuid>.mission.json` relative to the **API process cwd** (`field_tools.go:19,140`), while the mod fallback reads `$profile:missions/<missionId>.json` with `missionId` like `msn_8f3a2c`. Different directory root, different filename pattern, different id namespace; no bridge script maps one to the other.
- **MED** — `./scripts/ticket brief` prints `BRANCH: ticket/T-090` and `Makefile`/docs mention `ticket/T-0xx` branches, while CLAUDE.md policy (and memory) is **commit directly to `main`, never branch**. The generator contradicts the process it drives.
- **LOW** — `apps/website/frontend/docs/pages/` is a stale duplicate of the moved `docs/website/frontend/pages/` tree (CLAUDE.md: surface specs are "not under `apps/`") — 2 orphaned files carrying 28 broken links.

---

## 3. DISCORD INTEGRATION

- **MED** — **no 429 handling anywhere**: `services/discord.go:180-193` (`do`) and `webhook.go:107-117` treat 429 as a generic non-2xx failure — no `Retry-After` respect, no backoff, no retry. A rate-limited webhook push simply fails; a rate-limited OAuth exchange fails the login. For a community-scale app this is survivable, but announcement pushes and login bursts should honor `Retry-After` (Discord ToS expectation).
- **MED** — embed limits unenforced: `webhook.go:80-89` truncates the description to 500 runes (safe against the 4096 cap) but **`a.Title` is passed through unbounded** — Discord rejects embeds with titles > 256 chars, so a long CMS title turns into a 400 "webhook push failed" with no hint. Pre-validate/truncate title (256) and footer (2048).
- **LOW** — `handlers.go:42-48`: `DiscordService`/`WebhookService` are constructed even when unconfigured (empty client id / URL) — correct behavior is preserved by `Enabled()` checks, but `AuthorizeURL` with an empty `client_id` still redirects users to a broken Discord consent URL rather than failing fast when OAuth env is blank (known-blank in dev per CLAUDE.md; a guard + clear error would prevent a confusing prod misconfig).
- **OK (verified)** — failure isolation is correct: webhook push failure maps to a 502 with an audit trail and never crashes the request path (`cms.go:243-269`); `FetchGuildMember` tolerates non-members (`nil, nil` on 404) so login works for non-guild users; OAuth `state` cookie is httpOnly/SameSite=Lax/Secure-outside-dev with constant-time compare; tokens ride the URL *fragment* (not query) to keep them out of server logs; Discord user access tokens are used once and never stored; both HTTP clients carry a 10 s timeout; role sync (`role_sync.go`) is transactional with priority-based resolution (its `ResyncAllRoles` full-table N+1 loop is fine at community scale, noted only for growth).
- **OK (verified)** — payload schema matches the current webhook API: `embeds[]` with `title/description/color/timestamp/footer`, `?wait=true` for the message id, RFC3339 timestamp.

---

## 4. UX & UI DESIGN THEORY

### Strengths worth keeping (verified against live code)
The Mission Creator shell (`MissionCreatorPage.tsx`) is genuinely strong UX engineering: a determinate load gate with phase-labelled progress and an indeterminate-sweep fallback only where no total exists; a blocking, *actionable* restore-failure overlay (C3) instead of a silently empty editor; a forced-choice conflict dialog that can't be dismissed by outside-click; a degraded-basemap toast; field-focus-guarded shortcuts; and an Eden-faithful docked shell. The Save dialog's phase/progress/debug-report pipeline (T-060.1.x) is exemplary failure UX.

### Gaps and anti-patterns
- **MED** — **destructive folder delete with no confirmation**: `EditorLayersSection.tsx:132` calls `removeEditorLayer` (documented as deleting the folder's whole subtree, T-037) from a hover-row action. Undo is the only net; Eden's own paradigm and the rest of the app (`admin.tsx` confirms a delete) set the expectation of a confirm for subtree-destroying actions — or at minimum an "N items deleted — Undo" toast.
- **MED** — **dual-view basemap is logically unsound in its current shipped state**: the manifest advertises a `map` pyramid that doesn't exist (§1), the Mission Settings radio is "present but disabled" (`basemapView.ts:3-4`), yet `setBasemapView('map')` is exported and a persisted `'map'` renders a silent grid-only canvas with no degraded feedback (§2). Until T-090.1.1, the three layers (manifest, preference store, renderer) disagree about whether Map view exists. The Google-Maps-style toggle *concept* (shared bounds/zoom, single Y-flip point) is sound.
- **MED** — **conflict-resolution loop**: choosing "Load saved version" doesn't persist the adopted state locally (§2), so a user who loads-server and walks away is re-asked the identical question next session — a classic trust-eroding repeat prompt.
- **MED** — mission lifecycle has no exit: users can create, edit, submit, get rejected, resubmit — but never archive or delete (§2). The library becomes an append-only pile; "My Missions" grows forever, and mission_makers will ask admins to clean up via SQL.
- **LOW** — export has no feedback loop: the Export action downloads (or silently fails, §2) with no success/failure toast, and the local export intentionally ships empty `gameMode`/`maxPlayers`/`armory` (`exportSchema.ts:45-51`) — a user comparing it to the server export will think data was lost. A one-line "local export — server fields omitted" note in the download flow would close that.
- **LOW** — backend 409 nuance is flattened at registration (`events.tsx:395`): "slot already taken" vs "squad is reserved by a leader" produce the same generic toast; the ORBAT selector loses the chance to teach squad-reservation semantics at exactly the moment the user hits it.
- **LOW** — journey seam on `/missions/:id/edit` for non-UUID ids: the editor stays fully interactive under a small yellow banner ("needs a real mission id", `MissionCreatorPage.tsx:245-251`) — work done there persists only to IndexedDB and can never be saved to the server; an interactive-but-unsavable editor is a data-loss trap dressed as a warning.
- **LOW** — `window.confirm` in the admin Event Manager (§2) breaks the otherwise consistent frosted-Dialog language; keyboard model is strong overall (undo/redo/copy/paste/space/delete) but none of it is discoverable in-UI (no shortcut cheat-sheet, tooltips carry no keybinds).

---

## 5. MICROSCOPIC SEMANTIC ERRORS

**Method:** hunspell + custom sweeps over all 486 repo markdown files (node_modules excluded), a resolver that checked every relative link target on disk (155 broken), table/format lint against `DOCUMENTATION_STANDARDS.md`/`CODING_STANDARDS.md`, and close-reads of CLAUDE.md, ticket docs, and every code comment read in Passes 1–3. Living docs are notably clean of misspellings — the true typos concentrate in the `eden-wiki` scrape (verbatim external content; flagged but arguably faithful-as-scraped).

### Factual/consistency errors in living docs
- `CLAUDE.md` §Status "### ACTIVE SLICE — T-090" block: says "active slice **T-090.3.0** (Workbench export spike); **T-090.1** … **queued**" and "**.3.0** Workbench spike **active** · **.1** basemap tiles (queued)" — **contradicts its own header** ("ACTIVE NOW: T-090.1.2.4") and the registry (`activeSlice: T-090.1.2.4`; brief lists T-090.3.0, T-090.1, T-090.1.2.x as shipped/DO-NOT-REOPEN). Stale block, two places.
- `CLAUDE.md` T-049 bullet: "the camera + base grid resize to Everon 12800 vs **Arland 10240**" — Arland is **4096** everywhere authoritative (`coords/terrains.ts:59`, `terrain-registry.json:18`, `arland/manifest.json:4`).
- `MissionCreatorPage.tsx:44` code comment repeats it: "Everon 12.8km vs **Arland 10.24km**" — same wrong number in live code.
- `apps/website/CLAUDE.md` redirect: links "[`../CLAUDE.md`](../CLAUDE.md)" = `apps/CLAUDE.md` — **does not exist** (root is two levels up). The canonical-context pointer is broken. Same one-level-short bug in `apps/website/frontend/README.md → ../../CLAUDE.md`.
- `scripts/ticket brief` output: `BRANCH: ticket/T-090` vs the repo-wide "never branch, commit to main" rule — generator text contradicts policy (also §2).
- `packages/map-assets/everon/manifest.json:5` `metersPerPixel: 1` vs schema instruction + `demNativeMetersPerPixel: 2` (also §1) — a numeric self-contradiction inside one file.
- `layers/tileUrl.ts:24`: the computed value is the on-disk **XYZ** (north-first) row, but the variable is named `tmsY` — inverted terminology (TMS is south-first); comment and name disagree with each other.
- `docs/mod/STAGING-SERVER.md` V2/V3 gate rows + `docs/mod/MILESTONES.md:21` + `docs/mod/tbd-reforger-platform-build-plan.md:44,168,290` describe `GET /api/missions/{id}/compiled` and `GET /api/game/events/{id}/roster` as live, expected-200 endpoints — no longer true of the current backend (also §2 chain break).
- `apps/mod/README.md` is the **pre-monorepo README** (dated 2026-06-14, "Repo: github.com/darkforce09/tbd-reforger-platform", pre-move paths) — 25 broken links; the monorepo migration's doc-link repair missed it. Same class: `apps/website/README.md` (8 broken links, pre-move relative paths).
- `apps/website/frontend/docs/pages/` — stale duplicate doc tree left under `apps/` (CLAUDE.md: surface specs live at `docs/website/frontend/pages/`, "not under apps/"); its `mission-editor.md` alone carries 28 dead links.

### Broken relative markdown links — 155 total, by source file (target that does not resolve)
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
- None found above the noise floor — the hand-written corpus (CLAUDE.md, specs, runbooks, standards) survived hunspell + manual close-reads with no genuine misspellings; flagged candidates all resolved to identifiers, domain vocabulary (ORBAT, supertexture, hillshade…), deliberate British/American mixing ("honour", "neighbourhood" — inconsistent but not wrong; CODING/DOCUMENTATION standards don't mandate a dialect), or quoted historical names (`REORG_CHANGELOG.md:45`'s "Mission ccreator" documents the *old* directory's actual name).

`.ai/artifacts/eden-wiki/` scrape (verbatim Bohemia-wiki copies — external prose, listed for completeness; "fixing" them would diverge from the source):
- `Eden_Editor__Composition.md:27` + `Eden_Editor__Custom_Composition.md:27` — "overwite" → overwrite.
- `Eden_Editor__Composition.md:143` + `Eden_Editor__Custom_Composition.md:143` — "extremly" → extremely.
- `Eden_Editor__Object_Categorization.md:183,267` — "aircrafts" → aircraft.
- `Eden_Editor__Object_Categorization.md:419` — "unamaged" → undamaged.
- `Eden_Editor__Object_Categorization.md:497` — "deigner" → designer.
- `Eden_Editor__Object_Categorization.md:585` — "immediatelly" → immediately.
- `Eden_Editor__Trigger.md:19` — "civillian" → civilian.
- `Eden_Editor__Trigger.md:286` — "interuptable" → interruptible.
- `Eden_Editor__Scenario_Attributes.md:645` — "respawn_guerrila" (sic — Arma engine class name; correct as quoted).

### Formatting / standards drift
- `CLAUDE.md` "### ACTIVE SLICE — T-090" stale block (above) sits inside the `<!-- ticket-sync:status -->`-adjacent §Status that the docs say must be regenerated, not hand-drifted — the contradiction indicates a missed `./scripts/ticket sync` after the registry advanced to T-090.1.2.4.
- `docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md:29,32`: mermaid labels use literal `\n` inside quoted strings ("payload\neditor", "string\nnative") — renders as `\neditor`/`\nnative` in some mermaid versions; use `<br/>`.
- `apps/website/frontend/docs/` + `apps/website/internal/handlers/missions/` (empty dir) — two orphans that violate the reorg's own layout contract.
- Domain dialect inconsistency (minor, pervasive): "artefact/artifact", "honour/honor", "modelled/modeled", "visualised/visualized" both appear across living docs; standards don't pick a side.

---

*End of audit. 6 passes executed; findings above are those that survived re-verification against the cited sources. Nothing in the repository was modified; this report is the only new file.*
