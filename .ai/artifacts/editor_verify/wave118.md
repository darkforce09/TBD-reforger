# Wave 118 — adversarial verification (editor factory)

Range: `4e362214` (base, wave 117 closed) → `86921ad4` (HEAD).
Merges verified: T-637 `4d0fa47b`, T-698 `1e8bf5e6`, T-699 `86921ad4`. Merge topology is clean
(three single-parent branches off base, no duplicated trees — `4d0fa47b^{tree} == e0fcaec4^{tree}`).
Verifier ran `cargo test -p website-frontend` at HEAD before and after all perturbations:
**877 passed / 0 failed** both times; working tree byte-identical to `86921ad4` at close
(`git status` clean, `git diff HEAD` empty). `map-engine-core` untouched by this range — not re-run
(pre-existing `dem::peaks` / clippy debt not re-reported per instruction).

---

## Findings

### MAJOR | apps/website/frontend/src/eden_tree.rs:323–330 | the `CONTAINER_H` justification is false, and the defect it excused is real

**Evidence.** The shipped comment (eden_tree.rs:324–329) and the slice's argument claim the T-169
smoke "asserts `rendered < total` at a seeded 80 slots — a check that a tall enough window would
silently stop testing." The smoke asserts more than that. `tools/tbd-tools/src/smokes.rs:2651`
(`v3_windowRendersSubset`) pins `e_rend1 > 0 && e_rend1 < e_total1 && e_rend1 <= 60`, and the
`<= 60` cap **predates this wave** (present at base `4e362214`, blame T-339).

Arithmetic (derived from eden_tree.rs:999–1001: `rendered = ceil(H/ROW_H) + 2×OVERSCAN`,
`OVERSCAN = 6`, `ROW_H = 16`; totals from smokes.rs:2620/2632: 80 seeded + 8 unfiled ≈ 88):

- At the slice's own cited 1080p (its measured 958 px tree region): `ceil(958/16)+12 = 72`
  rendered. `72 < 88` → `rendered < total` **still passes** (so windowing would NOT have "stopped
  being tested" by that assertion), and `72 > 60` → **v3 fails loudly**.
- At the gate's actual 1440×900 viewport (cdp.rs:384): region ≈ 778 px → `ceil(778/16)+12 = 61`
  rendered → v3 fails by 1.

Both halves of the recorded claim are false: an h-full scroller would have gone **red**, not
silently green, and the assertion the slice cited would not have gone quiet either.

**Impact.** The defect the justification excused stands: above `VIRTUAL_SLOT_THRESHOLD` (50 rows)
the tree renders in a fixed 420 px inner scroller inside the dock's `flex-1` region — at 1080p
that is ~538 px of the "~900 px of void" the ticket's own measurement block claims is "now the
tree's." The ≤50-row eager path is genuinely fixed, and the densification itself is real (26 rows
per 420 px vs 17), so the miss is scoped to large missions. But the ticket's headline claim fails
for exactly the missions dense trees exist for, and the in-code comment now enshrines a wrong
description of what the gate tests.

**Disposition.** Not fixed, not ticketed (standing instruction). The honest resolutions are: size
the scroller from the region and raise/parametrise the smoke's `<= 60` cap in the same change, or
keep the cap and say the true reason (the smoke's rendered cap, not `rendered < total`). The
comment at eden_tree.rs:324–329 should not survive the next wave as written.

### MINOR | apps/website/frontend/src/arsenal.rs:1100–1110, editor_ops.rs:2083, store.rs:2733 | "counts what the sink actually took" is stronger prose than the code

**Evidence.** `commit_writes` increments `done` once per closure **invocation**, unconditionally.
The production sink `MissionDocCore::update_slot_loadout` (crates/map-engine-core/src/doc/
store.rs:2733) returns `()` and silently no-ops when the id is not in `self.slots` (`if let Some`
falls through). So in production `commits` always equals `writes.len()`, the `WARNING` arm of both
receipts is unreachable, and a stale-id write would be counted as landed.

**Impact.** Today unreachable in practice: targets are resolved and committed in one synchronous
task (`apply_loadout_buffer_to_selection` — even `window.confirm` blocks the JS loop), so an id
cannot go stale between plan and commit. The receipt machinery itself is honest and tested
(`remove_receipt(3, 2)` does warn), and the native pin has real teeth — I re-ran the
silently-skipped-write perturbation (skip-but-count in `commit_writes`) and it went red at
arsenal.rs:5641 with "3 writes planned, sink took 2", which is exactly the assertion T-736's
failed pin lacked. The gap is that the *sink* offers no acknowledgement for the counter to count;
if `update_slot_loadout` ever grows an early-return (lock, validation), the receipt overstates
with no test noticing.

**Disposition.** Latent, documented here so the T-732 batch-write fix returns a real count
(`update_slot_loadout` → `bool`) instead of inheriting the invocation counter.

### MINOR | apps/website/frontend/src/attributes.rs:374 vs arsenal.rs:1909–1911 | the T-649 honesty banner is now false for three verbs (item 18, rated)

**Evidence.** On a multi-selection the Loadout tab shows the T-649 banner "Loadout edits apply to
this one entity (id), not to the whole selection" — and, lower in the same panel, three T-699
controls that act on the whole selection, beside a counter-note "These three act on the whole
selection, not on this one entity — unlike every pick above."

**Impact.** The banner's unqualified claim is false for Copy/Apply/Remove Everything; the author
is shown two contradictory sentences in one modal. Mitigating: T-699 knew (`arsenal.rs:1904–1908`
says so), the counter-note sits directly on the verbs it governs, `attributes.rs` was outside the
slice's owns, and every per-pick statement the banner makes is still true. **Rated MINOR** — a
wording defect with a local correction, not a data hazard. Note the banner text is pinned by
`mission_editor.rs:8745`, so amending it must touch that pin in the same change.

### MINOR | T-699's item-17 residue claim about `live_production_src()` is false in its alarming form

**Evidence.** The claim: `include_str!` of the whole of arsenal.rs including its ~2,400-line test
module leaves "every pin one un-narrowed `.contains` away from T-759's hollow shape." The input is
the whole file, but `class_r_scrub::scrub()`'s **first** pass is `cut_test_module()`
(arsenal.rs:3905, 3670–3675), which kills everything from the file's first `#[cfg(test)]`
(arsenal.rs:2673 — before all production-pin needles' fixtures) to EOF. Proved by perturbation,
not by reading: with `cut_test_module()` disabled, the suite goes **18 red** across seven files
(ambiguity refusals and fixture bleed in eden_top_strip, client, server_control, ui,
create_mission_dialog, attributes, mission_editor) — i.e. when the cut stops working, the pins
fail LOUD, they do not go hollow-green. Restored; suite back to 877/877.

**Impact.** The repo-wide `class_r_scrub` machinery (21 files, 93 references) shares the cut and
the `split_only` ambiguity refusal; this is **not** "bigger than T-759" and nobody should schedule
remediation off that residue line. The kernel of truth: a future pin that bypasses the scrubber
and greps raw `include_str!` output IS exposed — the two raw-`include_str!` pins this wave added
(eden_layout.rs:1147, 1193) are negative-contains with assembled needles, which is the safe shape.

### NIT | apps/website/frontend/src/eden_help.rs:244 | the ControlsHint close button shrank with the shared recipe

`BTN_ICON`'s `p-1.5 → p-0.5` shrinks the close button's hit box from ~36 px to ~20 px. Behaviour
intact (it composes `HOVER_FILL` at the call site, so the hover fill and transition the old baked
states provided survive; it is never disabled, so losing the baked `disabled:` pair costs nothing)
and the brightening is the fix reaching this caller as intended. Just a small target in a modal
header with room to spare.

### Verified pre-existing (T-698's claim checked, NOT a new report): server_intel.rs:215–223

Confirmed exactly as the slice reported: `let _ = win.navigator().clipboard().write_text(...)`
drops the promise and `toasts.success("Server address copied")` fires unconditionally — the
reported-success-over-nothing shape T-698's own `write_clipboard` exists to prevent. (Their cited
:219 is the middle of the statement; the call chain spans 215–223.)

---

## Is `main` safe to build the next wave on — **yes.**

Nothing in this wave breaks input correctness, loses data, or reports success over unexamined
code. The MAJOR is a scoped density shortfall on >50-row trees plus a false justification in a
comment; both receipts, both gates, and every geometry contract hold under live perturbation.

---

## Verified-clean register — attacked and FAILED to break (no re-audit needed)

**T-637**
1. **The unprojection contract, both directions, live.** Perturbed `DOCK_LEFT_MOUNT` `w-60 → w-64`
   → `the_mounted_dock_width_and_the_pointer_unprojection_are_one_number` red at
   eden_layout.rs:1018. Perturbed `DOCK_PX` `240 → 256` (class untouched) → that pin **plus**
   `the_docks_are_one_equalised_width` red. Restored; green. `mission_editor` renders the four
   mount consts (exactly-four `eden_layout::DOCK_` reads pinned); `select_tool` reads only the
   `dock_left_px()/dock_right_px()/strip_top_px()/toolbelt_band_px()` accessors (select_tool.rs:534–539) — no literal to drift.
2. **`STRIP_TOP_PX` = 48.0, untouched** — the diff carries it as context only; the equalisation
   pin asserts it and `ROW_MENUS_PX + ROW_TOOLS_PX == STRIP_TOP_PX` still splits it.
3. **The T-634 tripwire, both bodies read in the diff.** The inverted pin asserts everything the
   original did modulo the rename (bright rest, 4 identical call sites, banned fallback recipe,
   rule-3 title) **plus** two new constraints (no baked `hover:`/`disabled:` in the recipe; no
   `TOOL_ICON` anywhere in the body). Not weakened — strengthened. "Same glass" now compares
   `STRIP_ROWS` against `DOCK_L` verbatim-prefix (the property `STRIP` only proxied) and keeps the
   `border-b border-white/10` edge check.
4. **eden_top_strip stayed inside its permission.** All 10 hunks accounted for: imports, local
   const deletions (the fold-back), four `TOOL_ICON→BTN_ICON` class swaps, and the two t634 test
   rewrites. Zero markup/layout changes. All 42 strip tests green: 48 px two-row split, one
   primary action, exports behind one secondary trigger, gear-beside-environment, bidirectional
   ellipsis pin, settle-only scrubber, `ControlsHint open=hint_open` inside the gated subtree.
5. **`BTN_ICON` blast radius is exactly five call sites** (four strip + eden_help close), same set
   as at base — no caller dropped, every one composes `HOVER_FILL` (+`DISABLED_GLYPH` where
   disableable). See NIT above for the one cosmetic consequence.
6. **One row geometry.** All six recipes (`ROW`, `ROW_ACTIVE`, `PALETTE_LEAF`, `ROW_STATIC`,
   `ROW_UNFILED`, `ROW_FACTION`) are `ROW_GEOM`-prefixed and state `h-4`; `ROW_H` is read back via
   `tw_len_px`, not retyped; `py-*`/`p-*` banned beside the stated height; `ROW_ACTIVE`'s
   `border-t` sits inside the border-box so no recipe is taller; every glyph is `size-4` cell +
   `text-sm leading-none` (whole-production-half pin); SL badge `h-3 leading-none shrink-0`.
7. **CONTAINER_H** — see MAJOR; the *density* half of the pins is real and green.
8. **The five deleted buttons were pure decoration**: all `disabled=true`, titled "(visual only)",
   T-172 B9 React-mock parity, no handlers to orphan; negative pin `!contains("strip_btn")` holds.

**T-698**
9. **The grid pin is behavioural and has teeth.** It drives real `edge_eastings`/`edge_northings`
   through a bounded `OrthoCamera` and equates exporter output with the furniture's own label text
   at every visible intersection (≥4 enforced), plus a between-lines six-figure case. Perturbed
   `format_grid_ref`'s easting to a divergent derivation → red at mission_commands.rs:1910.
   Restored. Both halves of production share the one `grid_ref_3digit`, so no second convention
   exists to drift.
10. **The clipboard write is awaited** (`JsFuture::from(promise).await`), success toast on the
    `Ok` arm only, failure arm names the browser's reason; `clipboard_api()` refuses an
    undefined/null `navigator.clipboard` with the secure-context message instead of throwing;
    empty-classname selections refuse rather than copying an empty string; the harness preview
    bridge deliberately never touches the clipboard.
11. **The selection bridge cannot drift**: `__editorSelection.ids()` closes over the same
    `Rc<RefCell<Vec<String>>>` minted at mission_editor.rs:2173 and handed to the ops context —
    one allocation, two readers, no copy.

**T-699**
12. **The randomisation**: widening multiply (`(r × len) >> 64`), per-ordinal decorrelated
    SplitMix64, deterministic in `(seed, ordinal, len)`, degenerate at `len ≤ 1`; uniformity
    asserted at ±5% over 30k draws, non-lockstep, replay, reroll — all read and green. **The gate
    runs over the whole buffer before any draw** (plan_apply source order + the 64-seed behavioural
    pin `a_bad_entry_refuses_the_apply_whatever_the_die_says`); `Err` plans zero writes and the
    committer is only reached on `Ok`. Gate equivalence with `try_import` is structural (one
    extracted `loadout_rule_refusals`, verbatim) and pinned reason-for-reason.
13. **The anti-reseed**: `stripped_loadout()` emits explicit `cargo: []` with the wear vocabulary
    drawn from `ROWS`; `rules::seed_cargo` (the same function the live
    `editor_ops::seed_cargo_in_core` path calls) demonstrably fires on `None` and demonstrably
    refuses the stripped document — behavioural test, green.
14. **Undo honesty pin re-run**: the silently-skipped-write perturbation goes red with the exact
    "planned vs sink took" message T-736's pin could not produce; receipts state N undo steps from
    the commit count and warn on mismatch. (Prose overclaim about the sink → MINOR above.)
15. **No inheritance, no per-category verbs**: `BufferedLoadout` is bytes + a receipt-only source
    id; the anti-inheritance test drops the sources and the plan stays complete; the ops pin bans
    the six obvious per-category verb names and repo grep finds none; "inherit" appears only in
    comments explaining its absence.

**Cross-slice**
16. **No weakened pin in any `#[cfg(test)]` edit.** The only removed assertion across all three
    diffs is the T-634 premise line replaced by its strictly-stronger inversion (finding: none).
    T-698/T-699 test changes are pure additions; no test module was deleted anywhere in the range.
17. **`class_r_scrub` is sound and self-protecting** — proved by disabling `cut_test_module()`
    (18 pins go red across 7 files) and by its own decoy battery + `split_only` ambiguity refusal.
    The T-699 residue line overstates the hazard (MINOR above).
18. Banner contradiction verified and rated (MINOR above).

Full suite at HEAD: 877/877 before and after; tree left byte-identical to `86921ad4`.
