# Wave 133 — adversarial verify (T-750 · T-766 · T-756)

**Verified HEAD:** `e31391d0c8ff129d902073f7c89b34e85ab805a0`  
(`e31391d0` T-756 merge tip)

**Wave base:** `210afb3f` (wave 109 CLOSED — editor wave 132) — ancestor of HEAD.  
**Merges in wave:** `9b0060d3` (T-750), `f0f63b45` (T-766), `e31391d0` (T-756).  
(Pre-merge / rustfmt SHAs `74e7f8e0`/`8cb253b9`, `76e5e59c`/`562b1ebf`, `93fdf305` land the same diffs.)

**Gate (orchestrator):** PASS.  
**Method.** HOST cargo only (`~/.cargo/bin/cargo`, rustc 1.95.0). Private targets under
`~/.cache/tbd-target-wave133-{verify,perturb,base}` (deleted before this report). Mutations only in
detached worktrees at HEAD / base (`~/.cache/tbd-verify133-{perturb,base}` — deleted). Main checkout
left byte-clean (`git status` empty; HEAD unchanged).

**NO-DEFERRAL note.** Orchestrator will fix all findings — reported honestly; no self-authored deferrals.

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD `--list` | **985** | `cargo test -p website-frontend -- --list`, private dir |
| HEAD run | **985 passed / 0 failed** | same private dir; `--list` == run |
| base (`210afb3f`) `--list` | **981** | isolated worktree + private dir |
| Net delta | **+4** | four new named frontend pins |

**New frontend pins this wave**

| test | ticket |
|---|---|
| `mission_editor::t750_registry_fetch_failure_signal::mark_registry_fetch_failed_writes_all_three_signals` | T-750 |
| `mission_editor::t750_registry_fetch_failure_signal::err_arm_and_retry_gen_are_wired_on_the_page` | T-750 |
| `eden_dock_right::tests::favourites_panel_failure_arm_has_retry` | T-750 |
| `eden_settings::t766_clear_briefing_mirror::clearing_a_briefing_calls_the_clear_mutator` | T-766 |

T-756 extends existing `t670_scale_readout::readout_table_across_the_zoom_clamp` + flips
`m_per_px_is_two_pow_neg_zoom` (no new frontend test name).  
T-766 also adds behavioural `doc::store::tests::t766_clear_meta_briefing_drops_key_blank_apply_does_not`
in `map-engine-core` (`--features doc,mission`) — not in the frontend suite count.

981 + 4 = **985**. Confirmed. List == run.

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — MAJOR | `apps/website/frontend/src/eden_settings.rs` (`set_presentation` Ok/Briefing arm) — **T-766 blank clear is unreachable under a non-blank call-site gate (hollow wire)**

**Evidence.**  
(a) Live code calls `mirror_briefing_into_document(&next)` unconditionally on PATCH-Ok Briefing
(`eden_settings.rs` ~676), and the mirror body correctly `clear_meta_briefing()`s on
`trim().is_empty()` (~405-406). Core behavioural pin proves `clear_meta_briefing` drops the key
and blank `apply_row_meta` does not.  
(b) T-766's only frontend pin greps the **mirror function body**. T-671's
`a_saved_briefing_reaches_the_documents_meta` only requires `set_presentation`'s body to *contain*
`mirror_briefing_into_document(` (and the mirror body still uses `apply_row_meta`).  
(c) Perturbation (HEAD worktree, private target): wrap the call as
`if !next.trim().is_empty() { mirror_briefing_into_document(&next); }` →
`clearing_a_briefing_calls_the_clear_mutator` **ok**, `a_saved_briefing_reaches_the_documents_meta`
**ok**. Production defect of wave-117 returns for clears: row PATCHes `""`, `meta.briefing` keeps
the deleted text, same-session Export ships it.  
(d) Contrast: deleting the call entirely → T-671 pin **RED** (*"a saved briefing must be mirrored…"*).
Restoring early-return-on-empty inside the mirror → T-766 pin **RED**. Core no-op clear → core pin
**RED**. So body + mutator are load-bearing; **blank-path reachability at the call site is not**.

**Impact.** Today’s main *does* clear on blank, but a one-line non-blank gate (or any equivalent
that keeps the call identifier while skipping empty `next`) ships green while restoring the ticket’s
exact failure mode. Standing hollow-pin family (wave 127–132 #2).

**Disposition — fix this wave.** Pin that blank `next` still reaches `mirror_briefing_into_document`
(e.g. require the call is not under a non-empty guard in `set_presentation`, or a behavioural
host test that drives blank through the same Ok arm). Perturbation above must go RED.

---

### F2 — NIT | `eden_toolbelt.rs` ScaleBar/StatusBar docs — **wave-115 NIT-3 note corrections are unpinned**

**Evidence.** T-756 correctly rewrote comments: live `scale_mpp` is seeded to `4.0`, and the
`camera_snapshot()` arm is dead on the only real caller. Reverting those prose sentences in a
worktree would not fail any Class-R/behavioural pin (formatter pins are separate and solid — see
claim attacks).  
**Impact.** Doc lie can return; behaviour of the formatter corners remains gated.  
**Disposition — fix this wave** (optional needle in an existing ScaleBar wire pin, or accept as
comment-only risk). Not behavioural.

---

## Claim attacks (by ticket)

### T-750 — Favourites terminal failure + Retry

| claim | result |
|---|---|
| `mark_registry_fetch_failed` Fails both catalogs + `registry_failed` | **HELD** — behavioural Owner flip RED if any of the three skips; source + behavioural both RED on full gut |
| Decoy string `"registry_failed.set(true)"` (live_code blanks literals) | **HELD** — source pin RED |
| Skip only `vehicle_catalog` Failed | **HELD** — source can still see `CatalogState::Failed` once, but **behavioural RED** |
| Err arm calls the helper (not catalog-only Fail) | **HELD** — strip call → `err_arm_and_retry_gen_are_wired_on_the_page` RED |
| Effect reads `registry_fetch_gen.get()` for Retry | **HELD** — replace with `0u64` → RED |
| Favourites failure arm + Retry bump + Resolving retained | **HELD** — strip arm → RED on bump; comment-only bump decoy → RED (`live_code`) |
| `registry_failed` starts false | **HELD** — live `RwSignal::new(false)` needle in page live_code |

### T-766 — `clear_meta_briefing` when author empties library blurb

| claim | result |
|---|---|
| `clear_meta_briefing` drops `meta.briefing` | **HELD** — no-op body → core pin RED |
| Blank `apply_row_meta` stays "not supplied" (hydrate guard) | **HELD** — blank-as-clear → core pin RED |
| Mirror blank arm calls clear; non-blank still `apply_row_meta` | **HELD** — early-return restore → T-766 pin RED; clear-before-trim → order assert RED |
| Full call-site deletion | **HELD via T-671** — `a_saved_briefing_reaches_the_documents_meta` RED |
| Blank `next` still reaches the mirror | **UNPINNED** → **F1 MAJOR** (non-blank gate greens T-766 + T-671) |

### T-756 — scale formatter corners

| claim | result |
|---|---|
| Non-finite zoom → em-dash (not `1.00 m/px`) | **HELD** — restore unit-scale `1.0` fallback → `readout_table…` RED (`got 1.00 m/px`) + `m_per_px_is_two_pow_neg_zoom` RED |
| Bare `powf` without finite check | **not the old bug** — still yields NAN/0/INF that format already em-dashes; suite stayed green (expected) |
| `format_m_per_px` em-dash for non-finite mpp | **HELD** — fabricate `"1.00 m/px"` on degenerate → RED |
| Band-top carry `9.996` → `10.0` (not `10.00`) | **HELD** — drop re-pick → `left: "10.00 m/px"` RED; `99.96`/`0.09996` co-pinned |
| wave-115 NIT-3 note corrections | **COMMENT ONLY** → **F2 NIT** |

---

## Standing attack surfaces (wave 127–130 families)

| surface | attack | result |
|---|---|---|
| 1. z=None flatten / `keep_z_rows` | Gut body → always `None` | **FAILED to break** — `an_attributes_x_or_y_commit_carries_the_slots_current_z_back_in` **RED** |
| 2. Hollow source pins | T-750 helper/Retry/Err pins load-bearing; T-756 behavioural load-bearing; **T-766 blank call-site hollow** | **F1 MAJOR** |
| 3. Stale thread_local / `Rc::ptr_eq` | No new `thread_local`/`ptr_eq` in wave owns (`git diff -G`); t754 suite **14/14** green | **FAILED to break** |
| 4. Affordance clickable IFF | `a_finding_row_is_clickable_iff…` + t754 affordance pins green | **FAILED to break** |
| 5. Shared `CARGO_TARGET_DIR` / list vs run | Private `~/.cache/tbd-target-wave133-*` only; list **985** == run **985**; dirs deleted | **FAILED to break** |

---

## Main safe to build the next wave on?

**yes** — with **F1 fixed in-wave** (orchestrator NO-DEFERRAL). Main is not broken today; open risk is
regression-blind blank-path wiring for T-766, not a live data-loss bug on HEAD. F2 is comment-only.
T-750 and T-756 claims hold under perturbation.

---

## Verified-clean register (claims re-proved)

- T-750: helper writes all three signals (behavioural); Err arm calls helper; Effect reads
  `registry_fetch_gen`; Favourites failure arm bumps gen + keeps Resolving; decoy strings / comment
  bumps fail closed under `live_code`.  
- T-766: core clear vs blank-apply behavioural pin; mirror body clear-on-empty + order vs trim;
  T-671 still pins *some* call of the mirror from `set_presentation`.  
- T-756: unit-scale `1.0` restore RED; band-top re-pick RED; format degenerate em-dash RED.  
- Suite: 985 list == 985 run; base 981; +4.  
- Main: `e31391d0`, clean tracked tree after cleanup.

---

## Attacked and FAILED to break

1. **T-750 helper three-signal write** — gut / decoy / skip-vehicle all RED (behavioural and/or source).  
2. **T-750 Err-arm helper call** — catalog-only Fail without helper RED.  
3. **T-750 Retry gen read + Favourites bump** — `gen.get()` removal and bump decoy RED.  
4. **T-766 `clear_meta_briefing` mutator** — no-op RED.  
5. **T-766 blank `apply_row_meta` hydrate guard** — blank-as-clear RED.  
6. **T-766 mirror body early-return / wrong-order clear** — both RED.  
7. **T-766 full call-site deletion** — T-671 RED.  
8. **T-756 non-finite zoom → em-dash** — unit-scale `1.0` restore RED.  
9. **T-756 band-top carry** — drop re-pick → `10.00` RED.  
10. **`keep_z_rows` sticky-z pin** — full body→`None` RED.  
11. **Affordance IFF / t754 (14/14)** — green.  
12. **Private-target list/run mismatch** — none (985==985).  

---

## Not attacked successfully (finding)

- **T-766 blank-path call-site gate** — see **F1**; suite stays green while clear never runs on empty saves.

---

## Focused re-verify (F1+F2 only)

**When:** 2026-08-08 post-fix (`0e9218f5` F1, `958fc913` F2).  
**HEAD:** `958fc9132ee4341926a77d91fe0b5fa1b2ee6f74`  
**Method.** HOST cargo (`~/.cargo/bin/cargo`, rustc 1.95.0). Private target
`~/.cache/tbd-target-w133-reverify`. Mutations only in detached worktree at HEAD
(`~/.cache/tbd-verify133-reverify` — removed after). Main checkout left byte-identical
except this append (`git status` → only `?? .ai/artifacts/editor_verify/wave133.md`;
HEAD unchanged). No fix/commit.

### Suite

| measurement | value |
|---|---|
| `--list` | **986** |
| run | **986 passed / 0 failed** |
| vs pre-fix verify | +1 (`blank_next_reaches_the_mirror_at_the_ok_briefing_arm`); F2 extends existing `the_scale_bar_resolves_from_the_same_signal` |

List == run.

### F1 — non-empty call-site gate → RED (then restore)

**Perturbation (worktree):** wrap Ok/Briefing arm as
`if !next.trim().is_empty() { mirror_briefing_into_document(&next); }`.

| test | result |
|---|---|
| `blank_next_reaches_the_mirror_at_the_ok_briefing_arm` | **RED** — `no trim().is_empty gate between Briefing => and the mirror call` |
| `clearing_a_briefing_calls_the_clear_mutator` | **ok** (body pin still greens — hollow without F1) |
| `a_saved_briefing_reaches_the_documents_meta` | **ok** (T-671 still greens) |

**Restore:** `git checkout -- eden_settings.rs` → F1 pin **ok**.

### F2 — strip ScaleBar NIT-3 doc needles → RED (then restore)

**Perturbation (worktree):** replace ScaleBar prop docs
`(seeded 4.0); … dead on the only real caller (wave-115 NIT-3)` with
unpinned prose (`mount always supplies this; … may still run`).

| test | result |
|---|---|
| `the_scale_bar_resolves_from_the_same_signal` | **RED** — `ScaleBar docs must keep the NIT-3 seed note (4 m/px default)` |

**Restore:** `git checkout -- eden_toolbelt.rs` → same pin **ok**.

### Safe to build the next wave on?

**yes** — F1 blank-path reachability and F2 NIT-3 doc needles both load-bearing under
perturbation; suite 986==986 green on restored HEAD.
