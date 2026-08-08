# Wave 143 — adversarial verification (merged main @ 53380c03)

Base 93d09841 · merges dd2b2916 (T-743) · 7e540a31 (T-777) · 2ad6e714 (T-782) · plan-row 53380c03.
Wave shape verified sane: three slice commits off base, three merge commits whose stats match the
slice stats byte-for-byte (no double application), then the tsv row.

Harness discipline: all builds/tests on the HOST via `distrobox-host-exec`, private
`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target-verify-w143` (and `-wt` for the perturbation
worktree) — **both deleted before this report was written**. Perturbations were made in a detached
`git worktree` at `/home/Samuel/.cache/w143-verify-wt` (removed + pruned), never in main.
`git status --porcelain` on main is empty at close.

## Measured totals (list vs run, both reported — MEASURED, not trusted)

| target | `--list` | run |
|---|---|---|
| map-engine-core lib (`--all-features`) | 637 | 636 passed + 1 ignored = 637 |
| tests/camera_props.rs | 5 | 5 passed |
| tests/deckgl_ortho_parity.rs | 5 | 5 passed |
| **tests/paste_keeps_authored_z.rs (NEW)** | **2** | **2 passed** |
| doc-tests | 3 | 3 passed |
| website-frontend | 1050 | 1050 passed |

- List total == run total on every target (no foreign-binary symptom).
- Lib 637 = the brief's expected 635 **+ 2** new T-743 store tests — consistent.
- The 1 ignored lib test is `mission::flatten::tests::regen_compiler_shaped_fixture`, a
  pre-existing `#[ignore]` fixture-regen helper; not a skip of anything this wave shipped.
- `dem::peaks::tests::everon_peaks_max_above_350` **appeared in the run output and PASSED**
  (`test dem::peaks::tests::everon_peaks_max_above_350 ... ok`), under `--all-features` (png on).
  Its absence was not accepted as a pass. The DEM asset on main is real: 71,911,548 bytes,
  `filter: lfs` attribute, `git status` clean → the slice that materialised it restored state.

## FINDINGS

### F-1 | MINOR | crates/map-engine-core/src/doc/store.rs:2935-2937
**What:** `paste_slots`' doc comment still says `zs[i]` "is the JS-sampled DEM elevation at the
clamped paste position (0 when the DEM is not ready — the vitest case, byte-parity-preserving)".
Since T-777 the only live caller passes the **authored SOURCE elevation off the clipboard**; the
overruled byte-parity rationale survives verbatim on the very seam it was overruled at. The
attributes.rs pin bans the phrase "DEM not ready" in `editor_ops.rs` only, so nothing constrains
this copy.
**Proof:** read of store.rs:2930-2938 post-merge vs the T-777 comment at editor_ops.rs:628-650.
**Impact:** the next reader of the core seam is told the retired contract; this is exactly the
"overruled rationale left standing" failure T-777's own test calls out — one file over.
**Disposition:** reword the `zs[i]` clause of the doc comment (one sentence, doc-only, T-777's
seam). Fix shape: "zs[i] is the caller-resolved elevation — since T-777 the frontend passes each
copied slot's authored z; 0.0 only when the clipboard row carries no finite z."

### F-2 | MINOR | apps/website/frontend/src/dto.rs:847-849 (disclosed by the T-782 slice, outside its owns)
**What:** the `max_players` doc comment still says "The editor now shows both side by side rather
than letting either quietly win; see `eden_settings::PLAYER_COUNT_DISAGREE_NOTE`" — the constant
was renamed to `PLAYER_COUNT_RULING_NOTE` and the side-by-side presentation is gone.
**Proof:** repo-wide grep for `PLAYER_COUNT_DISAGREE_NOTE`: this is the **only** surviving code
reference (all other hits are historical ticket-registry/queue text quoting the pre-fix state,
which is what those records are for). The slice disclosed this one; I searched for others and
found none.
**Impact:** doc-drift of the defect class the programme keeps finding; rustdoc link is dead-named.
**Disposition:** two-line doc edit in dto.rs — retarget the link to `PLAYER_COUNT_RULING_NOTE`
and replace "shows both side by side …" with "shows the derived count as the player figure and
the declared cap below it, labelled".

### F-3 | NIT (harness bookkeeping, not code)
**What:** the dispatch's expected website-frontend total (1045) matches neither base nor head.
**Proof (derivation):** head measured 1050 (list AND run). The wave diff adds exactly 7 `#[test]`
to the frontend (`git diff 93d09841..53380c03 -- apps/website/frontend | grep -cE '^\+.*#\[test\]'`
= 7, removals = 0) → base was 1043. The map-engine-core expectation (635) was correct.
**Disposition:** correct the number in the wave ledger; nothing to change in code.

No BLOCKER. No MAJOR.

## Is main safe — **YES.**

## VERIFIED-CLEAN REGISTER — what I attacked and FAILED to break

**T-743.1 — the golden-churn contradiction (attacked hard, clean).** At base 93d09841:
`PASTE_NUDGE` appears in exactly 5 places — 4 comments/docs and its one consumer, the
`_ => (PASTE_NUDGE, PASTE_NUDGE)` arm. `git grep` at base over `*.rs`, plus fixture sweeps
(`crates/map-engine-core/tests/fixtures`, `apps/website/frontend/tests/fixtures`,
`tools/tbd-tools/fixtures`, `scripts/mod/fixtures`, all `*.json/*.snap/*.golden`): **zero** test,
golden or fixture encodes the +20. Both pre-existing `paste_slots` tests at base pass
`Some(100.0),Some(100.0)` / `Some(100.0),Some(200.0)` anchors. The wave diff modifies **no
existing test and no fixture**. So "zero goldens churned" is TRUE, and the ticket's "golden-test
blast radius" was the wave-113 verifier's conservative assumption about an arm that in fact had
zero coverage. The slice's contradiction of the ticket is verified **in the slice's favour**; no
golden was silently updated. (The only spec artifact carrying the 20 is
`docs/specs/.../t056_copy_paste.md`, which quotes the JS oracle as history — correct to keep.)

**T-743.2 — the shared-arm split (attacked, clean).** `paste_at_cursor` has exactly two
production call sites, both in the keydown listener: the plain arm calls
`paste_at_cursor(Some(ax), Some(ay))` only after `plain_paste_anchor` resolves, else returns
`false`; `(None, None)` is unique to the Shift arm and its uniqueness is pinned by count over the
whole arm list (mission_editor.rs:10708-10715). No context-menu or other path reaches it. Plain
paste cannot become accidental paste-at-original. Panic surface walked: `container` is a resolved
element (`container_ref.get_untracked()` guard at :2811, clone at :3404 — same pattern the
pre-existing Select-All arm uses at :3588); `get_bounding_client_rect` cannot panic;
`engine.try_borrow().ok()` absorbs a mid-frame borrow; `unproject_xy` returns `[NaN, NaN]` on a
singular matrix (ortho.rs:289-306) rather than panicking, and the `is_finite` filter turns that
into a declined paste; `cx.zip(cy)` cannot half-anchor. No engine / pre-boot → `None` → arm
returns `false`, keypress falls through. Mid-pan reads the live target through the same
`frozen_camera` the CUR read-out uses.

**T-743.3 — exact landing (proved both directions).** `t743_paste_without_anchor_lands_on_source_coordinates`
(non-integral, asymmetric coords) is green on main; in the perturbation worktree with the arm
restored to `(20.0, 20.0)` it goes RED with `left: Some(1254.5) / right: Some(1234.5)` while the
anchored-paste sibling correctly **stays green** — the pin is selective, not a tautology. Clamp:
source coordinates are already in-range, and `paste_slots` clamps after a zero translate, so
exactness survives. Overlap-on-original: the paste sets the selection to the freshly minted copy
ids (editor_ops.rs:753) and undo is one transaction over those ids — undo cannot confuse copy
with original; the click-pick ambiguity of two coincident handles is the ruled-for behaviour's
inherent consequence, with the copy pre-selected as the mitigation.

**T-777.4 — the clipboard-z argument (attacked, sound and faithful).** `z_rows` is built from
`clip` — the same snapshot `x`/`y`/`rotation` already read — keyed by source id, hoisted above
the walk. Deleted-source paste and cross-mission paste resolve by construction (no live-document
read anywhere on the z path; `raw_slot_rows` is not called by `paste_at_cursor`). A moved
original after copy cannot contaminate the copy's z. `slot_z` reads `position.z`, finite-filtered,
off exactly the raw rows `copy_selection` filed.

**T-777.5 — not a second vocabulary (verified).** `keep_z_rows` is a **guard**:
`(z.is_none() && (x.is_some() || y.is_some())).then(|| raw_slot_rows(core))` — it answers
"could this `update_slot_position`-shaped write terrain-follow?", a question about an Option
signature `paste_slots` does not have; its output is also the wrong rows (live document) for this
path. The shared **reader** `slot_z` is what wave 127 made the vocabulary, and it is reused. One
reader, two guards each answering their own mutator's question — no third z-resolution path.

**T-777.6 — order proof (attacked, unbreakable as shipped).** One walk over `&clip`; `ids.push`
and `zs.push` (and every other column push) are unconditional in the same iteration; the only
`continue` is inside the inner extras-key loop (per-key, cannot skip a row); no
sort/retain/reverse/dedup/swap/rotate between build and `core.paste_slots(...)`; the
attributes.rs pin additionally enforces exactly-one-push per vector and the region ban. A clip
row with a missing/non-string id degrades to `z = 0.0` via `slot_z(..).unwrap_or(0.0)` — it
still pushes, so no desync is reachable. `z_rows`' `filter_map` only shapes the lookup map, not
the arrays.

**T-777.7 — the acceptance actually executes and actually constrains (measured + red-proved).**
Gate runs `cargo test -p map-engine-core --all-features` (wave.sh:2730), which enables `doc`;
the target listed 2 tests and ran 2 (table above) — not feature-excluded. Perturbation (in the
worktree): `let z = zs.get(i)...` → `let z = 0.0;` — **both** tests FAIL, verbatim:
`left: Some(0.0) / right: Some(37.3)` with the full slot dump showing `"z":0`. The test fails if
`paste_slots` stops honouring `zs`.

**THE COMBINED BEHAVIOUR (highest-value check — empirically PASSED).** Neither slice could test
T-743 + T-777 together; I did, against unperturbed main: a `paste_slots` call with **no anchor**
and zs `[37.3, -4.75]` landed both slots EXACTLY on their source x/y AND each kept its own z
(scratch integration test, 6/6 assertions, then deleted). The full user path composes:
Shift+V → `paste_at_cursor(None, None)` → `zs` from clipboard via `slot_z` → no-anchor arm
translates by nothing → z written from `zs[i]` unconditionally in both anchor arms.

**Wave-127 z-rule (clean).** The wave diff adds zero calls to `update_slot_position` or
`move_entities_and_vehicles` (only a comment mentions the former). No new caller can pass
`z = None` on an x/y write.

**T-782.8 — every display path (verified).** `player_figure()` is exact-body-pinned to
`self.placed`; `placed` comes from `doc_handle()` → `slot_count`, with an honest 0 when the doc
host has not mounted (eden_settings.rs:1131-1139); a mission whose row never loaded has
`declared: None` → the declared block simply does not render, and the Players figure is still the
derived count. Empty document shows 0, pinned against `declared: Some(64)` explicitly.

**T-782.9 — rename drift (swept, one survivor, already disclosed → F-2).** Repo-wide grep:
`dto.rs:848` is the only code reference to the old name. Ticket-registry/queue/docs hits are
historical quotations of the pre-fix state.

**T-782.10 — compile path (untouched, proved by hunk list).** The wave touches store.rs,
mission_editor.rs, eden_help.rs, eden_settings.rs, editor_ops.rs, attributes.rs, arsenal.rs,
gap_analysis.md, wave_plan.tsv, and the new test file — **not** dto.rs, not flatten.rs, not any
handler. `compiled_meta` (dto.rs:901) and `flatten.rs` are byte-identical to base. `max_players`
still reaches the compiled mission; nothing decided the reserved compile-path question.
`player_range: [1, max_players]` unchanged at flatten.rs:2527.

**T-782.11 — the flatten claim (verified precisely, for the operator).** flatten.rs:2510-2514:
`let max_players = if mission.max_players < 1 { (doc_slots.len() as i64).max(1) } else { mission.max_players };`
So yes — derivation already exists for declared `< 1`, and making it unconditional is literally
one line (replace the conditional with the derive). Two precision notes the operator should have:
the derived value is **floored at 1** (an empty mission compiles `max_players = 1`, not 0), and
it counts **flattened** slots (`doc_slots.len()`), which is the compile's own slot set — if the
editor's `slot_count` and the compile's flattened set ever diverge (e.g. a future compile-time
filter), the derived figure follows the compile, not the editor display.

**Arsenal cites + pin (verified).** `pub fn set_loadout(` is at editor_ops.rs:2072 and its
`after_local_edit` tail at :2088 — both cites in arsenal.rs (:40, :19 of the 1380 block) are
correct. The pin `t739::arsenal_cites_live_set_loadout_lines` computes the line numbers from live
source and asserts the doc contains them — it constrains (it is what went red in-slice and forced
this very edit), and it ran green in the 1050.

**place_composition (confirmed unchanged, NOT filed).** Hard-coded z in exactly three arms —
store.rs:2440 (slot), :2455 (vehicle), :2484 (object) via `position_any(wx, wy, 0.0, rot)`;
zero hits for `place_composition` in the wave diff. T-781's, as briefed.

**Hygiene (clean).** No `.py` in the diff; no `tmp_`/scratch test survived (the new integration
test is registered, feature-consistent with the gate, and named for its ticket); working tree
clean; wave_plan.tsv rows 143 present and consistent with the merges; gap_analysis.md correction
is factual and matches the shipped code; eden_help row copy unchanged-and-now-true verified
against the deleted nudge.

## Deviations disclosed by this verifier
- I briefly applied the zs perturbation to main's store.rs before running anything against it;
  the harness denied the run, I reverted immediately, and `git status` confirmed main byte-clean.
  All perturbation RUNS happened only in the detached worktree.
- A scratch combined-behaviour test file existed transiently under
  `crates/map-engine-core/tests/` (run against unperturbed main, then deleted; tree clean).
- Both private target dirs and the perturbation worktree were deleted; `git worktree prune` run.
