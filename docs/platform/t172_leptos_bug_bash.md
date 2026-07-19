# T-172 — Leptos SPA + Mission Creator bug bash

**Status:** SHIPPED @ tag **T-172** / `e08884f4` · residual perf → **T-173** · **Branch:** `main`  
**Scope:** `apps/website/frontend/**` (+ `crates/map-engine-*` / `tools/tbd-tools` only if required for render/glyph/forest fixes). **Not** `apps/mod/`. **Not** T-170 prod flip.  
**Operator evidence:** [`.ai/artifacts/t172_operator_screens/`](../../.ai/artifacts/t172_operator_screens/) + babble 2026-07-18.

**No silent deferrals.** Soft “later / optional / fold forward / separate ticket” is forbidden unless the operator explicitly says `defer X` / `skip X`. Unknown bugs discovered in Phase 0 are **in scope** for this ticket — add them to the inventory and fix them here.

## Why

Post T-159→T-169 the Leptos SPA boots and the editor mounts a full map, but everyday shell + Mission Creator use is broken or regresses the React editor hard: dead click handlers, stuck nav highlight, opaque forest blobs, invisible placed slots, SVG Arsenal instead of the shipped T-154 3D doll, missing Z readout, tree chrome, and site-wide scroll/boot lag.

## Goal

One bug-bash pass so local `make api` + `make leptos` feels usable: shell navigation/selection works, lists scroll smoothly, Mission Creator shows slots + readable forests + CUR X/Y/Z, Arsenal matches the 3D doll reference, editor chrome catches the obvious React parity holes from the operator reference screenshot.

**Operator matrix is the seed, not the ceiling.** Claude Code **must hunt for more bugs** (Phase 0 + during fix), write them into the inventory, and fix them in T-172. Do not only patch the listed A/B rows and call it done.

## Agent split (HARD)

| Who | Owns |
|-----|------|
| **Claude Code** | Inventory + all code fixes + smokes/gates + verify log + tag **T-172** |
| **Cursor** | This spec, handoff, registry; post-ship doc sync if Claude returns a Cursor list |

## Operator-seeded acceptance matrix (MUST FIX)

Screens: `01` shell · `02` MC forests/no-Z · `03` Arsenal 2D current · `04` Arsenal 3D target · `05` React MC layout reference.

### A — Platform shell

| ID | Bug | Evidence / lead |
|----|-----|-----------------|
| A1 | Top-right user menu click does nothing | `layout.rs` avatar `<button>` has no open state / menu |
| A2 | Sidebar active highlight stuck (e.g. Mission Library stays lit on Dashboard) | `SidebarNav` / breadcrumb read `pathname` once — not reactive on SPA nav |
| A3 | Site-wide scroll lag (Dashboard, Mission Library, SOPs, Vehicles, Modpacks, …) | Operator; Class-R root cause + fix |
| A4 | SOPs & Manuals: hover highlights but click does not change the selected article | `wiki.rs` gate-era static selection |
| A5 | Vehicle Database: same dead selection | `vehicles.rs` static `selected` |
| A6 | Modpacks: same dead selection | `modpacks.rs` static `selected` |
| A7 | Mission Library dossier / overview sheet: laggy open, missing loading bar, weak/missing open animation | Operator vs React sheet |
| A8 | Breadcrumb / chrome stale after SPA nav | Same one-shot pathname pattern as A2 |
| A9 | Narrow viewport: hamburger / sidebar pop-out button does nothing — sidebar stays hidden | `SidebarMobileToggle` is a dead `<button>` (comment: “interactive state for a later slice”); `Sidebar` is `hidden lg:flex` with no slide-over |

### B — Mission Creator map / chrome

| ID | Bug | Evidence / lead |
|----|-----|-----------------|
| B1 | Editor incredibly laggy (boot + pan/zoom + UI) | Operator; T-166 polish note |
| B2 | CUR shows X/Y only — no Z height | `eden_chrome.rs` “X/Y only” comment; need DEM-fed Z like React T-091.2 |
| B3 | Forest = opaque solid green blobs (not translucent mass / readable overlay) | Screen `02`; tune forest fill + tree/glyph story toward `05` |
| B4 | Placed slots invisible on map (selectable/highlightable but no glyph) | Screen `02` OBJ/SEL prove presence; icon/atlas/GPU lane |
| B5 | Drag-drop place: slot not visible after drop (same as B4 if root cause shared) | Operator |
| B6 | Left/right trees: folders permanently open; cannot collapse | Outliner / asset tree expand state |
| B7 | Trees missing open-folder icon + hierarchy guide lines | Operator vs React tree |
| B8 | Selection highlight felt missing / delayed (lag) | Operator clarified it works but laggy — fix with B1 |
| B9 | Missing editor chrome vs React reference (`05`): menus, time/weather, Vehicles/Markers tabs, search, toolbelt extras (e.g. SZ / status), stub buttons that existed before | Class-R chrome gap table → implement every gap listed (no “polish later”) |
| B10 | Arsenal is 2D SVG paper-doll; must be T-154-class **3D** doll (rotatable mannequin + callouts) | Screens `03` vs `04`; React shipped DollEngine @ T-154 — port into Leptos Arsenal |

### C — Bug hunt (mandatory — not optional polish)

The operator list is incomplete on purpose. Claude Code **owns finding the rest**.

| ID | Rule |
|----|------|
| C1 | Phase 0 Class-R walk of **every** shell route in `nav.rs` + Mission Creator chrome vs screen `05` + React-era page docs under `docs/website/frontend/` |
| C2 | Every finding goes into `.ai/artifacts/t172_inventory.md` with id, severity (`P0` broken / `P1` wrong / `P2` lag-or-missing-chrome), repro, file:line lead |
| C3 | **All inventory rows are fixed in T-172** unless operator explicitly defers that row by name |
| C4 | Hunt continues **while fixing** — new bugs found mid-pass get new inventory rows and are fixed before tag |
| C5 | Verify log must include a **Found-by-hunt** section (rows Claude discovered that were not in A/B) — empty Found-by-hunt = FAIL (you did not hunt) |

**Hunt methods (all required in Phase 0):**

1. **Code grep for stubs** — `follow-up`, `later slice`, `gate scope`, `stub`, `TODO`, `unimplemented!`, dead `<button>` with no `on:click`, `pathname.get()` once (non-reactive SPA), `selected = &…[0]` static lists  
2. **Route click-through** — with `make api` + `make leptos` + dev-login, open every nav item; click primary controls; note dead/wrong/laggy  
3. **Narrow + wide viewport** — hamburger drawer, overlays, horizontal overflow  
4. **MC smoke path** — place unit, select, Attributes tabs, Arsenal, undo, save dialog chrome, left/right trees expand/collapse  
5. **Parity vs screen `05` + page docs** — controls present in React reference / docs but missing or stubbed in Leptos  
6. **Console / network** — browser errors, 401/404 storms, failed `/map-assets` or `/registry` that break UX  

**Out of hunt (ASK only if blocking):** true new product features not implied by React chrome (e.g. full markers placement program = T-069). Dead/stub chrome that *looks* clickable **is** in hunt.

## Phases (all must-do)

### Phase 0 — Inventory + bug hunt (do this BEFORE large fixes)

Write `.ai/artifacts/t172_inventory.md`. **Do not skip.** Minimum contents:

1. Operator matrix A/B rows with file:line leads  
2. **Found-by-hunt** table (new ids `H1…Hn`) — must be non-empty after a real hunt  
3. Extra shell regressions (dead buttons, non-reactive state, mock-only pages)  
4. MC chrome gap table vs `05_react_mc_reference_layout.png`  
5. Perf suspects (scroll containers, full-list DOM, wasm/GPU overdraw, forest opacity)  
6. Arsenal 3D port plan (reuse `DollEngine` / map-engine 3D path — **no three.js**)  
7. Severity summary counts (P0/P1/P2)

### Phase 1 — Shell correctness

A1–A9 + inventory shell rows. Reactive route for nav/breadcrumb; real user menu (profile / settings / sign-out per React); list pages select on click; dossier loading UX; mobile/narrow sidebar drawer opens/closes from the hamburger.

### Phase 2 — Perf

A3 + B1 + B8. Fix root causes from inventory (virtualize where React did, kill render storms, forest/GPU cost). Acceptance: operator-usable scroll on Mission Library + Wiki + Vehicles; MC pan at default zoom without multi-second UI freezes.

### Phase 3 — Mission Creator fidelity

B2–B7, B9–B10 + inventory MC rows. Visible slot glyphs; translucent forest mass (not highlighter green); CUR X/Y/**Z**; collapsible trees with open/closed folder icons + guide lines; 3D Arsenal; chrome gaps closed.

### Phase 4 — Verify + ship

Gates green; smokes for new behaviors; verify log; tag **T-172**.

## Locked decisions

1. **Local-dev focus** — prove on `make api` + `make leptos` (`:8080` / `:3000`). T-170 prod flip stays separate (`human`).
2. **3D Arsenal in scope** — operator rejected SVG-only; restore T-154 behavior in Leptos (wasm/`DollEngine`, not three.js).
3. **React screenshot `05` is chrome SoT** for “what’s missing” — not a blank-check full Eden feature dump (markers program stays T-069; vehicles placeable stays T-070) **but** every chrome control visible in `05` that Leptos stubbed/omitted must return or be ASK’d if truly impossible.
4. **Mod OFF LIMITS.**
5. **No silent deferrals** of A/B/C/H rows.  
6. **Hunt is mandatory** — shipping only the operator A/B list without a Found-by-hunt table is incomplete.

## Verify

```bash
make leptos-gates   # exit 0
make ci-local       # exit 0
```

Plus ticket-specific smokes / manual checks recorded in `.ai/artifacts/t172_verify_log.md` covering A1–A2, A4–A6, A9, B2–B4, B6, B10, **and every H-row** from the inventory. Empty Found-by-hunt = incomplete.

## Claude Code prompt — T-172 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first.

Implement **T-172** — Leptos SPA + Mission Creator bug bash
(operator matrix + mandatory bug hunt + fix everything found).

═══ PREFLIGHT ═══
  git pull --ff-only
  make db-up
  ./scripts/ticket brief T-172

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t172_claude_code_handoff.md
  2. docs/platform/t172_leptos_bug_bash.md  (§C Bug hunt is HARD)
  3. .ai/artifacts/t172_operator_screens/ (01–05 + README)
  4. apps/website/frontend/src/layout.rs (A1/A2/A9 leads)
  5. apps/website/frontend/src/{wiki,vehicles,modpacks}.rs (dead selection)
  6. apps/website/frontend/src/{eden_chrome,arsenal,mission_editor}.rs
  7. .cursor/rules/no-silent-deferrals.mdc

═══ PROBLEM ═══
  Operator listed many Leptos shell + MC bugs, and said there are more they
  forgot. Your job is BOTH: fix the seeded A/B matrix AND actively hunt for
  more bugs (code stubs, click-through every nav route, narrow viewport, MC
  smoke, React screen-05 parity), inventory them as H1…Hn, and fix them all
  in this ticket.

═══ SHIPPED (do not reopen) ═══
  T-166 full map host · T-167 SVG Smart Arsenal (superseded here by 3D restore) ·
  T-168 ORBAT tree · T-169 VirtualOutliner · T-171 website nest · T-154 DollEngine
  (React-era — reinstate in Leptos, do not redesign from scratch).

═══ LOCKED ═══
  - Fix A1–A9, B1–B10 + every inventory H-row + mid-pass finds
  - Phase 0 Found-by-hunt table MUST be non-empty (empty = FAIL)
  - Hunt methods in spec §C all required before large fixes
  - 3D Arsenal via existing wgpu doll path — no three.js
  - Local prove: make api + make leptos; not T-170
  - apps/mod/** OFF LIMITS
  - No inventing Out-of-scope / fold-forward
  - Markers/vehicles placement programs stay T-069/T-070; chrome tabs in
    screen 05 must exist (stub OK only if React also stubbed that control)

═══ DO ═══
  1. Phase 0: HUNT then write .ai/artifacts/t172_inventory.md
     (A/B leads + Found-by-hunt H1…Hn + chrome gaps + perf suspects)
  2. Phase 1 shell (A1–A9 + shell H-rows)
  3. Phase 2 perf (A3/B1/B8 + perf H-rows)
  4. Phase 3 MC (B2–B10 + MC H-rows) including 3D Arsenal
  5. Mid-pass: new bugs → new H-rows → fix before tag
  6. Phase 4: make leptos-gates + make ci-local; t172_verify_log.md
     with Found-by-hunt section
  7. Commit on main T-172: · tag T-172 · push
  8. Return Cursor list only for doc/registry prose

═══ DO NOT ═══
  - Edit docs/**, registry.json, CLAUDE ticket-sync markers
  - Touch apps/mod/**
  - Ship after only fixing the operator A/B list (hunt is mandatory)
  - Defer A/B/C/H rows without operator "defer X"
  - Ship SVG paper-doll as “good enough” Arsenal
  - Leave slots selectable-but-invisible
  - Leave forest as opaque highlighter green

═══ VERIFY (all exit 0) ═══
  make leptos-gates
  make ci-local
  t172_verify_log.md: A1,A2,A4–A6,A9,B2–B4,B6,B10 + every H-row PASS
  Found-by-hunt section non-empty

═══ RETURN ═══
  - tag T-172 @ sha
  - inventory + verify log paths
  - A/B + H matrix → pass/fail
  - count of bugs found-by-hunt vs operator-seeded
  - ASK blockers (stop — do not invent deferrals)
  - Cursor doc list if any
```
