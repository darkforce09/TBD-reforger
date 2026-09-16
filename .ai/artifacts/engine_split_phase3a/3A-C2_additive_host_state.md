# 3A-C2 — the three additive host-state files cross into the engine

Agent 5 of 8. Your scope is exactly three files. `3A-C1` already moved the seven engine-only
`entity/` implementations — **do not revisit them.**

## Purely additive. Delete nothing.

`3A-C` adds; `3A-D` removes. The thirteen thin adapters stay exactly where they are; `3A-D3`
deletes the whole `state/operations/` directory at the end.

## These are NOT duplicates

Each of the three has a same-named engine counterpart, and each **adds host/session state the
engine does not model**. Move that state engine-side. Do not discard it, and do not "reconcile"
it away by deciding the engine version is already good enough — it is not, that is the point.

| frontend file | LOC | engine counterpart | LOC | the additive part |
|---|--:|---|--:|---|
| `tactical_graphics.rs` | 299 | `operations/tactical_graphics.rs` | 107 | the `TG_STATE` draw machine — selected / draft / `VertexDrag` |
| `cargo.rs` | 202 | `operations/cargo.rs` | 120 | `CARGO_DEFAULTS` / `LOADOUT_BUFFER` / `APPLY_SEED` thread-locals |
| `entity/placement.rs` | 194 | `operations/entity/placement.rs` | 111 | placement policy: crew toggle, active-layer resolution, cargo seeding |

`tactical_graphics.rs` is the lopsided one: only about 4 of its ~20 public fns forward to the
engine; the rest **is** the session state machine.

## The types you need already exist engine-side

Checked, so you do not have to: `CargoRow` is `data/store/operations/cargo_rules.rs:20` and
`BufferedLoadout` is `data/store/operations/cargo.rs:12`. The frontend's
`crate::editor::arsenal::{arsenal_rules::CargoRow, BufferedLoadout}` paths are re-exports of those.
So `CARGO_DEFAULTS: RefCell<HashMap<String, Vec<CargoRow>>>` and
`LOADOUT_BUFFER: RefCell<Vec<BufferedLoadout>>` cross without dragging any frontend type with them.

`TacticalDraft` is already engine-side at `operations/tactical_graphics.rs:16`. `TgState` is
frontend-defined and crosses with the machine.

## One implementation, not two

Do not leave the state in both crates. The engine takes ownership; the frontend file keeps its
public names and signatures and holds only the `OPS_CTX` borrow chain, the signal reads/writes,
and the `mission_history::after_local_edit()` tail. `3A-D3` deletes those shims with the
directory. This is the shape `3A-C1` landed — read commit `ced6a7860` and match it.

## Learn from C1's two traps

1. **Declare engine test modules from each production file, never from a `mod.rs`.** A
   `#[cfg(test)]` in `entity/mod.rs` truncates the `DOMAIN_ENTITY` source scrub at that line and
   reddened nine frontend pins.
2. **Never `cd` before cargo.** C1 created five stray `target-container/` directories inside the
   source tree that way, costing 6.2 GB. Run cargo from the repo root; use `( cd sub && … )` if
   you truly need another cwd.

## Boundaries

- `data/store/mod.rs` pins its public surface via
  `#[cfg(test)] #[path = "tests/reexports.rs"] mod reexport_pins;`. **Update it as the surface
  grows** — that pin is the contract. C1 added a second case there; add yours alongside.
- **Gate rule 7** reads zero in both directions and must stay zero: nothing under `data/` may name
  `crate::{camera,frame,io,overlay,spatial,streaming,world}` or `website_graphics_engine`.
- Anything in `data/store/` must be browser-free — no `web_sys`, no `leptos`, no signals.
- Law 7: everything you create is born compliant, tests in sibling files.

## Done when

The engine holds the additive state of all three, the frontend still compiles and passes
unchanged, and:

```
CARGO_TARGET_DIR=target-container cargo xtask verify engine-layers
CARGO_TARGET_DIR=target-container cargo test -p website-map-engine --all-features
CARGO_TARGET_DIR=target-container cargo test -p website-frontend
```

Baseline: `verify engine-layers` PASS on all 8 rules; map-engine **1384** passed / 0 failed / 2
ignored as the floor; frontend **1317** passed / 0 failed and it must not fall.
