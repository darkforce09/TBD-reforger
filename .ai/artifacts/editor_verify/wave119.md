# Wave 119 — adversarial verification (editor factory, LAST wave)

Range: `6dfe4ade` → `63b3bb4f` (T-697 `dc3acbf6`, T-700 `9484d5ce`, T-703 `63b3bb4f`).
Suite on merged HEAD: **905 passed / 0 failed** (`cargo test -p website-frontend` via xtask, exit 0).
Nothing was fixed, committed, or filed. `main` is exactly as found.

---

## Findings

### MAJOR | eden_help.rs:556 (`editor_surface()`) | The census misses 2 of 13 live editor-surface listeners

**Evidence.** The census declares six modules / 11 listeners and every T-703 pin holds against that
declaration. But `mission_editor.rs:4399` mounts `faction_manager::FactionManagerDialog` and
`mission_editor.rs:4402` mounts `eden_chrome::OrbatManagerDialog` — and `eden_chrome.rs:39` is a
bare re-export of `orbat_manager::OrbatManagerDialog` (component at `orbat_manager.rs:226`). Each
installs a window-level `window_event_listener(leptos::ev::keydown, …)` — `faction_manager.rs:70`
and `orbat_manager.rs:249` — that is live whenever the Mission Creator is up. Neither file is in
`editor_surface()`; neither exclusion is covered by the scope note (which excuses only `ui`'s
Dialog/Sheet and `layout`'s nav). So the true editor surface is **13** window keydown listeners,
the census sees **11**, and the module header's "eleven window-level keydown listeners in six
editor-surface modules" — derived, per T-740 — is derived from an incomplete input.

**Impact.** Today: none exploitable. I read both missed bodies: each binds **Escape only**, each is
gated on `open.get_untracked()`, and Escape is the declared shared channel — so no collision and no
undocumented binding ships tonight. Forward: both listeners sit outside
`every_editor_surface_listener_is_censused` (no growth tripwire), outside
`every_shared_channel_claimant_reads_live_state` (their state-gating is unpinned), and outside the
T-692 coverage pins. A key added to either dialog's listener would be an undocumented,
collision-uncheck­ed binding with every pin green — the exact defect shape T-738 found in the old
2-of-11 census, reproduced at 11-of-13. The slice's headline promise ("DISCOVERS every window-level
keydown closure in the editor surface rather than being handed a list") is what does not hold.

**Disposition.** Not a BLOCKER by the letter — the pins examine exactly the code they name, real
listeners, really parsed (this is not a hollow gate), and no collision exists today (verified by
hand across all 13). It is a MAJOR because the ticket's central claim — the census is complete —
is false, and the operator's own framing ("a collision test that is itself wrong … licenses
everyone to stop looking") applies squarely. Fix is one-line-per-file: add both files to
`editor_surface()` with expected count 1 each (total 13); both then also fall under the
shared-channel claimant pin for free. Not fixed per standing instruction.

### MAJOR (pre-existing, reported by T-700, unfixed) | attributes.rs:506,509-523,539 | Focus + blur with no typing commits the ROUNDED value over the doc's precise one

**Evidence.** `number_field`: `rounded = StoredValue::new(format!("{}", value.round()))` (line 506);
`on:focus` seeds the draft from it (535); `on:blur=move |_| commit()` fires **unconditionally**
(539); `commit` parses the draft and calls `on_commit(n)` with no changed-check (516-523). Downstream,
`editor_ops::attrs_update_position` (editor_ops.rs:1293-1321) has **no value-equality skip**: it
calls `core.update_slot_position(...)` and then `after_local_edit()` — a real write and an undo
step. Path: click into an `x = 412.37` field, click away → `on_commit(412.0)` → doc now holds 412.

**Impact.** Silent corruption of authored precision — a tab-through of the Transform section
degrades every coordinate it touches to integers, marks the doc dirty, and mints undo steps the
operator never meant. **T-700's nudge widens the blast radius**: PageUp on that field nudges from
the rounded seed (412 → 413), so the new feature commits `413`, not `413.37` — the nudge inherits
the rounding through `seed()`. Rated MAJOR: it can destroy operator-authored work, silently;
recovery via undo exists only if the operator notices.

**Disposition.** Reproduced by code-path analysis at every layer (seed → draft → unconditional blur
commit → unconditional core write). Pre-existing (all cited lines are unchanged context in the
T-700 diff); T-700 reported it honestly and did not fix, which was in-scope discipline. Needs its
own ticket after the run; not filed per standing instruction.

### MINOR | eden_help.rs:376 | "Escape is claimed by nine listeners" is unpinned prose and true only under one reading

Census-internal Escape claiming sites number **ten**: nine `ev.key()` listeners plus the editor
keydown's guarded `"Escape" if !modk` arm — the sentence separates the arm out, so "nine" is
defensible, but it is exactly the kind of retyped count T-740 exists to kill (the derived-numbers
pin covers only the `//!` header, not this module doc), and with the two uncensused dialogs the
real pile-up is twelve. If Finding 1 is fixed, this sentence goes stale silently.

### NIT | eden_help.rs:953-961 | The shared-channel live-state pin is a per-listener substring, not a per-claim gate

`every_shared_channel_claimant_reads_live_state` passes if `get_untracked()` (or `.escape()`)
appears **anywhere** in the claimant's source. A listener whose Escape path is unconditional but
which reads an unrelated signal untracked elsewhere would pass. All ten current claimants genuinely
gate their Escape path (read each by hand), so this is latent, not live.

### NIT | eden_help.rs:1084-1094 | `there_is_exactly_one_extractor` scans only the six surface files + eden_help

A fifth `fn keydown_arms(` in any other module (`eden_dock_left`, `editor_ops`, `ui`, …) would go
undetected. Repo-wide grep confirms none exists today (sole definition: eden_help.rs:829;
`keydown_arms_drive_snap_and_variant_state` does not match the needle).

### NIT | eden_help.rs:684-686 | A keydown closure reading neither `ev.key()` nor `ev.code()` is silently dropped from discovery

The filter runs before the count, so such a listener never inflates `found` — a *new* listener
using another accessor idiom (or a closure param not named `ev`) would be invisible rather than
red. Partially mitigated: a renamed-accessor body that still mentions `ev.key()`/`ev.code()` but
yields no arm goes red via the empty-bindings check (1009-1016), and a third registration idiom is
already acknowledged as invisible (LISTENER_HEADS comment, 567-568).

### NIT | eden_dock_left.rs:1527-1537, 547-550 | A faction-only hit misattributes its matched field

A plain query that hits only via the faction/folder path (e.g. `BLUFOR`) returns every entity of
that faction — deliberate, documented subtree semantics — but `search_document` records
`field = ` the entity's **first text attribute**, so the row's title reads `matched name "Alpha
1-1"` when the name matched nothing. Cosmetic honesty gap in an otherwise honest surface.

---

## Is `main` safe — **yes.**

Both MAJORs are census-completeness / pre-existing-precision issues with no data at risk from
tonight's merges themselves; no gate is hollow; no pin was weakened; 905/905 green on merged HEAD.

---

## Verified-clean register — re-proved, not taken on trust

**T-703 (attacked hardest, per brief):**
- **Census re-derived independently, twice.** (a) grep of both registration idioms across every
  frontend module; (b) my own parser (scratchpad, not committed) replicating slice→scrub→arm-walk.
  Both agree with the census on its declared scope: mission_editor 4 (3 `window_event_listener` +
  1 raw `Closure`), mission_history 1, attributes 1, eden_top_strip 1, context_menu 1,
  eden_settings 3 = **11 listeners, 21 distinct codes** — the header's three derived numbers
  (21/11/6) are all true of the scanned scope. My parser counts **32 bindings**; the brief's
  "39 bindings" figure appears nowhere in code, commit, or artifacts — the code never claims it.
  The scope itself is short two files (Finding 1).
- **Headline claim TRUE, both halves.** `context_menu.rs:917-957` binds Escape/ArrowDown/ArrowUp/
  Enter via `match ev.key().as_str()`, unguarded, gated only by `menu.get_untracked()`. At base
  `6dfe4ade`, all four extractor copies sliced from `match ev.code().as_str()` over
  mission_editor + mission_history only — context_menu was not even an input — and base
  `SHORTCUTS` had no ArrowUp/ArrowDown/Enter rows, so every T-692 pin was structurally blind to
  them. Undocumented-for-the-whole-programme-with-pins-green: confirmed.
- **Guard parsing fail-closed.** Every term not in the six-entry table panics (eden_help.rs:503).
  Tried to construct mis-parsing shapes: `||`-joined terms, parenthesized groups, non-modifier
  calls, `== false` forms — all panic rather than widen or narrow. Alternated arm heads
  (`"A" | "B" if g =>`) give both literals the real guard (`rfind("if ")`, verified). The one
  wrong-predicate shape found (`modk && !modk`, last-write-wins) requires an unsatisfiable guard
  no one writes and errs toward a FALSE collision, the safe direction.
- **Overlap is really the 8-event enumeration.** `Mods::matrix` walks all `(modk, alt, shift)`
  triples; `overlaps`/`covered_by` quantify over it; `overlap_is_modifier_aware_not_code_aware`
  pins Ctrl+V vs Ctrl+Shift+V = no collision and Ctrl+V vs ANY = collision, and ran green.
- **Precondition model fails SAFE.** Needle present verbatim at mission_history.rs:490; if
  reworded, `every_declared_precondition_is_still_in_the_source` is red AND `precondition()`
  returns `ANY`, so arms WIDEN → more collisions, not fewer. Arm-guard/precondition contradiction
  panics (`Mods::and`). The bare `"KeyZ"` arm is correctly censused as Ctrl/Cmd-without-Alt.
- **SHARED_CHANNELS both tripwires exist and bind.** ≥2-claimant check (922-938) and live-state
  check (944-961); Escape is genuinely multiply-claimed within scope; the editor keydown's Escape
  arm is additionally pinned to act only on a real `.escape()` dismissal (962-979). Sneak attempt
  found only the substring weakness (NIT above), not a bypass that ships a real collision.
- **All three anti-hollow claims hold.** Needle assembled at runtime (1082); `keydown_arms`
  refuses a first match sitting after a top-level `#[cfg(test)]` (832-837 — column-0 anchored,
  which matches the real module shape); a discovered listener yielding no binding is red
  (1009-1016). The floor (≥25 bindings, both accessors represented) also holds (1137-1163).
- **Four extractor copies → one, consumers strengthened.** Base had mission_editor.rs:8091/8446/
  8772 + eden_help; repo-wide grep now finds the single definition at eden_help.rs:829. t648 gains
  the cfg(test) refusal; **t649 moved off the raw-text variant onto the scrubbed one — strictly
  narrower acceptance** (a comment can no longer satisfy its needles), and its
  `container.get_bounding_client_rect()` / `select_all_in_view` assertions survive against the
  scrubbed slice; t669's blurb count now derives from `keymap_census` with a new
  docs-total == census-total equality that closes the old circularity. No assertion was dropped
  anywhere in the range except the four consumed `expect`s and the folded number-speller.
- **Hunted for a missed collision and found none.** Checked all 13 real listeners (including the
  two uncensused dialogs — Escape-only, gated), layout's two (not mounted: the editor route is
  `chromeless: true`, layout.rs:49/74 takes the full-viewport branch; and both are signal-gated
  regardless — though the census's stated justification for excluding layout, "gated on
  `modal_stack::is_topmost_open`", is true only of `ui`), ui Dialog/Sheet (really
  modal_stack-gated, ui.rs:486/561), and element-level handlers (fire only focused; the editor
  keydown bails on `in_editable_field()` at mission_editor.rs:2540, so T-697's new inputs cannot
  trigger `KeyE/KeyR/G/1/2/[/]`). PageUp/PageDown (T-700) are bound by no window listener.
  **Verdict "no unresolved collision today" stands, even over the full 13.**

**T-697:**
- **Projection faithful.** `filter_catalog` (asset_catalog.rs:1052-1104): Label recurses with
  self-match-keeps-subtree → root label = faction behaves exactly like an addon folder; ClassName
  is leaf-only (root has `payload: None`) prefix over `id` or `classname_tail` → lands on
  `class_name` and nothing else; Mod filters depth-0 only, prefix over root label = faction. Glob
  is whole-string per attribute (attributes searched separately, eden_dock_left.rs:1527-1531).
  Half-typed (`class:`) and broken (`/[/`) queries return none and reuse T-084's three empty-state
  sentences verbatim (tested at 2380-2389).
- **`is_selectable` == the real router.** Read the registered closure (mission_editor.rs:2392-2432):
  slot SoA lookup, then `vehiclesById.position`, then refuse — nothing else; `Slot | Vehicle`
  matches exactly. Inert rows are a `div` with `aria-disabled="true"`, `unselectable_reason` as
  title, no cursor class, no handler (eden_dock_left.rs:569-580). Perturbation pin
  `only_the_kinds_the_router_resolves_are_live_affordances` (2396-2439) goes red on any widening
  of `is_selectable` and forbids a second select path (`!contains("editor_ops::select_slot(")`).
- **`selection_facets` emits proper subsets only**: `<2` rows → none, `ids.len() < total` on both
  axes → homogeneous selection yields no chips; empty faction gets an explicit "no faction" chip
  so counts sum (1564-1609).
- **`matches_query` plain behaviour unchanged.** `SearchPattern::Plain` is a lowercased substring
  for Label (asset_catalog.rs:552-563), same as the old `to_lowercase().contains()`; empty/blank
  still match all; T-696's five assertions stand verbatim (eden_dock_left.rs:1817-1821) and pass.
  The deliberate widening (`*`, `?`, `/…/`, operators diverting) is documented in place.
- **Count honest under the cap**: header renders `Found {total}` / `Found {total} — showing
  {shown}` from `hits.len()` **before** `.take(MAX_DOC_HITS)` (508-522).

**T-700:**
- **Gate before arithmetic, not laxer.** `gate.locked_now()` returns before `nudged(` is reached
  (attributes.rs:567-570); `locked_now` is byte-for-byte `locked` with `get_untracked` — `shut`
  first, unconditional `||` — and a source pin asserts both the ordering and the body (tests
  1173-1200). A refused/un-opted field refuses the keyboard.
- **Step property over all 2^3 combos**: first-match finest-first; the test enumerates all eight
  and asserts no combo exceeds the largest solo step (100); re-checked the arithmetic by reading
  `nudge_step` — Ctrl dominates everything, Shift dominates Alt. Holds.
- **Draft-only, coalescing, disagree-refuses.** The nudge's only write is `draft.set(format!(…))`;
  exactly one `on_commit(` site exists in `number_field` (pinned); a disagreeing multi-selection
  seeds an empty draft → `"".parse::<f64>()` errs → `nudged(None,…)` → no write (code + test).
  Nudge keys claim `prevent_default` before any refusal, so the modal never scrolls instead.
- **SearchBox sound, zero callers, zero blast radius.** Stateless (no internal signal — pinned),
  clear routes through the same `on_input`, no window listener (census unaffected), T-668
  vocabulary consumed not retyped, WebKit cancel/decoration parts switched off with the contract
  pin scrub-aware (T-759 shape). T-700's ui.rs diff is purely additive; no non-editor page touched.

**Cross-slice:**
- Full-range diff of `#[cfg(test)]` bodies: **no weakened pin**; every removal is a consumed
  duplicate (four extractor copies, one speller). This being the last wave, nothing relaxed ships.
- Interactions checked: T-697 inputs vs editor keydown (guarded by `in_editable_field`); T-700's
  element-level keydown vs the census (element-level, correctly out of window-census scope);
  T-703's census over the files T-697/T-700 touched (counts still exact on merged HEAD, 905 green).

**Falsification attempts that found nothing:** a guard shape `parse_guard` silently mis-reads; an
arm-literal phantom from body strings (`matches!`/pipe-adjacent literals — none exist in any
listener body); a `_ =>`-less match causing over-slicing (all 11 matches terminate in `_ =>`); a
census self-match through the LISTENER_HEADS needle strings (they are consts in eden_help, which is
not in `editor_surface()`); a shadowed-arm false negative in mission_history (`"KeyZ" if shift`
before bare `"KeyZ"` — later arm not covered, correctly alive); an out-of-order commit path in the
nudge; a second click-to-select path in the dock; a DOM-cap miscount; a laxer `locked_now`.
