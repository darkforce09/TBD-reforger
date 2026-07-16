# T-159.22 — Editor Layers outliner + Asset palette DnD — verify log

Fills the two Eden dock placeholders the T-159.21 scaffold left: a live **Editor Layers** outliner off
the hosted `MissionDocCore`, and a **Factions** palette (live `GET /api/v1/registry`) whose leaves
drag onto the map to place a slot. Also clears both items T-159.21 deferred: the **wheel over a dock**
no longer zooms the map, and the **CUR** toolbelt math is now a committed gate.

- **Worktree** `.ai/artifacts/worktrees/TBD-T-159/` · **branch** `t-159-leptos-ui` · **base** `54c8a4bd` (T-159.21 code baseline `f02fed5a`).
- **Executor:** claude-code.
- **Result: PASS.** `cargo check` wasm32 clean (zero warnings) + native unchanged (the same **8** pre-existing dead-code warnings); `cargo test` **42/42** (+10 new); clippy adds **zero** new lints (per-file split byte-identical to .21); `trunk build --release` green; **2 new gates PASS**; **all 9 prior editor smokes PASS**.
- **⚠ Found a pre-existing defect in the T-159.21 undo lane** — see **§Pre-existing defect** below. Not introduced here, not fixed here (out of scope), **proven** against the untouched `54c8a4bd` build.

## What shipped

- **`apps/website-leptos/src/asset_catalog.rs`** (new, ungated, natively tested) — `RegistryItem` →
  palette tree. A **verbatim port of the recovered `buildCatalogTree.ts`** (T-068.3; deleted at
  `c4ccb9c3` when T-152 swapped React's palette onto the T-153 Faction Library). Recovered from git
  because that builder — not the Faction Library — is what spec O2 names and what the golden matches.
  Its four rules are ported and unit-tested: `kind == "character"` only; folders = the category path
  **minus its last segment** (the leaf is `display_name`, so `NATO/US_Army/Rifleman` →
  `NATO` > `US_Army` > "US Rifleman", with **no** `Rifleman` folder); folder id = the accumulated
  prefix, `defaultExpanded` at depth 0 only; leaf id = the full `resource_name`. Plus `CatalogState`
  (Loading / Failed / Ready).
- **`apps/website-leptos/src/outliner.rs`** (new, ungated, natively tested) — `build_outliner`, the
  `EditorLayersSection.tsx:51-81` `buildTree` port + the **"Unfiled" pseudo-root** (see §Divergences).
  Owns plain `LayerRow`/`SlotRow` rather than importing `SlotSoa`, because `map-engine-core` is a
  **wasm32-only** dependency — that keeps the module ungated and its 7 unit tests on the native shell.
- **`apps/website-leptos/src/editor_ops.rs`** (new, ~250 L, wasm-only) — the dock commands, peer of
  `mission_history`/`mission_commands` and the same `thread_local` ctx shape for the same reason (the
  `!Send` `Rc` handles can't cross into the native view shell): `select_slot`, `set_active_layer`,
  `begin_place`/`has_pending`/`cancel_pending`/`place_at`, `refresh_docks`, plus `layer_rows` /
  `slot_rows` / `mint_id` / `ensure_layer`.
- **`apps/website-leptos/src/dto.rs`** (+45) — `RegistryItem` typed (was `data: Vec<Value>` with an
  "Items typed at T-159.22" marker), so `registry_envelope()` now proves the **row field-set**, not
  just the envelope.
- **`apps/website-leptos/src/eden_chrome.rs`** (+~190/−12) — `DockLeft` (live outliner) / `DockRight`
  (Factions palette) replace the placeholders; recursive `outliner_rows` / `palette_rows`; CUR spans
  gain `title="Cursor X"` / `"Cursor Y"`.
- **`apps/website-leptos/src/mission_editor.rs`** (+~70) — 4 dock signals; the `/registry` fetch;
  `editor_ops::set_ctx`; the place branch atop `pointerup` + `cancel_pending` on `pointercancel`; the
  wheel target guard; `data-eden-chrome` on the chrome host; dock wiring.
- **`apps/website-leptos/src/mission_history.rs`** (+6) — one line in `refresh_signals` →
  `editor_ops::refresh_docks()`, so the dock mirrors refresh from the single existing mutation point.
- **`apps/website-leptos/src/main.rs`** (+10) — three module declarations.
- **Gates** — `smoke_outliner_palette_editor.mjs` (new, 5309/9369) · `smoke_cur_editor.mjs` (new,
  5310/9370) · `smoke_undo_editor.mjs` (A6 expected strings updated — see §Gate change).

## Design notes

### The handoff's seed assumption is wrong, and it drives the whole slice
The handoff states "Seed creates 8 slots under a default layer". It does not: `new_seeded_doc`
(`mission_doc.rs:39`) calls `seed_random` (`store.rs:348-371`), which writes **only** the `slots` map
— no layers, no squads, no `assetId`. Each seed slot gets `role: "Rifleman"` and a **dangling**
`squadId: "sq"` with no squad in the map. All 8 materialize with `layer_idx = NONE_IDX`.

React's `buildTree` renders only layers and their `entityIds`, so a literal port would have shown an
**empty dock while OBJ read 8**. And a boot-time `add_editor_layer` is impossible:
`smoke_save_export_editor:77` asserts `editor.editorLayers.length === 0`. Hence the Unfiled root + the
lazily-minted layer (§Divergences). This is also why `smoke_save_export_editor` stays green: the seed
is untouched and nothing exists until an operator places.

### Placement mints no squad
React's `addSlot` runs `ensureDefaultSquad` + `ensureDefaultLayer`; only the layer half is ported.
`add_slot`'s squad/layer appends are guarded (`store.rs:298`, doc comment `:266`), so `squad_id: ""`
files the slot **without** creating a squad — required, because the same gate asserts
`squads.length === 0`. Not a hack: the seed's own slots already carry a dangling `squadId: "sq"`. With
an empty squads map the field is inert (compile derives ORBAT from squads; the ORBAT tree is O7).

### Pointer-drag, not HTML5 DnD (O3 asks the choice be documented)
Every existing smoke drives trusted `Input.dispatchMouseEvent`, which synthesizes real pointer events
into the Leptos handlers; HTML5 DnD would need `Input.setInterceptDrags` + `dispatchDragEvent` and is
fragile headless. The chrome host stops `pointerdown` (`mission_editor.rs:772`), so a palette press
cannot also open a map gesture — `left`/`pan_px` stay `None` and the drag's moves only drive CUR. It
does **not** stop `pointerup`, so a release over a dock bubbles to the container too: the place branch
insets the release px by the same `eden_chrome` consts `select_tool::farthest_empty_px` uses, so
"not under chrome" means one thing editor-wide. World position comes from the same `frozen_camera`
unproject the pick and CUR use, so a slot lands exactly where CUR said it would.

### Wheel over dock
The listener is capture-phase on the container **by design** (it is what lets `prevent_default` beat a
child), so the chrome can't opt out by listener order — the handler reads `ev.target()` and returns
**before** `prevent_default`, leaving the event native so `overflow-y-auto` scrolls. No new web-sys
feature needed (`Element`/`EventTarget` were already enabled). Marked via `data-eden-chrome`, not a
class: the class list is a styling contract a Tailwind edit could silently change under the guard.

### DOM handles
Rows/leaves are `<button aria-label=…>` — real, focusable, activatable, the `aria-label="Undo"`
precedent. The CUR cells use **`title`**, not `aria-label`: they are roleless `<span>`s, where an
`aria-label` is ignored by AT and would be a fake a11y name; `title` is a real tooltip and matches the
toolbelt's existing `title="Cursor"` idiom. No test-only attributes were added.

### `RegistryItem` — full field set, `skip_serializing_if`
The golden's rows carry exactly 9 fields (verified across all 21), but the TS oracle
(`types/models/registry.ts:45-72`) declares 9 further optionals the backend `omitempty`s away.
`assert_golden` does `from_str → to_string → canon`-compare, so an optional serialized as `null` would
add a key the golden lacks and fail. All optionals are therefore
`#[serde(default, skip_serializing_if = "Option::is_none")]` (the `LinkStatus`/`MissionDetail`
precedent, `dto.rs:158`). Two traps handled: `abstract` is a reserved Rust word (`r#abstract` +
explicit rename), and `kind` stays a **`String`, not an enum** — the vocabulary is versioned and
growing ("T-068.10.2 v3"), and an enum would hard-fail the day the backend adds a kind.

## Gate results (all exit 0)

**New — `editor-outliner-palette-smoke`** (ports 5309/9369, default WebGPU/Dawn): `pass:true`,
**15/15** checks, `panics:[]`. The registry golden is served to the app's **real**
`api_get::<RegistryResponse>("/registry")` via CDP `Fetch.fulfillRequest` (the `gate_r_auth.mjs`
pattern) — zero test-only surface, and no dependence on a live DB.

| Assertion | Evidence |
|---|---|
| P1 palette | `Factions` / `NATO` / `US_Army` mounted; **8** `US …` leaves; `registryHits: 2` |
| O1 outliner | `Unfiled (8)` + `slot_count() === 8` |
| O2 select | click row 0 → `ids() === ["s0"]` — exact, because Unfiled is id-sorted |
| D1 place | `slot_count` **8 → 9**; OBJ text **8 → 9**; digest changed; `edit_persist_count` **0 → 1** |
| D2 position | placed `n0` at `xBits 1170571264` / `yBits 1170325504` — **bit-exact**, no tolerance |
| D2 payload | `role === "US Rifleman"` (from the palette leaf) |
| D3 layer | digest layer column `=== "layer-1"`; outliner shows `Layer 1` **and** still `Unfiled (8)` |
| W1 wheel | over the left dock `z` stays **-2**; over the canvas `z` → **-1.52** |

**D2 is a derivation, not a fixture.** At the default cam (`tx/ty 6400`, `z -2` ⇒ `scale = 2^-2` =
0.25 px/m ⇒ 1 px = 4 m), releasing at (700, 500) unprojects to
`x = 6400 + (700−720)·4 = 6320`, `y = 6400 + (450−500)·4 = 6200` (`flipY:false` is north-up).
`slots_digest` emits `f32::to_bits`, and 6320/6200 lie in `[4096, 8192)` where the f32 ULP is
`2^-11 ≈ 0.000488` — so the ~1e-9 matrix-inverse error in `unproject_xy` is absorbed by the f64→f32
truncation and the bits land on **exactly** `Math.fround(6320)` / `Math.fround(6200)`. Hence an exact
`===` on the bit patterns rather than an epsilon.

**New — `editor-cur-smoke`** (ports 5310/9370, default WebGPU/Dawn): `pass:true`, **4/4**,
`panics:[]`. Commits the read-out T-159.21 could only check by hand (its log: "not a committed gate").

| Assertion | Evidence |
|---|---|
| C0 camera | `__editorCam()` = `tx 6400 / ty 6400 / z -2` — the gate proves its own premise first |
| C1 centre | (720, 450) → `X 6400.000  Y 6400.000` |
| C2 offset | (600, 300) → `X 5920.000  Y 7000.000` |
| C3 off-map | at boot, before any pointer move → `—` / `—` |

The C1/C2 numbers are **derived from source** (`OrthoCamera::new`'s `scale = zoom.exp2()`,
`ortho.rs:127`; `INITIAL_TARGET`/`INITIAL_ZOOM`, `mission_editor.rs:83-84`), and `clamp_target`
provably cannot bite: at 1440×900 the half-extents are 2880 m / 1800 m and 6400 sits inside both
`[2880, 9920]` and `[1800, 11000]`. They independently reproduce the .21 manual table — agreement is
the point, not the source. C2 is the one that catches a sign flip (a flipY regression reads
`Y 5800.000`). **C3 is asserted at boot** (`cursor` is `RwSignal::new(None)`, `mission_editor.rs:53`)
rather than by driving a pointer-leave: the container fills the viewport, so a real leave isn't
something CDP does reliably — the boot state renders through the same `fmt_coord(None)` arm the
shipped `pointerleave → None` handler feeds. The gate must run **before** any `probe()` (which
re-centres the camera as a test hook); it never calls one.

**Regression — 9 prior editor smokes (exit 0, `pass:true`):** `editor-smoke` · `editor-selfcheck`
(`?force=webgl`) · `editor-pan-smoke` · `editor-doc-smoke` · `editor-persist-smoke` ·
`editor-select-smoke` · `editor-marquee-drag-smoke` (`?force=webgl`) · `editor-save-export-smoke` ·
`editor-undo-smoke` (`?force=webgl`).

### Gate change — `smoke_undo_editor` A6
`a6_docksMounted` asserted the .21 **scaffold's placeholder headings** (`'ORBAT / Layers'` /
`'Assets'`) — text that slice's own header called out as what the outliner + palette would replace. It
was the only check that went red (verified: 8/9 smokes green before the edit, this one failing on
`a6_docksMounted` alone, every behavioural gate untouched). The assertion's **intent — both docks are
mounted — is unchanged**; only the pinned headings moved to the real ones (`ORBAT` + `Editor Layers`;
`Factions`, matching React's `AssetBrowser` `<h2>`). No behavioural assertion was weakened.

## ⚠ Pre-existing defect found (NOT introduced here, NOT fixed here)

**Undo granularity collapses: consecutive local transactions merge into ONE undo step.**

`store.rs:75-78` states "capture_timeout_millis = 0 → every transaction is its own undo step … no
same-millisecond merge", and `mission_history.rs:5` repeats it ("one drag-move = one undo").
**Observed behaviour contradicts both.** Repro (no T-159.22 code on the path — `probe_move` + drag +
Ctrl+Z are all .19/.21):

```
after 2 drags: canUndo=true, d1!=d0, d2!=d1
undo1 -> == d1 (only the 2nd drag reverted)? false | == d0 (BOTH reverted)? true | canUndo=false
```

One Ctrl+Z reverts **both** drags and empties the stack. **Proven pre-existing**: `git stash` →
rebuild the untouched `54c8a4bd` → identical output. `smoke_undo_editor` cannot catch it because it
performs exactly **one** mutation, so it never exercises step *boundaries*.

Same shape via the new place path: two places → one undo removes **both** slots (`9`→`10`→`8`), with
the lazily-minted layer as a separate item removed by a second undo. So items appear to group by root
map (`slots` vs `editorLayers`), not per transaction.

**Mechanism unresolved.** Reading yrs 0.27.2 `undo.rs:266-270`, `extend` requires
`inner.last_change > 0`, and the core's `ZeroClock` pins `now`/`last_change` to 0 — so `0 > 0` is
false and every txn *should* push a new `StackItem`. Static reading predicts the documented behaviour;
observation disagrees, so the mechanism needs a targeted dig (a `map-engine-core` unit test driving
two `LOCAL` txns then `undo()` would isolate it in-crate, away from the browser).

**Left untouched deliberately**: out of scope (a `map-engine-core` fix ripples into React's
`map-engine-wasm` consumer + the core's own parity tests), and the prompt forbids fixing pre-existing
drift outside this slice's files. **Recommend a dedicated slice** — user-visible (an operator's Ctrl+Z
throws away more work than they made) and it invalidates a documented core invariant. Note the
consequence for T-159.22: **the first place is one undo step, not the two this slice's plan predicted**
— it removes the slot and the layer together, which happens to match React's single-`transact`
semantics, but by accident rather than by the documented mechanism.

## Build

- `cargo check -p website-leptos --target wasm32-unknown-unknown` — clean, **zero** warnings.
- `cargo check -p website-leptos` (native shell) — **8** pre-existing dead-code warnings, the same 8
  the .21 log records (nav/router/layout); this slice adds **none**. The palette leaf's native
  `let _ = &payload;` is what keeps the wasm-gated `begin_place` from leaving an unused capture (the
  `announcements.rs` `let _ = store;` idiom).
- `cargo test -p website-leptos` — **42/42** (was 32): +3 `asset_catalog` (the golden yields
  NATO > US_Army > 8 leaves in `sort_order` order; leaf id/payload = the ResourceName; the 13 `gear_*`
  rows are excluded), +7 `outliner` (seed boot state id-sorted under Unfiled; filed slot leaves
  Unfiled; no Unfiled root when all filed; child folders precede slots; `'Unit'` fallback; dangling
  `entityIds` skipped; `parentId` cycle terminates).
- `cargo clippy -p website-leptos --target wasm32-unknown-unknown` — **11** lints, per-file split
  **byte-identical** to T-159.20/.21 (`mission_editor` 5 / `leaderboards` 3 / `event_manager` 2 /
  `personnel` 1). The four new/changed files contribute **zero**. (One `unnecessary operation` in
  `editor_ops` was fixed during the slice: a `{ … };` block guarding `RefMut` drop order became a
  named binding, which drops before `guard` by reverse declaration order anyway.)
- `trunk build --release` — `✅ success`; wasm dist **5,588,587 B** (T-159.21: 5,486,743 → **+101,844**).

## Divergences from React (deliberate, forced)

- **"Unfiled" pseudo-root** — Leptos-only. React cannot have an unfiled slot (its `addSlot` always
  runs `ensureDefaultLayer`); `seed_random` files nothing, and a boot-time layer would break
  `smoke_save_export_editor`. `UNFILED_ID` is **not** a doc id: it renders as an inert header, is
  never the active layer, and is never a drop target.
- **Unfiled children are id-sorted** — `materialize()` row order is arbitrary (yrs map iteration);
  React's folders order by `entityIds` insertion, which unfiled slots have no analogue for. Sorting is
  what makes the tree stable for the operator and `ids() === ["s0"]` exact for the gate.
- **Placed slots carry `squadId: ""`** where seed slots carry a dangling `"sq"`; both inert with an
  empty squads map.
- **Default layer is lazy** (first place, LOCAL origin ⇒ undoable), mirroring React's
  `ensureDefaultLayer`-inside-`addSlot`.
- **Both trees render fully expanded** — React's `TreeView` seeds an expanded set from
  `defaultExpanded` and lets rows collapse. At seed scale the outliner is two shallow folders and the
  palette is NATO > US_Army > 8 leaves, so everything is visible either way; expand/collapse is
  deferred with the rest of the `TreeView` port. `CatalogNode::default_expanded` is carried because it
  is part of the ported `buildCatalogTree` contract, and is consumed when collapse lands.

## Non-goals held (spec O6/O7)

Attributes modal · Arsenal · Faction Manager · ORBAT stays a stub header · reparent DnD · rename /
delete · full VirtualOutliner @ 367k (no threshold gate) · Mission Settings · cluster · palette search
box · Vehicles/Markers/Objectives tabs · `abstract` / `variant_of` picker-hiding (T-068.10.5 — the
recovered oracle predates those fields and the golden has none).

## Known / deferred

- **The undo-granularity defect above** — the significant one; recommend a dedicated slice.
- **No palette retry button** on a failed `/registry` (React's `AssetBrowser` has one) — the dock
  renders "Could not load the catalog."
- **Row highlight is selection-only** — the map's tint lane is still a no-op until an atlas uploads
  (unchanged from .19), so the outliner row is currently the only visual selection feedback.
- **CUR keeps updating while the pointer is over a dock** (map coords under chrome). Pre-existing:
  `pointerleave` doesn't fire for a descendant, and the panels are descendants of the container — so
  the .21 comment claiming it fires "when the pointer enters a chrome panel" is inaccurate. Cosmetic;
  untouched here.
- **Chrome has no DOM-parity gate** (unchanged from .21): the editor route isn't V-gated, so the docks
  are verified structurally + behaviourally, not by a React DOM diff.
- The strip title still shows the mission **id**; binding `meta.title` waits on the settings/hydrate lane.

## Next

Ready for Cursor → **T-159.23** (Attributes / ORBAT depth — confirm with Cursor). Return: SHA + tag
**T-159.22** + this log. Please also triage the **pre-existing undo-granularity defect** into its own
slice.
