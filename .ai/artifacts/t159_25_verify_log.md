# T-159.25 — suite live-wire — verify log

**Slice:** T-159.25 (finish program stream 2). Sub-commits `.25a`…`.25e`.
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159` · branch `t-159-leptos-ui` · **base** `6839c1ec` (T-159.24).
**Executor:** claude-code (solo session). **Result: PASS.**

## Goal

Wire every suite page to real behavior: all React mutations, the SSE telemetry stream, the 6
mock-data pages, and the CreateMissionDialog — closing the gap between "byte-identical default
render" (already shipped) and "the product actually works".

## What shipped (by sub-commit)

- **`.25a` `0bc5be3a`** — `toast.rs` (sonner parity: top-right, 4 s, success/error, **renders no DOM
  while empty** so V captures are unaffected) + AppLayout mount; `client.rs` `ApiErr = (u16,
  Option<String>)` carrying the backend `{"error"}` body (T-127 U5) + `api_error_message`; **Settings**
  Generate Link Code / Unlink live (`POST`/`DELETE /me/link`); **Event Hub** full `OrbatSelector`
  (faction tabs → squad list → slot rows; register / withdraw / reserve / release + `AssignPicker`
  member typeahead `PUT …/assign`; refetch chain) + my_state badge + faction dossiers with real
  armory; standalone ORBAT route wired; **Mission Library** live scope tabs + `q`/terrain/mode/
  player filters, featured hero, card grid, dossier `Sheet` (shared `dossier_body` + armory tabs,
  archive/delete + Aegis confirm, OPEN IN MISSION CREATOR), `CreateMissionDialog` (`POST /missions`
  → editor) + Cmd/Ctrl+N; `ui.rs` `Dialog`/`Sheet` primitives + `badge_class`.
- **`.25b` `71533c4b`** — **Approvals** live tabs + row selection + `ReviewInspector` with live
  approve/reject (`POST /approvals/:id/{approve,reject}`); **Personnel** live `?q=` search + dossier
  with role editor (`PATCH /admin/users/:id`, revert-on-fail) + ban (`POST …/ban`, `window.prompt`
  reason). `datefmt::format_short_date`.
- **`.25c` `3f417c5f`** — `sse.rs` (**useServerTelemetry port** — Bearer `fetch` + ReadableStream
  reader, `\n\n` frame splitter, typed `ServerStatusDto` → signals); **Server Intel** full populated
  panel (default-server pick, SSE-fed telemetry with row-status fallback, copy-address, launch stub,
  theater + env columns); **Operations Calendar** live `/events` + `/missions`, month paging, day
  selection, Schedule dialog (`POST /events` + per-mission `POST /events/:id/missions`), delete
  (`DELETE /events/:id`). DTOs `EventListItem`, `ServerStatusDto`. web-sys +Headers/Request/
  RequestInit/Response/ReadableStream(Reader)/Navigator/Clipboard.
- **`.25d` `030faff2`** — **Mortar** signal inputs + Calculate → `POST /fire-missions/solve`
  (useSolveFireMission) → live solution card; **Content Manager** local docs (seeded from React's
  MOCK_DOCS — no docs API exists) + New/select/Save-Draft + Publish (`POST /cms/announcements`,
  category→tag map; SOP local-only) + live base-ui Switch. `ListDetailItem` gains `on_click`.
  DTO `FireSolution`.
- **`.25e` (this commit)** — the live mutation gate + `serve.mjs` same-origin `/api` proxy + verify log.

## Oracle-truth scope note (the "6 mock pages")

The plan listed wiki / vehicles / modpacks / server_control / content / event_manager as
"mock → live". Reading the **oracle** (`pages/doctrine.tsx`, `pages/utility.tsx`, `pages/admin.tsx`):
React's **ServerControl (`MOCK_SERVERS`), Modpacks (`MOCK_MODPACKS`), Wiki (`MANUALS`), Vehicle DB,
and the Content docs list are themselves client-mock-driven** — the `useModpacks`/`useWikiPages`/
`useVehicles`/`useServerRcon` hooks are declared but **not consumed** for the list data (no list
endpoints exist yet). So the Leptos consts already **ARE parity** for those. The genuine
behavior gaps closed here are the ones with real endpoints: Content **publish** (`/cms/announcements`),
Server Intel **SSE** + the RCON console is React-local echo (ported as local in a follow-up if
filed), and Event Manager (already fully live above). No page renders fabricated data the React
oracle doesn't also render.

## Gates

| Gate | Result |
|---|---|
| `cargo check -p website-leptos --target wasm32-unknown-unknown` | clean (0 warnings) at every sub-commit |
| `cargo check -p website-leptos` (native) | **9** warnings — stash-diff baseline (zero new) at every sub-commit |
| `cargo clippy … wasm32` | 12 — stash-diff baseline (zero new) |
| `trunk build --release` | ✅ success at every sub-commit |
| `cargo test -p website-leptos` | **42/42** (client single-flight contract + DTO R-api round-trips, incl. the new DTOs) |
| **`smoke_mutations`** (NEW, live backend) | **pass** — `authedRender` + `noCodeBefore` + `codePanelAfter` + `toastShown`, 0 panics |
| **11 editor smokes** (post-`.25`, fresh release dist) | **11/11 PASS** (shared files client.rs/dto.rs/ui.rs/main.rs changed — regression-clean) |

### The live mutation gate — `smoke_mutations.mjs`

Proves the mutation path works against the **running backend**, not just that it compiles: seeds a
real dev-login `tbd-auth` session into localStorage, serves the dist with `serve.mjs`'s new
same-origin `/api` proxy (→ `:8080`), boots the app (session bootstraps via `/me` → 401 →
single-flight refresh → retry 200), clicks **Generate Link Code**, and asserts the live `POST
/me/link` response reaches the DOM (the mono "Link code: …" panel) + a `role=status` toast.

```json
{"gate":"suite-mutations-smoke",
 "checks":{"authedRender":true,"noCodeBefore":true,"codePanelAfter":true,"toastShown":true},
 "panics":[],"pass":true}
```

Two harness fixes were needed (recorded so the pattern is reusable): (1) a `window.fetch` override
breaks trunk's `WebAssembly.instantiateStreaming` — use a **same-origin server proxy** instead
(`serve.mjs` `apiProxy`); (2) the seeded `tbd-auth` `user` must carry the **full** `auth.rs::User`
field set, or `from_persist_json` returns `None` and bootstrap silently early-returns (guest).

## Ops

- Live gates need `make db-up` + `make api` (:8080) + `make seed` (dev users/registry). Tokens minted
  via `GET /api/v1/auth/dev-login?role=admin`.

## Next

**T-159.26** — Mission Creator editor completion: Attributes modal (.23) → server-hydrate/conflict/
dirty → Mission Settings → ORBAT dock → keyboard → VirtualOutliner.
