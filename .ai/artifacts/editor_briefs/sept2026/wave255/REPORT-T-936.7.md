# REPORT — T-936.7 · Tactical graphics: phase lines, boundaries, curved arrows

> Transcribed by the command centre: the agent harness refuses report-file writes and requires
> findings be returned as text. Content is the agent's. Independently re-verified by the command
> centre: branch clean, two commits, 17 files (14 owned + 3 overflow), the three overflow paths owned
> by no live ticket and no wave-255 sibling, and the two decoy edits confirmed as the mechanical
> re-points described. `T-936.7.toml` `owns` was widened to cover all three after the fact.
> Merged to main as `30f09d8a444d0094b3e41137fb0648b8313c4726`.

## pwd_branch
```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-936.7
slice/T-936.7
```
Branch point `c73d39668`.

## defect_verified
Proved before any production edit. A payload carrying `tacticalGraphics` flattened **without** it —
the key was absent from the wire document entirely.
```
  left: Null
 right: "phase_line"
test mission::flatten::tests::authored_tactical_graphics_survive_flatten_to_mod_document ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1062 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p map-engine-core --lib`
```

## changes

### T-946.53 — **nine** places, not seven
The brief listed seven. Two more exist, both decoy `tacticalGraphics` witnesses outside the owns
list; both go red the instant the row lands.

| # | Place | Change |
|---|---|---|
| 1 | `extensions.rs` `AUTHORED_BLOCKS` | new row → `tactical_graphics::validate` |
| 2 | `extensions.rs` `assert_eq!(len, 6)` | → `7` |
| 3 | `extensions.rs` negative assert + copy-test decoys | flipped positive + `DOCUMENT_OWNED_BLOCKS` negative; decoys → `notAnAuthoredBlock` |
| 4 | `flatten.rs` `EditorPayload` | `#[serde(rename = "tacticalGraphics")] tactical_graphics: Option<Value>` |
| 5 | `flatten.rs` `authored_block_value` | new match arm |
| 6 | `compile.rs` decoy test (message named this ticket) | re-pointed |
| 7 | `mission.schema.json` | root property + `$defs/tacticalGraphic` + `$defs/tacticalGraphicStyle` |
| **8** | `tasks.rs` decoy | re-pointed — **outside owns** |
| **9** | `spawn_modules.rs` decoy | re-pointed — **outside owns** |

The decoy is re-pointed to `notAnAuthoredBlock`, not to a next block's key: `tacticalGraphics` is the
**seventh and last** T-936 block, so there is no eighth key to hand the baton to. No future
registration can falsify it again. `audio.rs` / `weather.rs` were checked and needed no change.

### The lying comments, corrected in the commit that falsifies them
- `extensions.rs:10-13` "Nothing in `compile.rs`, nothing in `flatten.rs`" → replaced with a named
  **"A ROW IS NOT A WIRE"** section stating the mechanism (named-field `EditorPayload`, no
  `#[serde(flatten)]` catch-all by design, `_ => None`).
- `mission-editor-payload.schema.json:34` "touch neither `compile.rs` nor `flatten.rs`" → same
  correction.
- **Two further copies found while in the files** and also corrected: `extensions.rs`'s
  `ExtensionBlocks` doc, and `flatten.rs:801`'s `extensions` field doc ("with no edit here").

### The structural guard the class never had
`flatten::tests::every_authored_block_key_reaches_the_wire` walks `AUTHORED_BLOCKS` itself,
deserialises a sentinel through the real `EditorPayload` per key, and asserts both
`authored_block_value` and `authored_blocks_root` return it. A future row missing either `flatten.rs`
half now fails **by name** — which is what `weatherTimeline` / `audio` never had (T-946.44, T-946.47).

### Core / schema / canvas
- `mission/tactical_graphics.rs` (new, 15 tests). `min_points` = 2 for the three open polylines,
  **3 for `curved_arrow`** (a Catmull-Rom through two points collapses to the straight
  `axis_of_advance`, making two kinds indistinguishable on the wire). `MAX_POINTS = 128`. Style
  reuses T-673's marker vocabulary verbatim (`color` = `$defs/hexColor`, `alpha` 0..1, `brush` = the
  same eight Eden names) plus `widthM` — the one addition the geometry forces.
- `$defs/polygon` (`minItems: 3`) deliberately **not** reused; a two-point phase line is the
  commonest case. Per-kind floor stays in Rust (JSON Schema cannot make `minItems` depend on a
  sibling without an `if`/`then` per kind, and Rust gives the author a sentence).
- `canvas/tactical_graphics.rs` (new, 13 tests) — T-780's shape: one document read, drawn AND picked.
  Centripetal Catmull-Rom; arrowheads off the final **drawn** span so a curve's head follows its
  tangent; boundary ticks.
- `state/operations/tactical_graphics.rs` (new) — every write is one `update_environment` = one
  Ctrl+Z. Vertex drag writes only session state until pointerup; a release with no travel writes
  nothing. Deleting the last graphic writes `null`, not `[]`, so bytes return to pre-T-936.7.
- `history.rs` — lane rebound from the committed document inside `after_doc_change`; **no existing
  public signature changed** (brief trap 2).
- **No new render lane**: rides `role_id::MISSION_ZONES` via the generic `upload_hairline_segments`.
  `map-engine-render` untouched.

## perturbation
Three, each with a live `Compiling` line (never a replayed verdict — T-596), each restored **and
`touch`ed** (T-421).

**A · `min_points` `Some(2)` → `Some(1)`**
```
   Compiling map-engine-core v0.1.0 (...)
thread 'mission::tactical_graphics::tests::a_one_point_phase_line_is_refused' (574019) panicked at crates/map-engine-core/src/mission/tactical_graphics.rs:568:10:
a one-point phase line is not a line: [AuthoredTacticalGraphic { id: "tg-short", kind: "phase_line", points: [[1.0, 2.0]], label: None, side_key: None, style: None }]
thread 'mission::tactical_graphics::tests::a_malformed_block_is_refused_at_the_carrier_with_a_readable_clause' (574015) panicked at crates/map-engine-core/src/mission/tactical_graphics.rs:818:9:
assertion failed: carried.get("tacticalGraphics").is_none()
test result: FAILED. 14 passed; 2 failed; 0 ignored; 0 measured; 1063 filtered out; finished in 0.00s
```

**B · golden's phase line cut 2 pts → 1** — `hcargo xtask schema validate` exit 1:
```
  FAIL  schema-1_3-tactical-graphics.json
        /tacticalGraphics/0/points [[7620.0,4380.0]] has less than 2 items
```
This is what makes the new `$def` non-vacuous — without a golden carrying the block, nothing
validates it.

**C · `drawn_polyline` stops tessellating `curved_arrow`**
```
   Compiling website-frontend v0.1.0 (...)
assertion `left == right` failed
  left: 4
 right: 26
test result: FAILED. 12 passed; 1 failed; 0 ignored; 0 measured; 1344 filtered out
```
**This caught a weak test of the slice's own.**
`the_pick_follows_the_drawn_curve_not_the_authored_chord` stayed GREEN under it — its probe was read
back out of the function under test, so a collapse to the chord moved the probe onto the chord and
still "hit". Rewritten in the same pass to probe the drawn point furthest from **both** authored
chords, assert clearance exceeds the pick tolerance, and assert the identical click **misses** an
`axis_of_advance` through the same vertices.

## gate_verdict_tail
```
  cargo check              PASS
  wasm32 (frontend)        PASS
  fmt (changed)            PASS
  clippy (changed crates)  PASS
  test (frontend, changed) PASS
  schema                   PASS
  T-278 catalogue drift    PASS
  ... (13 more)            PASS
  no-python (T-620)        PASS

  gate verdict PASS @ 190ad13ca6db recorded: .ai/artifacts/verdicts/T-936.7.json
SLICE GATE: PASS
```
Preceded by `touch_workspace: invalidated 671 workspace .rs file(s) and 133 include_str!/include_bytes!
input(s)` — a real build, not a replay.

Suites run directly (the gate's test step is frontend-only):
- `map-engine-core --all-features` → **1076 passed, 1 failed**. The failure is
  `dem::peaks::tests::everon_peaks_max_above_350`, `Decode("Invalid PNG signature.")` —
  `everon-dem-16bit.png` is an unmaterialised LFS pointer in this worktree (verified: file begins
  `version https://git-lfs.github.com/spec/`). Pre-existing, environmental.
- `website-frontend` → **1357 passed, 0 failed**.
- `schema validate` → `All contracts valid.`
- clippy core (all-features, all-targets) → zero warnings, with a `Checking map-engine-core` line
  after a touch. clippy frontend wasm32 → no errors; `manual_contains` @ `gestures.rs:629` and
  `collapsible_match` @ `gestures.rs:977` are pre-existing.

## files_outside_owns
**Three.** All uncontested this wave, all mechanically forced, all following precedent `33be8aff9`
(T-936.6), which edited exactly this class of sibling file for exactly this reason.

1. **`crates/map-engine-core/src/mission/tasks.rs`** — 6 lines, decoy re-point. Owned by
   T-133 / T-936.2 / T-936.6, none in wave 255.
2. **`crates/map-engine-core/src/mission/spawn_modules.rs`** — 6 lines, same. Owned by T-936.6
   (shipped wave 253).
3. **`packages/tbd-schema/golden-missions/schema-1_3-tactical-graphics.json`** — new. The ticket's
   `approach[]` asks for it; `schema-1_3-tasks.json` (T-936.2) is the direct precedent;
   `schema_gates.rs` globs the directory. Without it the new `$def` is validated by nothing and
   perturbation B is impossible.

The agent did not stop the slice over 1 and 2: leaving them red fails the gate and hands the next
agent a broken tree, which the HARD GATE forbids more strongly than the owns list forbids a four-line
decoy re-point in an uncontested file. **Command-centre disposition:** verified uncontested (no live
ticket and no wave-255 sibling owns any of the three), accepted, and `T-936.7.toml` `owns` widened to
cover all three so the registry matches what shipped.

## found_not_fixed
1. **`begin_tactical_draw` has no caller — the draw tool is complete and unreachable.** Everything
   downstream is live (per-click vertex append in `gestures.rs`, right-click finish, Esc abandon,
   floor-enforcing commit), but the ARM needs a surface and **both candidates are outside owns**: a
   control under `editor/panels/`, or a keybinding — which compiles but fails two census tests in
   `panels/help_modal.rs`: `every_binding_has_a_help_entry` (any new code literal needs a `Shortcut`
   row) and `no_two_listeners_claim_the_same_chord` (`Enter` is already claimed by
   `context_menu.rs`, `attributes_modal.rs`, `orbat_manager.rs`; `SHARED_CHANNELS` exempts only
   `Escape`). One call site under `panels/` makes the path live with no change here. Recorded in code
   on the function; the four `never used` warnings are left **standing** rather than
   `#[allow]`-silenced, because they are the honest signal.
2. **`role_id::MISSION_ZONES` is squatted.** Declared by T-592 for zone rings; no editor code feeds
   it, and `pages/debug/building_interior.rs` already reuses it for an LoS ray. A future zone-ring
   producer must merge into this pack — a second upload to one role replaces the lane rather than
   adding to it, and the last writer wins silently. Stated in the module header; no compiler check
   exists.
3. **`label` reaches the canvas but is not drawn** — no text lane without a new render lane.
4. **Nothing mod-side reads `tacticalGraphics`** — recorded in the schema description rather than
   implied.
5. The `peaks` LFS failure above.

## deviations
1. **The spec prompt's `DO NOT` is stale and was overridden.** `t936_mission_logic.md` says "No
   compile.rs/flatten.rs edits"; the ticket's `requirement[]` carries the T-946.53 correction
   demanding both, `owns[]` includes both, and the defect RED proves the requirement right.
2. **The brief's seven-place list was incomplete** (two more decoys).
3. **A golden mission was added** — T-936.6 added none; the ticket's `approach[]` asks for one and
   the schema half is otherwise unverified.
4. **Draw completion is right-click, not Enter** — forced by the keymap census; reasoned in code at
   both the absent-arm site and the `oncontextmenu` site.
5. **Tactical selection is session state, not a signal on `EditorGestureContext`** — that struct is
   built in `mission_editor.rs` (T-190's this wave).
6. **A tactical draw does not ride `Pending`** — that enum is in `context.rs`; the real consequence
   (`cancel_pending` is a false negative) is named and handled by wiring Esc explicitly, the same fix
   T-792 needed for the zone draw.
7. **The ops module carries no `#[cfg(test)]`** — `operations.rs` is
   `#![cfg(target_arch = "wasm32")]`, so tests there are compiled by nothing natively and would be a
   green over unexamined code. The pure halves (`TacticalDraft::needed`, `mint_graphic_id`) were
   moved to `canvas/tactical_graphics.rs` where they actually run.
8. Brief trap 1 respected: no inline `#[cfg(test)]` anywhere in `flatten.rs` above line 2979
   (production edits ~1409-1470, tests inside the existing `mod tests` ~6250). Every `#[cfg(test)]`
   in the new files sits below all production items.
9. **REPORT file not written** — harness refuses report `.md` writes; content returned as text.

## commits
| sha | subject |
|---|---|
| `cdbba19ec` | T-936.7: tacticalGraphics reaches the wire (schema + core validator) |
| `190ad13ca` | T-936.7: the canvas draws, picks and edits tactical graphics |

Merged to main as `30f09d8a444d0094b3e41137fb0648b8313c4726`.

## manual_checklist
Everything below is reachable today **except** arming a draw (needs the `panels/` call site).
1. `cargo xtask db up` → `mk rust-api` → `mk leptos`; dev-login; open the Mission Creator.
2. **Render**: seed `meta.environment.tacticalGraphics` from
   `golden-missions/schema-1_3-tactical-graphics.json`. Expect a blue 2-point phase line; an amber
   3-point boundary with perpendicular vertex ticks; a green 2-point axis with an arrowhead; a red
   4-point arrow that curves through its vertices with the head on the curve's tangent.
3. **Selection**: click a line → amber. Click empty ground → clears. Click a slot the line passes
   under → the slot wins and the tint clears.
4. **Vertex drag**: press within ~9 px of an authored vertex, drag (line follows live), release.
   Ctrl+Z once restores the vertex; a second Ctrl+Z must undo the edit *before* it.
5. **Drag cancel**: press a vertex, drag, Esc before release → snaps back, no undo step filed.
6. **Click without drag**: press+release a vertex → selects only.
7. **Delete**: with a graphic selected, Delete removes it; Ctrl+Z restores. Deleting the last one
   must remove the key entirely (not leave `"tacticalGraphics": []`).
8. **Wire**: `GET /api/v1/missions/:id/compiled` carries `tacticalGraphics` with all four rows and
   `style.color` verbatim — the acceptance the defect RED was about.
9. **Refusal**: hand-author a one-point phase line and re-fetch `/compiled` → refused with
   `…points has 1 point(s) — a phase_line needs at least 2`; the rest of the document still compiles
   and the mission does not 500.
