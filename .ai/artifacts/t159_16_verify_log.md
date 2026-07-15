# T-159.16 — MissionDoc host in Leptos editor — verify log

**Slice:** T-159.16 · **Branch:** `t-159-leptos-ui` · **Baseline:** T-159.15.2 (`035c841e`)
**Spec:** `docs/platform/t159_16_mission_doc_host.md` · **Handoff:** `.ai/artifacts/t159_16_claude_code_handoff.md`
**Date:** 2026-07-15

## What shipped
The Leptos editor now hosts a **mission document** — a plain Rust `MissionDocCore` (map-engine-core
`doc` feature) in the SAME wasm module as the render engine — with a deterministic seed, a Rust Class R
encode round-trip self-check, an optional doc→engine SoA bind, and a thin `window.__missionDoc` smoke
bridge. Lifecycle + bridge only; no `ydoc.ts` mutator port, no persist/outliner/save (→ .17+).

### Files
| File | Change |
|------|--------|
| `apps/website-leptos/Cargo.toml` | + `map-engine-core { features = ["doc"] }` under the wasm32 target block |
| `apps/website-leptos/src/main.rs` | + `#[cfg(target_arch = "wasm32")] mod mission_doc;` |
| `apps/website-leptos/src/mission_doc.rs` | **new** — `MissionDocCore` host: `new_seeded_doc`, `roundtrip_ok`, `register_mission_doc` (`window.__missionDoc`) |
| `apps/website-leptos/src/mission_editor.rs` | mount seam (build+seed+bridge before `spawn_local`; optional `slots_bind_soa` after engine `Some`) |
| `.ai/artifacts/t159_gates/driver/smoke_doc_editor.mjs` | **new** — Class R headless gate (port 5302 / debugPort 9362) |

## Gate results — ALL PASS

### Round-trip oracle (native cargo)
```
$ cargo test -p map-engine-core --features doc encode_decode_roundtrip_is_stable
test doc::store::tests::encode_decode_roundtrip_is_stable ... ok
test result: ok. 1 passed; 0 failed; 97 filtered out
```

### Compile + build
```
$ cargo check -p website-leptos --target wasm32-unknown-unknown
   Checking yrs v0.27.2
   Checking map-engine-core v0.1.0 …
   Checking website-leptos v0.1.0 …
    Finished `dev` profile [optimized] target(s) in 20.85s      # clean, no warnings on new code

$ ( cd apps/website-leptos && trunk build --release )
    Finished `release` profile [optimized] target(s) in 27.23s
    INFO ✅ success                                              # → apps/website-leptos/dist/
```

### NEW Class R gate — `smoke_doc_editor.mjs` (exit 0)
```json
{
  "gate": "editor-doc-smoke", "path": "/missions/smoke/edit",
  "slotCount": 8, "seeded": true, "roundtripOk": true, "encodeStable": true,
  "encodeHexLen": 2138, "encodeHexHead": "01300100270105736c6f7473027330012800010002696401",
  "panics": [], "pass": true
}
```
- **seeded** — `slot_count() === 8` (SEED_N); the `seed_random` generator wrote 8 slots.
- **roundtripOk** — `roundtrip_ok() === true`: re-encode byte-identical **and** encode→apply→materialize
  slot-set equality (id/x/rot), all computed in Rust (the live equivalent of the core unit test).
- **encodeStable** — `encode_hex()` byte-identical across two reads of the same hosted doc.

### Regression gates (T-159.15.x) — still PASS
| Gate | backend | pass | key asserts |
|------|---------|------|-------------|
| `smoke_editor.mjs` | webgpu | ✅ | `viewChangedOnWheel`, no panic |
| `selfcheck_editor.mjs` | webgl2 (`?force=webgl`) | ✅ | calibration + texture byte-exact readback |
| `smoke_pan_editor.mjs` | webgpu | ✅ | `panMoved && zoomChanged && panContinued`, no "already mapped" |

The doc host did not regress the camera/render/self-check wiring.

## Recorded seed golden (spec D3)
Deterministic seed = `MissionDocCore::seed_random(n=8, w=12800, h=12800, seed=0x00715916)` under `INIT`
origin. Its `encode_state()` (Yjs-wire v1) golden — captured live from the browser bridge:
- **slot_count:** `8`
- **encode length:** `1069` bytes (`encodeHexLen = 2138` hex chars)
- **first 24 bytes:** `01300100270105736c6f7473027330012800010002696401`
  (decodes as the v1 update header + the `slots` root map — `736c6f7473`=`"slots"`, `7330`=`"s0"`).

The full stream is byte-stable on re-encode (fixed `CLIENT_ID=1` + deterministic v1 encode; asserted by
both `encodeStable` above and the core test `store.rs:1266`).

## Decisions / notes

**D2 — linking (same wasm, no JS shim).** The doc core reaches Leptos via a direct **workspace dep on
`map-engine-core` with the `doc` feature**, wasm32-gated (`apps/website-leptos/Cargo.toml`). This is the
spec-sanctioned D2 path; the `map-engine-wasm` `MissionDoc` wrapper + its `bind_mission_doc` free fn were
**not** pulled in (that would be the second JS wasm shim D2 forbids). `doc` adds `yrs 0.27.2`; its
`serde_json` is already shared with render's `world` feature. Leptos calls `RenderEngine::slots_bind_soa`
directly with the `SlotSoa` from `MissionDocCore::materialize()`.

**Class R vs Class S (term reconciliation).** map-engine-core's own doc taxonomy (`doc/mod.rs:5`) calls
the doc's JS-parity contract **Class S** — *result-set equality*, explicitly NOT CRDT-byte-identity. The
spec's "Class R gate" for this slice refers to the **encode round-trip**, which the `roundtrip_ok`
self-check proves via two properties: (1) re-encoding the *same* doc is byte-identical (deterministic v1
+ fixed client id), and (2) a fresh peer that replays the update materializes the same slot set (the
Class S property). Both are the exact assertions of the core test `encode_decode_roundtrip_is_stable`.

**Lifecycle — leaks like the engine (no double-free).** The doc is held in the same
`Rc<RefCell<Option<_>>>` idiom as the engine and, like the engine, leaks on route-leave: `Rc` is `!Send`
and `on_cleanup` is `Send`-bound, so it cannot drop the doc (documented at `mission_editor.rs:249-259`).
"Free on dispose / no double-free" is satisfied by construction — `MissionDocCore` is plain Rust
(`Drop` runs at most once); there is no wasm-bindgen `.free()`, so the React StrictMode double-free
hazard does not exist. A proper `!Send` drop path is later polish, tracked with the engine's.

**D5 bind — provably safe.** `slots_bind_soa` early-returns while the slot atlas is unuploaded
(`engine.rs:3326`), and the editor uploads no slot atlas yet, so the bind is a pure cache write — it
cannot panic and cannot regress the smokes (confirmed: all three still pass). It proves the doc→engine
SoA wire compiles + runs; the 8 seeded slots render nothing until a later slice uploads the atlas.

**D6 bridge.** `window.__missionDoc` exposes `slot_count()`, `encode_hex()`, `change_version()` (host
counter, `1` post-seed — bumped by mutators in .17+), `roundtrip_ok()`. Registered synchronously in
`on_load` (before the async engine create), so the Class R gate is GPU-independent.

## Out of scope (untouched)
`ydoc.ts` mutator port, Zustand mirror, IDB `yrsPersist` (→.17), select/marquee/drag, outliner,
save/export, Arsenal, Eden chrome. No `docs/**` or `.ai/tickets/registry.json` edits. `GpuTimer` /
`unproject_xy` untouched. `disable_frame_timing` / `poll` / pan / wheel preserved.

## Ready for
Cursor doc sync → **T-159.17** (yrsPersist / editor session).
