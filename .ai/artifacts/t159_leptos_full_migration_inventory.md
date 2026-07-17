# T-159 — Full Leptos migration inventory (evidence-based)

**Date:** 2026-07-14 · **Worktree:** `.ai/artifacts/worktrees/TBD-T-159` @ `t-159-leptos-ui`  
**Oracle:** React SPA at `apps/website/frontend/` on the same commit as worktree HEAD  
**Method:** `wc -l` + `rg` + file enumeration (not estimates). “110%” means **automated parity gates**, not eyeballing.

---

## 0. Honesty boundary (what “every character” means)

A human cannot usefully re-type 35k LOC character-by-character. This inventory is:

1. **Complete at file/export/route/hook/dependency granularity** (measured).
2. **Port work is sliced** so each slice has a **Class R / S / V / T** gate (below).
3. Slice is **not done** until gates exit 0 — no “looks good.”

### Verification classes (binding — T-151 discipline adapted)

| Class | Meaning | Pass condition |
|-------|---------|----------------|
| **R** | Response / state equality | Same API URL + method → body byte-equal or JSON deep-equal after canonical sort; wasm doc `encode_state` equal after same mutator sequence |
| **S** | Structural inventory | Checklist row count matches React oracle (routes, hooks, tokens, components) — `diff` of manifests |
| **V** | Visual identity | Fixed viewport screenshots: React oracle vs Leptos; pixel match within declared ε (default **ε=0** for chrome; map GPU may use Class R of GPU readback instead) |
| **T** | Interaction trace | Scripted pointer/keyboard sequence → identical store/doc snapshot hash |
| **G** | Build/green | `cargo check`/`trunk build` + React still builds until cutover; clippy on new crates |

**Fail any gate → slice fails.** No partial ship.

---

## 1. Measured baseline

| Metric | Value | Command evidence |
|--------|------:|------------------|
| Total `.ts`/`.tsx`/`.css` LOC under `src/` | **35,823** | `find … \| xargs wc -l` |
| Files | **235** | same |
| `features/` | **23,479** | 170 files |
| `pages/` | **6,494** | 10 files |
| `components/` | **1,098** | 23 files |
| `hooks/` | **986** | 4 files |
| `types/` | **1,114** | 10 files |
| `wasm/` glue TS | **1,625** | 2 files (plus pkg) |
| Feature vitest LOC | **5,888** | 53 files |
| Routes (prod) | **26** path entries | `router.tsx` (+ DEV spike) |
| Query hooks | **24** | `hooks/queries.ts` |
| Mutation hooks | **22** | `hooks/mutations.ts` |
| Named UI/layout exports | **43** | components inventory |
| Runtime npm deps | **25** | `package.json` |
| Dev npm deps | **33** | `package.json` |
| Aegis `@theme` color tokens | **44** color + typography + layout | `index.css` 329 LOC |
| `className=` sites | **1,346** | `rg` |

**Map engine / mission doc are already Rust/wasm** (~2.9k LOC of TS is thin host). The rewrite is UI + HTTP + chrome + hosting that wasm — not re-deriving wgpu.

---

## 2. Dependency → Leptos replacement matrix (complete)

### Must reimplement (no drop-in)

| npm | Import sites | Replacement | Gate |
|-----|-------------:|-------------|------|
| `react` / `react-dom` | 75+1 | Leptos | G |
| `react-router-dom` | 16 | `leptos_router` | S: route table |
| `@tanstack/react-query` | 4 | Leptos `Resource` + invalidation registry | R: same GETs |
| `zustand` | 3 | signals + context | R: auth persist shape |
| `@base-ui/react` | 5 | Custom Aegis primitives | V: dialog/sheet/switch |
| `sonner` | 16 | Custom toast | T: toast on save fail |
| `lucide-react` | 22 | `leptos-icons` / SVG | S: icon name list |
| `@tanstack/react-virtual` | 1 | Custom virtual window | T: 367k scroll smoke |
| `comlink` | 4 | `postMessage` protocol or in-wasm | R: compile blob bytes |
| `axios` | 2 | `gloo-net` / `reqwest` wasm | R: refresh single-flight |
| `class-variance-authority` / `clsx` / `tailwind-merge` | 4 | Rust `cn` helper | G |
| `date-fns` | 1 | `chrono` | R: format strings |

### Keep (assets / wasm)

| Item | Strategy |
|------|----------|
| `map_engine_wasm` | **Keep**; Leptos canvas host |
| `@fontsource/inter`, `jetbrains-mono` | Copy woff2 into Leptos assets |
| Tailwind + Aegis `index.css` | Port CSS wholesale (L4) |
| `public/map-assets` symlink | Same deploy layout |
| Material Symbols CDN | Keep or self-host (document) |

### Drop (unused or oracle-only)

| Package | Reason |
|---------|--------|
| `react-hook-form`, `@hookform/resolvers` | **0 imports** |
| `@tailwindcss/forms` | not wired |
| `zod` schemas in `src/schemas/` | unused by UI |
| Deck.gl / luma / supercluster / rbush (prod) | oracles / dead; wasm owns pick/cluster |
| `buffer`, `pngjs` (prod) | DEM in wasm |
| Vite/ESLint/Prettier TS toolchain | → trunk + clippy |

---

## 3. Route manifest (Class S checklist — every path)

Oracle: `apps/website/frontend/src/router.tsx`

| # | Path | Page component | fullBleed | chromeless | Router auth | Page AuthGate |
|--:|------|----------------|-----------|------------|-------------|---------------|
| 1 | `/login` | `LoginPage` | — | — | none | — |
| 2 | `/auth/callback` | `AuthCallbackPage` | — | — | none | — |
| 3 | `/` | `DashboardPage` | Y | N | none | AuthGate |
| 4 | `/server-intel` | `ServerIntelPage` | Y | N | none | AuthGate |
| 5 | `/announcements` | `AnnouncementsPage` | Y | N | none | AuthGate |
| 6 | `/deployments` | `DeploymentsPage` | Y | N | none | AuthGate |
| 7 | `/leaderboards` | `LeaderboardsPage` | Y | N | none | AuthGate |
| 8 | `/missions` | `MissionLibraryPage` | Y | N | none | AuthGate |
| 9 | `/missions/:id` | `MissionOverviewPage` | N | N | none | AuthGate |
| 10 | `/missions/:id/edit` | `MissionEditorPage` | Y | **Y** | `mission_maker+` | — |
| 11 | `/events` | `EventSchedulePage` | Y | N | none | AuthGate |
| 12 | `/events/:id` | `EventHubPage` | Y | N | none | AuthGate |
| 13 | `/events/:id/missions/:emid/orbat` | `OrbatSelectionPage` | N | N | none | AuthGate |
| 14 | `/wiki` | `WikiPage` | Y | N | none | AuthGate |
| 15 | `/wiki/:slug` | `WikiPage` | Y | N | none | AuthGate |
| 16 | `/vehicles` | `VehicleDatabasePage` | Y | N | none | AuthGate |
| 17 | `/modpacks` | `ModpacksPage` | Y | N | none | AuthGate |
| 18 | `/tools/mortar` | `MortarCalculatorPage` | Y | N | none | AuthGate |
| 19 | `/settings` | `SettingsPage` | N | N | none | AuthGate |
| 20 | `/admin/events` | `EventManagerPage` | N | N | admin | AdminGate |
| 21 | `/admin/approvals` | `MissionApprovalsPage` | Y | N | admin | AdminGate |
| 22 | `/admin/server` | `ServerControlPage` | Y | N | admin | AdminGate |
| 23 | `/admin/personnel` | `PersonnelRosterPage` | Y | N | admin | AdminGate |
| 24 | `/admin/content` | `ContentManagerPage` | Y | N | admin | AdminGate |
| 25 | `/admin/audit` | `AuditLogsPage` | Y | N | admin | AdminGate |
| 26 | `*` | `NotFoundPage` | N | N | none | — |
| D | `/_spike/wgpu` | `WgpuSpikePage` | — | — | DEV only | optional port |

**Gate S-routes:** Leptos router table CSV must `diff` equal to this list (minus DEV spike until ported).

---

## 4. Shell / Aegis primitives (must port 1:1)

### Layout (Class V chrome)

| Component | Path | Notes |
|-----------|------|-------|
| `AppLayout` | `components/layout/AppLayout.tsx` | fullBleed/chromeless from route handle |
| `Sidebar` + mobile toggle | `components/layout/Sidebar.tsx` | `nav-item-active` |
| `TopNav` | `components/layout/TopNav.tsx` | logout |
| `AuthGate` / `AdminGate` / `ProtectedRoute` | components/ | browse-mode passthrough |
| `QueryState` / `PageHeader` / `PageShell` / `OpsCard` / `StatusPill` / `MaterialIcon` | components/ | |

### UI kit (`components/ui/` — 11 files)

`Badge`, `Button`, `Card*`, `Dialog*`, `GlassPanel`, `HudBar`, `Input`, `Label`, `ListDetailItem`, `Sheet*`, `SplitPane`, `Switch`

### CSS tokens

Port entire `index.css` (329 LOC): **44** Aegis colors + typography scale + utilities (`.glass`, `.bg-topo-map`, `.custom-scrollbar`, MC load bar, …).

**Gate S-tokens:** extract CSS variable names from React `index.css` vs Leptos CSS → set equality.  
**Gate V-shell:** screenshot `/` chrome (sidebar+topnav) ε=0 at 1440×900.

---

## 5. API surface (every hook — Class R)

### Queries (24) — each needs a Leptos `Resource` or equivalent

`useMe`, `useRegistry`, `useLinkStatus`, `useDashboard`, `useLeaderboards`, `useServers`, `useServer`, `useAnnouncements`, `useDeployments`, `useEvents`, `useEvent`, `useOrbat`, `useMemberSearch`, `useMissions`, `useMission`, `useModpacks`, `useCurrentModpack`, `useWikiPages`, `useWikiPage`, `useVehicles`, `useApprovals`, `usePersonnel`, `useAuditLogs`, `useFactionLibrary`

### Mutations (22)

`useLogout`, `useRegisterMission`, `useWithdrawMission`, `useReserveSquad`, `useReleaseSquad`, `useAssignSlot`, `useAddEventMission`, `useApproveMission`, `useRejectMission`, `useCreateMission`, `useCreateEvent`, `useDeleteEvent`, `usePublishAnnouncement`, `useGenerateLinkCode`, `useUnlinkArma`, `useServerRcon`, `useSolveFireMission`, `useSetMissionStatus`, `useDeleteMission`, `useUpdateUserRole`, `useBanUser`, `useSaveFaction`, `useDeleteFaction`

### Auth plumbing (Class R + T)

| Piece | Path | Must preserve |
|-------|------|---------------|
| Axios client + 401 retry | `api/client.ts` | single retry after refresh |
| Single-flight refresh | `api/refresh.ts` | **one** POST /refresh under concurrency |
| Auth store persist | `useAuthStore.ts` | refreshToken+user+expiresAt; **not** accessToken |
| Bootstrap | `useAuthBootstrap.ts` | cold load path |
| OAuth hash parse | `auth.tsx` | fragment → tokens → /me |
| SSE telemetry | `useServerTelemetry.ts` | stream parse |

**Gate R-auth:** two concurrent 401s → exactly one refresh (integration test).  
**Gate R-api:** golden curl suite vs Leptos client for all 24+22 endpoints (status + JSON schema validate).

### Types

Hand-written `types/api` + `types/models/*` → **shared Rust structs** (T-159 L7).  
Generated `types/contract/*` → Rust from `make schema-codegen` (or serde from same JSON Schema).

---

## 6. Pages — port units (LOC from inventory)

| Unit | Source file(s) | ~LOC | Gate V route | Depends on |
|------|----------------|-----:|--------------|------------|
| Auth | `pages/auth.tsx` | (in 6.4k pages) | `/login`, `/auth/callback` | API client |
| Dashboard | `Dashboard.tsx` | | `/` | shell, queries |
| Server Intel | `ServerIntel.tsx` | | `/server-intel` | SSE |
| Settings | `Settings.tsx` | | `/settings` | link mutations |
| Missions | `missions.tsx` | | `/missions`, `/missions/:id` | Sheet, CreateMissionDialog |
| Events | `events.tsx` | | `/events`, `/events/:id`, orbat | OrbatSelector |
| Operations | `operations.tsx` | | announcements, deployments, leaderboards, schedule | SplitPane |
| Doctrine | `doctrine.tsx` | | wiki, vehicles, modpacks, mortar | |
| Admin | `admin.tsx` | | 5 admin routes | AdminGate |
| Utility | `utility.tsx` | | `/admin/server`, 404 | |

**Per-page gate:** V screenshot + R query payloads for that page’s hooks + S “no missing CTA from React DOM inventory” (exported component tree checklist per page — generated by scraping React tree in CI).

---

## 7. Mission Creator — the hard 23k (Class R/T/V)

### Already Rust (do not rewrite — host only)

| Capability | Hosted via |
|------------|------------|
| `RenderEngine` / world / DEM / slots GPU | `map_engine_wasm` |
| `MissionDoc` yrs | wasm |
| Pick / cluster indices | wasm |
| DollEngine | wasm |

### Must reimplement in Leptos (thin host + UI)

| Subsystem | LOC (approx) | Key files | Gates |
|-----------|-------------:|-----------|-------|
| `WgpuTacticalMap` host | 679 | `WgpuTacticalMap.tsx` | T: pan/zoom/select; R: camera matrix vs oracle |
| wgpu React glue hooks | ~1.1k | `useWgpu*.ts` | G + T |
| Select tool | 385 | `useSelectTool.ts` | T: marquee/drag/undo |
| Map store mirror | 281+ | `useMapStore.ts` | R: snapshot after mutators |
| ydoc mutators | 749 | `ydoc.ts` | R: encode_state after scripted edits |
| useMissionDoc / Editor | 189+603 | hooks | R: IDB round-trip; T: save 201 |
| yrsPersist | 116 | IDB v3 | R: blob equal |
| Compiler worker | 881 | compiler/ | R: compiled JSON Class R vs React worker |
| Registry worker | 915 | registry/ | R: canAttach/canEquip |
| Layout shell | 2,794 | TopCommandStrip, outliner, palette, Attributes | V + T |
| Virtual outliner | ~400 | VirtualOutliner + flatten | T: 367k scroll |
| Loadout / Arsenal | 2,819 | loadout/ | R: loadout JSON; V: Arsenal tab |
| CreateMissionDialog | 192 | | T: create → navigate edit |
| Factions dialog | 329 | | |
| TreeView / DnD | ~800 | tree/ | T: reparent |

### Browser platform seams (explicit Leptos/`web-sys` work)

COOP/COEP headers, WebGPU canvas, Workers, IndexedDB, localStorage/sessionStorage, SSE, OAuth hash, DnD, pointer capture, rAF loops, 600s upload timeout, map-assets static serve.

---

## 8. Expanded slice plan (verifiable units)

Replace the thin T-159.2–.4 stubs with this ladder. Each slice = one tag + verify log with gate table.

| Slice | Scope | Exit gates (all required) |
|-------|--------|---------------------------|
| **T-159.1** | Scaffold crate + hello | G |
| **T-159.2** | Aegis CSS + shell chrome stubs (Sidebar/TopNav empty routes) | S-tokens, V-shell |
| **T-159.3** | HTTP client + single-flight refresh + auth store + bootstrap + login/callback | R-auth, T-login |
| **T-159.4** | `leptos_router` full route table (stubs OK) + ProtectedRoute | S-routes |
| **T-159.5** | Shared Rust API types crate (from schemas/models) | R: serde round-trip goldens |
| **T-159.6** | All 24 query + 22 mutation Resources wired | R-api suite |
| **T-159.7** | UI kit (11 primitives) + toast | V-kit, T-toast |
| **T-159.8** | Dashboard page parity | V+R for `/` |
| **T-159.9** | Operations pages (announcements, deployments, leaderboards, schedule) | V+R ×4 |
| **T-159.10** | Missions library + overview + CreateMissionDialog | V+R+T |
| **T-159.11** | Events hub + ORBAT | V+R+T |
| **T-159.12** | Doctrine (wiki, vehicles, modpacks, mortar) | V+R ×4 |
| **T-159.13** | Settings + Server Intel (SSE) | V+R+T-sse |
| **T-159.14** | Admin suite (5 pages) | V+R ×5 |
| **T-159.15** | MC: canvas host + camera/pointer bridge (no full chrome) | T-map, R-camera |
| **T-159.16** | MC: WasmMissionDoc + Zustand→signals mirror + undo | R-doc |
| **T-159.17** | MC: yrsPersist IDB + editor session | R-idb |
| **T-159.18** | MC: select/marquee/drag tools | T-tools |
| **T-159.19** | MC: TopCommandStrip + settings + save/export + compiler worker | R-compile, T-save |
| **T-159.20** | MC: Outliner + virtual list + AssetPalette DnD | T-outliner |
| **T-159.21** | MC: Attributes + Arsenal/loadout | R-loadout, V-arsenal |
| **T-159.22** | MC: registry compat worker | R-compat |
| **T-159.23** | Visual ε=0 sweep all 26 routes + MC at fixed fixtures | V-suite |
| **T-159.24** | Cutover: default serve Leptos; React behind flag or removed | G + operator sign-off |
| **T-159.25** | Delete React frontend + npm toolchain from default path | G CI |

Parallelism: **159.5–6** can overlap page ports once shell+auth exist. **MC 15–22** serial after shell. Arsenal polish on `main` merges carefully into worktree (L8).

---

## 9. What is NOT in scope (until named)

- Workbench / Enfusion / AR rewrite
- Reimplementing `map-engine-*` in Leptos
- Changing API contracts for migration convenience
- “Good enough” visual (ε>0) without operator waiver in verify log

---

## 10. Deliverables to generate next (Cursor/Claude)

1. Machine-readable manifests (checked into `.ai/artifacts/t159_manifests/`):
   - `routes.csv`, `hooks.csv`, `components.csv`, `css_tokens.txt`, `deps.csv`
2. Screenshot oracle harness (Playwright or CDP) for React baseline hashes.
3. Update [`t159_leptos_ui_program.md`](../../docs/platform/t159_leptos_ui_program.md) slice index to §8.
4. Proceed T-159.1 scaffold; do not start pages until 159.2–.3 gates green.

---

## 11. Bottom line

| Fact | Number |
|------|-------:|
| LOC to retire from React UI path | **~35,823** |
| Already Rust (wasm) — host, don’t rewrite | map + mission doc engines |
| Prod routes to match | **26** |
| API hooks to match | **46** |
| UI primitives | **11** + layout shell |
| Feature tests to re-home or keep as oracles | **53** files / **5,888** LOC |
| Verifiable slices proposed | **T-159.1 … T-159.25** |

Identity = **gates**, not vibes. Next operator action: approve this ladder → Cursor expands hub `slices[]` → Claude runs **T-159.1**.
