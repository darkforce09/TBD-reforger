# T-946.86 — the wave 255 dead-code repair

Filed 2026-09-08 at wave 256 pre-dispatch. Supersedes **T-946.82, T-946.83, T-946.84, T-946.85**
(all cancelled into this row) and closes **T-946.69**.

Verified against `main @ f71d2d0b2`.

## Why this exists

The wave 255 adversarial verifier found **three of that wave's five shipped tickets** in the same
state: the code landed, its unit tests passed, and nothing in production called it. A fourth finding
is worse — a slice report claimed a UI control had been added that the commit does not contain.

This is the repo's signature defect wearing a new coat: *a check reports success over an input it
never examined.* Here the check is a unit test whose subject is unreachable from the running app, so
the test can only ever prove the function computes correctly in a vacuum.

The repair runs one wave after the defect, while the code is fresh.

## The four

| Finding | Symbol | Call sites | Anchor |
|---|---|---|---|
| T-946.82 | `set_z_drag_readout`, `snap_elevation`, `format_height_readout` | 0 | `canvas/overlays.rs:32`, `canvas/gizmo_z.rs:15,23` |
| T-946.83 | `plan_drop`, `DragSet` | 0 | `panels/outliner_drag.rs:9` |
| T-946.84 | `begin_tactical_draw` | 0 | `state/operations/tactical_graphics.rs:188` |
| T-946.85 | `duplicate_slot_ids` | 0 | `state/operations/slot_ids.rs:4` |

### T-946.82 — the Z gizmo arm arms and then does nothing

`gestures.rs` has exactly four occurrences of `z_drag`: declared `:161-172`, cloned into the
`onpointermove` closure `:374`, **written** in `onpointerdown` `:626-627`, cloned into the
`onpointerup` closure `:832`. Neither body ever `.borrow()`s it. `container.set_pointer_capture()` at
`:628` then `return` at `:630` — none of the eight capture releases in the file (`:870`, `:987`,
`:1010`, `:1030`, `:1038`, `:1224`, `:1422`, `:1547`) belongs to this arm, so capture strands.

The readout is worse than dead: `read_z_drag_readout` (`overlays.rs:35`) **is** wired at `:220`, so
the chip renders and can never be populated. `gizmo_z.rs` itself is sound — only `hit_z_arm` (`:4`)
is live (`gestures.rs:596`); the rest of its math has no caller. **Do not change `gizmo_z.rs`.**

### T-946.83 — the multi-select drag builds the right set and drops the wrong one

`outliner_tree.rs:1029-1044` walks the real multi-selection into `ids`; `:1045` builds
`DragSet { anchor, ids }`; `:1048` stores it in `outliner_drag::PENDING_DRAG`. Then `:1049` **also**
calls `operations::begin_layer_drag(id_down)`, which stores a single id in a *different* thread-local
(`entity.rs:769`, written `:1538`). On pointerup, `:1060` calls `complete_layer_drop_onto_folder`
(`entity.rs:1560`), which reads the single-id store. Only the anchor moves. `PENDING_DRAG` is read
only for `.is_some()` at `:1428` and cleared at `:1436` — never consumed.

`begin_layer_slot_drag` (`:72`), `begin_layer_comment_drag` (`:76`) and `begin_refile` (`:80`) in
`outliner_drag.rs` are dead too, shadowed by the `entity.rs` versions that `operations.rs:53-56`
re-exports.

A Class-R pin at `outliner_tree.rs:1845` asserts the string `complete_layer_drop_onto_folder` still
appears in the file.

**Constraint:** the fix must stay inside `outliner_drag.rs` and `outliner_tree.rs`.
`state/operations.rs` is T-939.2's this wave. Nothing is needed from it — `with_batch` is already
re-exported at `operations.rs:46` and `refile_slot` is already public at `entity.rs:3145`.

### T-946.84 — the tactical draw path has no trigger, and a report said it did

`begin_tactical_draw` (`tactical_graphics.rs:188`) has zero call sites. Every other occurrence is
prose: `operations.rs:83` (a comment naming it as uncalled), `history.rs:386`, its own doc block
`:172-186`, and `mission_editor_tests/t936_7_tactical_lane_bind.rs:9,51` (the second is a string
inside an assert message, not a call).

`REPORT-T-939.1.md` claimed: *"Folded in T-946.69 UI button: Added Phase Line tactical draw button in
the outliner header."* Commit `d164435caa5a` contains no such button.

The code names its own remedy at `:184-185` — *"one call site under `panels/` makes the whole path
live"* — and rules out the alternative at `:175-182`: a keybinding in `canvas/commands.rs` compiles
but reddens `help_modal.rs`'s `every_binding_has_a_help_entry` and
`no_two_listeners_claim_the_same_chord`. The precedent to copy is `panels/zones_panel.rs:37-44`,
whose `arm` closure calls `ops::begin_zone_draw(&kind, shape, DrawTarget::Zone)` at `:42`.
Downstream — `gestures.rs` vertex append, `commands.rs` Esc — is already live.

**Constraint:** the button lands in `zones_panel.rs`. T-939.4 runs this wave, owns `context_menu.rs`,
`top_strip.rs` and `help_modal.rs`, and has been told not to add a tactical-draw entry.

### T-946.85 — the duplicate guard was written and never called

`duplicate_slot_ids` (`slot_ids.rs:4`) is exported (`operations.rs:90-91`) and consumed by nothing but
its own test. `save_now` (`commands_hotkeys.rs:955-1010`) clears findings `:961`, compiles `:966`,
builds the body `:967` and POSTs `:973` with no duplicate check.

The upload path is protected by a **private near-twin**,
`check_duplicate_slot_ids_in_payload` (`mission_library.rs:1519`, called once at `:1565`), which reads
`payload.editor.squads[]` JSON rather than the live doc.

**The twins disagree.** `slot_ids.rs:32` gates on `doc.slot_exists(id_str)`; the library version does
not. A payload with a dangling duplicate id is refused on upload and would pass the doc-side guard.
Collapsing them is in scope; if they stay separate, the divergence must be documented in code.

## Acceptance instrument

Every function named above must have at least one production call site, proven by grep in the slice
report. That is the only assertion that distinguishes this repair from the state it is repairing.
