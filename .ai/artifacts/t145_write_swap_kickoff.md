# T-145 Phase 3.2 write-swap — kickoff (fresh session)

Standalone brief to continue the **doc-core cutover to `yrs`**. Goal: make the wasm `yrs` doc
**authoritative** and remove `yjs` + `y-indexeddb`. Read this + memory `t145-wasm-port.md` +
`[[wasm-react-lifecycle]]` first. Full plan: `~/.claude/plans/done-everything-s-committed-and-wondrous-fairy.md`.

## Where things stand (branch `t-145-rust-rewrite`, tree clean)

The read side + the **complete mutator port (batches 1–3)** are done, committed, green — the Rust
write API now mirrors **every** `ydoc.ts` mutator, each byte-parity-proven. Gates: **cargo
`-p map-engine-core --all-features` 55/55** (incl. the reseed native test), **`make wasm-ci` clean**
(clippy `--all-features -D warnings`), **frontend `npm test` 323/323** (`mutatorParity` 24),
`npm run build`+`lint` clean (`format:check` = 23 pre-existing non-`worldmap` debt files, none ours).
**Next = the flip** (see below). Tip `8ca31ddf`.

| commit | what |
|---|---|
| `a7fdd44c`…`60564815` | Phase 3.0 spike — yrs `MissionDoc` (SoA + apply/encode/undo), all 6 criteria proven (operator: 90 fps @1M + IDB round-trip) |
| `53f2349a` `60c191be` `a204deb2` | 3.1 — cluster + pick indices → Rust in the app; **supercluster off runtime** |
| `ae21ed86` `e2c347c0` | 3.2 Stage 1 — live **DEV shadow** yrs doc in `useMissionDoc`, synced from `md.doc.on('update')`, gated by `checkDocShadowParity` (`state/docShadow.ts`) |
| `89ce6042` | 3.2.2 — `small_maps_json` + whole-snapshot gate |
| `dd2233a6` | 3.2.3 — `slots_json` + `snapshotFromShadow(shadow): MapSnapshot` (SoA f32 = render-only; exact readers use JSON), proven `.toEqual(docToSnapshot)` |
| `184c1163` | batch 1 — `update_slot`/`update_slot_position`/`move_entities`/`remove_slots` (byte-parity) |
| `84071eaf` | batch 2 — `add_editor_layer`/`rename_editor_layer`/`reparent_editor_layer`(cycle guard)/`move_slot_to_layer` (byte-parity) |
| `37d742f3` | batch 3a — full `add_slot`(squad/layer + all fields) + `add_faction`/`add_squad` + `factions` handle (byte-parity) |
| `0e11a10a` | batch 3b — `paste_slots` (bulk: centroid/clamp/per-squad index, batched appends) (byte-parity) |
| `4c40c8d1` | batch 3c — `remove_editor_layer`(reseed cascade) + `set_title`/`update_environment`/`apply_row_meta`/`seed_meta` + `meta` handle + `doc`→serde_json (byte-parity + reseed native test) |
| `8ca31ddf` | batch 3d — `hydrate` lossless loader (lossy orbat→editor transform stays JS) (byte-parity) |

**Rust `MissionDoc` write API (COMPLETE)** (`crates/map-engine-core/src/doc/store.rs`): readers
`apply_update`, `encode_state`, `materialize` (SoA), `small_maps_json`, `slots_json`,
`undo`/`redo`/`can_*`; **full-fidelity mutators** `add_slot`(squad/layer wiring + all fields),
`add_faction`, `add_squad`, `paste_slots`, `update_slot`, `update_slot_position`, `move_entities`,
`remove_slots`, `add_editor_layer`, `rename_editor_layer`, `reparent_editor_layer`,
`move_slot_to_layer`, `remove_editor_layer`(reseed), `set_title`, `update_environment`,
`apply_row_meta`, `seed_meta`, `hydrate`; spike-only `set_slot_position`/`remove_slot`/`seed_random`
remain. Root handles (all undo-scoped): `slots`, `squads`, `factions`, `editor_layers`, `meta`
(the 5 non-tracked maps — loadouts/items/objectives/vehicles/markers — are `get_or_insert`'d inline
in `hydrate`). Shared helpers: `append_id`, `read_id_array`, `remove_slots_in_txn`, `read_env_map`,
`json_str_to_any`/`value_to_any`, `load_rows`/`load_row`, `position_any`, `retain_ids`. **JS mints
all ids** and passes them + resolved squad/layer + (for `hydrate`) the editor-shaped payload in.

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

## Batch 3 — DONE (`37d742f3` → `8ca31ddf`; plan `~/.claude/plans/plan-written-committed-replicated-gosling.md`)

All shipped byte-parity, no app change (4 reversible sub-commits — see the table above). Every
`ydoc.ts` mutator now has a Rust twin proven via `snapshotFromShadow(yrs).toEqual(docToSnapshot(md))`
in `mutatorParity.test.ts` (batches 3a–3d, 13 new cases) + a `remove_editor_layer` reseed native test.
**Design decisions worth carrying to the flip:**
- **`hydrate` is a lossless verbatim loader** — the lossy `orbat[]`→graph rebuild (which mints ids)
  **stays in JS**; the flip's `hydrateMissionDoc` wrapper runs the existing lossy transform → an
  `editor`-shaped payload → `wasm.hydrate(payloadJson, defaultLayerId)`. Proven by the "lossy orbat →
  JS-minted editor dicts reconstructed byte-for-byte" parity test.
- **Deferred to the flip (do NOT skip):** **(1) undo origin semantics** — Rust tracks every txn today;
  `ydoc` splits `LOCAL_ORIGIN` (undo-tracked) vs `INIT_ORIGIN` (untracked: `seedMeta`/`applyRowMeta`/
  `hydrate`/`seedDefaultLayer`). The flip needs origin-scoped transactions (`doc.transact_mut_with(origin)`
  + `UndoOptions.tracked_origins` = the LOCAL origin) so load/seed aren't undoable. Batch 3's parity is
  snapshot-state only, so this was invisible — it becomes real at the undo cutover. **(2) real DEM z on
  write** — the Rust mutators write `z = 0` (batch parity held because vitest has no DEM → `terrainZ`=0);
  the flip wrapper must sample `terrainZ(x,y)` JS-side and write it (`add_slot`/`paste_slots`/
  `move_entities`/`update_slot_position` all touch z). For `paste_slots` this means the wrapper needs the
  clamped px/py — either have Rust return them or clamp JS-side. **(3) chunked/progress hydrate** for the
  load overlay (`hydrateMissionDocWithProgress`); the Rust load is one fast call, so this is UI-only.

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
> first. **The full mutator port is DONE** — batches 1–3 (`8ca31ddf`), the Rust `MissionDoc` write API
> mirrors every `ydoc.ts` mutator, each byte-parity-proven in `mutatorParity.test.ts`, no app change.
> Now **plan the flip** (§The flip): `ydoc.ts` mutators → thin wrappers that mint id(s) JS-side + call
> `md.wasm.<mutator>` (yrs authoritative, drop `Y.Doc`); bindings/store fed by `snapshotFromShadow`
> (mutators return changed ids → O(k) patch); `UndoController`→Rust undo **+ origin-scoped transactions**
> (the deferred undo-origin split); persistence→a yrs update-stream IDB adapter; **remove `yjs` +
> `y-indexeddb`**; delete `docToSnapshot`/`incPatchPlan`/`state/undo.ts`/the bindings observe path; sample
> real DEM z JS-side in the wrappers. High blast radius — write a plan (EnterPlanMode), get sign-off, then
> execute as one checkpoint. Acceptance: editor byte-identical, ≥60 fps @500k+1M, compile byte-identical,
> undo + IDB round-trip, `yjs`/`y-indexeddb` gone from `package.json`.
