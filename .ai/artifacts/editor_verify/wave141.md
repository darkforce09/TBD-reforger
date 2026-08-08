# Wave 141 — adversarial verify (T-748 · T-738)

**Verified HEAD:** `e3147bf11e4a72c32e591b60161ee9980ed1a09e`  
(`e3147bf1` merge slice/T-748 — tip of editor wave 141)

**Wave base:** `d82bca08` (wave 117 CLOSED — editor wave 140) — ancestor of HEAD.  
**Slice commits / merges:**

| ticket | slice sha | merge sha |
|---|---|---|
| T-738 | `959f61b0` | `fe51e454` |
| T-748 | `74d92fdf` | `e3147bf1` |

**Gate (orchestrator):** PASS (30/30) over base `d82bca08`.  
**Method.** HOST cargo only (`~/.cargo/bin/cargo`, rustc 1.95.0). Private targets under
`~/.cache/tbd-target-w141-verify*` (deleted after this report). Mutations only in detached
worktrees at HEAD / base (`~/.cache/tbd-verify141-{perturb,base}` — deleted). Main checkout left
byte-clean at HEAD aside from this report. No fix / commit / ticket filing / registry edit.

**PRIMARY ATTACK.** T-748 `MissionComments` lane + `comments_bind` feed from
`after_doc_change`/`rebind_engine_from_doc` and pick-bridge isolation (not `slots_bind_soa` /
`last_ids`); T-738 Escape help shared-channel honesty +
`known_escape_ev_key_sites_are_censused` / `escape_help_documents_the_shared_channel`. Named
`found_not_fixed` composability residue re-checked. Suite reconciliation base→HEAD with
**worktree-cwd rebuilds** (a first base `--list` against a polluted target dir falsely showed HEAD
pins on `d82bca08` — discarded; clean rebuild: base 1023 FE / 61 MER).

**NO-DEFERRAL note.** Every severity reported honestly — including the named composability residue
and a hollow feed-pin class proven by comment-out. Registry rows for T-738/T-748 still read
`deferred` on this HEAD (process note for the orchestrator close; not a code defect and not filed
here).

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD frontend `--list` | **1025** | private dir `tbd-target-w141-verify-fe` |
| HEAD frontend run | **1025 passed / 0 failed** | same dir |
| base (`d82bca08`) frontend `--list` | **1023** | worktree cwd + **fresh** private dir |
| Net frontend delta | **+2** | T-738 pins below |
| HEAD map-engine-render `--lib --list` / run | **64** / **64 passed** | `tbd-target-w141-verify-mer` |
| base map-engine-render `--lib --list` | **61** | clean worktree rebuild |
| Net render delta | **+3** | T-748 pins below |
| HEAD map-engine-core `--all-features` | **635** listed unit / **634 pass + 1 ignored** | Makefile / preferred |

**New frontend pins this wave**

| test | ticket |
|---|---|
| `eden_help::t692_help_covers_every_binding::known_escape_ev_key_sites_are_censused` | T-738 |
| `eden_help::t692_help_covers_every_binding::escape_help_documents_the_shared_channel` | T-738 |

1023 + 2 = **1025**. Confirmed. List == run.

**New map-engine-render pins this wave**

| test | ticket |
|---|---|
| `draw_order::lane_order_pins::mission_comments_sit_between_markers_and_squad_links` | T-748 |
| `draw_order::t748_comments_bind_feed::rebind_and_after_doc_change_both_feed_comments_bind` | T-748 |
| `draw_order::t748_comments_bind_pick_bridge::comments_bind_body_does_not_touch_last_ids` | T-748 |

61 + 3 = **64**. Confirmed. List == run.

Base had **neither** T-738 pin name nor any `t748` / `mission_comments` render pin (after clean rebuild).

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — MINOR / NOTE | T-748 — composability / selection lane still absent (named `found_not_fixed`)

**Evidence.** Slice commit message and ticket claim this residue. Live on HEAD:

```text
apps/website/frontend/src/outliner.rs:101
  comment is in no selection lane and has no Attributes modal
apps/website/frontend/src/editor_ops.rs:3294
  place_comment: This does NOT touch the selection.
```

Map glyph half is shipped: `LaneRole::MissionComments` (`draw_order.rs:70`),
`comments_bind` (`engine.rs:4479`), fed from both `rebind_engine_from_doc` (`mission_history.rs:332`)
and `after_doc_change` (`:393`) via `comment_lane_xy` (`:434`). `place_comment` ends in
`after_local_edit()` (`editor_ops.rs:3309`) → `after_doc_change`.

**Impact.** Spec row "COMPOSABLE" remains unmet; operators still cannot put a comment into a
composition via selection. Glyph / undo rebind half closes. Ticket already disclosed this.

**Disposition — NOTE (named found_not_fixed).** No new ticket from this verifier.

---

### F2 — NIT | T-748 — feed pin is comment-substring hollow

**Evidence.** `t748_comments_bind_feed` source-inspects function bodies for the tokens
`comments_bind` and `comment_lane_xy` (`draw_order.rs` `t748_comments_bind_feed`). Hollow attack
**H5** (detached worktree): replaced the live `after_doc_change` call with

```rust
// e.comments_bind(&comment_lane_xy(doc)); // keep string for hollow check
```

while leaving `rebind_engine_from_doc`'s real call intact → pin stayed **GREEN**.

**Impact.** A future edit that comments out the `after_doc_change` feeder (undo/redo/persist path)
would leave the Class-R pin green while glyphs go stale after document edits. Production HEAD still
has both live calls (verified). Same hollow class as other token-presence source pins; the pick-bridge
pin (H2) and delete-call attack (H1) remain load-bearing.

**Disposition — NIT.** No wave work. Orchestrator may fold into a future pin-hardening pass if one
exists; not filed here.

---

## Hollow-pin attacks

| id | mutation (detached worktree) | pin | result |
|---|---|---|---|
| H1 | delete `comments_bind` call from `after_doc_change` only | `t748_comments_bind_feed` | **RED** |
| H2 | `comments_bind` body references `slot_bridge.last_ids` | `t748_comments_bind_pick_bridge` | **RED** |
| H3 | Escape help action → `"Dismiss measurement tools"` | `escape_help_documents_the_shared_channel` | **RED** (missing `Save`) |
| H4 | `faction_manager.rs` `Escape` → `Escapx` | `known_escape_ev_key_sites_are_censused` | **RED** |
| H5 | comment-out `after_doc_change` call, keep token text | `t748_comments_bind_feed` | **GREEN** (hollow — F2) |

H1–H4 went RED as claimed for the load-bearing mutations. H5 falsifies absolute trust in the feed
pin's comment resistance.

---

## Claims attacked and FAILED to break (verified-clean register)

| claim | attack | result |
|---|---|---|
| T-748 `LaneRole::MissionComments` between markers and squad links | read `lane_order` 25; lane pin present; order asserts | PASS |
| T-748 `comments_bind` uploads `MissionComments`, not pick bridge | H2; body has `upload_slot_role_lane(MissionComments)` / no `slots_bind_soa` | pin RED on last_ids; clean PASS |
| T-748 both `rebind` + `after_doc_change` feed the lane | H1 delete after feed | pin RED; clean PASS (live calls at :332/:393) |
| T-748 place path reaches rebind tail | `place_comment` → `after_local_edit` | present |
| T-748 `comment_lane_xy` reads `position.{x,z}` matching store | `set_comment_position` / `add_comment` use x,z | match |
| T-748 draw path binds slot atlas for MissionComments | `engine.rs` match arm `MissionComments => slot_base_bind` | present |
| T-748 composability still out of owns | outliner + place_comment docs | residue true (F1) |
| T-738 scrape already widened (T-703/T-774) | `known_escape…` requires 7 `ev.key()` Escape files incl. faction/orbat | PASS; H4 RED |
| T-738 Escape help documents shared channel | live action names measurement/Save/Attributes/picker/settings/context/Faction/ORBAT/this card; H3 | pin RED; clean PASS |
| T-738 prose census numbers derived | `the_prose_census_numbers_are_derived` | PASS (doc still says twelve, under derived pin) |
| T-738 shared channel still legal | `every_shared_channel_is_really_shared` | PASS |

---

## Safe? table

| question | answer |
|---|---|
| Main compile/test surface green for this wave's crates? | **yes** — frontend 1025/1025; map-engine-render 64/64; map-engine-core `--all-features` 634 pass + 1 ignored |
| Any BLOCKER (main broken / vacuous gate success)? | **no** |
| Any MAJOR (ticket claim false / authored work at risk)? | **no** |
| Named found_not_fixed residues still true? | **yes** — F1 (composability / selection) |
| Hollow pins load-bearing? | **mostly** — H1–H4 RED; H5 proves feed pin comment-hollow (F2 NIT) |
| Main safe to build the next wave on? | **yes** |

---

## Main safe to build the next wave on?

**yes**

Severity tally: **BLOCKER 0 · MAJOR 0 · MINOR/NOTE 1 · NIT 1**

**SAFE: yes** — BLOCKER 0, MAJOR 0, MINOR/NOTE 1, NIT 1
