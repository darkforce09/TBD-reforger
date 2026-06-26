# T-048 — Mission create from Library (macOS Dialog)

**Status:** shipped (T-048)  
**Git tag on ship:** T-048  
**Authority:** UX decision locked in [`agent_execution.md`](agent_execution.md) Decisions log · macOS methodology in [`docs/platform/macos_ux_architecture.md`](../../docs/platform/macos_ux_architecture.md) §2

---

## Problem

Mission creation today requires a **separate sidebar destination** (`Mission Creator` → `/missions/create`) — a full-page wizard that breaks context and contradicts the platform's macOS UX refactor (Event Manager create = Dialog over list, T-024).

**Current flow:**

```
Sidebar "Mission Creator" → /missions/create (full page) → POST /missions → /missions/:id/edit
Mission Library → dossier only (no create entry point)
```

**Target flow:**

```
Mission Library → "+ New Mission" (mission_maker+) → CreateMissionDialog → POST /missions → /missions/:id/edit
```

No `/missions/create` route. No sidebar item. Create is a **transient action** on the library surface (Apple Mail "New Message" pattern).

---

## Locked decisions (user confirmed — do not re-litigate)

| Decision | Choice |
|----------|--------|
| Dialog title | **New Mission** |
| Submit button | **Create Mission Draft** |
| Editor labels | **Keep "Mission Creator"** on dossier CTA + edit-route breadcrumb — only remove sidebar wizard tab |
| Form on close | **Reset on every close** — clean slate on reopen (macOS Mail pattern) |
| Entry points | **Header + New Mission** (mission_maker+) **+ empty-state CTA** on **My Missions** tab when that list is empty |
| Keyboard | **Cmd/Ctrl+N** on Mission Library opens create dialog (mission_maker+ only) |
| Dossier + create | **Close dossier Sheet first**, then open create dialog (one overlay at a time) |
| My Missions empty CTA | Only when **zero missions AND no active filters/search** on My Missions (true empty, not filtered empty) |
| Shortcut discoverability | **Tooltip** on New Mission button hover/focus (e.g. "New Mission (⌘N)" / Ctrl+N on Windows) |

### Git / doc commit strategy (AI clarity)

**Recommended: one T-048 commit on `main`** — code + all doc finalization together.

Why: surface specs (`mission-library.md`) already describe UI that does not exist yet. Landing docs on `main` without code misleads the next agent session. Either:

1. **Preferred:** Keep pre-staged doc edits uncommitted until code ships → single T-048 commit (code + doc finalize + CLAUDE §Status), or
2. If docs must be committed early: doc-only commit with every affected spec still marked `in-progress` / `planned` and MC ROADMAP **IN PROGRESS** — never `doc-complete` until code exists.

Do **not** split T-048a docs / T-048b code unless you accept a window where main docs lie about shipped UI.

---

## Code audit (2026-06-21)

| Area | Current state | Action |
|------|---------------|--------|
| [`frontend/src/pages/missions.tsx`](../../frontend/src/pages/missions.tsx) | `MissionLibraryPage` — no create CTA; `MissionCreatorPage` export (lines 521–647) = full-page wizard | Extract wizard → dialog; wire button in library header |
| [`frontend/src/router.tsx`](../../frontend/src/router.tsx) | `missions/create` under `ProtectedRoute minRole="mission_maker"` (lines 129–133) | **Remove route** |
| [`frontend/src/config/navigation.ts`](../../frontend/src/config/navigation.ts) | `{ label: 'Mission Creator', path: '/missions/create', … }` | **Remove nav item** |
| [`frontend/src/components/ui/dialog.tsx`](../../frontend/src/components/ui/dialog.tsx) | Frosted macOS Dialog (Base UI) — `max-w-lg`, scroll body | **Reuse** — reference [`admin.tsx`](../../frontend/src/pages/admin.tsx) Event Manager create (~347+) |
| [`frontend/src/hooks/mutations.ts`](../../frontend/src/hooks/mutations.ts) | `useCreateMission()` → `POST /missions` | **Unchanged** |
| [`frontend/src/features/mission-creator/MissionCreatorPage.tsx`](../../frontend/src/features/mission-creator/MissionCreatorPage.tsx) | 2D **editor** shell (Deck.gl) — **not** the wizard | **Do not rename/confuse** — editor stays at `/missions/:id/edit` |
| Dossier edit path | `[ OPEN IN MISSION CREATOR ]` → `/missions/:id/edit` | **Unchanged** |
| [`frontend/src/lib/stitch-map.ts`](../../frontend/src/lib/stitch-map.ts) | Maps `/missions/create` → stitch key | Remove entry |

**Naming collision (known):** `MissionCreatorPage` in `pages/missions.tsx` (wizard) vs `features/mission-creator/MissionCreatorPage.tsx` (editor). T-048 removes the pages export; optional follow-up: rename editor file only if needed — **out of scope**.

---

## UX specification

### Entry point — Mission Library header

- **Location:** [`MissionLibraryPage`](../../frontend/src/pages/missions.tsx) header row — right-aligned opposite title or inline with segmented scope tabs.
- **Label:** `New Mission` with `MaterialIcon name="add"` — match Event Manager button (not "+ New Mission" text prefix unless design prefers both icon + label)
- **Visibility:** `hasMinRole('mission_maker')` only — **hidden** for enlisted (no disabled tease).
- **Style:** `rounded-full bg-action px-6 py-3` — same classes as Event Manager [`admin.tsx`](../../frontend/src/pages/admin.tsx) L192–199.

### Empty state — My Missions tab

When `scope === 'mine'` and `missions.length === 0` (after load, not loading):

- Centered empty panel in grid area (glass card or dashed border — match existing QueryState empty patterns).
- Copy e.g. **"No missions yet"** + secondary line **"Create a draft to open the Mission Creator."**
- Primary CTA button opens the same `CreateMissionDialog` (reuse `setCreateOpen(true)`).
- Only for `mission_maker+`; enlisted on My Missions sees generic empty message without create CTA.
- **True empty only:** active when `scope === 'mine'`, `missions.length === 0`, and **no** active `q`, terrain, mode, or player filters. If user filtered/searching and got zero hits, keep generic **"No missions found."** (no create CTA).

### Dossier Sheet interaction

If `previewId != null` (dossier open) and user triggers create (header button, empty CTA, or Cmd/Ctrl+N):

1. `setPreviewId(null)` — close Sheet
2. Then `setCreateOpen(true)` — open dialog

One overlay at a time (user confirmed).

### Keyboard shortcut

- **Cmd+N** (macOS) / **Ctrl+N** (Windows/Linux) on Mission Library page.
- Only when `hasMinRole('mission_maker')` and create dialog is not already open.
- `useEffect` + `keydown` listener on `MissionLibraryPage`; `preventDefault()` when handled.
- Do not fire when focus is in an input/textarea/select (standard guard).
- Do not fire when dossier is open — close sheet first (same as button path) or ignore until sheet closed; **prefer close-then-open** for consistency.

### Tooltip

- `New Mission` header button: `title` attribute or accessible tooltip — **"New Mission (⌘N)"** on macOS, **"New Mission (Ctrl+N)"** elsewhere (detect `navigator.platform` or use generic **"New Mission — ⌘N / Ctrl+N"**).

### CreateMissionDialog

**New file:** [`frontend/src/features/mission-creator/CreateMissionDialog.tsx`](../../frontend/src/features/mission-creator/CreateMissionDialog.tsx)

```tsx
interface CreateMissionDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Default: navigate to /missions/:id/edit */
  onCreated?: (missionId: string) => void
}
```

**Dialog chrome:**

| Field | Value |
|-------|-------|
| `title` | **New Mission** |
| `description` | Define terrain and environment before opening the 2D editor. |
| Primitive | `Dialog` + `DialogContent` from `@/components/ui/dialog` |
| `className` | `max-w-lg` default OK; use `max-w-xl` if terrain tiles need width |

**Form fields** (preserve wizard behavior exactly):

| Field | Control | Default | Notes |
|-------|---------|---------|-------|
| Title | text input | `''` | Required; toast on empty |
| Terrain | 2 selectable tiles | `everon` | `everon`, `arland` — `terrainLabel()` |
| Game mode | select | `pve_coop` | pve_coop / pvp / zeus |
| Time of day | `type="time"` | `14:00` | |
| Weather | select | `clear` | clear / overcast / heavy_rain / dense_fog |
| Max players | select | `64` | 16, 32, 48, 64, 96, 128 |

**Styling upgrade (macOS):** migrate wizard controls from legacy `border-border-subtle bg-surface` to Event Manager patterns:

- Pill inputs: `rounded-full bg-white/5 px-5 py-3 focus:ring-1 focus:ring-primary/50`
- Terrain tiles: `rounded-xl border border-white/10` with `border-primary bg-primary/10` when selected
- Submit: full-width primary at bottom of dialog body — **Create Mission Draft**
- Pending: disable submit, label **Creating…**

**Submit:** `useCreateMission().mutate({ title, terrain, game_mode, weather, time_of_day, max_players })`

**Success:** toast "Mission created" → `onCreated(id)` or `navigate(\`/missions/${id}/edit\`)` → close dialog → reset form state.

**Error:** toast with API error message (same as current wizard).

**Close:** backdrop / Escape closes dialog; **always reset form state** on close (user confirmed — clean slate on reopen).

### Route removal

- Delete `missions/create` from [`router.tsx`](../../frontend/src/router.tsx).
- Remove `MissionCreatorPage` import from router (pages export).
- Direct navigation to `/missions/create` → app catch-all / 404 (verify [`not-found`](../../frontend/src/pages/not-found.tsx) behavior).

### Sidebar

- Remove Mission Creator from [`navigation.ts`](../../frontend/src/config/navigation.ts) Mission Hub section.
- Mission Hub = **Mission Library only** for all roles.

---

## API contract (unchanged)

`POST /api/v1/missions` — `mission_maker+`

```json
{
  "title": "string",
  "terrain": "everon|arland",
  "game_mode": "pve_coop|pvp|zeus",
  "weather": "clear|overcast|heavy_rain|dense_fog",
  "time_of_day": "HH:MM",
  "max_players": 64
}
```

Response includes `id` → navigate to editor.

---

## Files to change (implementation checklist)

| File | Change |
|------|--------|
| `frontend/src/features/mission-creator/CreateMissionDialog.tsx` | **Create** — extracted dialog + form |
| `frontend/src/pages/missions.tsx` | Wire dialog + header button; **delete** `MissionCreatorPage` export |
| `frontend/src/router.tsx` | Remove `missions/create` route |
| `frontend/src/config/navigation.ts` | Remove Mission Creator nav item |
| `frontend/src/lib/stitch-map.ts` | Remove `/missions/create` key |

**Optional export:** `frontend/src/features/mission-creator/index.ts` barrel export for `CreateMissionDialog` if other surfaces need it later.

### Header layout (match Event Manager)

Use the same flex pattern as [`admin.tsx`](../../frontend/src/pages/admin.tsx) lines 185–199:

```tsx
<header className="mb-6 flex flex-wrap items-start justify-between gap-4">
  <div>
    <h1>…</h1>
    <p>…subtitle…</p>
    {/* scope tabs stay below title block */}
  </div>
  {isMaker && (
    <button className="flex items-center gap-2 rounded-full bg-action px-6 py-3 …">
      <MaterialIcon name="add" />
      New Mission
    </button>
  )}
</header>
```

Import `MaterialIcon` and `useAuthStore` → `hasMinRole('mission_maker')` in `MissionLibraryPage` (dossier already uses this pattern).

### Imports cleanup after wizard removal

| Remove from `pages/missions.tsx` if unused | Still used by |
|-------------------------------------------|---------------|
| `PageHeader`, `OpsCard` | Check `MissionOverviewPage` — if yes, keep |
| `useCreateMission` | Moves to `CreateMissionDialog.tsx` |
| `MissionCreatorPage` export | Delete entirely |

Router: remove `MissionCreatorPage` from [`router.tsx`](../../frontend/src/router.tsx) import line 16 — keep `MissionLibraryPage`, `MissionOverviewPage`.

### Optional polish (same commit if trivial)

| Item | Note |
|------|------|
| Edit route breadcrumb | **Keep `'Mission Creator'`** per user decision |
| Dossier footer CTA | **Keep `[ OPEN IN MISSION CREATOR ]`** — no rename |
| `.cursor/rules/tbd-documentation.mdc` | Create Cursor auto-rule pointing at checklist + T-048 spec |

### Already correct (no change)

- Editor invalid-id banner: *"create one from Mission Library first"* — [`MissionCreatorPage.tsx`](../../frontend/src/features/mission-creator/MissionCreatorPage.tsx) L129
- `useCreateMission` invalidates `['missions']` — library grid refreshes on return
- No E2E tests reference `/missions/create`
- `POST /missions` API unchanged — no backend work

---

## Verification

```bash
cd frontend && npm run build && npm run lint
```

### Manual test plan

1. **mission_maker** dev-login → `/missions` → **New Mission** visible → dialog opens.
2. **Cmd/Ctrl+N** on Library → dialog opens (maker only).
3. **My Missions** empty → empty-state CTA opens dialog.
4. Submit valid form → toast → redirects to `/missions/:id/edit` → editor loads.
5. Close dialog without submit → reopen → **form is empty** (reset).
6. Empty title → error toast, dialog stays open.
7. **enlisted** dev-login → no create button, no shortcut, no empty-state create CTA.
8. `/missions/create` → 404 (or not-found).
9. Sidebar Mission Hub → only **Mission Library** (no Mission Creator tab).
10. Open existing mission dossier → **OPEN IN MISSION CREATOR** still opens editor.

---

## Documentation sync (same commit as code — T-048)

Use [`docs/AGENT_COMMIT_CHECKLIST.md`](../../docs/AGENT_COMMIT_CHECKLIST.md).

### Pre-staged (already in repo — do not revert)

- `mission-library.md`, `mission-creator.md` stub, `INDEX.md`, `sidebar.md`, `mission-editor.md`
- Both ROADMAPs (IN PROGRESS section), `agent_execution.md` Decisions log row, `macos_ux_architecture.md`
- `docs/AGENT_COMMIT_CHECKLIST.md`, `docs/README.md`, `CLAUDE.md` checklist link, `TAGS.md`

### Finalize on ship (same T-048 commit)

| Doc | Change |
|-----|--------|
| [`frontend/docs/pages/mission-library.md`](../../frontend/docs/pages/mission-library.md) | Status → `doc-complete`; M3/M4 milestones checked |
| [`frontend/docs/shell/sidebar.md`](../../frontend/docs/shell/sidebar.md) | Check off T-048 milestone |
| [`docs/frontend/ROADMAP.md`](../../docs/frontend/ROADMAP.md) | Move T-048 from IN PROGRESS → DONE shipped table |
| [`Design_Docs/.../ROADMAP.md`](ROADMAP.md) | Move T-048 from IN PROGRESS → DONE (or remove IN PROGRESS block) |
| [`t048_library_create_dialog.md`](t048_library_create_dialog.md) | Status → **shipped** |
| [`CLAUDE.md`](../../CLAUDE.md) §Status | T-048 Done bullet; bump latest-feature line |
| [`agent_execution.md`](agent_execution.md) | **Update stale `/missions/create` refs** in repository map (lines ~92, ~151), phase table (~207), DEFERRED table (~448) — Decisions log already correct |
| [`frontend/docs/TRACKING.md`](../../frontend/docs/TRACKING.md) | Points to `docs/TICKET_LEAD.md`; mission editor doc lives under `pages/mission-editor.md` |
| [`.cursor/rules/tbd-documentation.mdc`](../../.cursor/rules/tbd-documentation.mdc) | Create Cursor rule (optional but recommended) |
| [`docs/backend/architecture.md`](../../docs/backend/architecture.md) | Optional: "Mission Creator wizard" → "Library create dialog" (line ~547) |

**Do not update:** archive stitch HTML, Eden wiki artifacts, historical T-034/T-039 bullets in CLAUDE.

---

## Claude Code handoff prompt

```
Read CLAUDE.md and docs/AGENT_COMMIT_CHECKLIST.md first.

Implement T-048 per Design_Docs/Mission_Creator_Architecture/t048_library_create_dialog.md.
Docs are PRE-STAGED — do not revert target-state docs; FINALIZE per spec §Documentation sync on ship.

- Create CreateMissionDialog.tsx (extract wizard from pages/missions.tsx)
- Add + New Mission to MissionLibraryPage header (mission_maker+ only; match admin.tsx header flex)
- Remove /missions/create route, sidebar nav, MissionCreatorPage export
- macOS Dialog styling (match admin Event Manager create pattern)
- On ship: update agent_execution.md stale /missions/create refs + TRACKING pointer + CLAUDE §Status
- Verify: npm run build && npm run lint + manual test plan
- Commit on main as T-048 with Co-Authored-By trailer when I ask
```

---

## Related

- Surface spec (target): [`frontend/docs/pages/mission-library.md`](../../frontend/docs/pages/mission-library.md)
- Frontend ROADMAP: [`docs/frontend/ROADMAP.md`](../../docs/frontend/ROADMAP.md)
- MC ROADMAP: [`ROADMAP.md`](ROADMAP.md)
