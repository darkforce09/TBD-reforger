# Wave 134 — adversarial verify (T-736 · T-755 · T-776)

**Verified HEAD:** `6ec74d23fe831ee2df2107ce85b87f638028f07f`  
(`6ec74d23` T-776 merge tip)

**Wave base:** `44a3b757` (wave 110 CLOSED — editor wave 133) — ancestor of HEAD.  
**Merges in wave:** `13d4882b` (T-736), `7f57a454` (T-755), `6ec74d23` (T-776).  
(Pre-merge SHAs `55b396ec`, `259e07d4`, `d5bd5b13` land the same diffs.)

**Gate (orchestrator):** PASS.  
**Method.** HOST cargo only (`~/.cargo/bin/cargo`, rustc 1.95.0). Private targets under
`~/.cache/tbd-target-wave134-{verify,perturb,base}` (deleted after this report). Mutations only in
detached worktrees at HEAD / base (`~/.cache/tbd-verify134-{perturb,base}` — deleted). Main checkout
left byte-clean at HEAD (`git status` → only pre-existing `?? tbd-target-T-736/` /
`?? tbd-target-T-755/` plus this report file; HEAD unchanged). No fix / commit / ticket filing.

**PRIMARY ATTACK.** For each strengthened pin, replay the ORIGINAL hollow regression named in the
ticket and confirm RED. Then probe NEW hollow shapes on the same pins.

**NO-DEFERRAL note.** Orchestrator will fix all findings — reported honestly; no self-authored deferrals.

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD `--list` | **987** | `cargo test -p website-frontend -- --list`, private dir |
| HEAD run | **987 passed / 0 failed** | same private dir; `--list` == run |
| base (`44a3b757`) `--list` | **986** | isolated worktree + private dir |
| Net delta | **+1** | one new named frontend pin |

**New frontend pin this wave**

| test | ticket |
|---|---|
| `eden_dock_left::t697_document_search::a_faction_only_hit_names_faction_not_the_first_text_attribute` | T-776 |

T-736 / T-755 / the other T-776 hardenings strengthen **existing** pins in place (no extra names).  
986 + 1 = **987**. Confirmed. List == run.

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — MAJOR | `eden_help.rs` `every_shared_channel_claimant_reads_live_state` — **claim-path “live state” is still a substring, not a gate**

**Evidence.**  
(a) T-776 correctly closed the *listener-wide* substring hollow: moving
`open.get_untracked()` **above** `if ev.key() == "Escape"` (unrelated decoy) → pin **RED**
(`eden_settings.rs#0 … claim path never reads live state`).  
(b) NEW shape (HEAD worktree): ungated Escape with a decoy *inside* the claim body —

```rust
if ev.key() == "Escape" {
    let _decoy = open.get_untracked();
    blur_focused_control();
    open.set(false);
}
```

→ `every_shared_channel_claimant_reads_live_state` **ok**.  
(c) `shared_channel_claim_gated` only asks whether the claim site **contains**
`get_untracked()` / `.escape()` (or an early-return latch in a match prelude) — it does not
require that the untracked read participate in the condition that decides whether the arm acts.

**Impact.** Today’s claimants *do* gate. A one-line move of the latch into the body (or any
`let _ = open.get_untracked()` decoy beside an unconditional Escape act) ships green while
restoring the exact shared-channel collision the pin narrates: Escape fires for a closed dialog
alongside every other claimant. Standing hollow-pin family (wave 127–134).

**Disposition — fix this wave.** Require the live-state read to be part of the **predicate** that
guards the act (e.g. `get_untracked()` appears before `&&` / in an `if` condition that wraps the
side effects, or `.escape()` is the act’s condition) — not merely present somewhere in the
balanced claim site. Perturbation (b) must go RED.

---

### F2 — MINOR | `eden_settings.rs` `a_default_value_is_built_in_exactly_one_place` — **alias-spelled Schema constructor still greens**

**Evidence.**  
(a) Path-spelled `SettingDefault::Schema { value: … }` second site → pin **RED** (T-755 close holds).  
(b) NEW shape: `use SettingDefault as SD; SD::Schema { value: …, pointer: … }` in a dead helper →
pin **ok** (`Self::Schema` count stays 1; path needle `SettingDefault::Schema` stays 0).

**Impact.** Same mechanism class T-755 just closed for path spelling — a second value-carrying
constructor remains a second source of truth while the pin stays green. Behavioural key-for-key
schema agreement still backstops live defaults.

**Disposition — fix this wave.** Count any `::Schema {` initialiser with `value:` (regardless of
path prefix / alias), or ban non-`Self::` constructions of the value-carrying variant entirely.
Perturbation (b) must go RED.

---

### F3 — NIT | `eden_top_strip.rs` ControlsHint position pin — **`mount_at > strip_at` ≠ tree containment**

**Evidence.**  
(a) ControlsHint moved **before** `<div class=STRIP_ROWS>` → pin **RED** (T-755 close holds).  
(b) NEW shape: ControlsHint moved to **after** the STRIP_ROWS closing `</div>` (still inside
`TopCommandStrip`’s `view!`) → pin **ok** (`mount_at > strip_at` still true).

**Impact.** The pin narrates “inside the STRIP_ROWS subtree, not beside/above it.” Byte-order after
the open tag does not prove ancestry; a sibling *after* the close still greens. Behavioural
chrome_hidden risk is lower than F1 — `TopCommandStrip` as a whole remains the Backspace gate —
but the POSITION claim is still overstated (same class as the wave-115 NIT this ticket closed).

**Disposition — fix this wave.** Brace/div-balance (or require the mount between STRIP open and its
matching close). Perturbation (b) must go RED.

---

## Claim attacks (by ticket)

### T-736 — one-commit + one-tail

| claim | result |
|---|---|
| `into_iter().map(…).count()` N-step import (ORIGINAL hollow) | **HELD** — leftover `let_into_itermap_count` → `the_import_applies_in_one_commit` **RED** |
| Same `into_iter` shape on **base** `44a3b757` (pre-strengthen) | **CONFIRMED HOLLOW** — base pin **ok** (spelling blacklist missed it) |
| Bare `loop { persist; break; }` | **HELD** — leftover tokens **RED** |
| Tail-in-loop `after_local_edit` inside `for id in ids` (ORIGINAL) | **HELD** — `assert_after_local_edit_outside_ids_loop` **RED** |
| Same tail-in-loop on **base** | **CONFIRMED HOLLOW** — base pin **ok** (count==1 blind) |

### T-755 — three narrated pins

| claim | result |
|---|---|
| Path-spelled `SettingDefault::Schema { value: … }` (ORIGINAL) | **HELD** — path_inits **RED** |
| `FLOW_DEFAULT_*` inside `from_schema_node` (ORIGINAL) | **HELD** — `no_flow_constant…` **RED** |
| Contours adjust line between bind and ladder call (ORIGINAL) | **HELD** — adjacent-feed assert **RED** |
| Upstream `let zoom = …` rebind before m_per_px | **HELD** — no-rebind assert **RED** |
| ControlsHint **before** STRIP_ROWS (ORIGINAL outside) | **HELD** — `mount_at > strip_at` **RED** |
| ControlsHint **after** STRIP close | **UNPINNED** → **F3 NIT** |
| Alias `SD::Schema { value: … }` | **UNPINNED** → **F2 MINOR** |

### T-776 — four census/search pins

| claim | result |
|---|---|
| Unrelated `get_untracked()` outside Escape claim (ORIGINAL substring) | **HELD** — per-claim pin **RED** |
| `get_untracked()` decoy **inside** Escape body, ungated act | **UNPINNED** → **F1 MAJOR** |
| Second `fn keydown_arms(` outside old surface list (ORIGINAL) | **HELD** — crate-wide walk finds `toast.rs` → **RED** |
| Silent-drop keydown (no `ev.key`/`ev.code`) (ORIGINAL) | **HELD** — `every_editor_surface_listener_is_censused` **RED** |
| Faction-only hit mislabelled as first text attribute (ORIGINAL) | **HELD** — `a_faction_only_hit…` **RED** (`field=label`) |

---

## Standing attack surfaces (wave 127–133 families)

| surface | attack | result |
|---|---|---|
| 1. z=None flatten / `keep_z_rows` | Gut body → always `None` | **FAILED to break** — sticky-z pin **RED** |
| 2. Hollow source pins | Original charter hollows all RED; **three new hollow shapes** on T-755/T-776 pins | **F1 MAJOR / F2 MINOR / F3 NIT** |
| 3. Stale thread_local / `Rc::ptr_eq` | `git diff 44a3b757..6ec74d23 -G 'thread_local\|Rc::ptr_eq'` empty on frontend owns | **FAILED to break** |
| 4. Affordance clickable IFF | `a_finding_row_is_clickable_iff…` green | **FAILED to break** |
| 5. Shared `CARGO_TARGET_DIR` / list vs run | Private `~/.cache/tbd-target-wave134-*` only; list **987** == run **987**; dirs deleted | **FAILED to break** |

---

## Main safe to build the next wave on?

**no** — not until F1–F3 are fixed in-wave (orchestrator NO-DEFERRAL).  
The charter strengthenings **do** close every ORIGINAL hollow named in T-736 / T-755 / T-776
(all attacked-and-RED; base contrast proves the pre-wave pins were hollow). Residual risk is the
same class this wave was filed to kill: pins whose assertion is still weaker than the sentence
describing them — especially F1 on the Escape shared-channel exemption.

---

## Verified-clean register (claims re-proved)

- T-736: leftover-token one-commit pin catches `into_iter` / `loop` (base blacklist did not);
  brace-matched one-tail pin catches tail-in-loop (base count==1 did not).  
- T-755: path-spelled Schema, FLOW_DEFAULT in `from_schema_node`, contours adjust + zoom rebind,
  ControlsHint-before-STRIP all RED.  
- T-776: per-claim live-state (listener-wide decoy), crate-wide extractor census, silent-drop
  keydown fail-closed, faction-only field label — all RED on the original shapes.  
- Suite: 987 list == 987 run; base 986; +1.  
- Main: `6ec74d23`, tracked tree untouched by this verify.

---

## Attacked and FAILED to break

1. **T-736 `into_iter` one-commit** — leftover tokens RED (base was GREEN).  
2. **T-736 `loop` one-commit** — leftover tokens RED.  
3. **T-736 tail-in-loop** — outside-loop assert RED (base was GREEN).  
4. **T-755 path-spelled Schema** — path_inits RED.  
5. **T-755 FLOW_DEFAULT in `from_schema_node`** — banned-token scan RED.  
6. **T-755 contours adjust line** — adjacency RED.  
7. **T-755 zoom rebind** — no-`let zoom` RED.  
8. **T-755 ControlsHint before STRIP** — order assert RED.  
9. **T-776 listener-wide get_untracked decoy** — per-claim pin RED.  
10. **T-776 second `keydown_arms` in `toast.rs`** — crate walk RED.  
11. **T-776 silent-drop keydown** — census assert RED.  
12. **T-776 faction field mislabel** — behavioural pin RED.  
13. **`keep_z_rows` sticky-z** — full gut RED.  
14. **Affordance IFF** — green.  
15. **Private-target list/run mismatch** — none (987==987).

---

## Not attacked successfully (findings)

- **Escape claim-site decoy untracked read** — see **F1**.  
- **Alias-spelled Schema constructor** — see **F2**.  
- **ControlsHint after STRIP close** — see **F3**.

---

## Focused re-verify

**When:** 2026-08-08 post-fix (`431ef5a3` F1, `a9de7432` F2, `d30ac7d8` F3).  
**HEAD:** `d30ac7d8b29ab8f06b98a321f53135a9b8d38180`  
**Method.** HOST cargo (`~/.cargo/bin/cargo`, rustc 1.95.0). Private target
`~/.cache/tbd-target-w134-reverify`. Mutations only in detached worktree at HEAD
(`~/.cache/tbd-verify134-reverify` — removed after). Main checkout left byte-identical
except this append (`git status` → only pre-existing `?? tbd-target-T-736/` /
`?? tbd-target-T-755/` plus this report file; HEAD unchanged). No fix / commit.

### Suite

| measurement | value |
|---|---|
| `--list` | **987** |
| run | **987 passed / 0 failed** |
| vs pre-fix verify | same **987** (fixes strengthen existing pins; no new named frontend pins) |

List == run. Confirmed.

### F1 (`431ef5a3`) — Escape body decoy → RED (then restore)

**Perturbation (worktree):** `MissionSettingsDialog` Escape arm rewritten to ungated
`if ev.key() == "Escape" { let _decoy = open.get_untracked(); … act }` (decoy inside body).

| test | result |
|---|---|
| `eden_help::keymap_census::every_shared_channel_claimant_reads_live_state` | **RED** — exit 101; `eden_settings.rs#0` Escape claim path never gates the act on live state |

**Restore:** `git checkout -- eden_settings.rs` → pin **ok**.

### F2 (`a9de7432`) — `SD::Schema` alias → RED (then restore)

**Perturbation (worktree):** dead helper
`use SettingDefault as SD; SD::Schema { value: Null, pointer: … }`.

| test | result |
|---|---|
| `eden_settings::t688_aggregated_settings::a_default_value_is_built_in_exactly_one_place` | **RED** — exit 101; `left: 2` / `right: 1` (`::Schema { value: }` count)

**Restore:** `git checkout -- eden_settings.rs` → pin **ok**.

### F3 (`d30ac7d8`) — ControlsHint after STRIP close → RED (then restore)

**Perturbation (worktree):** move `<crate::eden_help::ControlsHint open=hint_open />` to
immediately after the matching `</div>` of `STRIP_ROWS` (still inside `TopCommandStrip`'s
`view!`).

| test | result |
|---|---|
| `eden_top_strip::t692_help_surface::the_toggle_is_checked_in_the_gutter_and_mounted_here` | **RED** — exit 101; must sit between STRIP open and matching close |

**Restore:** `git checkout -- eden_top_strip.rs` → pin **ok**.

### Findings

| severity | count |
|---|---|
| CRITICAL | **0** |
| MAJOR | **0** |
| NIT | **0** |

All three prior findings (**F1 MAJOR, F2 MINOR, F3 NIT**) are **CLOSED** — original hollow
attacks now go RED; restores green. Post-fix gate PASS (orchestrator).

### Main safe to build the next wave on?

**yes** — F1–F3 closed; suite 987 == 987; three named hollow shapes RED-then-restored.
