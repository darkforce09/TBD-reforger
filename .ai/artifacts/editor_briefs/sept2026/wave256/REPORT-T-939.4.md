# REPORT — T-939.4 · Arrange tools in context menu with shortcuts · wave 256

> Transcribed by the command centre from the slice agent's returned text: its harness blocks
> subagents from writing report `.md` files. Content verbatim; verification note at the end.

## pwd_branch
```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-939.4
slice/T-939.4
```
Clean at start. All cargo/rustfmt via `hcargo`/`hrustfmt`; tests via
`hcargo xtask platform wave test --slice T-939.4 -p website-frontend` (private dir
`tbd-target-T-939.4`). Every green quoted carries its own `Compiling website-frontend` line (T-596).

The brief is **not in the worktree** — it was committed to `main` after the branch point (merge-base
`dc073f7c2`). Read read-only from the main checkout.

## defect_verified
Two tests written first, compiling against unmodified code, both RED (committed as `1be6bad1a`
before any production code):
```
   Compiling website-frontend v0.1.0 (…/worktrees/T-939.4/apps/website/frontend)
running 2 tests
test editor::panels::context_menu::t939_4_arrange_in_the_context_menu::a_multi_selection_offers_arrange ... FAILED
test editor::mission_editor::t939_4_arrange_chords::the_editor_keydown_binds_the_arrange_chords ... FAILED

---- …::a_multi_selection_offers_arrange stdout ----
panicked at apps/website/frontend/src/editor/panels/context_menu.rs:1853:9:
T-939.4: the right-click menu over a multi-selection offers no Arrange row — the align / space / orient tools are reachable only from the top strip. Rows: ["Connect", "Go Here", "Play as the Character", "Select", "Edit", "Transform", "Grid", "Log", "Connections...", "Save Custom Composition...", "Find in Asset Browser...", "Find in Config Viewer...", "Edit Loadout...", "Reset Loadout", "Attributes..."]

---- …::the_editor_keydown_binds_the_arrange_chords stdout ----
panicked at apps/website/frontend/src/editor/mission_editor.rs:3580:13:
T-939.4: the editor keydown has no `"KeyL" if !modk && ev.alt_key() && !ev.shift_key() =>` arm — the Arrange chord is ignored and align / space are reachable only with the mouse

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 1392 filtered out; finished in 0.23s
```

## changes

**`top_strip.rs` — the one list.** `ArrangeKind` (19-variant id enum) + `ArrangeEntry
{kind,label,code,chord}` + `ARRANGE`. The menu bar's rows are no longer typed beside it:
`ARRANGE_ITEMS` is BUILT from `ARRANGE` by a `const fn` and `MENUS`'s tuple is
`("Arrange", &ARRANGE_ITEMS)` — a pin asserts the menu slice points *at* that array
(`std::ptr::eq`), so a fresh slice literal there is red. `MenuItem` gained `#[derive(Clone, Copy)]`
only; no field added, the other ~30 rows untouched. New surface: `ARRANGE`,
`ARRANGE_MIN_SELECTION` (=2), `arrange_for_code`, `arrange_chord_for_label`, `run_arrange`.
`run_action`'s four placement arms collapsed into one arm delegating to `run_arrange_action` — the
same body `run_arrange` calls, so click and chord are literally one path. Menu rows now print their
chord right-aligned (`MENU_CHORD`).

**`context_menu.rs` — the submenu.** `ContextItem::Arrange` (parent) +
`ContextItem::ArrangeRun(ArrangeKind)` (leaf carrying the id, not a label). `is_submenu_parent` +=
Arrange; `submenu_entries` maps `ARRANGE` with `.with_shortcut(e.chord)`. The parent is spliced by a
new `MenuState::base_entries` — **not** added to either `MenuTake`, because those are the pinned
verbatim Eden transcription and cannot see the selection. Renders directly after `Transform`, only
when `target_ids.len() >= 2`. `dispatch` += one arm → `top_strip::run_arrange(kind)`.

**`mission_editor.rs` — the chords.** One wasm-gated window keydown with six arms
(`Alt+L/R/T/B` align left/right/top/bottom, `Alt+H/V` distribute h/v), each a single-expression call
to `arrange_chord` — a 4-line helper that returns `false` below `ARRANGE_MIN_SELECTION` and otherwise
hands to `top_strip::run_arrange`. No placement logic here; a pin refuses
`align_selection`/`space_selection`/`orient_selection`/`apply_pattern_to_selection` in the listener
body. Behind `in_editable_field()`; `prevent_default` only when the chord acted, so a chord below the
floor lets the key through. `window_event_listener` + `on_cleanup` (no `.forget()` leak across
mission switch).

**`help_modal.rs` — census and card.** `editor_surface` += `("mission_editor.rs", …, 1)`, total
13 → 14; the header's four derived numbers re-derived by the pin (26 codes, 14 listeners, 10 modules,
41 bindings). `GROUPS` gained a 7th heading `Arrange`; `SHORTCUTS` gained three rows covering the six
codes — same commit as the listener, so both pins stayed green.

**Chord safety, measured.** `KeyL/KeyT/KeyB/KeyH` were bound by nothing. `KeyR` is claimed only bare
(`!modk && !alt && !shift`) and `KeyV` only under `modk` — neither overlaps `!modk && alt && !shift`
on the census's eight-wide modifier matrix. Guards use exactly the six terms `parse_guard`
understands. **No tactical-draw entry or chord added.**

**Tests, all at the BOTTOM of their file** (below every existing `#[cfg(test)]`):

| file | module | what |
|---|---|---|
| `context_menu.rs` | `t939_4_arrange_in_the_context_menu` | multi-selection offers Arrange · **single selection hides it entirely** (3 states incl. forced `open_submenu`) · submenu == the top strip's list (order/ids/chords/enabled) · Arrange follows Transform |
| `mission_editor.rs` | `t939_4_arrange_chords` | six arms exist · bound codes == the list's chorded rows · **gate-before-return-before-invoker** ordering + live `selection_len()` · thin callers only, `in_editable_field()` present |
| `top_strip.rs` | `t939_4_one_arrange_list` | menu points at the derived array · code/label lookups agree · empty-code sentinel matches nothing · unique labels+codes · floor is 2 · `run_action` and `run_arrange` share one invoker |
| `help_modal.rs` | `t939_4_arrange_help_rows` | help chord text contains the menus' spelling · no orphan Arrange row |

Native suite: **1408 passed, 0 failed** (1394 at branch point).

## perturbation
Dropped the `Alt+T` / Align Top arm, nothing else. RED verbatim:
```
   Compiling website-frontend v0.1.0 (…/worktrees/T-939.4/apps/website/frontend)
test editor::mission_editor::t939_4_arrange_chords::every_chord_arm_is_a_thin_caller_of_the_shared_invoker ... FAILED
test editor::mission_editor::t939_4_arrange_chords::the_editor_keydown_binds_the_arrange_chords ... FAILED
test editor::mission_editor::t669_clipboard_completion::the_help_blurb_counts_the_bindings_correctly ... FAILED
test editor::panels::help_modal::t692_help_covers_every_binding::no_help_entry_invents_a_binding ... FAILED
test editor::panels::help_modal::keymap_census::the_prose_census_numbers_are_derived ... FAILED
failures:
thread '…::every_chord_arm_is_a_thin_caller_of_the_shared_invoker' (2270679) panicked at apps/website/frontend/src/editor/mission_editor.rs:3759:9:
assertion `left == right` failed: T-939.4: every one of the 6 arms must hand off to the shared helper. Body:
  left: 5
 right: 6
thread '…::the_editor_keydown_binds_the_arrange_chords' (2270681) panicked at apps/website/frontend/src/editor/mission_editor.rs:3660:13:
T-939.4: the editor keydown has no `"KeyT" if !modk && ev.alt_key() && !ev.shift_key() =>` arm — the Arrange chord is ignored and align / space are reachable only with the mouse
thread '…::the_help_blurb_counts_the_bindings_correctly' (2270388) panicked at apps/website/frontend/src/editor/mission_editor_tests/t669_clipboard_completion.rs:235:5:
assertion `left == right` failed: T-740: the help table documents 26 distinct codes but the editor binds 25 ({"ArrowDown", "ArrowUp", "Backspace", "BracketLeft", "BracketRight", "Delete", "Digit1", "Digit2", "Digit3", "Enter", "Escape", "KeyA", "KeyB", "KeyC", "KeyD", "KeyE", "KeyG", "KeyH", "KeyL", "KeyR", "KeyV", "KeyX", "KeyY", "KeyZ", "Space"}) — the count in `eden_help`'s header cannot be right about both
  left: 26
 right: 25
thread '…::no_help_entry_invents_a_binding' (2271388) panicked at apps/website/frontend/src/editor/panels/help_modal.rs:1644:9:
thread '…::the_prose_census_numbers_are_derived' (2271368) panicked at apps/website/frontend/src/editor/panels/help_modal.rs:1443:13:
test result: FAILED. 1403 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 27.58s
```
Five pins fired on one dropped arm — three pre-existing. `KeyT` vanishing from the bound-code set in
the `t669` message is the census reading the live listener, not a string match. Restored
(`git checkout --`), `touch`ed, re-ran:
```
   Compiling website-frontend v0.1.0 (…/worktrees/T-939.4/apps/website/frontend)
test result: ok. 1408 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.65s
```

## gate_verdict_tail
`hcargo xtask mk ci-local-leptos` — all four steps pass:
```
:225  cargo clippy -p website-frontend --target wasm32-unknown-unknown --all-targets
:3750 test result: ok. 1408 passed; 0 failed; …
:4220     Finished `release` profile [optimized] target(s) in 1m 45s
:4222 INFO applying new distribution / INFO ✅ success
```
`fmt --check` no diff; wasm32 clippy no `error` line; the trunk release build is what actually
compiles the wasm-gated listener.

`hcargo xtask platform wave gate --slice T-939.4` (detached, polled on `^SLICE GATE:` only):
```
gate: WAITING for the gate lock — holder: slice T-946.86 pid 2357409 … (serialisation, not a hang)
gate: lock acquired after ~120s.
touch_workspace: invalidated 678 workspace .rs file(s) and 134 include_str!/include_bytes! input(s) across 9 member(s)
  cargo check              PASS
  wasm32 (frontend)        PASS
  fmt (changed)            PASS
  clippy (changed crates)  PASS
  test (frontend, changed) PASS
  schema                   PASS
  … (T-278/db_migrate/T-439/T-444/T-440/T-438/T-456/T-468/T-437/T-586/T-296/T-452/no-python all PASS)

  gate verdict PASS @ 098c46782a63 recorded: .ai/artifacts/verdicts/T-939.4.json
SLICE GATE: PASS
```

## files_outside_owns
**None.** `git diff --name-only <merge-base>..HEAD` is exactly the four owned files. No sibling file
touched.

## found_not_fixed

1. **The brief's keydown anchor is stale, and it changed where the listener had to go.** The brief
   says *"`mission_editor.rs` … holds the wasm keydown closure"*. It does not, and has not since
   T-934.14: `add_event_listener_with_callback("keydown"` has exactly two sites in `editor/` —
   `state/history.rs:789` and `canvas/commands.rs:470` — and `mission_editor.rs` contains no
   `KeyboardEvent` at all. `help_modal.rs:622-625` says so: *"the editor's own keydown dispatch moved
   out of `mission_editor.rs` into `canvas/commands.rs` (the page keeps ZERO window-level keydown
   listeners now…)"*. The clean home for six new arms is therefore **`canvas/commands.rs`** —
   **outside this slice's `owns`** (and outside every sibling's), so I did not touch it. Instead the
   page takes back a listener, which is what the ticket requirement literally asks for and is the only
   in-`owns` option that survives Backspace hide-chrome (`top_strip.rs`, where the list lives,
   unmounts behind the `chrome_hidden` gate and would take the chords with it). **Recommend a
   follow-on relocating the six arms into `canvas/commands.rs`'s existing closure and dropping
   `mission_editor.rs` back off the census surface** — pure relocation, arms are already one line
   each.
2. **Tactical draw deliberately absent** — no entry, no chord, nothing in `ARRANGE`. Arrange acts on
   `selection`; tactical graphics live in their own collection. T-946.86 owns it.
3. **Thirteen Arrange rows still have no chord** (4 patterns, 2 centre aligns, along-line, 6 orients)
   — the ticket's scope. Adding one is filling `code`/`chord` on an `ARRANGE` row; the keydown arm
   stays a deliberate separate step so the collision census can see it.
4. **The submenu is 19 rows deep** in an accordion inside an already 15-row menu. That is what
   "entries match the top-strip menu" produces. If a later UX pass wants a short list, the cut belongs
   in `ARRANGE` or a filter over it — the pins will fight a second array, on purpose.
5. **Pre-existing:** `clippy --target wasm32 --all-targets -- -D warnings` reports ~29 lints on files
   I do not own (dead code in `state/operations/*`, `panels/outliner_drag.rs`, `arsenal/mod.rs`,
   `router.rs`). `ci-local-leptos` runs that clippy **without** `-D warnings`, so not a gate failure;
   recorded because a future flip lands there, not here.

## deviations

1. **`mk leptos-gates` was NOT run** — stated explicitly, not silently skipped. The operator's
   `trunk serve --release` is live: **pid 1264262, cwd `…/apps/website/frontend`, listening
   `127.0.0.1:3000`**. The instruction was to run it only if that server is stopped, while also
   leaving `:3000`/`:8080` up — both cannot hold. Two things that bear on the stated reason:
   `Trunk.toml` has `dist = "dist"` (relative), so this worktree's `trunk build --release` writes
   `…/worktrees/T-939.4/apps/website/frontend/dist`, not the main checkout's — the `ci-local-leptos`
   run above did exactly that with the operator's server serving normally — and `gate serve` defaults
   to port 5198, not 3000. So the dist/port collision may not actually bite from a worktree; I still
   did not run it because of the instruction **and** because `gate editor-suite`'s
   persist/save-export/mutations smokes drive the live API on `:8080` against the dev database.
   **Needs an operator call: stop `trunk serve` and re-run `hcargo xtask mk leptos-gates` here, or
   confirm it may run against the live stack.** `leptos-gates` is Chromium-only and deliberately
   outside `wave gate` (`mk_build.rs:340-342`), so the PASS above is unaffected.
2. **`mission_editor.rs` rejoins the keydown census surface**, undoing part of T-934.14's "zero
   window-level keydown listeners". Knowing, documented at both ends; see found_not_fixed #1 for the
   proper undo. Census count bumped 13 → 14 with the reason written down.
3. **Top-strip Arrange rows now print their chord.** Not in the acceptance, but the requirement is a
   shared list of *(label, chord, invoker)* consumed by *both* menus — a `chord` field no surface
   renders would be a field, not a shared list.
4. **`run_action`'s four placement arms collapsed into one delegating arm** — behaviour-preserving
   refactor of owned code, and what makes "the chord performs the same operation as its menu entry"
   structural. Pinned by `the_menu_click_and_the_chord_share_one_invoker`.
5. Brief read read-only from the main checkout (not in the worktree). Nothing written outside the
   worktree.

## commits
```
098c46782  T-939.4: rustfmt
0fcfcad1c  T-939.4: the acceptance lines, as tests
1ef3589e5  T-939.4: Arrange reaches the right-click menu and the keyboard
1be6bad1a  T-939.4: the defect, as two failing tests
```
Branch `slice/T-939.4`, merge-base `dc073f7c2`, 4 files, **+1080 / −124**. Working tree clean. Not
shipped, not merged, not pushed. `:3000` and `:8080` still listening.

## manual_checklist
Nothing below is covered natively — the listener and dispatch are wasm-only. Run
`cargo xtask mk leptos`, dev-login, open a mission.

1. **Menu teaches the keys.** Arrange menu with ≥1 selected: 19 rows; the four aligns and two
   distributes show `Alt + L/R/T/B/H/V` right-aligned, the other 13 show nothing.
2. **Right-click, 2+ selected.** Marquee two, right-click one: **Arrange ▸** sits directly under
   `Transform`. Expand: 19 indented rows, same labels and order as the menu bar, six showing chords.
3. **Right-click, 1 selected.** No Arrange row at all — absent, not greyed. Right-click bare ground:
   also absent.
4. **The chord acts.** Two entities at different X, `Alt + L` → align left, one undo step. Same for
   `Alt + T/B/R`; `Alt + H/V` distribute.
5. **Inert below the floor.** One selected: `Alt + L` does nothing, no undo step, no toast. Nothing
   selected: same.
6. **Menu and chord agree.** Same pair twice — context-menu `Align Left`, then `Alt + L`. Identical
   result and undo granularity.
7. **Typing is safe.** Caret in an Attributes text field, type letters `l r t b h v` **with Alt held**
   — nothing moves.
8. **Chrome hidden.** `Backspace` to hide the interface, then `Alt + L` on a multi-selection: still
   aligns. (The reason the listener is on the page, not the strip.)
9. **Help card.** Help ▸ Keyboard Shortcuts: an **Arrange** section, three rows, chords spelled
   exactly as the menu spells them.
10. **No stray key.** `Alt + K`, `Alt + G`, `4`, `5` still do nothing (reserved-unbound digits not
    taken).

---

## Command-centre verification (not the agent's words)

Re-checked independently before acceptance, 2026-09-08:

- `files_outside_owns []` — **true**. Exactly the four owned files, working tree clean.
- **The stale-anchor finding is CONFIRMED, and the brief was wrong.** On `main`,
  `add_event_listener_with_callback("keydown"` has exactly two sites under
  `apps/website/frontend/src/editor/` — `state/history.rs:789` and `canvas/commands.rs:470` — and
  `mission_editor.rs` contains **0** occurrences of `KeyboardEvent`. The stale claim originated in
  T-939.4's own `context` line and was propagated into the brief unverified.
- **Deviation 2 is accepted for this wave, not endorsed.** Re-homing the six arms into
  `canvas/commands.rs` would require widening `owns` mid-wave, and a repack while T-946.86 is in
  flight is the wave-236 hazard (an incidental repack reshaping the wave being gated). The agent was
  correct to refuse the widen. Filed as a follow-on instead.
- **Deviation 1 is a genuine open operator question**, not an agent deferral: `mk leptos-gates` needs
  either `trunk serve` stopped or explicit permission to run against the live `:8080` stack.
