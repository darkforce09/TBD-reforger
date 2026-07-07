# T-145 Phase 3.2 write-swap — kickoff (fresh session)

Standalone brief to continue the **doc-core cutover to `yrs`**. Goal: make the wasm `yrs` doc
**authoritative** and remove `yjs` + `y-indexeddb`. Read this + memory `t145-wasm-port.md` +
`[[wasm-react-lifecycle]]` first. Full plan: `~/.claude/plans/done-everything-s-committed-and-wondrous-fairy.md`.

## Where things stand (branch `t-145-rust-rewrite`, tree clean)

The read side + the mutator port (batches 1–2) are done, committed, green. Gates: **cargo
`-p map-engine-core --all-features` 54/54**, **`make wasm-ci` clean** (clippy `--all-features -D
warnings`), **frontend `npm test` 308/308**, `npm run build`+`lint`+`format:check` clean.

| commit | what |
|---|---|
| `a7fdd44c`…`60564815` | Phase 3.0 spike — yrs `MissionDoc` (SoA + apply/encode/undo), all 6 criteria proven (operator: 90 fps @1M + IDB round-trip) |
| `53f2349a` `60c191be` `a204deb2` | 3.1 — cluster + pick indices → Rust in the app; **supercluster off runtime** |
| `ae21ed86` `e2c347c0` | 3.2 Stage 1 — live **DEV shadow** yrs doc in `useMissionDoc`, synced from `md.doc.on('update')`, gated by `checkDocShadowParity` (`state/docShadow.ts`) |
| `89ce6042` | 3.2.2 — `small_maps_json` + whole-snapshot gate |
| `dd2233a6` | 3.2.3 — `slots_json` + `snapshotFromShadow(shadow): MapSnapshot` (SoA f32 = render-only; exact readers use JSON), proven `.toEqual(docToSnapshot)` |
| `184c1163` | batch 1 — `update_slot`/`update_slot_position`/`move_entities`/`remove_slots` (byte-parity) |
| `84071eaf` | batch 2 — `add_editor_layer`/`rename_editor_layer`/`reparent_editor_layer`(cycle guard)/`move_slot_to_layer` (byte-parity) |

**Rust `MissionDoc` write API so far** (`crates/map-engine-core/src/doc/store.rs`): `apply_update`,
`encode_state`, `materialize` (SoA), `small_maps_json`, `slots_json`, `undo`/`redo`/`can_*`; mutators
`add_slot` **(spike: SIMPLIFIED — slot-only, no squad/layer wiring, no index/tag/assetId/loadoutId)**,
`set_slot_position`, `remove_slot`, `seed_random`, `update_slot`, `update_slot_position`,
`move_entities`, `remove_slots`, `add_editor_layer`, `rename_editor_layer`, `reparent_editor_layer`,
`move_slot_to_layer`. Root map handles held: `slots`, `squads`, `editor_layers` (undo-scoped). **No
`factions` handle yet** — add it for `add_faction`.

## Strategy (locked)

**Full Rust mutators; `ydoc.ts` becomes thin wrappers; JS mints the ids.** The 26 `ydoc.ts` mutators
mint ids via `crypto.randomUUID` and maintain cross-refs. Port each mutator's LOGIC to Rust (one wasm
call, fast for bulk); keep id minting + `ensureDefault*` orchestration in JS (crypto stays JS, avoids
wasm entropy). The mirror model keeps `yjs` authoritative until the flip — batches are **isolated,
reversible, test-only** (no app change).

## The mutator-port pattern (repeat per mutator)

1. **Rust** (`store.rs`): add `pub fn <name>(&self, id: &str, …) { let mut txn = self.doc.transact_mut(); … }`
   mirroring the `ydoc.ts` twin. Reuse `position_any`, `read_position`, `read_str`, `retain_ids`,
   `is_layer_descendant`. Borrow rule: `self.map.get(&txn, id)` returns an owned `Out` (ends the immut
   borrow) → then `x.insert(&mut txn, …)`. Collect ids before mutating while iterating.
2. **wasm shim** (`crates/map-engine-wasm/src/lib.rs`, `impl MissionDoc`): `pub fn <name>(&self, …) {
   self.inner.<name>(…) }`. `Option<String>`→nullable string, `Vec<String>`→`string[]`, `Option<f64>`→
   `number|undefined`. `#[allow(clippy::too_many_arguments)]` if >7 params (self counts).
3. **`make wasm-ci`** (compile+clippy) → **`make wasm`** (rebuild the gitignored pkg).
4. **Differential-parity test** (`features/_wasm/mutatorParity.test.ts`): build a base via the REAL
   `ydoc.ts` (Yjs) mutators → `baseSync` a fresh yrs doc (`apply_update(Y.encodeStateAsUpdate(md.doc))`,
   **no ongoing subscription**) → run the SAME op on **both** (Yjs mutator + Rust twin, **feed the Yjs
   id to Rust**) → `expect(snapshotFromShadow(yrs)).toEqual(docToSnapshot(md))`. Ids match (base-synced),
   so the whole `MapSnapshot` compares — real byte-parity.

## Batch 3 — remaining mutators (do next)

- **Upgrade `add_slot` to full fidelity** (the flip needs it): the spike `add_slot` is slot-only. New
  signature ~ `add_slot(id, squad_id, layer_id, index, role, tag: Option<String>, asset_id:
  Option<String>, x, y, z, rotation)` — write the full Slot + append to `squad.slotIds` +
  `layer.entityIds`. `ensureDefaultSquad`/`ensureDefaultLayer` stay in the **JS wrapper** (mint
  faction/squad/layer ids there, create via `add_faction`/`add_squad`/`add_editor_layer`, pass concrete
  `squad_id`/`layer_id` in). Parity test: drive Yjs `addSlot` (empty doc → mints faction+squad+layer+slot
  ids), read those 4 ids back from the doc, replay via the Rust primitives with the same ids, compare.
- **`add_faction`** (`ydoc.addFaction`), **`add_squad`** (append to `faction.squadIds`). Add the
  `factions` root handle + undo scope.
- **`paste_slots`** (bulk, `ydoc.pasteSlots`@180): centroid translate to `anchorAt` (or +20 nudge),
  per-slot re-attach to its source squad (or default), file into layer, clamp x/y, z=0. JS mints the
  k ids + resolves squad/layer + passes arrays; Rust does the translate/clamp + writes in one txn.
- **`remove_editor_layer`** (`ydoc.removeEditorLayer`@500): collect the subtree (id + descendants),
  delete each filed slot (the `remove_slots` cascade) + the layers; **if the subtree was every layer,
  reseed a default** — JS passes a `reseed_id` used only then (matches "JS mints ids"). Parity-test the
  non-reseed case byte-exact; reseed case = structural native test.
- **meta**: `set_title`, `update_environment` (merge), `apply_row_meta`, `seed_meta`. Needs a `meta`
  root handle (already read via `get_or_insert_map("meta")` in `small_maps_json`).
- **`hydrate`** (`ydoc.hydrateMissionDoc`@535 + `…WithProgress`): rebuild the whole doc from a payload
  (lossless `editor` path + lossy `orbat` rebuild). Big; the lossy path mints ids (JS passes them).

## The flip (the finale — its own checkpoint + plan before executing)

Once the mutators are ported: (1) `ydoc.ts` mutators → thin wrappers that mint id(s) in JS and call
`md.wasm.<mutator>` (the Rust doc becomes authoritative — no `Y.Doc`); (2) `state/bindings.ts` +
`useMapStore`: replace `observeDeep`+`docToSnapshot`+`incPatchPlan` — the wasm mutators **return the
changed ids** so the store patches O(k) (structural → `snapshotFromShadow`); (3) `UndoController`
(`state/undo.ts`) → the Rust `MissionDoc` undo (`undo`/`redo`/`can_*` + expose a **change-version** the
`subscribe` polls); (4) persistence → a **yrs update-stream IDB adapter** (the 3.0.d
`features/_spike/yrsIndexeddb.ts` pattern) replacing `persistence/slotChunkStore`+`missionMetaStore` +
`y-indexeddb`; (5) **remove `yjs` (6 sites) + `y-indexeddb` (1)**; delete
`docToSnapshot`/`docToSnapshotWithProgress`/`incPatchPlan`/the bindings observe path/`state/undo.ts`;
un-gate the shadow (it IS the doc). This is an unavoidable big-bang — plan it as its own checkpoint.
Acceptance: editor byte-identical; ≥60 fps @500k+1M; compile output byte-identical; undo + IDB
round-trip; `yjs`/`y-indexeddb` gone from `package.json`.

## Commands

- `make wasm` — rebuild the gitignored pkg (before any frontend build/test). `make wasm-ci` — fmt +
  clippy `--all-features -D warnings` + core tests.
- `cargo fmt -p map-engine-core -p map-engine-wasm` (fmt has no `--manifest-path`).
- Frontend: `cd apps/website/frontend && npm run test -- mutatorParity` / `lint` / `build` /
  `format:check`. `npx prettier --write <file>` (worldmap/* has pre-existing prettier debt — don't touch).
- `make rust-test-it > /tmp/it.log 2>&1; echo $?` — **never `| tail`** (masks exit + truncates). DB
  `tbd_reforger_db` on :5434 (`make db-up` if down).

## Gotchas

- **wasm handles in React hooks: effect-local, not `useMemo`** — StrictMode double-frees a shared
  handle; wasm `.free()` isn't idempotent (`[[wasm-react-lifecycle]]`; the Stage-1 crash).
- **SoA positions are f32** (render-only, lossy). Exact readers (compile/persistence/store) use
  `slots_json`/`snapshotFromShadow` (f64). Never compile from the SoA.
- **Yjs encodes integer numbers as `Any::BigInt`** — the position reader accepts BigInt + Number.
- yrs `undo::Options::default()` is `#[cfg(not(target_family="wasm"))]` → build `Options` explicitly
  with a `ZeroClock` (already done).
- Parity test: `baseSync` must NOT keep an `on('update')` subscription (else the Yjs op double-applies
  to the yrs doc). Sync once, then mutate directly.

## Opening prompt for the new session

> Continue T-145 Phase 3.2 write-swap (yrs-authoritative doc-core). Read
> `.ai/artifacts/t145_write_swap_kickoff.md` + memory `t145-wasm-port.md` + `wasm-react-lifecycle.md`
> first. The Rust `MissionDoc` mutator port is at batch 2 (slot lifecycle + editor layers, byte-parity
> vs `ydoc.ts`, all green, no app change). Do **batch 3**: upgrade `add_slot` to full fidelity
> (squad/layer wiring + all fields), add `add_faction`/`add_squad` (+ the `factions` root handle),
> `paste_slots` (bulk), `remove_editor_layer` (reseed cascade), meta ops, and `hydrate` — each a full
> Rust mutator with JS minting the ids, proven byte-parity in `mutatorParity.test.ts`
> (`snapshotFromShadow(yrs).toEqual(docToSnapshot(yjs))`). Commit per batch; `make wasm` before frontend
> test/build. Then checkpoint before the flip (the big-bang: ydoc→wasm wrappers, yrs authoritative,
> remove yjs+y-indexeddb) and I'll plan it. Hold the byte-parity bar.
