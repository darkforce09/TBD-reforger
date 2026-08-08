# Wave 137 — adversarial verify (T-741 · T-749 · T-772)

**Verified HEAD:** `2af211f669464ff7c059c84a4e79c500d87e0dd8`  
(`2af211f6` T-772 merge tip)

**Wave base:** `68b06355` (wave 113 CLOSED — editor wave 136) — ancestor of HEAD.  
**Merges in wave:** `fb648552`/`c3132a72` (T-741), `f74e7982`/`ddff9e57` (T-749),
`f649f1a0`/`2af211f6` (T-772).

**Gate (orchestrator):** PASS.  
**Method.** HOST cargo only (`~/.cargo/bin/cargo`, rustc 1.95.0). Private targets under
`~/.cache/tbd-target-wave137-{verify,perturb,base}` (deleted after this report). Mutations only in
detached worktrees at HEAD / base (`~/.cache/tbd-verify137-{perturb,base}` — deleted). Main checkout
left byte-clean at HEAD (`git status` → only pre-existing `?? tbd-target-T-*` plus this report file;
HEAD unchanged). No fix / commit / ticket filing.

**PRIMARY ATTACK.** Highest-risk claims first (T-741 subtitle wiring / full-selection count /
`live_code` hollows). Then T-749 pin-claim honesty + admitted `ui.rs` residual. Then T-772
call-site `HINT_CLOSE_BTN` vs shared `BTN_ICON`. Hollow-pin attacks on every new pin. Standing
`BTN_ICON` dense-row family when T-772 touched recipes.

**NO-DEFERRAL note.** Every severity reported honestly — orchestrator fixes ALL in-wave. No
soft-pedal of hollow pins.

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD frontend `--list` | **1003** | `cargo test -p website-frontend -- --list`, private dir |
| HEAD frontend run | **1003 passed / 0 failed** | same private dir; `--list` == run |
| base (`68b06355`) frontend `--list` | **997** | isolated worktree + private dir |
| Net frontend delta | **+6** | four T-741 + two T-772; T-749 modified an existing pin (no +1) |

**New frontend pins this wave**

| test | ticket |
|---|---|
| `attributes::tests::attrs_multi_subtitle_counts_slots_and_names_excluded_vehicles` | T-741 |
| `attributes::tests::modal_view_routes_multi_subtitle_through_the_honesty_helper` | T-741 |
| `attributes::tests::multi_edit_copy_names_slots_not_every_selected_entity` | T-741 |
| `attributes::tests::attrs_multi_ids_still_filters_selection_to_slot_soa` | T-741 |
| `eden_help::t772_controls_hint_close_hitbox::close_button_uses_call_site_padding_not_shared_recipe_alone` | T-772 |
| `eden_help::t772_controls_hint_close_hitbox::call_site_padding_identifier_is_live_not_hollow` | T-772 |

997 + 6 = **1003**. Confirmed. List == run.

**Modified (not new):** `eden_top_strip::t633_aegis_controls::the_scrubber_still_commits_only_on_settle` (T-749).

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — MAJOR | T-741 `modal_view_routes_multi_subtitle_through_the_honesty_helper` — **selection length not pinned into the helper**

**Evidence.**  
(a) Wiring pin only requires (in `live_code` bodies):
`attrs_multi_subtitle(multi_n, selection_n)` inside `fn modal_view(` and a bare
`attrs_selection_len()` presence inside `pub fn AttributesModal(`. It does **not** require that
the value passed into `modal_view` is that `attrs_selection_len()` result.  
(b) **Hollow B** (perturb worktree): keep the helper + call shape; replace
`let selection_n = crate::editor_ops::attrs_selection_len();` with
`let _ = crate::editor_ops::attrs_selection_len(); let selection_n = multi.len();`.  
All four T-741 pins → **GREEN** (`test result: ok. 4 passed; 0 failed`).  
(c) **Hollow B2:** keep `let selection_n = attrs_selection_len();` but pass `multi.len()` as the
third `modal_view` argument (ignore the binding). All four T-741 pins → **GREEN** again.  
(d) Contrast — pins that **do** catch their named defects:
- Original entities header / banner (A) → wiring + copy pins **RED**.  
- Helper counts full selection (C) → behaviour pin **RED**.  
- Comment decoy call (D) → wiring pin **RED** (`live_code` blanks comments).  
- Swapped args / stripped SoA filter / gutted `attrs_selection_len` body → **RED**.

**Impact.** Production on HEAD is honest today (`selection_n = attrs_selection_len()` passed
through). The ticket’s highest-risk claim — disclose when selection is wider than
`attrs_multi_ids` — is **regression-blind**: a one-hunk disconnect never shows “vehicles
excluded” while every T-741 pin stays green. Matches the brief’s named risk (subtitle wiring /
hollow live presence without value flow).

**Disposition — fix this wave.** Strengthen the wiring pin so B and B2 go RED, e.g. require in the
`AttributesModal` wasm arm (same `live_code` / `only_body` window):
1. `let selection_n = crate::editor_ops::attrs_selection_len()` (assignment, not a dead call), and  
2. the `modal_view(` argument list passes that `selection_n` (not `multi.len()` / `multi_n`).  
Keep the helper behaviour pin and banner/`attrs_multi_ids` pins. Perturbations (b)/(c) must RED.

---

### F2 — MINOR | T-749 / `ui.rs` Slider docs — **found_not_fixed (outside owns)**

**Evidence.**  
T-749 owns only `eden_top_strip.rs`. Strip production comment and settle-pin claim were corrected
(`Settled/authored HH:MM… is NOT live drag feedback`; needle tightened to
`{move || env.get().time}`).  
`apps/website/frontend/src/ui.rs` **still** says (Slider rustdoc ~148–149):

> A caller that needs live-drag feedback should render its own readout from the same signal
> (which is what the top strip's `HH:MM` label does)

That sentence is false on HEAD: the strip label tracks settled `env` via `doc_tick`, not mid-drag
preview. Outside T-749 owns — ticket called this out as found_not_fixed.

**Impact.** Shared Slider API docs still teach the pre-T-749 lie. Future callers can “fix”
live-drag by copying the strip pattern and get a frozen label. Does not break runtime.

**Disposition — fix this wave (NO-DEFERRAL).** Edit `ui.rs` Slider rustdoc: drop the claim that the
top strip HH:MM is live-drag feedback; point mid-drag preview at a local drag signal (or omit the
strip as example). Outside prior owns, but orchestrator fixes all in-wave findings.

---

### NIT — T-749 settle pin name vs narrowed body

**Evidence.** Pin comments now honestly say they do not scan strip-local `on:input` (Save-dialog
handlers already exist in `TopCommandStrip`). Perturbation C inserted a decorative
`<span … on:input=…>` beside the scrubber → settle pin stayed **GREEN**. Commit path needles
(`on_change` → `author_env` → `row_mirror.set_time`) still hold. Function name remains
`the_scrubber_still_commits_only_on_settle`.

**Impact.** Name slightly overclaims relative to what the body asserts; behaviour contract for the
Slider settle path is still pinned.

**Disposition — fix this wave (nit):** rename or narrow the test title/doc to “settle commit path”
(or restore an intentional scrubber-local `on:input` absence needle that ignores Save-dialog
sites). Not a behavioural defect on HEAD.

---

## Main safe to build the next wave on?

**no** — not until F1 (and in-wave F2 / NIT under NO-DEFERRAL) are fixed.  
Production behaviour for T-741 / T-749 / T-772 looks correct on HEAD under source inspection +
attack contrast, but F1 leaves the wave’s multi-edit honesty claim regression-blind (exact
highest-risk miss). F2 keeps a false Slider-doc claim on `main` outside the strip fix.

---

## Verified-clean register (claims re-proved)

- **T-741 production copy:** multi subtitle helper emits
  `"N slots selected · multi-edit"` / `· vehicles excluded` when `selection_n > slot_n`; banner
  says “every selected slot”; `attrs_multi_ids` still filters via `soa.ids.iter().any(|r| r == s)`;
  `attrs_selection_len` reads `ctx.selection.borrow().len()`.  
- **T-749 strip honesty:** production comment no longer calls HH:MM live drag feedback; pin needle
  `{move || env.get().time}` **RED** when the span loses that binding (perturb B); settle commit
  path still requires `on_change` + `author_env` + `row_mirror.set_time`.  
- **T-772:** `HINT_CLOSE_BTN` is `p-1.5` without `p-0.5`; ControlsHint close composes
  `HINT_CLOSE_BTN` + `HOVER_FILL` and does not size from `BTN_ICON`; `BTN_ICON` remains dense
  `p-0.5` (`eden_layout::t637_dock_geometry::btn_icon_rests_bright_and_fits_a_dense_row` green).  
- Suites: frontend **1003 == 1003** (base 997, +6).  
- Main: `2af211f6`, tracked tree untouched aside from this report.

---

## Attacked and FAILED to break

1. **T-741 original entities overclaim (A)** — inline `"{multi_n} entities selected · multi-edit"`
   + banner “every selected entity” → wiring + copy pins **RED**.  
2. **T-741 helper counts full selection (C)** — `format!("{selection_n} slots…")` ignoring
   `slot_n` → behaviour pin **RED**.  
3. **T-741 comment decoy call (D)** — `// attrs_multi_subtitle(multi_n, selection_n)` + format! →
   wiring pin **RED** (`live_code`).  
4. **T-741 swapped helper args (E)** — `attrs_multi_subtitle(selection_n, multi_n)` → wiring **RED**.  
5. **T-741 SoA filter stripped (F)** — `attrs_multi_ids` returns raw `sel` → filter pin **RED**.  
6. **T-741 `attrs_selection_len` body gutted to `0` (G)** — selection-len body pin **RED**.  
7. **T-749 HH:MM binding (B)** — replace `{move || env.get().time}` with non-reactive
   `{env.get().time.clone()}` → settle pin **RED** (tightened needle holds).  
8. **T-772 original BTN_ICON-alone close (A)** — both t772 pins **RED**.  
9. **T-772 comment decoy HINT_CLOSE_BTN (B)** — both pins **RED**.  
10. **T-772 widen shared BTN_ICON to p-1.5 (C)** — t772 close pin **RED** (standing dense-row
    recipe defended).  
11. **T-772 HINT_CLOSE_BTN = p-0.5 (D)** — close pin **RED**.  
12. **T-772 string-literal class without identifier (E)** — `live_code` identifier pin **RED**.  
13. **T-772 cn co-compose HINT_CLOSE_BTN + BTN_ICON (F)** — both pins **RED**.  
14. **Frontend list↔run mismatch** — none (1003 == 1003).  
15. **T-772 standing family:** `btn_icon_rests_bright_and_fits_a_dense_row` remains green on HEAD
    (dense `p-0.5`, not widened).

---

## Safe? / register summary

| question | answer |
|---|---|
| **Main safe to build the next wave on?** | **no** (F1 MAJOR hollow wiring; F2/NIT in-wave under NO-DEFERRAL) |
| **Attacked-and-FAILED-to-break** | See numbered register above (15 held attacks; F1 hollows B/B2 are the misses) |

---

## Focused re-verify

**Post-fix HEAD:** `f5b641629217805cd6dd68f34463625d4ba71743`  
**Fix commits:** `5f11ac4c` (F1 T-741), `f5b64162` (F2+NIT T-749).  
**Gate (orchestrator post-fix):** PASS.  
**Method.** HOST cargo only. Detached worktree
`~/.cache/tbd-verify137-reverify` at HEAD; `CARGO_TARGET_DIR=~/.cache/tbd-target-w137-reverify`.
Mutations only in that worktree; both deleted after this section. Main left byte-identical aside
from this append. No fix / commit / ticket filing. **No wave close.**

**Scope.** F1 · F2 · NIT only (original dispositions above).

### Suite reconciliation (post-fix HEAD)

| measurement | value | how |
|---|---|---|
| frontend `--list` | **1003** | `cargo test -p website-frontend -- --list` → `1003 tests, 0 benchmarks` |
| frontend run | **1003 passed / 0 failed** | `test result: ok. 1003 passed; 0 failed; … finished in 12.18s` |
| list ↔ run | **match** | 1003 == 1003 |

### F1 — MAJOR | T-741 selection_n flow pin — **CLOSED**

**Original disposition.** Strengthen wiring pin so hollow B (`let _ = attrs_selection_len(); let selection_n = multi.len()`) and hollow B2 (keep binding, pass `multi.len()` into `modal_view`) go RED.

**Baseline (GREEN).**  
`test attributes::tests::modal_view_routes_multi_subtitle_through_the_honesty_helper ... ok`  
`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1002 filtered out; finished in 0.10s`

**Hollow B (RED).** Production in `AttributesModal` wasm arm replaced with:
```text
let _ = crate::editor_ops::attrs_selection_len();
let selection_n = multi.len();
```
Assertion (verbatim):
```text
AttributesModal must bind `let selection_n = crate::editor_ops::attrs_selection_len()` (not a discarded call); body was:
```
`test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1002 filtered out; finished in 0.10s`

**Hollow B2 (RED).** Restored honest binding; third `modal_view` arg set to `multi.len()`:
```text
let selection_n = crate::editor_ops::attrs_selection_len();
Some(modal_view(
    attrs,
    multi,
    multi.len(),
    …
```
Assertion (verbatim):
```text
AttributesModal must pass `selection_n` into modal_view (not multi.len()/multi_n); body was:
```
`test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1002 filtered out; finished in 0.10s`

**Restore (GREEN).** Production returned to `selection_n = attrs_selection_len()` passed into `modal_view`.  
`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1002 filtered out; finished in 0.10s`

**Verdict.** Fix holds — both original hollows now RED; honest path GREEN.

### F2 — MINOR | T-749 `ui.rs` Slider rustdoc — **CLOSED**

**Original disposition.** Drop false claim that the top strip HH:MM is live-drag feedback.

**HEAD (false claim absent).** Live rustdoc (`ui.rs` ~148–149):
```text
/// on `input`. A caller that needs live-drag feedback should keep a local mid-drag preview signal
/// (the top strip's `HH:MM` label tracks settled `env` time via `doc_tick`, not thumb drag) rather
```
Needles **absent** on HEAD:
- `render its own readout from the same`
- `which is what the top strip's \`HH:MM\` label does`

**Restored false claim (detectable regression).** Re-applied the pre-fix sentence; `git diff` showed exact revert of the F2 hunk (`0aac3299` → `7cf0b110` content). Both bad needles present after restore. Restored honest doc via `git checkout -- ui.rs`. Bad needles absent again after restore.

**Verdict.** Doc-only fix holds — false claim gone; restoring it is a detectable regression (string/diff probe).

### NIT — T-749 settle pin rename — **CLOSED**

**Original disposition.** Rename/narrow title so it does not overclaim `on:input` absence; settle commit path remains pinned.

**HEAD (GREEN).**  
`fn the_scrubber_settle_commit_path()` present; doc lead-in `**Settle commit path is not regressed.**`  
`test eden_top_strip::t633_aegis_controls::the_scrubber_settle_commit_path ... ok`  
`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1002 filtered out; finished in 0.16s`  
Old overclaiming filter `--exact --list` for `…::the_scrubber_still_commits_only_on_settle` → **`0 tests, 0 benchmarks`**.

**Original defect restored (detectable).** Renamed back to `fn the_scrubber_still_commits_only_on_settle()` + restored overclaiming doc (`**The debounce is not regressed.**`).  
Honest-name filter `--exact --list` for `…::the_scrubber_settle_commit_path` → **`0 tests, 0 benchmarks`** (regression detectable: fixed name gone).  
Old name still runs: `test …::the_scrubber_still_commits_only_on_settle ... ok`.

**Restore (GREEN).** `git checkout -- eden_top_strip.rs`.  
`fn the_scrubber_settle_commit_path()` restored;  
`test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1002 filtered out; finished in 0.15s`

**Verdict.** Rename holds — overclaiming title absent on HEAD; restoring it drops the honest filter name (detectable).

### Focused re-verify summary

| finding | status |
|---|---|
| F1 | **CLOSED** (hollow B + B2 RED; restore GREEN) |
| F2 | **CLOSED** (false claim absent; restore detectable) |
| NIT | **CLOSED** (honest name present; overclaim restore detectable) |

| question | answer |
|---|---|
| **Findings CLOSED count** | **3 / 3** (F1, F2, NIT) |
| **Main safe to build the next wave on?** | **yes** (focused re-verify of F1/F2/NIT only; post-fix HEAD `f5b64162`; list==run 1003) |
| **Wave close?** | **no** — append-only re-verify; orchestrator owns close |
