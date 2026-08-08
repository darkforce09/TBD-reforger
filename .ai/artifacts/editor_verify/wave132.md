# Wave 132 — adversarial verify (T-771 · T-758 · T-765)

**Verified HEAD:** `614ac6050b2f5bee4007ac4682ab5651d6ab4c15`  
(`614ac605` T-765 merge tip)

**Wave base:** `0e2e05da` (wave 108 CLOSED — editor wave 131) — ancestor of HEAD.  
**Merges in wave:** `7e08a281` (T-771), `895d6c08` (T-758), `614ac605` (T-765).  
(Pre-merge slice SHAs `784b3015` / `a0abd8fe` / `360e2d24` land the same diffs.)

**Gate (orchestrator):** PASS.  
**Method.** HOST cargo only. Private targets under `~/.cache/tbd-target-wave132-{verify,perturb,base}` (deleted before this report). Mutations only in detached worktrees at HEAD / base (`~/.cache/tbd-verify132-{perturb,base}` — deleted). Main checkout left byte-clean (`git status` empty of tracked dirt; HEAD unchanged).

**NO-DEFERRAL note.** Orchestrator will fix all findings — reported honestly; no self-authored deferrals.

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD `--list` | **977** | `cargo test -p website-frontend -- --list`, private dir |
| HEAD run | **977 passed / 0 failed** | same private dir; `--list` == run |
| base (`0e2e05da`) `--list` | **972** | isolated worktree + private dir |
| Net delta | **+5** | +6 new names − 1 renamed pin |

**New / renamed pins this wave**

| test | ticket |
|---|---|
| `mission_editor::…::the_arsenal_tab_discloses_one_entity_picks_and_whole_selection_buffer_verbs` | T-771 (renamed from `…admits_it_edits_one_entity_under_a_multi_selection`) |
| `eden_settings::t758_inert_row_a11y::{a_mission_owned_row_is_inert_with_a_reason, an_unroutable_entity_row_is_inert_with_a_reason, an_inert_row_is_not_a_focusable_button, inert_shape_still_asks_subject_id_routes_not_a_kind_list}` | T-758 (+4) |
| `asset_catalog::tests::glob_case_folding_is_symmetric_for_non_ascii` | T-765 (+1) |

972 − 1 + 1 + 4 + 1 = **977**. Confirmed.

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — NIT | `apps/website/frontend/src/arsenal.rs:1943-1946` — **T-771 leaves a stale comment beside a redundant counter-note (outside owns)**

**Evidence.**  
(a) Live Attributes banner (`attributes.rs:392-394`) now says pick/cargo = one entity **and** Copy/Apply/Remove Everything = whole selection.  
(b) `arsenal.rs:1943-1946` still claims the Attributes banner “tells a multi-selection that ‘loadout edits apply to this one entity’” — that unqualified string is gone (T-771 negative pin forbids it).  
(c) The adjacent UI span (`:1949`) still correctly says the three buffer verbs act on the whole selection; after T-771 that disclosure is **redundant** with the Attributes banner (and outside T-771 owns — slice only touched `attributes.rs` + `mission_editor.rs`).

**Impact.** Comment lie / dual UI copy only; behaviour and the amended Attributes pin are sound.

**Disposition — fix this wave** (comment refresh; optionally drop or shorten the arsenal span now that the Attributes banner owns both scopes). Not behavioural.

---

### F2 — NIT | `apps/website/frontend/src/asset_catalog.rs:631-632` — **`ß → ss` lore in GlobPattern::parse is false**

**Evidence.**  
(a) Comment claims ``char::to_lowercase` can expand (ß → ss)``.  
(b) HOST `rustc` probe: `'ß'.to_lowercase()` → `['ß']`; `"straße".to_lowercase()` stays `"straße"`; `"STRASSE".to_lowercase()` → `"strasse"`.  
(c) Debug of live `GlobPattern::parse("straße*")` tokenized `Ch('ß')`, not two `s` chars. `STRASSE*` vs `straße_x` does **not** match under `to_lowercase` (expected — not full Unicode casefold).  
(d) The **CAFÉ\*** behavioural pin is unrelated and still load-bearing (see claim attacks).

**Impact.** Misleading comment only; T-765’s É/é fix and pin are correct.

**Disposition — fix this wave** (edit the comment to match Rust `to_lowercase`, or drop the ß claim). Not behavioural.

---

### F3 — STANDING (found_not_fixed) | `validation_panel.rs:1048-1080` — **inert finding rows remain focusable `<button>`**

**Evidence.**  
(a) Attack claim asked to confirm peer debt. `finding_row_view` always emits one `<button>`; no `<div aria-disabled>` inert arm; click short-circuits on `selectable` only.  
(b) T-758 fixed the **All Settings** peer (`setting_row_view` button vs div). Affordance IFF pins still green (`a_finding_row_is_clickable_iff…`, t754 suite 14/14).

**Impact.** Keyboard/AT users can still tab onto inert validation rows (wave-115 MINOR class), while Settings no longer can. Not a regression of this wave’s owns.

**Disposition — standing / found_not_fixed** (confirm for orchestrator; not introduced by T-758). Do not treat as T-758 incomplete.

---

## Claim attacks (by ticket)

### T-771 — multi-select loadout honesty banner

| claim | result |
|---|---|
| Banner discloses one-entity picks/cargo **and** whole-selection buffer verbs | **HELD** — live copy at `attributes.rs:392-394` |
| Pin amended (old unqualified claim must not return) | **HELD** — restore `"Loadout edits apply…"` → pin **RED**; drop whole-selection half → **RED**; move needles outside `modal_view` → **RED**; `is_multi`→`false` → **RED** (`is_multi.then`) |
| Arsenal counter-note outside owns / redundant | **HELD as claimed** → **F1 NIT** (stale comment + redundant span) |

### T-758 — inert All Settings rows non-focusable

| claim | result |
|---|---|
| Inert rows are non-focusable `<div aria-disabled>` + reason | **HELD** — live `setting_row_view` inert arm; convert to `<button aria-disabled>` → pin **RED**; strip `aria-disabled` → **RED** |
| Selectable still `<button>` | **HELD** — exactly one `<button>` pin |
| Clickability via `owner_is_routable` → `subject_id_routes` (no kind list) | **HELD** — replace body with `matches!(Entity)` → `inert_shape_still_asks…` + unroutable-entity pin **RED**; Mission still inert |
| `validation_panel` same pattern found_not_fixed | **CONFIRMED** → **F3 STANDING** |

### T-765 — Unicode glob fold

| claim | result |
|---|---|
| Pattern + haystack share `to_lowercase` | **HELD** — parse uses `c.to_lowercase()`; `matches` uses `hay.to_lowercase()` |
| `CAFÉ*` pin | **HELD** — behavioural `glob_case_folding_is_symmetric_for_non_ascii`; revert pattern to `to_ascii_lowercase` → **RED**; hay to `to_ascii_lowercase` → **RED** (mixed-case hay assert) |
| ß expansion comment | **COMMENT FALSE** → **F2 NIT** (behaviour for CAFÉ still correct) |

---

## Standing attack surfaces (wave 127–130)

| surface | attack | result |
|---|---|---|
| 1. z=None flatten / `keep_z_rows` | Gut body → `None` | **FAILED to break** — `attributes::tests::an_attributes_x_or_y_commit_carries_the_slots_current_z_back_in` **RED** |
| 2. Hollow source pins | T-771 modal-scoped needles + negative unqualified; T-758 live_source button-count; T-765 behavioural | **No hollow MAJOR** this wave (F1/F2 are comment/copy NITs) |
| 3. Stale thread_local / `Rc::ptr_eq` | No new hook in wave owns; t754 suite **14/14** green | **FAILED to break** |
| 4. Affordance clickable IFF | validation_panel + eden_settings iff pins green | **FAILED to break** |
| 5. Shared `CARGO_TARGET_DIR` / list vs run | Private `~/.cache/tbd-target-wave132-*` only; list **977** == run **977**; dirs deleted | **FAILED to break** |

---

## Main safe to build the next wave on?

**yes** — merge claims hold under perturbation; suite 977==977; no CRITICAL/MAJOR. F1/F2 are comment/copy NITs; F3 is acknowledged standing peer debt (validation_panel), not a T-758 miss.

---

## Verified-clean register (claims re-proved)

- T-771: dual-scope banner in `modal_view`; pin requires both needles, forbids unqualified claim, requires `is_multi.then`.  
- T-758: inert `<div aria-disabled>` + `inert_settings_row_reason`; one selectable `<button>`; `subject_id_routes` via `owner_is_routable`.  
- T-765: Unicode fold on pattern literals; CAFÉ*/café_x behavioural pin RED on ascii regress.  
- Suite: 977 list == 977 run; base 972; +5 net.  
- Main: `614ac605`, clean tracked tree after cleanup.

---

## Attacked and FAILED to break

1. **T-771 dual-scope banner pin** — unqualified restore / half-drop / out-of-`modal_view` needles / `is_multi` gate all RED.  
2. **T-758 inert non-button shape** — inert `<button aria-disabled>` and stripped `aria-disabled` RED.  
3. **T-758 subject_id_routes clickability** — kind-list `owner_is_routable` RED on shape + unroutable-entity pins.  
4. **T-765 CAFÉ\* symmetry** — ascii pattern fold or ascii hay fold RED.  
5. **`keep_z_rows` sticky-z pin** — full body→`None` RED.  
6. **Affordance IFF / t754 (14/14)** — green.  
7. **Private-target list/run mismatch** — none (977==977).  
8. **Mission-owned settings inertness** — still inert even under an always-true probe.

---

## Focused re-verify

**When:** 2026-08-08 (post-fix).  
**HEAD:** `0c2ad04d` (includes `6368d6e1` F1, `adbd6621` F2, `0c2ad04d` F3).  
**Method.** HOST cargo only. Private `CARGO_TARGET_DIR=~/.cache/tbd-target-w132-reverify`. Mutations only in detached worktree `~/.cache/tbd-verify132-reverify-perturb` (removed after). Main checkout left byte-identical aside from this append (`git status` = untracked `wave132.md` only; HEAD unchanged; no tracked diffs).

### Suite

| measurement | value |
|---|---|
| `--list` | **981** |
| run | **981 passed / 0 failed** |
| vs pre-fix verify | 977 → **981** (+4 = `w132_inert_finding_row_a11y::*`) |

List == run. Confirmed.

### F1 (`6368d6e1`) — arsenal buffer-verb note

| check | result |
|---|---|
| Live comment refreshed (T-771 dual-scope ownership; no stale “loadout edits apply to this one entity” claim) | **HELD** — `arsenal.rs:1943-1945` |
| Live UI shortened to `"Buffer verbs: whole selection."` | **HELD** — `arsenal.rs:1947` |
| Perturbation: restore pre-fix stale comment + long span | **No Class-R pin RED** (NIT-only; Attributes dual-scope pin `the_arsenal_tab_discloses_…` stayed **GREEN** — that pin scopes `attributes` modal, not arsenal counter-copy) |

**Verdict:** fix present; comment/copy NIT with no dedicated pin (as designed).

### F2 (`adbd6621`) — GlobPattern `to_lowercase` comment

| check | result |
|---|---|
| Live comment documents `ß → ß (not ss)` | **HELD** — `asset_catalog.rs:631-633` |
| HOST rustc probe | `'ß'.to_lowercase()` → `['ß']`; `"straße"` stays `"straße"`; `"STRASSE"` → `"strasse"` |
| Behavioural load-bearing pin still RED on ascii regress | **HELD** — pattern fold → `c.to_ascii_lowercase()` ⇒ `glob_case_folding_is_symmetric_for_non_ascii` **RED** (`CAFÉ* must match café_x…`); restore ⇒ **GREEN** |

**Verdict:** comment truth matches rustc; CAFÉ\* pin still proves the Unicode fold.

### F3 (`0c2ad04d`) — inert validation rows non-focusable

| check | result |
|---|---|
| Live shape: selectable `<button>` / inert `<div aria-disabled>` + `inert_finding_row_reason` | **HELD** |
| Suite `w132_inert_finding_row_a11y` (4) | **4/4 GREEN** at HEAD |
| Perturbation: restore always-`<button>` (pre-fix) | `an_inert_finding_row_is_not_a_focusable_button` **RED** — *“inert finding must be a non-focusable element carrying aria-disabled and the reason…”* |
| Restore | **4/4 GREEN** again |

**Verdict:** standing F3 closed and pin-backed; button restore → RED proved.

### Main safe to build the next wave on?

**yes** — F1/F2/F3 present at `0c2ad04d`; suite **981==981**; F3 shape pin RED under always-button restore; F2 CAFÉ pin RED under ascii pattern fold; F1 live text matches fix (NIT, no pin). No further fix/commit from this pass.

