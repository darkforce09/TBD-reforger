**Status:** live

## SHELL — Route & layout

#### SHELL-ROUTER-001 — Chromeless fullscreen editor

| Field | Value |
|-------|-------|
| **Domain** | SHELL |
| **Goal** | Eden-style editor owns full viewport without platform chrome |
| **Trigger** | Navigate to `/missions/:id/edit` as `mission_maker+` |
| **Preconditions** | Auth gate passes; lazy chunk may still load |
| **Procedure** | 1. `routes.ts` lazy-loads `MissionCreatorPage`. 2. Route handle `chromeless` + `fullBleed` suppresses `Sidebar`/`TopNav` in `AppLayout`. 3. `TacticalMap` `absolute inset-0`; frosted panels in `z-10` overlay. |
| **Postconditions** | Editor visible; map fills center between docked panels |
| **Inputs** | Route URL `:id` |
| **Outputs** | Render tree; Suspense fallback while chunk loads |
| **Edge cases** | Suspense shows "Loading editor…" |
| **Acceptance** | `- [ ] No platform sidebar/topnav on edit route` `- [ ] Map visible between w-64 / w-80 panels` |
| **Eden parity** | Eden:FILE-SESSION-001 |
| **Status** | working |
| **Evidence** | `mission-creator/routes.ts`, `MissionCreatorPage.tsx`, `AppLayout.tsx` |

#### SHELL-UUID-001 — Invalid mission ID banner

| Field | Value |
|-------|-------|
| **Domain** | SHELL |
| **Goal** | Warn when URL `:id` is not a UUID (Save Version would fail) |
| **Trigger** | Editor mount with non-UUID `:id` |
| **Preconditions** | `useMissionEditor` `UUID_RE` fails |
| **Procedure** | 1. `invalidMissionId=true`. 2. Yellow banner below top strip. 3. Save Version returns same error string. |
| **Postconditions** | User informed; local edit/export still possible |
| **Inputs** | URL param |
| **Outputs** | Banner UI; blocked save error |
| **Edge cases** | Export/hydrate may still run locally |
| **Acceptance** | `- [ ] /missions/test/edit shows banner` `- [ ] Save shows UUID error` |
| **Eden parity** | N/A (web routing) |
| **Status** | working |
| **Evidence** | `hooks/useMissionEditor.ts` (`UUID_RE`, `invalidMissionId`), `MissionCreatorPage.tsx` |

#### SHELL-CONFLICT-001 — Local vs server load conflict dialog

| Field | Value |
|-------|-------|
| **Domain** | SHELL |
| **Goal** | Let user choose IndexedDB draft vs server `current_version` when both have content |
| **Trigger** | Local v2 store (or legacy y-indexeddb) synced + cold `GET /missions/:id` returns `json_payload` + local doc has authored entities |
| **Preconditions** | `hasLocalContent()` true; server version exists |
| **Procedure** | 1. `useMissionDoc` `onSynced`. 2. Fetch mission. 3. `setConflict(payload)`. 4. User picks hydrate or keep local. |
| **Postconditions** | Doc matches chosen source; dirty set if keep local |
| **Inputs** | Dialog button clicks |
| **Outputs** | `hydrateMissionDoc` or dismiss; `INIT_ORIGIN` |
| **Edge cases** | Cannot dismiss without choice (`onOpenChange` noop); empty local auto-hydrates; API fail → local-only |
| **Acceptance** | `- [ ] Conflict when both have slots` `- [ ] "Load saved" hydrates server` `- [ ] "Keep local" preserves IndexedDB` |
| **Eden parity** | N/A |
| **Status** | working |
| **Evidence** | `hooks/useMissionEditor.ts` (`conflict`, `resolveConflict`, `hasLocalContent`), `MissionCreatorPage.tsx` |

#### SHELL-FPS-001 — FPS debug HUD

| Field | Value |
|-------|-------|
| **Domain** | SHELL |
| **Goal** | Dev perf readout during Deck.gl tuning |
| **Trigger** | Editor open (always on) |
| **Preconditions** | None |
| **Procedure** | RAF loop counts frames / 500ms → color-coded label bottom-right |
| **Postconditions** | FPS displayed |
| **Inputs** | N/A |
| **Outputs** | `FpsCounter` overlay (`pointer-events-none`) |
| **Edge cases** | Not gated behind dev flag |
| **Acceptance** | `- [ ] FPS number visible in editor` |
| **Eden parity** | N/A |
| **Status** | working |
| **Evidence** | `FpsCounter.tsx` |

---

| TBD-CONFLICT-001 | IndexedDB vs server conflict UI | working |
