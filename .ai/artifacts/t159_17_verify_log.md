# T-159.17 — yrs IDB persist + warm editor session — verify log

**Slice:** T-159.17 (Leptos Mission Creator editor persistence).
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159/`, branch `t-159-leptos-ui`, base `61d91b09` (T-159.16).
**Executor:** claude-code. **Result:** PASS — all 5 editor gates green; `cargo check --target
wasm32-unknown-unknown` + `trunk build --release` clean.

## What shipped

React v3 persistence parity (`yrsPersist.ts` + `editorSession.ts`) ported to Rust/wasm so a reload
keeps the local mission doc:

- **`apps/website-leptos/src/yrs_persist.rs`** (new, wasm32) — IndexedDB access via the `idb` crate
  (DB `tbd-mission-yrs` v1 / store `doc-state`, out-of-line key = mission id, value = whole
  `encode_state()` blob as `Uint8Array`); `save_state`/`load_state`/`clear_state`; the debounced (5 s,
  resets each call) + **serialized-per-mission** (`futures::lock::Mutex`) writer + `flush_state` with
  the React guards (cancel-before-read, empty-blob skip); `register_flush_on_hide`
  (`visibilitychange`(hidden) + `pagehide`); and the read-only `window.__missionPersist` smoke bridge.
- **`apps/website-leptos/src/editor_session.rs`** (new, wasm32) — `sessionStorage["tbd-editor-session"]`
  warm marker, 24 h TTL, exact React JSON `{ missionId, readyAt, slotCount, currentSemver }`;
  `mark_ready`/`read_warm`/`clear`.
- **`apps/website-leptos/src/mission_editor.rs`** — reads the `:id` route param; keeps the synchronous
  seed + `__missionDoc` (so `smoke_doc_editor` is unaffected) and adds an async persist `spawn_local`:
  load-or-**SWAP-restore**, initial debounced persist, `mark_ready`, flush-on-hide, then `ready` last.
- **`apps/website-leptos/src/main.rs`** — `#[cfg(wasm32)] mod editor_session;` + `mod yrs_persist;`.
- **`apps/website-leptos/Cargo.toml`** — `idb = "0.6"` (wasm32 block; default features — its `tokio`
  dep is `features=["sync"]` only, wasm-safe) + web-sys `"Document"` (visibilitychange / `document.hidden()`).
- **`.ai/artifacts/t159_gates/driver/smoke_persist_editor.mjs`** (new) — Class R gate, serve `5303` /
  debugPort `9363`.

Async `flush()`/`clear()` bridges use `js_sys::Promise::new` + `leptos::task::spawn_local` — **no
`wasm-bindgen-futures` dep added**. Only new deps: `idb` (+ its lockfile deps) and the web-sys
`Document` feature.

## P6 decision — mission id source

The editor route is `path!("/missions/:id/edit")` (`app_routes.rs:122`), a real `:id` param. The id is
read once at mount via `use_params_map().get_untracked().get("id")` → **`"smoke"`** on the gate route
(fallback `"draft"`, mirroring React `missionId ?? 'draft'`). The warm marker + IDB key both use it.

## Correctness note — Class R is SEMANTIC, not byte-encode

The initial smoke asserted `encode_hex()` byte-equality across reload. The in-Rust proof caught that
this is **false**: `yrs`'s `encode_state_as_update_v1` is deterministic for the *same* doc but a fresh
peer that replays the update re-encodes to **semantically-identical, byte-different** bytes (only the
*materialization* is equal — exactly why the core's `encode_decode_roundtrip_is_stable` asserts
materialization equality, never `b.encode_state()==bytes`). Empirically `encode(new()+apply(H1)) != H1`.

Class R is therefore asserted on a **canonical semantic slot digest** (`slots_digest()`): rows keyed
by slot id and sorted, floats compared bit-exactly (`f32::to_bits`), every interned `*_idx` resolved to
its string (so arbitrary materialize row-order / dict order can't perturb it). Restore fidelity =
`slots_digest(warm) === slots_digest(cold)`. `loaded_from_storage()===true` proves the content came
from IDB (not a re-seed).

## Goldens (`/missions/smoke/edit`)

- `slot_count` = **8** (SEED_N). `encode_hex` length = **2138** (H1 = the T-159.16 seed golden; head
  `01300100270105736c6f7473027330012800010002696401`).
- `slots_digest` length = **359** (stable cold==warm). Warm marker: `{missionId:"smoke", slotCount:8,
  currentSemver:null}`.
- dist wasm ≈ **5,231,512 B**.

## Gate results (all exit 0)

`smoke_persist_editor` (new, serve 5303 / debugPort 9363):
```json
{ "gate":"editor-persist-smoke",
  "coldLoaded":false, "coldSlots":8, "coldDocRt":true,
  "warmLoaded":true, "warmSlots":8, "warmDocRt":true,
  "digestMatch":true, "digestLen":359, "encodeHexLen":2138,
  "warm":{"missionId":"smoke","slotCount":8,"currentSemver":null},
  "coldOk":true, "warmOk":true, "panics":[], "pass":true }
```
Prior smokes (regression — all `pass:true`):
- `smoke_editor` → `editor-smoke` pass
- `selfcheck_editor` → `editor-selfcheck` pass (calibration + texture, webgl2)
- `smoke_pan_editor` → `editor-pan-smoke` pass (webgpu)
- `smoke_doc_editor` → `editor-doc-smoke` pass (slotCount 8, seeded, roundtripOk, encodeStable) — **unaffected**

## Build

- `cargo check --target wasm32-unknown-unknown` — clean.
- `trunk build --release` — clean (`Finished release ... success`).

## Non-goals held (T-159.17)

No server hydrate/conflict GET, no mutator/autosave subscription (the core has no change hook — the
writer is driven by the post-seed/post-load encode), no Save-Version POST, no pick/select/drag UI, no
Eden chrome, no v1/v2 IDB migration, no `GpuTimer`/`unproject_xy` (`disable_frame_timing` + per-frame
`poll` unchanged). The doc leaks on route-leave like the engine (`!Send` `Rc`, no `.free()`).

## Next

Ready for Cursor → **T-159.18** (select / LMB tools).
