# T-939 — Editor usability: selection, gizmo, arrange, templates, diagnostics, search

Owner: command center. Source: master audit S4 (2026-09-04), verified against main @ 072988d57 (README.md in this
directory). Scope: TBD-Reforger only. Executor: claude-code. Frontend dir `apps/website/frontend/src/editor/`; the
editor keymap is the wasm keydown closure in `mission_editor.rs` (allowlisted SIZE-3 file — arms stay one-line callers).

## 1. Related existing tickets — referenced, never re-minted
T-704 command palette · T-142/T-157/T-158 shell · T-129 floor selector · T-131 route planner · T-141 slot naming · T-080
connection UI · T-070 vehicles placeable · T-309 faction squads · T-645 Arrange · T-650 Compositions · T-697 search · T-937.2 undo groups · T-936.1.

## 2. Verified anchors (2026-09-04)
| Finding | Anchor | Verdict | Slice |
|---|---|---|---|
| outliner single drag | panels/outliner_tree.rs:1110 (peers :1022, :664, :1101) | TRUE | T-939.1 |
| squad read-only, no faction selector | panels/attributes_modal.rs:1864-1872 (:1546-1607, :1749-1842) | TRUE | T-939.2 |
| no Z gizmo | canvas/overlays.rs:172-198, :200-210 | TRUE | T-939.3 |
| Arrange menu-only, no shortcuts | panels/top_strip.rs:249-252 (T-645), :378-380 | PARTIAL | T-939.4 |
| no squad templates | arsenal/asset_catalog.rs:321/:460; panels/dock_right.rs:63-92 | PARTIAL | T-939.5 |
| no badges; connections not drawn | canvas/overlays.rs:654-655, :679 | TRUE | T-939.6 |
| vehicles bypass virtualization; re-flatten | panels/outliner.rs:39; panels/vehicles_panel.rs:231-276 | TRUE | T-939.7 |
| Ctrl+F unreachable | panels/dock_left.rs:134-144 (T-697) | PARTIAL | T-939.8 |

## 3. Design
- **T-939.1** `panels/outliner_drag.rs`: `DragSet {anchor, ids}`, drop planning (order kept, self-drop rejected); one transaction.
- **T-939.2** `state/operations/reassign.rs`: `reassign_slots(ids, target)`; modal faction selector + squad picker inside `with_batch`.
- **T-939.3** `canvas/gizmo_z.rs`: Z arm geometry, hit test, dy→metres, snap (Shift suspends), readout; z via the translate op.
- **T-939.4** top_strip.rs shared entry list → context_menu.rs submenu (selection ≥ 2) + keydown chords + help rows.
- **T-939.5** `arsenal/squad_templates.rs`: defaults per faction + `from_composition`; catalog Squads section; one-op placement.
- **T-939.6** `canvas/diagnostics_overlay.rs`: badges per finding, wires per connection, hidden-layer aware, badge click selects.
- **T-939.7** outliner.rs memo keyed on the document version; vehicles_panel.rs through the virtual list above the threshold.
- **T-939.8** dock_left.rs `focus_search`; Ctrl+F/Cmd+F arm with preventDefault; Escape returns focus; help row.
Waves: A = .1 .3 .5 .7; B = .2 (after T-937.2), .4 (after .1), .6 (after .3: overlays.rs, canvas/mod.rs); C = .8 (after .4: mission_editor.rs, help_modal.rs).

## 4. Rules every slice encodes
Defect verified on main first (red pasted verbatim); perturbation with `touch` after restore; no `git add -A`, no `git stash`,
no `cargo xtask ci ci-local`; `skip:` = FAIL; no .py/.sh/.mjs committed; file-length allowlists never extended (new code → new
files); agents never merge/push/change status. Report schema: pwd_branch · defect_verified_on_main · changes · perturbation ·
gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits.

## Claude Code prompt — T-939.1

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-939.1 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-939_1_plan.md; panels/outliner_tree.rs:640-680,1000-1120; panels/mod.rs; the selection signal.
═══ PROBLEM ═══
begin_layer_slot_drag(one String) at :1110 and its peers carry a single id; dragging a selected row
moves only that row, so multi-selection is useless for reorganising layers.
═══ SHIPPED ═══
Single-row drag/drop; multi-selection state; doc-store transactions.
═══ LANGUAGE GATE ═══
Rust (Leptos, wasm).
═══ LOCKED ═══
- New logic in panels/outliner_drag.rs; outliner_tree.rs only threads DragSet.
- One doc-store transaction per drop; one Ctrl+Z reverts it.
- Drop into a dragged row is rejected; relative order is preserved.
═══ DO ═══
1. Verify on main: test dragging with two selected rows moves one; paste the red.
2. Write outliner_drag.rs (DragSet, plan_drop) with unit tests; register in panels/mod.rs.
3. Thread DragSet through :1110, :1022, :664, :1101; ghost shows the count.
4. Perturbation: drop the ids tail → two-row test red; restore, touch, green.
═══ DO NOT ═══
No selection-model rewrite; no files outside owns; no touching outliner.rs (T-939.7).
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-939.1
═══ MANUAL ═══
Select three rows, drag one to another layer: all three move in order; Ctrl+Z restores them.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-939.2

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-939.2 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-939_2_plan.md; panels/attributes_modal.rs:1540-1610,1740-1880; state/operations.rs; state/operations/batch.rs (T-937.2).
═══ PROBLEM ═══
The modal shows the squad read-only (:1864-1872) and the faction only as asset-id text; moving many
slots to another faction or squad is a slot-by-slot chore with one undo step each.
═══ SHIPPED ═══
T-937.2 with_batch; squad slotIds lists; side keys; the Attributes modal.
═══ LANGUAGE GATE ═══
Rust (Leptos, wasm).
═══ LOCKED ═══
- reassign_slots lives in state/operations/reassign.rs; the modal only calls it inside with_batch.
- Emptied squads survive; a squad of another faction is refused with a named reason.
- Side keys are updated for every moved slot.
═══ DO ═══
1. Verify on main: test asserting no faction and no squad control; paste the red.
2. Write reassign.rs with fixture tests (moves, refusal, side keys); register in state/operations.rs.
3. Add the faction selector and squad picker acting on the whole selection.
4. Perturbation: skip the side-key update → test red; restore, touch, green.
═══ DO NOT ═══
No changes to store.rs or batch.rs; no implicit squad deletion; no files outside owns.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-939.2
═══ MANUAL ═══
Select five slots, pick another faction: outliner shows them there; one Ctrl+Z brings them back.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-939.3

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-939.3 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-939_3_plan.md; canvas/overlays.rs:160-220; canvas/gestures.rs (translate drag); canvas/mod.rs.
═══ PROBLEM ═══
The gizmo draws Translate X/Y arms (:172-198) and a flat rotate ring (:200-210) only; height is
editable solely through attribute fields, so placing on roofs or bridges is blind.
═══ SHIPPED ═══
Translate/rotate gestures; per-entity z storage; the translate operation path.
═══ LANGUAGE GATE ═══
Rust (Leptos, wasm).
═══ LOCKED ═══
- Geometry, hit test, dy→metres and snapping live in canvas/gizmo_z.rs.
- z is written through the existing translate operation: one operation per gesture.
- Shift suspends snapping; camera scale is sampled at press.
═══ DO ═══
1. Verify on main: test that hit-testing above the origin returns None; paste the red.
2. Write gizmo_z.rs with unit tests (dy→metres, snap); register in canvas/mod.rs.
3. Add the enum variant and draw call in overlays.rs; route the press in gestures.rs; readout.
4. Perturbation: invert the dy sign → unit test red; restore, touch, green.
═══ DO NOT ═══
No connection or badge drawing here (T-939.6); no files outside owns.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-939.3
═══ MANUAL ═══
Drag the Z arm up: entity rises, readout shows metres; Shift disables snapping; Ctrl+Z reverts.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-939.4

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-939.4 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-939_4_plan.md; panels/top_strip.rs:240-260,370-390; panels/context_menu.rs; mission_editor.rs keydown closure; panels/help_modal.rs table.
═══ PROBLEM ═══
Arrange (align, distribute) exists only in the top-strip menu (:249-252, T-645); no context-menu
entry and no chord, so the tools are undiscoverable and slow.
═══ SHIPPED ═══
T-645 Arrange menu and its invokers; the context menu; the keydown closure; the help modal.
═══ LANGUAGE GATE ═══
Rust (Leptos, wasm).
═══ LOCKED ═══
- One shared entry list (label, chord, invoker) in top_strip.rs feeds both menus and the help modal.
- Keydown arms are one-line callers (mission_editor.rs is allowlisted SIZE-3).
- Submenu and chords are inert below two selected entities.
═══ DO ═══
1. Verify on main: test that the context menu lacks Arrange and the align chord is ignored; paste the red.
2. Extract the entry list; render the submenu in context_menu.rs.
3. Add the chord arms; add the help rows.
4. Perturbation: drop one chord arm → keydown test red; restore, touch, green.
═══ DO NOT ═══
No new files; no chord that clashes with an existing help-modal binding; no files outside owns.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-939.4
═══ MANUAL ═══
Right-click a multi-selection: Arrange submenu matches the menu; each chord acts; help modal lists them.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-939.5

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-939.5 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-939_5_plan.md; arsenal/asset_catalog.rs:300-340,440-480; arsenal/mod.rs; panels/dock_right.rs:55-100; T-141 naming helpers.
═══ PROBLEM ═══
The catalog has zero squad rows (:321, :460 drop *_base.et); Compositions are placeable only from
their own palette mode; building a squad means placing slots one by one.
═══ SHIPPED ═══
T-650 Compositions palette (doc-listed); T-141 slot naming; the faction catalog.
═══ LANGUAGE GATE ═══
Rust (Leptos, wasm).
═══ LOCKED ═══
- Defaults compile into arsenal/squad_templates.rs (no API compositions table exists).
- Placement is one doc-store operation: squad + role-named slots; one Ctrl+Z removes all.
- Name clashes get a numeric suffix; nothing is overwritten.
═══ DO ═══
1. Verify on main: test that a shipped faction's catalog has zero squad rows; paste the red.
2. Write squad_templates.rs (defaults, from_composition, suffixing) with tests; register in arsenal/mod.rs.
3. Merge a Squads section into asset_catalog.rs; wire placement in dock_right.rs.
4. Perturbation: drop the suffix logic → naming test red; restore, touch, green.
═══ DO NOT ═══
No API or seed changes; no panels/mod.rs edit; no files outside owns.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-939.5
═══ MANUAL ═══
Place a template twice: two squads with distinct names and role callsigns; Ctrl+Z removes the last.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-939.6

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-939.6 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-939_6_plan.md; canvas/overlays.rs:640-700 (post T-939.3); canvas/mod.rs; the validation findings signal; connection model.
═══ PROBLEM ═══
overlays.rs:654-655 says connections have no map glyph and :679 lists them in a panel only; validation
findings never reach the canvas, so authors hunt for problems and links in lists.
═══ SHIPPED ═══
T-939.3 gizmo layer; validation findings; T-080 connection UI (panel).
═══ LANGUAGE GATE ═══
Rust (Leptos, wasm).
═══ LOCKED ═══
- Badges, wires and tooltips live in canvas/diagnostics_overlay.rs; overlays.rs only mounts it.
- Hidden layers hide their badges and wires; dangling connection ends draw a badge.
- Badge click selects the entity and opens the matching validation entry.
═══ DO ═══
1. Verify on main: test that a slot with a finding renders no badge; paste the red.
2. Write diagnostics_overlay.rs with unit tests (placement, endpoints, hidden filter); register in canvas/mod.rs.
3. Mount it above the gizmo layer in overlays.rs.
4. Perturbation: skip hidden-layer filtering → test red; restore, touch, green.
═══ DO NOT ═══
No changes to the connection model or validation rules; no files outside owns.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-939.6
═══ MANUAL ═══
Break a slot: badge appears, click selects it; connect two entities: wire appears; hide a layer: both vanish.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-939.7

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-939.7 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-939_7_plan.md; panels/outliner.rs:30-60; panels/vehicles_panel.rs:220-290; the virtual list used by outliner_tree.rs:1245.
═══ PROBLEM ═══
VIRTUAL_SLOT_THRESHOLD (:39) gates only virtual_tree; vehicles_panel.rs:231-276 mounts every row;
the outliner re-flattens on every render, so big missions stutter on hover and selection.
═══ SHIPPED ═══
Virtual list for slots; document version signal.
═══ LANGUAGE GATE ═══
Rust (Leptos, wasm).
═══ LOCKED ═══
- Memo keyed on the document version; selection and hover never re-flatten.
- Vehicles use the same virtual list above the same threshold.
- outliner_tree.rs is untouched (T-939.1 owns it).
═══ DO ═══
1. Verify on main: test counting flatten calls across two selection changes shows two; paste the red.
2. Add the memo in outliner.rs.
3. Route vehicles_panel.rs through the virtual list.
4. Perturbation: drop the memo key → call-count test red; restore, touch, green.
═══ DO NOT ═══
No threshold change; no files outside owns.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-939.7
═══ MANUAL ═══
Load a 500-vehicle mission: only visible rows mount; hover and select without stutter.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-939.8

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-939.8 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-939_8_plan.md; panels/dock_left.rs:120-160; mission_editor.rs keydown closure (post T-939.4); panels/help_modal.rs.
═══ PROBLEM ═══
search_document (:134-144, T-697) has no keyboard path; Ctrl+F opens the browser find bar instead.
═══ SHIPPED ═══
T-697 document search; T-939.4 chord arms and help rows.
═══ LANGUAGE GATE ═══
Rust (Leptos, wasm).
═══ LOCKED ═══
- Ctrl+F and Cmd+F preventDefault and call focus_search; inert while a modal owns focus.
- focus_search expands a collapsed dock, focuses the input, selects its text.
- Escape returns focus to the canvas and keeps the query.
═══ DO ═══
1. Verify on main: test dispatching Ctrl+F leaves the input unfocused; paste the red.
2. Add focus_search in dock_left.rs; add the arm in mission_editor.rs; add the help row.
3. Perturbation: drop preventDefault → consumed-event test red; restore, touch, green.
═══ DO NOT ═══
No search-logic changes; no files outside owns.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-939.8
═══ MANUAL ═══
Collapse the dock, press Ctrl+F: dock opens, input focused with text selected; Escape returns to the canvas.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```
