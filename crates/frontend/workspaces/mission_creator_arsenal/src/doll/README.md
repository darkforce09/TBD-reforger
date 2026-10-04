# Arsenal doll hooks

The browser hooks of the [arsenal](/documentation/glossary/a_to_f.md#arsenal)'s 3D paper doll,
which the parent file `crates/frontend/workspaces/mission_creator_arsenal/src/doll.rs` registers once the
doll's engine exists.

## Contents

```text
crates/frontend/workspaces/mission_creator_arsenal/src/doll/
└── window_hooks.rs  `register_doll_hooks`: `window.__arsenalDoll` with backend, anchor, pick and self-check
```

## How it works

`ArsenalDoll` in `doll.rs` creates the `PaperDollRenderer` and then calls `register_doll_hooks`
with its engine handle. The function builds one JavaScript object with four closures and sets it
as `window.__arsenalDoll`: `backend()` returns the renderer's backend name, `anchor(idx)` the
region's anchor in css pixels, `pick(x, y)` the region a CPU pick finds at a css pixel, and
`doll_self_check()` a promise of the engine's readback check. Each closure borrows the engine for
the length of one read; with no engine the first three return `null` and the self-check promise
rejects with "engine not ready". The closures are leaked, so they stay callable while the page
lives.

## Boundaries

- Depends on: the parent `doll.rs` for `EngineHandle`; `paper_doll_renderer`'s
  `PaperDollRenderer` methods (`backend`, `anchor_px`, `pick_region`, `self_check`) through it;
  `js_sys`, `wasm_bindgen` and `wasm_bindgen_futures` for the JavaScript values.
- Used by: `doll.rs`, the only caller; the Arsenal browser smoke in
  `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/arsenal.rs` reads
  `window.__arsenalDoll`.
- Rules: wasm-only, like the parent module. The hooks only read the engine; no hook draws, picks
  for the user or writes the loadout.

## Related documentation

- [Arsenal loadout editor](/documentation/crates/frontend/workspaces/mission_creator_arsenal/arsenal_loadout_editor.md) — the
  Arsenal tab and its doll as the mission maker uses them.
