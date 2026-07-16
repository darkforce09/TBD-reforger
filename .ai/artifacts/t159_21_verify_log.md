# T-159.21 — Eden chrome scaffold + undo/redo — verify log

Ports the React Eden docked shell (Top Command Strip / Bottom Toolbelt / left+right docks) to the
Leptos/Rust-WASM Mission Creator as a **scaffold**, and wires **working undo/redo** onto the hosted
`MissionDocCore` undo stack. The docks are placeholders — the outliner tree and asset palette are
T-159.22 (spec C4/C7).

- **Worktree** `.ai/artifacts/worktrees/TBD-T-159/` · **branch** `t-159-leptos-ui` · **base** `eaea2601` (T-159.20 code baseline `c0e11d54`).
- **Executor:** claude-code.
- **Result: PASS.** `cargo check` wasm32 + native clean; clippy adds **zero** new lints (proved by a stash diff); `trunk build --release` green; new gate `editor-undo-smoke` PASS (12/12 checks); **all 8 prior editor smokes PASS**.

## What shipped

- **`apps/website-leptos/src/eden_chrome.rs`** (new, ~250 L, **not** cfg-gated) — chrome inset consts
  (`STRIP_TOP_PX` 48 / `DOCK_LEFT_PX` 256 / `DOCK_RIGHT_PX` 320 / `TOOLBELT_BAND_PX` 96), the React
  `layout/overlay.ts` recipes ported verbatim (`overlayPanel` / `overlayDocked`) with the `cn(…)` call
  sites pre-merged into literals (the `mortar.rs` idiom), and four components: `TopCommandStrip`
  (title · Undo/Redo · the T-159.20 Save/Export controls **moved in**, not duplicated · disabled
  Settings stub), `DockLeft` / `DockRight` (placeholder text), `BottomToolbelt` (Select active +
  Ruler/LoS disabled stubs, CUR X/Y + OBJ/SEL). `fmt_coord` mirrors React's `fmtCoord`
  (`toFixed(3).padStart(9,' ')`; off-map = 7 spaces + em dash).
- **`apps/website-leptos/src/mission_history.rs`** (new, ~270 L, wasm-only) — the **single** undo/redo
  code path (peer of `mission_commands`): `HistoryCtx` `thread_local` (doc/engine/selection/doc_ver/
  mission_id + 4 HUD signals), `undo`/`redo`, `after_local_edit`, `refresh_hud`, `after_doc_change`
  (the shared post-change tail), `register_editor_history` (`window.__editorHistory`, read-only),
  `register_key_handler` (window `keydown`), `in_editable_field`.
- **`apps/website-leptos/src/mission_editor.rs`** (+~90/−25) — `mission_id` hoisted to the page body
  (the chrome title binds it on both targets); 5 new signals; `doc_ver.clone()` (it was **moved** into
  `register_mission_doc`); history ctx/bridge/keydown registration + `refresh_hud()` at the 6 mutation
  sites; CUR unproject at the top of `onpointermove`; `pointerleave` → CUR `None`; the move commit's
  rebind/persist tail replaced by `after_local_edit()`; the new chrome view over an **unchanged**
  container + full-bleed canvas.
- **`apps/website-leptos/src/select_tool.rs`** (+~20/−5) — `EngineHandle` made `pub`;
  `farthest_empty_px` grid inset to the chrome-free region (see **Chrome/probe contract**).
- **`apps/website-leptos/Cargo.toml`** (+6) — web-sys `"KeyboardEvent"` (absent before; the keydown
  closure would not compile without it).
- **`apps/website-leptos/src/main.rs`** (+8) — `mod eden_chrome;` (ungated) + `#[cfg(wasm32)] mod mission_history;`.
- **`.ai/artifacts/t159_gates/driver/smoke_undo_editor.mjs`** (new, ~180 L) — the gate (ports 5308/9368).

## Design notes

### Undo/redo rides the existing core — no `map-engine-core` change
`MissionDocCore::undo/redo/can_undo/can_redo` already existed (`store.rs:900-919`), so the handoff's
"add a `can_redo` wrapper if missing" was moot. The core's `UndoManager` is built with
`capture_timeout_millis: 0` and `tracked_origins: {LOCAL_ORIGIN}` (`store.rs:79-90`) — one
`move_entities` is exactly one undo step, and the INIT-origin seed/hydrate/IDB-restore are not
undoable. `undo`/`redo` take `&mut self` (every mutator takes `&self`); the `DocHandle` `RefCell`
supplies both, with the `borrow_mut` scoped so it drops before the read borrows in the rebind.

### One path, no test-only surface
The toolbar buttons, the keyboard shortcuts, and the gate all funnel through
`mission_history::undo/redo`. `window.__editorHistory` is **read-only** (`can_undo`/`can_redo`) by
design — the gate drives undo via a real Ctrl+Z and redo via a real button click, so it can't prove a
path the user doesn't take. The buttons' DOM handle is their real `aria-label`, not a test hook.

### The move-commit refactor is provably equivalent
`after_doc_change` rebinds from `selection`, where the T-159.19 inline commit bound `ids`
(`mission_editor.rs:533` pre-change). These are always equal at a Move commit:
`select_tool::compute_move_ids` (`:178-185`) returns `selection.to_vec()` when the dragged slot **is**
selected, and `vec![hit]` otherwise — and in that second case the Pending→Move promotion (`:388-393`)
assigns `*selection.borrow_mut() = ids.clone()`. Both branches ⇒ `selection == ids`. The selection
prune (`retain` over the live id set) is therefore a no-op for a move, and exists for undo-of-an-*add*
(which deletes slots) — the case that would otherwise leave the selection pointing at dead ids.
`doc_ver` now also bumps on a move; it never did before (a strict improvement, and no smoke asserts an
exact `change_version`).

### Chrome/probe contract (the one real hazard, and why it's in this commit)
The chrome **overlays** the canvas; the container class is unchanged. Every `select_tool` probe builds
its camera from `container.get_bounding_client_rect()`, so shrinking the container would silently
invalidate the pan/select/marquee/move gates.

The panels are descendants of the gesture container, so the chrome host stops `pointerdown`
propagation — otherwise clicking Undo would also open an LMB map gesture and deselect. Its corollary:
`farthest_empty_px` (which feeds `probe().empty`, the "guaranteed-empty" click px) scanned a 21×13
grid over the **whole** container, whose corner cell centre is ≈ (34.3, 34.6) — under the `h-12` strip
and `w-64` dock, i.e. un-clickable once the chrome exists. Two shipped gates consume it:
`smoke_select_editor:80-82` (Test 2: click empty → `count === 0`) and
`smoke_marquee_drag_editor:114-115` (selection reset). So the inset **had** to land in this commit.

The change shrinks the search space; it does not weaken the property — the result is still the argmax
over candidates of the min distance to any projected slot. Sufficiency is structural, not incidental:
`pick` hits within `PICK_RADIUS_PX` (4) while grid candidates are ≥41 px apart, so one slot can shadow
at most one candidate and 8 seeds can never shadow all 273. `farthest_empty_px` has exactly **one**
caller (`probe_json:400`), so `probe_move`/`probe_marquee` are untouched.

**Empirically confirmed necessary:** the post-change select smoke reports
`empty = [276.57, 774.92]` — grid cell `ix = 0`, i.e. `256 + (0.5/21)×864 = 276.57`. The *same* cell
without the inset is `(0.5/21)×1440 = 34.3`, squarely under the 256 px left dock. The gate would have
gone red.

## Gate results (all exit 0)

**New — `editor-undo-smoke`** (`?force=webgl`, ports 5308/9368): `pass:true`, **12/12** checks, `panics:[]`.

| Assertion | Evidence |
|---|---|
| A0 seed not undoable | `can_undo/can_redo = false/false` on a cold boot (the INIT-origin seed is not a step) |
| A1 move | digest changed; `can_undo/can_redo` → `true/false` |
| A2 undo (**Ctrl+Z**, trusted `Input.dispatchKeyEvent`) | `d2 === d0` **byte-exact**; pair inverts → `false/true` |
| A3 redo (**Redo button click**, live rect) | `d3 === d1`; pair inverts → `true/false` |
| A4 selection | `ids() = ["s2"]` — undo of a move keeps the seed selected |
| A5 persist | `edit_persist_count` **1 → 2** across the undo |
| A6 chrome | Undo/Redo buttons + both dock placeholders mounted |

Digest equality is real byte equality, not a tolerance: `slots_digest` emits sorted rows of raw
`f32::to_bits`, and `yrs` restores the prior values rather than recomputing them. The moved seed `s2`
goes `x = 1163972963 → 1164628323 → 1163972963` across move/undo, and back to `1164628323` on redo.

**A5 note (a real bug caught in review, not by the gate):** `schedule_edit_persist` bumps
`EDIT_PERSIST_COUNT` unconditionally and the A1 drag calls it, so a baseline read *before* the drag
would make `count > baseline` pass on the drag alone and prove nothing about undo. The baseline is
read **after** the drag; the run shows `afterDrag: 1` — a pre-drag baseline of 0 would indeed have
passed vacuously.

**Regression — 8 prior editor smokes (exit 0, `pass:true`):** `editor-smoke` · `editor-selfcheck`
(`?force=webgl`) · `editor-pan-smoke` · `editor-doc-smoke` · `editor-persist-smoke` ·
`editor-select-smoke` · `editor-marquee-drag-smoke` (`?force=webgl`) · `editor-save-export-smoke`.

**CUR read-out — manual CDP check** (not a committed gate; the toolbelt has no gate of its own yet).
Verified numerically rather than by eye:

| Pointer | Toolbelt |
|---|---|
| off-map | `CUR X — Y —` |
| (720, 450) = container centre | `X 6400.000 Y 6400.000` — exactly the camera target (`__editorCam` `tx/ty = 6400`) |
| (600, 300) | `X 5920.000 Y 7000.000` |

That second row is the arithmetic proof: at `z = -2` the scale is 0.25 px/m (1 px = 4 m), so
dx = −120 px → −480 m → `6400 − 480 = 5920`; dy = −150 px → 600 m and `flipY:false` is north-up
(y increases upward) → `6400 + 600 = 7000`. `OBJ 8` (= `SEED_N`) / `SEL 0`, and the Undo button is
`disabled` at boot (mirrors `can_undo = false`).

## Build

- `cargo check -p website-leptos --target wasm32-unknown-unknown` — clean.
- `cargo check -p website-leptos` (native shell) — compiles; **8** pre-existing dead-code warnings,
  the same 8 the T-159.20 log records (this slice adds none).
- `cargo clippy -p website-leptos --target wasm32-unknown-unknown` — **11** lints, and the per-file
  split is byte-identical to T-159.20's (`mission_editor` 5 / `leaderboards` 3 / `event_manager` 2 /
  `personnel` 1). Proved by a `git stash -u` diff rather than by matching counts: the same 5
  `mission_editor` lints exist on the baseline (module-doc `:17-19` unchanged; the `contains()` and
  `type_complexity` sites merely shift `388→451` / `735→812` as the new code pushes them down). The two
  new files (`eden_chrome.rs`, `mission_history.rs`) are **clippy-clean**.
- `trunk build --release` — `✅ success`; wasm dist `website-leptos_bg.wasm` = **5,486,743 B**
  (T-159.20: 5,423,269 B → **+63,474**).

## Non-goals held (spec C7)

VirtualOutliner · AssetBrowser DnD · Attributes modal · Mission Settings dialog · Arsenal · cluster ·
File/Edit/View menu stubs · the time scrubber / weather select · the Save dialog (size estimate,
progress bar, debug panel — semver stays an inline input) · CUR Z (DEM-fed) · the SEL-mode coordinate
swap · the `SZ` payload estimate.

## Known / deferred

- **Wheel over a dock still zooms the map.** The `wheel` listener is capture-phase on the container
  (`mission_editor.rs:281`), so it fires before a panel can stop it. Pre-existing behaviour, out of
  scope for the scaffold; worth fixing when the docks gain scrollable content (T-159.22).
- **CUR is un-throttled.** React rAF-throttles because its cursor write re-rendered the whole page;
  here it feeds two text nodes through Leptos's fine-grained bindings, and the handler already read the
  bounding rect. Revisit only if a perf gate ever measures it.
- **Chrome has no DOM-parity gate.** The editor route is not V-gated (`gate_v.mjs` is `--path`-driven
  and no verify log targets `/missions/:id/edit`), so the chrome is verified structurally (A6) +
  behaviourally, not by a React DOM diff. It is a scaffold, not a parity port.
- The strip title shows the mission **id**; binding `meta.title` waits on the settings/hydrate lane.

## Next

Ready for Cursor → **T-159.22** (outliner / asset palette). Return: SHA + tag **T-159.21** + this log.
