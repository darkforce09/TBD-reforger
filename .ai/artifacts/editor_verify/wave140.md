# Wave 140 — adversarial verify (T-770 · T-768 · T-740)

**Verified HEAD:** `73d3646dbd924dbc29a43a718084249676fe224e`  
(`73d3646d` merge slice/T-768 — tip of editor wave 140)

**Wave base:** `19663d27` (wave 116 CLOSED — editor wave 139) — ancestor of HEAD.  
**Slice commits / merges:**

| ticket | slice sha | merge sha |
|---|---|---|
| T-740 | `c9178995` | `ee8302d5` |
| T-770 | `f3692f73` | `f0ad063b` |
| T-768 | `1c16c20a` | `73d3646d` |

**Gate (orchestrator):** PASS (30/30) over base `19663d27`.  
**Method.** HOST cargo only (`~/.cargo/bin/cargo`, rustc 1.95.0). Private targets under
`~/.cache/tbd-target-w140-verify*` (deleted after this report). Mutations only in detached
worktrees at HEAD / base (`~/.cache/tbd-verify140-{perturb,base}` — deleted). Main checkout left
byte-clean at HEAD aside from this report. No fix / commit / ticket filing / registry edit.

**PRIMARY ATTACK.** T-770 ack-vs-invocation hollow + production sink wiring + `set_loadout`
residue; T-768 LMB `complete_connect` caller / Esc / pointercancel hollow pins + RMB Complete
sibling; T-740 KeyY Cmd chord pin vs `mission_history` `ctrl\|\|meta`. Empty-txn undo probe on
missing-id `update_slot_loadout` (falsified). Suite reconciliation base→HEAD.

**NO-DEFERRAL note.** Every severity reported honestly — including `found_not_fixed` residues the
tickets already named. Registry rows for T-740/T-768/T-770 still read `deferred` on this HEAD
(process note for the orchestrator close; not a code defect and not filed here).

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD frontend `--list` | **1023** | `cargo test -p website-frontend -- --list`, private dir |
| HEAD frontend run | **1023 passed / 0 failed** | `~/.cache/tbd-target-w140-verify/frontend-clean` |
| base (`19663d27`) frontend `--list` | **1018** | isolated worktree + private dir |
| Net frontend delta | **+5** | see new pins below |
| HEAD map-engine-core bare `--list` | **140** | no features |
| HEAD map-engine-core `--features doc,mission` `--list` | **502** | wave.sh gate feature set |
| HEAD map-engine-core `--all-features` `--list` / run | **635** / **634 pass + 1 ignored** | Makefile / preferred target |

**New frontend pins this wave**

| test | ticket |
|---|---|
| `eden_help::t692_help_covers_every_binding::redo_chord_documents_cmd_for_key_y` | T-740 |
| `mission_editor::t768_connect_lmb_complete::pending_click_calls_complete_connect_when_armed` | T-768 |
| `mission_editor::t768_connect_lmb_complete::escape_arm_cancels_pending_connect` | T-768 |
| `mission_editor::t768_connect_lmb_complete::pointercancel_cancels_pending_connect` | T-768 |
| `mission_editor::t768_connect_lmb_complete::complete_connect_caller_is_load_bearing` | T-768 |

1018 + 5 = **1023**. Confirmed. List == run. T-770 widens an existing arsenal pin (no new name).

**map-engine loadout pin:** `doc::store::tests::update_slot_loadout_roundtrips_and_clears` PASS
(`--features doc,mission`).

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — MINOR / NOTE | T-770 — `set_loadout` still ignores the new `bool` ack (named `found_not_fixed`)

**Evidence.** Production Apply/Remove path is sound:

```text
apps/website/frontend/src/editor_ops.rs:2203
  commit_writes(writes, |id, json| core.update_slot_loadout(id, json))
```

`MissionDocCore::update_slot_loadout` returns `false` on unknown id (`store.rs:2784–2797`);
`commit_writes` only increments on `true` (`arsenal.rs:1135–1145`). The arsenal pin's miss path
asserts `miss_commits == 2` + WARNING text.

Outside owns, the single-entity wrapper still discards the ack:

```text
apps/website/frontend/src/editor_ops.rs:2005–2020
  core.update_slot_loadout(id, loadout_json);  // bool ignored
  true                                         // ctx/doc existence only
  → after_local_edit() whenever ctx+doc exist
```

Cargo-panel `set_loadout` call site remains the live consumer (`arsenal.rs` ~1253).

**Impact.** T-770's receipt honesty holds for Apply/Remove. The named residue is real: a
`set_loadout` against a missing id still reports "did" to the history/save tail. Same family as
pre-T-745 false-dirty shapes, but this ticket's owns list excluded `editor_ops` beyond the
`commit_loadout_writes` seam the slice already wired.

**Disposition — NOTE (named found_not_fixed).** Do not invent a ticket. Orchestrator may fold into
an existing loadout/dirty follow-on if one is already queued; otherwise leave as documented residue.

---

### F2 — MINOR / NOTE | T-768 — CONN-DEL line-select still absent (named `found_not_fixed`)

**Evidence.** Slice comment still accurate on HEAD:

```text
apps/website/frontend/src/editor_ops.rs:3435–3437
  CONN-DEL line-select still needs a connections render lane
  (mission_history rebind + draw_order) — panel Delete remains the disclosed substitute.
```

`delete_connection` + panel row remain the shipped delete surface (`editor_ops.rs:3580+`).
`draw_order` / connections render lane untouched (outside owns).

**Impact.** Eden CONN-START-001 LMB half is shipped; CONN-DEL line-select is not. Operators delete
from the Connections panel. Ticket already disclosed this.

**Disposition — NOTE (named found_not_fixed).** No new ticket.

---

### F3 — NIT | T-740 — KeyY Cmd pin is chord-string only (does not yoke `mission_history`)

**Evidence.** Live binding still uses `ctrl_key() || meta_key()` before the KeyY redo arm
(`mission_history.rs:532`, `:542–543`). Help chord is `Ctrl/Cmd + Y  or  Ctrl/Cmd + Shift + Z`
(`eden_help.rs:214`). Pin:

```text
eden_help.rs:1509–1522
  assert!(row.chord.contains("Ctrl/Cmd + Y"), …)
```

**Impact.** Reverting help to bare `Ctrl + Y` goes RED (H5). Reverting *only* the KeyY meta guard in
`mission_history` while leaving the help string would stay GREEN — the pin documents the chord, it
does not cross-lock the listener. Acceptable for a docs-only ticket whose code half is already
correct; residual hollow class relative to T-692's code↔table yoke.

**Disposition — NIT.** No wave work. Sixteen-prose half of T-740 is already closed by T-774
(`the_prose_census_numbers_are_derived` PASS; module prose spells twenty-one / thirteen /
thirty-four).

---

## Hollow-pin attacks (all went RED as claimed)

| id | mutation (detached worktree) | pin | result |
|---|---|---|---|
| H1 | `commit_writes`: unconditional `done += 1` (ignore bool) | `the_receipt_counts_the_writes…` | **RED** (`miss_commits` 3≠2) |
| H2 | `update_slot_loadout` early-return `true` on missing id | `update_slot_loadout_roundtrips_and_clears` | **RED** (`unknown id must not ack`) |
| H3 | delete LMB `complete_connect` block in pointerup | T-768 pending_click + canary | **RED** |
| H4 | replace Esc `connect_acted` with `false` | `escape_arm_cancels_pending_connect` | **RED** |
| H5 | help chord → bare `Ctrl + Y` | `redo_chord_documents_cmd_for_key_y` | **RED** |
| H6 | drop `cancel_connect` from `onpointercancel` | `pointercancel_cancels_pending_connect` | **RED** |

**Falsified attack (not a finding):** missing-id `update_slot_loadout` after `begin()` — empty LOCAL
txn does **not** push `undo_depth` (mixed ack batch depth == 2 for two real writes). Receipt
`commits` ↔ undo-step arithmetic holds on the refuse path.

---

## Claims attacked and FAILED to break (verified-clean register)

| claim | attack | result |
|---|---|---|
| T-770 `commit_writes` counts sink acks | H1 invocation recount | pin RED; clean PASS |
| T-770 `update_slot_loadout` → bool on unknown id | H2 always-true; store pin | pin RED; clean PASS |
| T-770 WARNING arm reachable when sink refuses | miss path in arsenal pin + `remove_receipt` | PASS (WARNING + "3…2…" text) |
| T-770 production Apply/Remove uses bool-returning sink | read `commit_loadout_writes` | wired to `update_slot_loadout` |
| T-770 empty refuse txn ≠ phantom undo | worktree probe | undo_depth stays 2 for 2 acks |
| T-768 LG::Pending LMB calls `complete_connect` when armed | H3; live_code scrub so comment `LG::Pending` cannot steal `nth(1)` | pin RED; clean PASS |
| T-768 Esc disarms connect (modal_stack gated like place) | H4; Esc arm sits under `any_open` else | pin RED; clean PASS |
| T-768 pointercancel disarms connect | H6 | pin RED; clean PASS |
| T-768 RMB Complete still a sibling caller | `context_menu.rs:871–874` unchanged caller | present |
| T-768 miss keeps arm (no `complete_connect` without hit) | read pointerup gate `if let Some(ref id) = hit` | present |
| T-740 help documents Cmd on KeyY | H5; live `ctrl\|\|meta` on redo listener | pin RED; binding matches |
| T-740 sixteen-prose already T-774 | `the_prose_census_numbers_are_derived` | PASS |

---

## Safe? table

| question | answer |
|---|---|
| Main compile/test surface green for this wave's crates? | **yes** — frontend 1023/1023; map-engine `--all-features` 634 pass + 1 ignored |
| Any BLOCKER (main broken / vacuous gate success)? | **no** |
| Any MAJOR (ticket claim false / authored work at risk)? | **no** |
| Named found_not_fixed residues still true? | **yes** — F1 (`set_loadout`), F2 (CONN-DEL) |
| Hollow pins load-bearing? | **yes** — H1–H6 all RED |
| Main safe to build the next wave on? | **yes** |

---

## Main safe to build the next wave on?

**yes**

Severity tally: **BLOCKER 0 · MAJOR 0 · MINOR/NOTE 2 · NIT 1**
