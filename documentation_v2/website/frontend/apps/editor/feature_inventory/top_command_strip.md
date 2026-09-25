**Status:** live

## TOP — Command strip

#### TOP-MENU-001 — Menu bar stubs

| Field | Value |
|-------|-------|
| **Domain** | TOP |
| **Goal** | Eden File/Edit/View/Mission/Environment menus |
| **Trigger** | Click menu labels |
| **Preconditions** | — |
| **Procedure** | `MENUS` buttons with title "(soon)"; no handlers |
| **Postconditions** | Nothing |
| **Inputs** | LMB |
| **Outputs** | None |
| **Edge cases** | — |
| **Acceptance** | `- [ ] Menus visible, no dropdown` |
| **Eden parity** | Eden:TOP-MENU-001 |
| **Status** | stub |
| **Evidence** | `TopCommandStrip.tsx` |

#### TOP-TITLE-001 — Inline mission title edit

| Field | Value |
|-------|-------|
| **Domain** | TOP |
| **Goal** | Edit mission title in doc |
| **Trigger** | Type in title input |
| **Preconditions** | Doc loaded |
| **Procedure** | `setTitle(md, value)` → Y.Doc `meta.title` |
| **Postconditions** | Title updated; dirty |
| **Inputs** | Text |
| **Outputs** | Y.Doc meta |
| **Edge cases** | T-049: hydrated from the server mission row on load via `applyMissionRowMeta` (even when `json_payload` is `{}`); placeholder "Untitled Mission" only until the GET resolves. No PATCH-back yet. |
| **Acceptance** | `- [x] Title edits persist in IndexedDB` · `- [x] Mission row title hydrates on load (T-049)` |
| **Eden parity** | Eden:FILE-TITLE-001 |
| **Status** | partial |
| **Ticket** | T-049 |
| **Evidence** | `TopCommandStrip.tsx`, `ydoc.ts` |

#### TOP-DIRTY-001 — Unsaved changes dot

| Field | Value |
|-------|-------|
| **Domain** | TOP |
| **Goal** | Indicate local edits since last save |
| **Trigger** | Y.Doc update with `LOCAL_ORIGIN` |
| **Preconditions** | — |
| **Procedure** | `useMissionEditor` listener → yellow dot beside title |
| **Postconditions** | `dirty=true` until save/hydrate |
| **Inputs** | Any edit |
| **Outputs** | UI dot |
| **Edge cases** | INIT/persistence origins don't dirty |
| **Acceptance** | `- [ ] Dot after edit` `- [ ] Clears on Save Version` |
| **Eden parity** | N/A |
| **Status** | working |
| **Evidence** | `useMissionEditor.ts`, `TopCommandStrip.tsx` |

#### TOP-ENV-001 — Time-of-day scrubber

| Field | Value |
|-------|-------|
| **Domain** | ENV |
| **Goal** | Eden time control |
| **Trigger** | Drag range 0–1439 min |
| **Preconditions** | — |
| **Procedure** | `updateEnvironment({ time })` → HH:MM label |
| **Postconditions** | `meta.environment.time` |
| **Inputs** | Slider |
| **Outputs** | Y.Doc |
| **Edge cases** | Duplicates Mission Settings |
| **Acceptance** | `- [ ] Time slider updates label` |
| **Eden parity** | Eden:ENV-TIME-001 |
| **Status** | working |
| **Evidence** | `TopCommandStrip.tsx` |

#### TOP-ENV-002 — Weather quick select

| Field | Value |
|-------|-------|
| **Domain** | ENV |
| **Goal** | Quick weather preset |
| **Trigger** | Change `<select>` |
| **Preconditions** | — |
| **Procedure** | `updateEnvironment({ weather })` — clear/overcast/heavy_rain/dense_fog |
| **Postconditions** | Environment updated |
| **Inputs** | Select |
| **Outputs** | Y.Doc |
| **Edge cases** | Synced with settings dialog |
| **Acceptance** | `- [ ] Weather changes persist` |
| **Eden parity** | Eden:ENV-WX-001 |
| **Status** | working |
| **Evidence** | `TopCommandStrip.tsx` |

#### TOP-HIST-001 — Version history (disabled)

| Field | Value |
|-------|-------|
| **Domain** | TOP |
| **Goal** | Visual-Git timeline |
| **Trigger** | History button |
| **Preconditions** | — |
| **Procedure** | Button `disabled` title "soon" |
| **Postconditions** | — |
| **Inputs** | — |
| **Outputs** | — |
| **Edge cases** | Deferred (Visual-Git timeline) |
| **Acceptance** | `- [ ] Button disabled` |
| **Eden parity** | TBD-only |
| **Status** | disabled |
| **Evidence** | `TopCommandStrip.tsx` |

#### TOP-UNDO-001 — Undo button

| Field | Value |
|-------|-------|
| **Domain** | KEY |
| **Goal** | Revert last edit |
| **Trigger** | Click Undo **or Cmd/Ctrl+Z** (T-052) |
| **Preconditions** | `canUndo()` |
| **Procedure** | `UndoController.undo()` Yjs stack |
| **Postconditions** | Prior state restored |
| **Inputs** | Button / keyboard |
| **Outputs** | Y.Doc revert |
| **Edge cases** | Hydrate not undoable; keyboard skipped while a form field is focused (T-052) |
| **Acceptance** | `- [x] Undo after move (button + Cmd/Ctrl+Z)` |
| **Eden parity** | Eden:KEY-UNDO-001 |
| **Status** | working |
| **Ticket** | T-052 |
| **Evidence** | `TopCommandStrip.tsx`, `MissionCreatorPage.tsx` keydown, `undo.ts`, `useMissionDoc.ts` (StrictMode `instanceKey`) |

#### TOP-REDO-001 — Redo button

| Field | Value |
|-------|-------|
| **Domain** | KEY |
| **Goal** | Reapply undone edit |
| **Trigger** | Click Redo **or Cmd/Ctrl+Shift+Z / Ctrl+Y** (T-052) |
| **Preconditions** | `canRedo()` |
| **Procedure** | `UndoController.redo()` |
| **Postconditions** | State forward |
| **Inputs** | Button / keyboard |
| **Outputs** | Y.Doc |
| **Edge cases** | Keyboard skipped while a form field is focused (T-052) |
| **Acceptance** | `- [x] Redo after undo (button + Cmd/Ctrl+Shift+Z / Ctrl+Y)` |
| **Eden parity** | Eden:KEY-REDO-001 |
| **Status** | working |
| **Ticket** | T-052 |
| **Evidence** | `TopCommandStrip.tsx`, `MissionCreatorPage.tsx` keydown |

#### TOP-SAVE-001 — Save Version dialog + POST

| Field | Value |
|-------|-------|
| **Domain** | DATA |
| **Goal** | Immutable semver snapshot to server |
| **Trigger** | Save → confirm dialog |
| **Preconditions** | Valid UUID; semver non-empty |
| **Procedure** | 1. `compileMissionWithProgress` (chunked, T-060). 2. `POST /missions/:id/versions` (256 MB route cap). 3. Clear dirty; update `currentSemver`. |
| **Postconditions** | Server version created |
| **Inputs** | Semver, notes |
| **Outputs** | API |
| **Edge cases** | 409 duplicate semver; invalid UUID; **413 payload >256 MB** → "Mission too large…" (T-060); backend `error` surfaced (no more generic-only) |
| **Acceptance** | `- [x] Save 0.1.1 succeeds on real mission` `- [x] Save @ ~360k fully diagnosed (T-060.1.3)` `- [x] Version POST 140 MB → 201 via curl (T-060.1.4)` `- [x] Browser Save @ ~367k → 201 (2026-06-23)` `- [x] >1 MB + production-like GlobalBodyLimit version POST round-trips (cargo xtask db test-it)` |
| **Status** | **working** — T-060 shipped (`b1fd25a`); Save @ ~367k/~142 MB → 201 (browser + curl) |
| **Eden parity** | Eden:FILE-SAVE-001 |
| **Evidence** | `TopCommandStrip.tsx`, `useMissionEditor.ts`, `compile.ts` |

#### TOP-EXPORT-001 — Export JSON download

| Field | Value |
|-------|-------|
| **Domain** | DATA |
| **Goal** | Download mission export envelope |
| **Trigger** | Export button |
| **Preconditions** | — |
| **Procedure** | `compileMission` + `toMissionExport` → blob download |
| **Postconditions** | File saved |
| **Inputs** | Button |
| **Outputs** | JSON file |
| **Edge cases** | Some envelope fields defaulted empty |
| **Acceptance** | `- [ ] Export downloads JSON` |
| **Eden parity** | Eden:FILE-EXPORT-001 |
| **Status** | working |
| **Ticket** | T-060 |
| **Evidence** | `TopCommandStrip.tsx`, `exportSchema.ts` |

#### TOP-SETTINGS-001 — Mission Settings gear

| Field | Value |
|-------|-------|
| **Domain** | ATTR |
| **Goal** | Global environment dialog |
| **Trigger** | Gear icon |
| **Preconditions** | — |
| **Procedure** | Open `MissionSettingsDialog` |
| **Postconditions** | Dialog visible |
| **Inputs** | Button |
| **Outputs** | UI |
| **Edge cases** | Terrain read-only |
| **Acceptance** | `- [ ] Settings dialog opens` |
| **Eden parity** | Eden:ENV-DIALOG-001 |
| **Status** | partial |
| **Evidence** | `MissionSettingsDialog.tsx`, `TopCommandStrip.tsx` |

---

| TBD-SAVE-001 | Semver Save Version (immutable) | working |
| TBD-EXPORT-001 | `editor` block lossless reload | working |
#### ENV-SETTINGS-002 — Thermals + view distance

| Field | Value |
|-------|-------|
| **Domain** | ENV |
| **Goal** | Global environment beyond time/weather strip |
| **Procedure** | `MissionSettingsDialog`: view distance, thermals toggle |
| **Status** | working |
| **Evidence** | `MissionSettingsDialog.tsx` L57–74 |

