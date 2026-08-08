# Wave 135 — adversarial verify (T-744 · T-751 · T-757)

**Verified HEAD:** `a90cb8f6934e0c63085290a75f8a5d7d3d4e071e`  
(`a90cb8f6` wave 135 fixup: rustfmt after merges)

**Wave base:** `1edb1d72` (wave 111 CLOSED — editor wave 134) — ancestor of HEAD.  
**Merges in wave:** `e72047db`/`8030a5b2` (T-744), `c2dca4f4`/`929595ff` (T-751), `42300a78`/`4dc8c7f0` (T-757).

**Gate (orchestrator):** PASS.  
**Method.** HOST cargo only (`~/.cargo/bin/cargo`, rustc 1.95.0). Private targets under
`~/.cache/tbd-target-wave135-{verify,perturb,base}` (deleted after this report). Mutations only in
detached worktrees at HEAD / base (`~/.cache/tbd-verify135-{perturb,base}` — deleted). Main checkout
left byte-clean at HEAD (`git status` → only pre-existing `?? tbd-target-T-736/` /
`?? tbd-target-T-755/` plus this report file; HEAD unchanged). No fix / commit / ticket filing.

**PRIMARY ATTACK.** For each strengthened pin, replay the ORIGINAL hollow / defect named in the
ticket and confirm RED. Then probe NEW hollow shapes on the same pins.

**NO-DEFERRAL note.** Remaining schema embeds (dock_right / context_menu) and the store undo layer-
filing assert gap are reported honestly below — including where the slice already labelled them
`found_not_fixed` outside `owns`. No self-authored deferral of in-scope pin strength.

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD `--list` | **992** | `cargo test -p website-frontend -- --list`, private dir |
| HEAD run | **992 passed / 0 failed** | same private dir; `--list` == run |
| base (`1edb1d72`) `--list` | **987** | isolated worktree + private dir |
| Net delta | **+5** | five new named frontend pins |
| map-engine-core pin | **1 new** | `editor_only_and_transitional_keys_stay_absent_from_compile_known_list` (`--features mission`); absent on base |

**New frontend pins this wave**

| test | ticket |
|---|---|
| `attributes::tests::read_attrs_gates_existence_on_raw_rows_not_soa_membership` | T-744 |
| `attributes::tests::attributes_modal_none_arm_still_closes_on_true_absence` | T-744 |
| `ui::t633_range_and_select::disabled_select_chevron_dims_with_peer_disabled` | T-751 |
| `eden_dock_right::tests::favourites_place_arm_stays_clone_free` | T-751 |
| `eden_settings::t688_aggregated_settings::zones_and_settings_share_one_mission_schema_embed` | T-757 |

987 + 5 = **992**. Confirmed. List == run.

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — MINOR | T-744 `read_attrs_gates_existence_on_raw_rows_not_soa_membership` — **raw-gate needle is presence-only**

**Evidence.**  
(a) ORIGINAL SoA Option gate restored (`let row = soa.ids.iter().position(|s| s == id)?`) → pin **RED**
(missing `!rows.contains_key(id)` / banned `?` gate present).  
(b) NEW shape: keep the substring `if !rows.contains_key(id) { … }` but **delete `return None;`**
(empty arm) while leaving the SoA `if let Some(row)` path intact → pin **GREEN**.  
(c) Pin asserts `body.contains("!rows.contains_key(id)")` and
`!body.contains("soa.ids.iter().position(|s| s == id)?")` plus fallback
`slot_attrs_from_raw` — it does **not** require that the raw check actually returns `None`.

**Impact.** Today’s production body does return `None`. A one-line gut of that return restores
“missing id still yields `Some`” (Attributes stays open on true undo-away / delete with empty or
stale fields) while the pin that narrates “Option must gate on raw membership” stays green.
Hide-closes-like-undo remains caught by (a); existence-means-absence is not pinned.

**Disposition — fix this wave.** Require the raw check to be a real absence exit (e.g. the
`return None` / early-`None` sits in the same arm as `!rows.contains_key(id)`, or a behavioural
host test that a deleted id closes while a hidden id stays open). Perturbation (b) must go RED.

---

### F2 — MINOR | T-757 `zones_and_settings_share_one_mission_schema_embed` — **`~40 KB` lore check cannot see comments**

**Evidence.**  
(a) Re-embed `include_str!(…mission.schema.json…)` in `eden_settings` → pin **RED**.  
(b) Dead second embed beside the shared alias → pin **RED**.  
(c) Restore the ticket’s stale **doc-comment** lore `~40 KB` on `eden_zones` → pin **GREEN**.  
(d) Same `~40 KB` planted as a **string literal** next to `MISSION_SCHEMA` → pin **RED**.  
(e) Pin uses `live_source`, which blanks comments; the ticket defect was specifically a comment
(`eden_zones` “~40 KB” vs ~91 KB).

**Impact.** Zones↔settings single-embed half is load-bearing (a/b). The stale-size half of T-757
is narrated but not enforced against the defect class that filed the ticket. Comment lore can
return without a RED.

**Disposition — fix this wave.** Scrub with a comment-preserving reader for the lore assert, or
`include_str!` the zones file raw for that one needle. Perturbation (c) must go RED.

---

### F3 — NIT | T-751 `favourites_place_arm_stays_clone_free` — **vehicle-only; Character/Object may clone**

**Evidence.**  
(a) Favourites vehicle arm switched to `begin_place_vehicle(payload.clone())` → pin **RED**
(clone count ≠ 1).  
(b) Character + Object favourites arms switched to `.clone()` while Vehicle stays move-form →
pin **GREEN**.

**Impact.** T-215’s inspected needle is the vehicle clone expression; vehicle distinctness holds
under (a). The pin title / ticket prose say the favourites arm stays clone-free — Character/Object
can still grow clones without a RED. Latent; no current T-215 Character/Object source pin to
poison.

**Disposition — fix this wave (or narrow the claim).** Pin all three match arms’ move-forms, or
rewrite the pin doc to say “vehicle arm only (T-215 needle)”.

---

### H1 — found_not_fixed (outside owns) | T-751 store undo layer-filing assert

**Evidence.** `comments_file_into_layers_and_each_gesture_is_one_undo_step`
(`crates/map-engine-core/src/doc/store.rs`) still: after `remove_comment` + `undo()`, asserts
`comment_count() == 1` only. **No** post-undo `entityIds` / L2 filing assert
(`entityIds` / `L2` absent after `doc.undo()`). Slice owns excluded `store.rs`; ticket item (3)
remains open.

**Impact.** Behaviour is still near-certain (one txn, both maps in undo scope) but unpinned —
same wave-114 NIT-3 honesty gap.

**Disposition — report only** (outside owns; do not pretend closed).

---

### H2 — found_not_fixed (outside owns) | T-757 remaining `mission.schema.json` embeds

**Evidence.** Frontend `include_str!(…mission.schema.json…)` sites after this wave:

| site | kind |
|---|---|
| `eden_zones.rs` (`pub(crate) MISSION_SCHEMA`) | **prod** — intended single shared embed |
| `eden_settings.rs` | **alias** `crate::eden_zones::MISSION_SCHEMA` — T-757 closed for owns |
| `eden_dock_right.rs:2801–2802` `MISSION_SCHEMA_JSON` | **prod** — still a second wasm embed; feeds `marker_icons()` |
| `context_menu.rs:1615–1616` | **test-only** (inside `#[test]` formation vocabulary pin) |

`eden_dock_right` header still says zones+settings “already make” the same embed (“Three embeds”)
— settings no longer embeds; dock_right remains a real second prod copy. Slice owns were
zones+settings only.

**Impact.** Ticket’s zones↔settings double-embed is closed. Wasm can still carry a second full
schema via dock_right. Test-only context_menu embed does not ship in release wasm the same way
but remains a third compile-time include.

**Disposition — report only** (outside owns; do not pretend “single embed crate-wide”).

---

## Claim attacks (by ticket)

### T-744 — Attributes stay open when slot hidden

| claim | result |
|---|---|
| Restore SoA `position()?` Option gate (ORIGINAL) | **HELD** — pin **RED** |
| Drop `slot_attrs_from_raw` else → `None` (hide closes again) | **HELD** — pin **RED** |
| Drop modal `None`-arm `close_attributes` (Esc remains) | **HELD** — dual-close pin **RED** |
| Keep `!rows.contains_key(id)` needle, remove `return None` | **UNPINNED** → **F1 MINOR** |

### T-751 — peer-disabled + favourites clone-free + compile divergence

| claim | result |
|---|---|
| Remove `peer-disabled:opacity-30` (ORIGINAL) | **HELD** — pin **RED** |
| Strip `peer` + plant decoy outside `Select` body | **HELD** — pin **RED** (scoped to `only_body(Select)`) |
| Wrong opacity (`opacity-40`) | **HELD** — pin **RED** |
| Favourites vehicle grows `.clone()` (ORIGINAL) | **HELD** — pin **RED** |
| Comment haystack + vehicle clone (raw `include_str` file) | **HELD** — clone-count **RED** |
| Character/Object favourites clone, vehicle move | **UNPINNED** → **F3 NIT** |
| Add `"comments"` to compile known list (ORIGINAL) | **HELD** — pin **RED** (`--features mission`) |
| Helper returns true for `comments`, const clean | **HELD** — pin **RED** |
| Store undo asserts layer filing restored | **NOT FIXED** → **H1** (outside owns) |

### T-757 — single MISSION_SCHEMA embed (zones+settings)

| claim | result |
|---|---|
| Settings re-`include_str!` (ORIGINAL) | **HELD** — pin **RED** |
| Dead second embed in settings | **HELD** — pin **RED** |
| Stale `~40 KB` in **comment** | **UNPINNED** → **F2 MINOR** |
| Stale `~40 KB` in **string literal** | **HELD** — pin **RED** |
| dock_right / context_menu embeds gone | **NOT FIXED** → **H2** (outside owns; expected) |

---

## Standing attack surfaces (wave 127–134 families)

| surface | attack | result |
|---|---|---|
| 1. z=None flatten / `keep_z_rows` | Gut body → always `None` | **FAILED to break** — `an_attributes_x_or_y_commit_carries_the_slots_current_z_back_in` **RED** |
| 2. Hollow source pins | Original charter hollows RED; **two new hollow shapes** on T-744/T-757 (+ vehicle-only NIT) | **F1 MINOR / F2 MINOR / F3 NIT** |
| 3. Stale thread_local / `Rc::ptr_eq` | `git diff 1edb1d72..a90cb8f6 -G 'thread_local\|Rc::ptr_eq'` empty on frontend/crates | **FAILED to break** |
| 4. Affordance clickable IFF | `a_finding_row_is_clickable_iff…` green | **FAILED to break** |
| 5. Shared `CARGO_TARGET_DIR` / list vs run | Private `~/.cache/tbd-target-wave135-*` only; list **992** == run **992**; dirs deleted | **FAILED to break** |

---

## Main safe to build the next wave on?

**no** — not until F1–F2 are fixed in-wave (orchestrator NO-DEFERRAL). F3 is a NIT on claim
width. H1/H2 are honest outside-owns residues — **not** closed by this wave and must not be
summarised as fixed.

The charter strengthenings **do** close every ORIGINAL hollow named for the in-owns work
(SoA gate, peer-disabled, favourites vehicle clone, compile key absences, settings re-embed),
all attacked-and-RED; base contrast shows the five frontend pins (+ compile pin) are absent on
`1edb1d72`.

---

## Verified-clean register (claims re-proved)

- T-744: raw existence gate + raw fallback + dual `close_attributes` — ORIGINAL SoA/`None`-arm
  regressions RED; inverted-return hollow remains (F1).  
- T-751: Select `peer` + `peer-disabled:opacity-30` scoped to Select body; favourites vehicle
  move-vs-clone census; compile known-list absences for
  zones/compositions/triggers/comments/connections — all RED on original shapes.  
- T-757: zones+settings share one `include_str!`; settings alias; settings cannot re-embed —
  RED on re-embed. Lore half hollow on comments (F2).  
- Suite: 992 list == 992 run; base 987; +5.  
- Main: `a90cb8f6`, tracked tree untouched by this verify.

---

## Attacked and FAILED to break

1. **T-744 SoA Option gate restored** — pin RED.  
2. **T-744 hide fallback removed (`else None`)** — pin RED.  
3. **T-744 modal None-arm close dropped** — dual-close pin RED.  
4. **T-751 peer-disabled removed** — pin RED.  
5. **T-751 peer decoy outside Select** — pin RED.  
6. **T-751 wrong peer-disabled opacity** — pin RED.  
7. **T-751 favourites vehicle clone** — pin RED.  
8. **T-751 comment-haystack + vehicle clone** — clone-count RED.  
9. **T-751 compile list gains `comments`** — pin RED.  
10. **T-751 compile helper lies about `comments`** — pin RED.  
11. **T-757 settings re-embed** — pin RED.  
12. **T-757 dead second settings embed** — pin RED.  
13. **T-757 `~40 KB` as string literal** — pin RED.  
14. **`keep_z_rows` sticky-z gut** — attributes sticky-z pin RED.  
15. **Affordance IFF** — green.  
16. **Private-target list/run mismatch** — none (992==992).  
17. **thread_local / Rc::ptr_eq wave diff** — empty.

---

## Attacked and BROKE the claim (new hollows / honesty)

1. **T-744 inverted raw gate (needle without `return None`)** — pin GREEN → **F1**.  
2. **T-757 `~40 KB` restored in doc comment** — pin GREEN → **F2**.  
3. **T-751 Character/Object favourites clone** — pin GREEN → **F3**.  
4. **Store undo layer filing still unasserted** — **H1** (outside owns).  
5. **dock_right prod (+ context_menu test) schema embeds remain** — **H2** (outside owns).

---

## Cleanup

- Worktrees: `git worktree remove` `~/.cache/tbd-verify135-{perturb,base}`  
- Targets: `rm -rf ~/.cache/tbd-target-wave135-{verify,perturb,base}`  
- Main HEAD unchanged at `a90cb8f6`.

## Focused re-verify

**When:** 2026-08-08 post-fix (`427a82ed` F1, `c5c30132` F2, `f0bc2848` F3, `2f9d0596` H1, `0ae7dbaa` H2, `8e8bd78f` rustfmt).  
**HEAD:** `8e8bd78f74af671cb8104b63b50ffc910d793784`  
**Method.** HOST cargo (`~/.cargo/bin/cargo`, rustc 1.95.0). Private target
`~/.cache/tbd-target-w135-reverify`. Mutations only in detached worktree at HEAD
(`~/.cache/tbd-verify135-reverify` — removed after). Main checkout left byte-identical
except this append (`git status` → only pre-existing `?? tbd-target-T-736/` /
`?? tbd-target-T-755/` plus this report file; HEAD unchanged). No fix / commit.

### Suite

| measurement | value |
|---|---|
| frontend `--list` | **992** |
| frontend run | **992 passed / 0 failed** |
| vs pre-fix verify | same **992** (fixes strengthen existing pins / production; no new named frontend pins) |
| H1 pin (`map-engine-core --features doc,mission --lib`) | **ok** at HEAD |

List == run. Confirmed.

### F1 (`427a82ed`) — empty raw-gate arm → RED (then restore)

**Perturbation (worktree):** `read_attrs` keeps `if !rows.contains_key(id) { … }` but deletes
`return None;` (empty arm — original adversarial hollow).

| test | result |
|---|---|
| `attributes::tests::read_attrs_gates_existence_on_raw_rows_not_soa_membership` | **RED** — exit 101; `raw absence arm must exit with None (empty arm must RED)` |

**Restore:** `git checkout -- editor_ops.rs` → pin **ok**.

### F2 (`c5c30132`) — `~40 KB` doc-comment lore → RED (then restore)

**Perturbation (worktree):** restore stale `~40 KB` in the `MISSION_SCHEMA` doc-comment on
`eden_zones.rs` (original hollow `live_source` could not see).

| test | result |
|---|---|
| `eden_settings::t688_aggregated_settings::zones_and_settings_share_one_mission_schema_embed` | **RED** — exit 101; `must not restate a drifted ~40 KB embed cost (comments count)` |

**Restore:** `git checkout -- eden_zones.rs` → pin **ok**.

### F3 (`f0bc2848`) — Character/Object favourites `.clone()` → RED (then restore)

**Perturbation (worktree):** `arm_favourite_place` Character + Object arms use
`payload.clone()`; Vehicle stays move-form (original hollow).

| test | result |
|---|---|
| `eden_dock_right::tests::favourites_place_arm_stays_clone_free` | **RED** — exit 101; `must stay clone-free across Character/Object/Vehicle` |

**Restore:** `git checkout -- eden_dock_right.rs` → pin **ok**.

### H1 (`2f9d0596`) — undo restores comment only (not L2) → RED (then restore)

**Perturbation (worktree):** split `remove_comment` into two txns (unfile, then delete row) so
one `undo()` restores `commentsById` while L2 `entityIds` stays empty — the hollow the
pre-fix count-only assert missed.

| test | result |
|---|---|
| `doc::store::tests::comments_file_into_layers_and_each_gesture_is_one_undo_step` | **RED** — exit 101; `Ctrl+Z must put c1 back in L2 entityIds… []` |

**Restore:** `git checkout -- store.rs` → pin **ok**.

### H2 (`0ae7dbaa`) — dock_right / context_menu re-`include_str!` → hollow returns

**At HEAD:** exactly **one** prod `include_str!(…mission.schema.json…)` —
`eden_zones.rs:629`. `eden_dock_right` and `context_menu` alias
`crate::eden_zones::MISSION_SCHEMA`.

**Perturbation (worktree):** restore dock_right prod embed + context_menu test embed
(original H2 hollow).

| check | result |
|---|---|
| Prod+test `include_str!(…mission.schema.json…)` sites | **2 returned** (dock_right + context_menu) beside zones |
| `zones_and_settings_share_one_mission_schema_embed` | **GREEN** (expected — pin still scopes zones+settings only) |

**Restore:** aliases restored; single prod embed again. H2 is production-closed; crate-wide
single-embed is **not** Class-R pinned beyond zones+settings (honesty, not a reopen of the
fixed dock_right/context_menu sites).

### Findings this pass

| severity | count |
|---|---|
| CRITICAL | **0** |
| MAJOR | **0** |
| MINOR | **0** |
| NIT | **0** |

All five prior items (**F1 MINOR, F2 MINOR, F3 NIT, H1, H2**) are **CLOSED** — original
hollow attacks now RED (F1–F3, H1) or hollow-returned then restored (H2); restores green.
Post-fix gate PASS (orchestrator).

### Main safe to build the next wave on?

**yes** — F1–F3 + H1 pin-backed RED-then-restore; H2 single prod embed held by census;
frontend suite **992 == 992**; main tracked tree untouched aside from this artifact append.
