# Wave 127 — adversarial verification (merged main @ 7ad4a6a9)

Base d64f5cfa · merges 61efc6a2 (T-775), aa9f5762 (T-753), 7ad4a6a9 (T-773) · gate claim PASS 30/30.
Scope check: `git diff d64f5cfa..HEAD --stat` touches exactly the six declared files — nothing else.
Main was left exactly as found: every mutation below was restored and re-proven green; final
`git status` clean, `git diff` empty, HEAD 7ad4a6a9.

Environment note (not a finding): this sandbox has no C linker; all builds/tests were executed on
the host via `host-spawn` + `cargo xtask ai run`, per the runbook. All rebuilds were forced
(`touch` on all six wave files) before any green was trusted — see the T-742 section.

---

## FINDINGS

### F-1 · MAJOR · apps/website/frontend/src/attributes.rs:593-644
**The browser's native number-input stepper still destroys authored precision — the T-775 defect
class survives through ArrowUp/ArrowDown and the spinner.**
- **Evidence.** The input at attributes.rs:593 is `type="number"` with **no `step` attribute** and
  no `appearance` CSS suppressing the spinner (CONTROL const, attributes.rs:35). The keydown
  handler (:605-644) intercepts only `Enter`/`PageUp`/`PageDown` and falls through (`_ => return`,
  no `prevent_default`) for everything else; `grep ArrowUp` over attributes.rs: zero hits. Per the
  WHATWG stepping algorithm (and Chrome's implementation, which is the gate browser), a number
  input with default `step=1`/step base 0 **snaps to the step grid** on ArrowUp/ArrowDown/spinner:
  on a focused field holding `412.37`, one ArrowUp sets the DOM value to `413` (not `413.37`),
  fires `input` → `draft = "413"` (:603), and blur/Enter commits `413` through `on_commit`.
  (Asserted from the HTML spec's stepping algorithm; no browser could be driven from this sandbox —
  worth one manual keypress in the gate browser to witness.)
- **Impact.** A single arrow keypress — far more instinctive than PageUp — silently rounds an
  authored coordinate to an integer, mints a real write + undo step, and (being an x/y write)
  also zeroes an authored Z via F-2. This is precisely the defect T-775 shipped to remove, alive
  on the adjacent key.
- **Disposition.** Fix in wave: add `step="any"` to the `number_field` input (arrows then step ±1
  from the *current* value with no grid snap: 412.37 → 413.37), and preferably intercept
  `ArrowUp`/`ArrowDown` in the existing keydown handler into the same `nudged()` path as
  PageUp/PageDown so modifiers scale consistently. Extend the T-775 source pin to require
  `step="any"` (or the Arrow interception) so it cannot regress.

### F-2 · MAJOR · crates/map-engine-core/src/doc/store.rs:2779-2783 (reachable from apps/website/frontend/src/attributes.rs:803-816)
**CONFIRMED (T-775's unfixed claim, item 4): a deliberate X or Y edit in the Attributes tab
silently discards a manually authored Z.**
- **Evidence.** `update_slot_position` (store.rs:2779-2783): `if z is None && (x.is_some() ||
  y.is_some()) { pz = 0.0 }`. The Attributes X field commits
  `commit_position(targets, Some(x), None, None, None)` (attributes.rs:808) →
  `attrs_update_position(id, Some(x), None, None, None)` (editor_ops.rs:1305-1334) → the mutator
  with `z = None`. Chain read end-to-end; every hop passes `None` for z. The comment's "DEM
  re-sampled JS-side" (editor_ops.rs:1303) is **not true for this path**: the drag path passes
  explicitly sampled `zs` (`move_entities`, `commit_positions` editor_ops.rs:1608), but nothing
  follows an attrs commit to re-sample — the document keeps the literal `0.0`. Same holds for the
  multi-selection stamp (`attrs_update_position_multi`, every member's Z zeroed by an X stamp).
- **Impact.** Operator authors Z (rooftop/parapet placement), later fine-tunes X by 1 m in the
  Attributes tab → Z drops to the deck, no warning, buried inside the same undo step as the X edit.
  Destroys operator-authored work.
- **Disposition.** Fix in wave: in `attrs_update_position`/`_multi`, when x/y are Some and z is
  None, read the slot's current z and pass it through (manual z sticks under an Attributes edit),
  OR sample the DEM as the drag path does. Note the z-policy doc on `update_slot_position`
  (store.rs:2747-2749) claims parity with `ydoc.updateSlotPosition` — change the frontend callers,
  not the core mutator, if that parity is still load-bearing; and correct the "DEM re-sampled
  JS-side" comments (editor_ops.rs:1303, attributes.rs:573-575) which launder the zeroing as
  temporary.

### F-3 · MINOR · apps/website/frontend/src/attributes.rs:583 (pin at :1395-1401)
**The `!gate.differs()` multi-stamp exemption is pinned by source scan only — nothing behavioural.**
- **Evidence.** The only guard is the string assert
  `commit.contains("let unchanged = !gate.differs() && n == value;")` in
  `an_untouched_field_commits_nothing_...`. `number_field` is `#[cfg(target_arch = "wasm32")]` and
  the repo has no wasm-bindgen-test harness (grep: zero hits), so no test *executes* the decision.
  The pin is honest about being a source pin and my mutations prove it fires (M4b below), but a
  semantically-equivalent refactor (e.g. inverting to `gate.differs() || n != value` as a commit
  condition) would be correct code that turns the pin red, and a subtly wrong rewrite that keeps
  the literal string shape would pass.
- **Disposition.** Fix in wave: extract the decision into a native pure function next to
  `field_display`/`nudged` — e.g. `fn should_commit(differs: bool, n: f64, settled: f64) -> bool`
  — call it from `commit`, and add a behavioural test (differs+equal ⇒ commit; agree+equal ⇒ skip;
  non-finite ⇒ refuse). Keep the source pin only for "commit calls should_commit".

### F-4 · NIT · apps/website/frontend/src/eden_settings.rs:2532-2534
**Stale doc comment describes the pre-T-753 mirror arrangement.**
- **Evidence.** "`eden_env` holds `FLOW_DEFAULT_TIMELIMIT_S = 5400` and friends — but those
  **mirror the literals** `mission::flatten`'s `ModFlow` splices in". Since aa9f5762 they mirror
  nothing: they are the compiler's own constants re-exported (eden_env.rs:207-209). T-753's own
  eden_env rewrite argues that exactly this kind of outlived comment "is how a real drift gets
  waved through". eden_settings.rs was outside the wave's owns, so this is residue, not a botch.
- **Disposition.** One-line reword: "…but those are the compiler's own constants re-exported
  (`pub use` from `mission::flatten`), a COMPILER FALLBACK, not a schema declaration."

No BLOCKER found. Specifically: no gate or test in this wave reports success on code it never
examined — every new pin was fed its own defect and went red (proofs below).

---

## Is main safe to build the next wave on?

**Yes** — with F-1 and F-2 queued for this wave's fix pass (both are precision/data-destruction
defects in shipped behaviour, neither corrupts existing documents at rest, neither breaks a build
or a gate).

---

## VERIFIED-CLEAN REGISTER — re-proved, not trusted

**T-775 / claim 1 (the ticket's own fix description is wrong; both halves needed).** Reasoning
re-derived and CONFIRMED: with a rounded seed, the skip can only compare the parse of the rounded
string (blur of an untouched `412.37` field parses `412`, `412 != 412.37`, commits the rounding),
and the nudge steps from the rounded base (PageUp → 413) — so the skip alone fixes nothing; and
without the skip, an idle focus/blur re-commits an equal value (dirty mission + undo step + F-2's
z-zeroing). Both halves present at attributes.rs:548-549 (`shown`/`exact` split) and :583-586
(skip), each independently load-bearing, and each independently pinned — mutation M4a (seed
reverted to `format!("{}", value.round())`) → pin RED at attributes.rs:1372; mutation M4b
(`let unchanged = false;`) → pin RED at :1397. Both restored, suite re-green.

**T-775 / claim 2 (displayed value cannot reach the document).** Attacked and FAILED to break via:
tab-between-fields (draft seeds from `exact`; Rust's `{}` float Display is shortest-round-trip, so
parse(seed) == value exactly and the skip holds); paste (equal value skips, different value is a
real edit); multi-selection (differs ⇒ seed and display both empty, empty draft parses to Err ⇒ no
accidental commit; a typed stamp commits because of the `!gate.differs()` exemption); re-render
while focused (doc_tick rebuild recreates the field's signals — the display string never enters any
draft); the Z field (commits `z = Some`, no terrain-follow branch); the nudge (reads the
exact-seeded draft, and its 3-decimal quantum equals `field_display`'s precision — pinned
behaviourally by the round-trip loop in
`the_display_keeps_the_working_resolution_and_never_flattens_to_an_integer`, which ran natively and
passed). The claim as *literally stated* survives: no path commits the display string. What breaks
the ticket's broader promise is the native stepper (F-1), whose committed value derives from the
browser's step algorithm, not from `field_display`.

**T-775 / claim 3.** `!gate.differs()` is load-bearing (Gate::differs is construction-time
`opt.is_some()`, attributes.rs:137-139 — latching the checkbox does not clear it, so a stamp onto
an opted-in differing field commits even when it equals the seed member's value). Pinned by source
only — filed as F-3.

**T-775 / claim 4.** CONFIRMED reachable — filed as F-2.

**T-753 / item 5 (`pub use` preserves the path).** Workspace grep: the four constants are defined
ONCE (flatten.rs:1777-1783); eden_env re-exports (eden_env.rs:207-209); eden_settings.rs:613 and
:1086-1099 consume the same names with identical values and unchanged `read_flow_seconds`
semantics. Proven compiling both natively (909-test suite) and on wasm32
(`cargo check --target wasm32-unknown-unknown` exit 0, zero warnings in any wave file).

**T-753 / item 6 (pin honesty).** HONEST, not the circular pattern: the old defect was frontend
literals asserted against the frontend's own copy while the authority moved freely; the new
in-file test restates literals in the SAME file as the single definition (a deliberate-edit
prompt), and the cross-crate guard now reads the authority's constant. Also spot-checked the
mod-side party the messages name: `TBD_SafestartManager.c:114 DEFAULT_COUNTDOWN_SECONDS = 300`
matches `FLOW_DEFAULT_SAFESTART_S`; `PolicyFromString`/`ArmRoundClock`/`OnEnterBriefing` all exist
in the mod scripts. (That Enfusion-side 300 remains an unenforceable cross-boundary copy — the
test message documents it honestly; no cargo test can reach a `.c` file.)

**T-753 / item 7 (the experiment, re-run).** Edited `FLOW_DEFAULT_BRIEFING_S` 600 → 900 in
flatten.rs. RED, three ways: core `flow_default_literals_are_the_contract` (left: 900, right: 600),
core `the_flow_defaults_are_the_values_the_committed_golden_carries` (golden mission), and —
the hole that shipped green in wave 115 — frontend
`eden_env::tests::flow_defaults_mirror_the_compiled_constants` FAILED cross-crate with
"map_engine_core::mission::flatten::FLOW_DEFAULT_BRIEFING_S moved… left: 900, right: 600".
Restored; `git diff` empty; both guards re-run green (2 passed core, 1 passed frontend).
Caveat for the record: flatten.rs's tests are behind the `mission` feature — a bare
`cargo test -p map-engine-core` (no `--all-features`) runs only 139 of 626 lib tests and would
MISS the core-side prompt; the frontend cross-crate guard fires regardless, which is what makes
the 600→900 edit un-shippable either way.

**T-773 / item 8 (do the new pins examine live code?).** Yes — both go through
`class_r_scrub::live_code` (arsenal.rs:3921), which cuts everything from the first `#[cfg(test)]`
to EOF (test modules are file-final in both files — checked), blanks string literals so a needle in
an assertion or toast message cannot satisfy a call-shaped pin, and `split_only` (arsenal.rs:3934)
REFUSES 0 or ≥2 marker matches rather than guessing. Mutation-tested against the programme's T-759
defect shape: (M2) reintroduced the exact shipped defect in `server_intel::copy_address`
(`let _ = win.navigator().clipboard().write_text(…)` + unconditional success toast) →
`class_r_copy_address_routes_through_the_awaited_clipboard_helper` RED on its first assert; (M3)
rewrote `write_clipboard` to fire-and-forget (`let _ = clipboard.write_text(&text);
toasts.success(…)`) → `class_r_write_clipboard_toasts_only_on_the_resolve_arm` RED on "must be
AWAITED". T-775's pins use the same scrubber (`attrs_src()` = `live_code`, attributes.rs:985-987;
`live_source` only for the one literal-bearing assert, correctly). Both files restored, re-green.

**T-773 / item 9 (one clipboard path; visibility-only promotion).** Crate-wide grep for
`write_text` / `.clipboard()` / `execCommand` / `ClipboardEvent`: the sole write is
mission_commands.rs:1091 inside `write_clipboard`; client.rs's navigator use is Web Locks, not
clipboard. `pub use imp::*` pre-existed at the wave base (d64f5cfa, then line 1359 — checked with
`git show`), and the diff to `write_clipboard` is `fn` → `pub(crate) fn` plus docs: visibility-only
as claimed. Cross-module resolution of `crate::mission_commands::write_clipboard` from
server_intel's wasm-only closure proven by the wasm32 check (exit 0).

**T-742 rebuild race.** All six wave files `touch`ed before the first trusted run.
`cargo test -p website-frontend`: **909 passed / 0 failed / 0 ignored**, and all five new tests
confirmed IN THE BINARY by exact-name `--list` (attributes ×2, eden_env flow mirror,
mission_commands class_r, server_intel t773). `cargo test -p map-engine-core --all-features`:
**625 passed / 0 failed / 1 ignored** (the ignored one is `regen_compiler_shaped_fixture`, a
deliberately-ignored fixture regenerator — pre-existing, not wave-related). No green in this report
predates a forced rebuild.

**Everon peaks on main.** `dem::peaks::tests::everon_peaks_max_above_350` PASSES on main
(ran inside the --all-features suite and again by name) — the PNG is real here; the worktree-only
LFS failure did not manifest, as expected.

## Attacked and FAILED to break (nobody needs to re-audit these)

1. The T-775 display/seed/skip triangle against tab-cycling, paste, multi-selection stamp,
   re-render-while-focused, the Z field, and the PageUp/PageDown nudge (the stepper got through —
   F-1 — but through the browser, not through any of these paths).
2. The T-753 single-definition claim: no hidden copy of any `FLOW_DEFAULT_*` literal anywhere in
   apps/ or crates/; the 600→900 edit is RED in two crates and cannot ship green.
3. The honesty of `flow_default_literals_are_the_contract` as a confirmation prompt.
4. Both T-773 pins against the exact historical defect and against the T-759 self-matching-scan
   defect (the scrubber's test-module cut + literal blanking + unique-match refusal all held).
5. The one-clipboard-path claim, including legacy paths (`execCommand`) and the `pub use` plumbing.
6. The wave's build integrity on both targets: native test suites green post-forced-rebuild, wasm32
   check clean with zero warnings in wave files.

## Focused re-verification of the fix pass

Re-verifier: Fable 5, 2026-08-08, on merged main at 0cc47a6b. Scope: the four fix commits only
(a38e7e73, 6d622a66, a5760231, 0cc47a6b). Working tree left untouched (mutations applied and
reverted via `git checkout --`; `git status` clean at exit, this file the only write).

**Execution environment note (matters for trust):** this session runs in a container whose glibc
(2.36) cannot execute the host-built gate binaries and which ships no C toolchain. Every green
below was still EXECUTED, not inferred: the suite was run through the host loader
(`/run/host/usr/lib64`), and mutation rebuilds were linked through a host-gcc shim into a scratch
CARGO_TARGET_DIR (never the shared one, never `target-gate-*`). The shared-target race (T-742)
was sidestepped entirely: nothing here reused a stale artifact — provenance of every binary was
proven by embedded-string content, not mtime.

### Findings

- MINOR | apps/website/frontend/src/editor_ops.rs:642 (`paste_at_cursor`) | Ctrl+V writes
  `zs.push(0.0)` for every pasted slot ("DEM not ready — byte-parity"), and the extras filter
  explicitly drops `position.z`, so a copy of a rooftop slot pastes at z=0 while the same paste
  preserves every OTHER authored key (T-220) — the same "nothing re-samples after the React
  deletion" reasoning the four fixes are built on. Pre-existing; NOT a caller of the
  `update_slot_position` / `move_entities*` family the fixes claim to close, there is no cut (copy
  only, original keeps its z), and no fix commit claims this path. Filed so the z-family ledger is
  complete, not as a defect in the fixes. Same class: `place_composition` (store.rs:2394/2409)
  stamps z=0.0 for both slots AND vehicles on composition drop (symmetric, capture never stored z).
- NIT | apps/website/frontend/src/mission_editor.rs:8265-8293 | The F-6 order pin proves `zs` is
  built from `slot_ids.iter().map(` and that the same `slot_ids` token is passed to the call, but
  it cannot see an edit BETWEEN the two statements (e.g. an inserted `slot_ids.sort()` /
  `retain()` would keep both greps green while breaking `zs[i]`↔`slot_ids[i]`). On current main
  the two statements are adjacent (3815-3826, nothing between) — correspondence verified by
  reading the live code, and `move_entities_in_txn` (store.rs:5343) indexes `zs.get(i)` over
  `ids` positionally, with locked-slot `continue` NOT desyncing `i`. Residual pin limitation
  only; no code change needed for this wave.

Nothing else. Specifically: no BLOCKER, no MAJOR. The four gate-binary test failures I hit on
first run (mission_title_prefer t570 x2, server_control t270 x2) are this container's missing
`cc` inside the self-compiling pin harnesses — they pass with a linker on PATH and pass on the
host; not a main defect.

### Are the wave-127 fixes complete and correct — **yes.**
### Is `main` safe to close this wave and build wave 128 on — **yes.**

### VERIFIED-CLEAN REGISTER (re-proved first-hand, not taken on trust)

1. **The z-family is closed.** `.move_entities(` has ZERO frontend call sites (method-call grep
   over apps/website/frontend/src; only comments/pin literals mention it).
   `.move_entities_and_vehicles(` has exactly ONE — mission_editor.rs:3826, the fixed arm.
   `core.update_slot_position(` has exactly five: editor_ops.rs:1391 and 1443 (F-2, z resolved
   via `keep_z_rows`/`slot_z` before the write), 1701 (F-5, resolved), 1514 and 1897
   (rotation-only, x=y=z=None). The mutator's zero arm (store.rs:2781 `else if x.is_some() ||
   y.is_some()`) provably cannot fire on the rotation-only shape — read, not assumed. Paste,
   composition-place, `orbat_add_slot`, and `place_at` are CREATE paths on other mutators (see
   MINOR above); undo/redo is the yrs UndoManager restoring exact document state
   (mission_history.rs:233/252 → store.rs:3831/3836) — no position rewrite; there is no frontend
   caller of `set_slot_position`; there is no cut-selection (Ctrl+X) path.
2. **F-6 ordering is structural on main.** `zs` is built by a total map (`unwrap_or(0.0)`) over
   the SAME `slot_ids` Vec passed as `ids`, statements adjacent, no filter/sort/dedup between
   (mission_editor.rs:3815-3826, read in full). Vehicle-only drag: `(!slot_ids.is_empty())`
   guard → no document read, empty `zs`, `move_entities_in_txn` iterates nothing. Mixed drag:
   vehicles ride `move_vehicles_in_txn`, which never touches z (store.rs:5444+). The
   delta-as-coordinate call `keep_z_rows(core, Some(dx), Some(dy), None)` is sound: the helper
   only asks WHICH fields are written (`z.is_none() && (x.is_some() || y.is_some())`).
3. **All five new/renamed pins are live and bite — proven by mutation, not by reading.** Six
   mutations applied to production code, crate rebuilt, suite run: EXACTLY the six matched tests
   went RED, all 907 others stayed green; reverted, rebuilt, 913/913 green again.
   - remove `step="any"` → `the_nudge_is_bound_to_the_page_and_arrow_keys_and_never_steps_on_a_grid` RED
   - `should_commit` body → `true` → `should_commit_writes_only_a_new_finite_value` RED (the
     behavioural test, exactly the F-3 design: the wiring pin alone stays green on a body lie)
   - commit bypasses `should_commit` → `an_untouched_field_commits_nothing_and_the_draft_seeds_from_the_exact_value` RED
   - delete `z.or_else(...)` in `attrs_update_position` → `an_attributes_x_or_y_commit_carries_the_slots_current_z_back_in` RED
   - `commit_positions` z back to `None` → `a_placement_commit_carries_each_slots_current_z_back_in` RED
   - drag `zs` back to `vec![0.0; n]` → `drag_move_commit_carries_each_slots_current_z` RED
4. **No pin is self-matching (T-759 class).** `class_r_scrub::scrub` (arsenal.rs:3899) runs
   `cut_test_module()` FIRST, on the comment- and literal-blanked scan (so a `#[cfg(test)]`
   inside a string or comment cannot fake the boundary), killing from the first real
   `#[cfg(test)]` to EOF before any pin greps. F-1 uses `live_source`+`only_body`; F-2/F-5 use
   `live_code`/`live_source`+`only_body` on editor_ops.rs (cross-file — their own module text is
   not even in the scanned file); F-6 uses `live_code` on a `MissionEditorPage`-anchored slice
   whose first `#[cfg(test)]` precedes t648_transform; F-3's behavioural test greps nothing.
   `only_body` panics on 0 AND on 2+ matches (shadow-copy refusal). Self-demonstrating check:
   the F-6 comment block itself contains the text "vec![0.0; n]" and the pin asserts its
   absence — the pin is green, so comments are provably stripped before the grep.
5. **The F-6 pin (and the other four) EXECUTE natively.** `t648_transform` is plain
   `#[cfg(test)]` (mission_editor.rs:7826), not wasm-gated; the scrubber's `cfg_eval` returns
   None for `target_arch` predicates so wasm production code survives into the scanned text.
   All five confirmed by EXACT name in `--list` (913 tests) and run by exact name: 5 passed.
   Then the entire claim re-established from scratch: I rebuilt the crate from current main
   sources in my own target dir and ran all 913 — 913 passed, 0 failed.
6. **The gate binary is genuine.** `target-gate-frontend`'s test binary (mtime 00:24:14, after
   the last fix commit 00:21:33) contains all five new test names, contains the post-fix wiring
   string `should_commit(gate.differs(), n, value)`, and contains ZERO occurrences of the
   pre-fix `let unchanged = !gate.differs() && n == value;`, the old pin name
   `the_nudge_is_bound_to_page_up_and_page_down_and_eats_the_scroll`, or the old
   `commit_positions` comment — it was compiled from post-fix sources, not a stale reuse.
   (The gate's 30/30 browser half was not re-run here — no Chrome in this container — but its
   native input is proven fresh, and the 913 native result is independently reproduced.)
7. **F-1 regression surface.** Read in full (attributes.rs:645-688): the keydown handler's
   `_ => return` sits BEFORE `ev.prevent_default()`, so exactly four keys are claimed
   (PageUp/PageDown/ArrowUp/ArrowDown); Enter blurs (which commits) and returns; typing, Tab,
   Home/End, selection and browser shortcuts fall through untouched. `step="any"` removes the
   step-mismatch grid entirely (no form submit exists in the modal to be affected). The full
   suite green confirms no existing pin objects to either half.
8. **F-2/F-5 hoisting and the hidden-layer/f32 hazards.** `keep_z_rows` reads
   `raw_slot_rows` → `core.slots_json()` (store.rs:699), the raw doc map — hidden-layer and
   `editorHidden` slots INCLUDED (the omission is `materialize`/SoA-only, per the T-665/T-701
   doc on store.rs:707-721), values exact f64 straight off the row. `commit_positions` hoists
   the read above the loop (editor_ops.rs:1686-1690, first-moved-slot probe; no slot moved → no
   read); `attrs_update_position_multi` hoists at 1420; the drag reads once per commit
   (mission_editor.rs:3803-3814). `SelPos::z` (the f32 SoA column) is carried but never
   committed for slots — the write at 1701 takes `slot_z` output only.
9. **F-4 is now true.** `pub use map_engine_core::mission::flatten::{... FLOW_DEFAULT_TIMELIMIT_S}`
   at eden_env.rs:207-209 is real; `pub const FLOW_DEFAULT_TIMELIMIT_S: i64 = 5400` at
   flatten.rs:1781; eden_env.rs:457 asserts the 5400 against the compiler's constant.
10. **The disclosed out-of-scope doc edit is accurate and inert.** The reworded `keep_z_rows`
    doc (editor_ops.rs:1247-1256) matches implemented behaviour (drag caller outside the module,
    pair widened `pub(crate)`, rows read once, mapped over `slot_ids` in order). All raw
    `include_str!("editor_ops.rs")` readers (orbat_manager.rs, eden_zones.rs, eden_tree.rs) are
    green, and the scrubbed pins cannot see comments at all — no pin matched the old text.
11. **Everon peaks passes on MAIN** — `dem::peaks::tests::everon_peaks_max_above_350` ok, inside
    a full `map-engine-core --all-features` run: 625 passed, 0 failed, 1 ignored. The worktree
    LFS failure does not manifest on main. Not a finding.

### Attacked and FAILED to break

1. The z-family closure: every method-call site of `update_slot_position`,
   `move_entities_and_vehicles`, `move_entities` in the frontend, plus paste, duplicate-less
   clipboard, composition place, undo/redo, import/hydrate, and both rotation-only paths.
2. The F-6 `zs`↔`slot_ids` correspondence (code reading + core indexing semantics + locked-slot
   skip alignment + vehicle-only and mixed drags + zero-fill and re-source mutations).
3. All five new/renamed pins, by neutering the exact production code each guards — six
   mutations, six reds, zero collateral, restoration to 913/913.
4. The scrubber's test-module cut, literal blanking, cfg handling, and unique-match refusal as
   used by these pins (including the self-referencing "vec![0.0; n]" comment decoy).
5. The F-1 keydown handler against typing, Tab, Home/End, Enter, selection, shortcuts, and
   prevent_default scope; `step="any"` against form/input semantics and every existing pin.
6. The hoisted batch reads against per-entity O(document) regression, and the raw-row z read
   against the f32 SoA and hidden-layer omission (T-665/T-701).
7. The freshness of every green I report: binaries proven post-fix by embedded content;
   the 913 count reproduced from a from-source rebuild in an isolated target dir.
