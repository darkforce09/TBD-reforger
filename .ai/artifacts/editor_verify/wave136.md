# Wave 136 — adversarial verify (T-745 · T-761 · T-763)

**Verified HEAD:** `3521d0bca83a89d28aee2ec09d4ad9d670906db0`  
(`3521d0bc` T-763 merge tip)

**Wave base:** `d3b8a468` (wave 112 CLOSED — editor wave 135) — ancestor of HEAD.  
**Merges in wave:** `7909caa1`/`99036704` (T-745), `ac5fb3ef`/`1b7a9fde` (T-761),
`3bd32dbd`/`3521d0bc` (T-763).

**Gate (orchestrator):** PASS.  
**Method.** HOST cargo only (`~/.cargo/bin/cargo`, rustc 1.95.0). Private targets under
`~/.cache/tbd-target-wave136-{verify,perturb,base}` (deleted after this report). Mutations only in
detached worktrees at HEAD / base (`~/.cache/tbd-verify136-{perturb,base}` — deleted). Main checkout
left byte-clean at HEAD (`git status` → only pre-existing `?? tbd-target-T-*` plus this report file;
HEAD unchanged). No fix / commit / ticket filing.

**PRIMARY ATTACK.** For each new pin, replay the ORIGINAL defect named in the ticket and confirm
RED. Then probe NEW hollow shapes on the same pins. T-745’s highest-risk claim (transient `/tmp`
probe, no lasting Class-R pin) attacked first.

**NO-DEFERRAL note.** Every severity reported honestly — orchestrator fixes ALL in-wave. No
soft-pedal of hollow pins.

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD frontend `--list` | **996** | `cargo test -p website-frontend -- --list`, private dir |
| HEAD frontend run | **996 passed / 0 failed** | same private dir; `--list` == run |
| base (`d3b8a468`) frontend `--list` | **992** | isolated worktree + private dir |
| Net frontend delta | **+4** | four new named frontend pins |
| HEAD `website-api --lib` `--list` | **268** | private dir |
| HEAD `website-api --lib` run | **268 passed / 0 failed** | `--list` == run |
| base `website-api --lib` `--list` | **267** | +1 = `t763_compiled_clean_mission_response_carries_diagnostics_count_zero` |
| `map-engine-core --all-features` | **642 == 642** (base == HEAD) | prose-only store.rs change; no new pin |

**New frontend pins this wave**

| test | ticket |
|---|---|
| `mission_editor::t761_compile_findings_cleared_on_hydrate::mission_editor_clears_compile_findings_on_hydrate` | T-761 |
| `validation_panel::t761_compile_findings_do_not_survive_mission_switch::a_second_mission_does_not_inherit_the_previous_missions_compile_findings` | T-761 |
| `validation_panel::t761_compile_findings_do_not_survive_mission_switch::clear_compile_findings_is_the_hydrate_reset_seam` | T-761 |
| `eden_dock_right::tests::marker_attributes_selects_by_faction_id_and_id` | T-763 |

**New API pin**

| test | ticket |
|---|---|
| `handlers::missions::tests::t763_compiled_clean_mission_response_carries_diagnostics_count_zero` | T-763 |

992 + 4 = **996**. Confirmed. List == run. **Zero T-745-named pins** in the frontend list.

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — MAJOR | T-745 `attrs_update_slot` — **no lasting Class-R pin (exact highest-risk miss)**

**Evidence.**  
(a) Diff `d3b8a468..HEAD` on `editor_ops.rs` is **+20 production lines only** (all-None early
return + `!raw_slot_rows(core).contains_key(id) → false`). No test module, no
`attributes::tests::…` addition, no list-name containing `745` / `noop` / `all_none`.  
(b) Strip both guards in the perturb worktree →
`attributes::tests::attrs_update_slot_routes_the_new_fields_through_update_slot_object` stays
**GREEN** (that pin only requires `update_slot_object` / slot-half gating).  
(c) Base body: `did = true` whenever ops ctx + document exist — all-None still calls
`update_slot_object` then fires `after_local_edit`; missing id likewise. HEAD early-returns
all-None before `OPS_CTX`, and missing-id returns `false` before the history tail. Source proof
only — `editor_ops` is `#![cfg(target_arch = "wasm32")]`, so native cannot call it.

**Impact.** The defect class this program cares most about (false dirty / false save) is fixed in
production today and **completely regression-blind**. A one-hunk revert of the two guards ships
green. Matches the brief’s named highest risk: transient probe, no lasting pin.

**Disposition — fix this wave.** Add a lasting Class-R pin (native `live_code` /
`only_body("pub fn attrs_update_slot(")` in `attributes.rs`, sibling of the existing
`attrs_update_slot_routes…` pin) that requires (1) the five-field all-None early `return` before
`let did`, and (2) the raw `!raw_slot_rows(core).contains_key(id)` → `false` arm. Perturbation
(b) must go RED. Optional: behavioural wasm probe if the harness gains one; source pin is the
minimum that closes the wave-113 F-3 class.

---

### F2 — MAJOR | T-763 `t763_compiled_clean_mission_response_carries_diagnostics_count_zero` — **pins the helper, not the route**

**Evidence.**  
(a) ORIGINAL defect (skip count when `diagnostics.is_empty()` inside
`compiled_diagnostics_response_headers`) → pin **RED** (`count` is `None`, not `Some("0")`).  
(b) NEW shape: keep the helper correct, but **bypass it in `get_compiled_mission`** — inline a
HeaderMap that omits the count on empty findings.  
  • `t763_…count_zero` → **GREEN** (still builds a response from the pristine helper).  
  • `t690_compiled_route_surfaces_the_structured_diagnostics` → **GREEN**.  
(c) Why t690 stays green: T-763 extracted `fn compiled_diagnostics_response_headers` into the
span between `get_compiled_mission` and `fn unreadable_stored_payload`. t690’s “body” window is
exactly that span, and its assert is
`body.contains("compiled_diagnostics_response_headers")` — satisfied by the **function
definition** even when the route no longer calls it. At base `d3b8a468` the same window contained
the inline `COMPILE_DIAGNOSTICS_COUNT_HEADER` insert and **no** helper def; the extraction
weakened the pre-existing source pin.

**Impact.** Ticket item (6) asked for a response-level pin that clean `/compiled` carries
`x-compile-diagnostics-count: 0`. What landed proves the helper’s HeaderMap, not that the HTTP
handler uses it. A clean-mission omit can return while both t763 and t690 stay green.

**Disposition — fix this wave.**  
1. Require the route body (window that **excludes** the helper def, or an explicit
   `compiled_diagnostics_response_headers(&diagnostics)` call needle **inside**
   `get_compiled_mission` only) to call the helper.  
2. Keep the response-level assert, but wire it so a route bypass that omits the count goes RED
   (call the same assembly path the handler uses, or hit the handler). Perturbation (b) must RED
   at least one of t690/t763.

---

### F3 — MINOR | T-761 `clear_compile_findings_is_the_hydrate_reset_seam` — **self-satisfies via its own assert string**

**Evidence.**  
(a) Gut production `clear_compile_findings` to an empty body (remove
`publish_compile_findings(Vec::new())`) →  
  • `a_second_mission_does_not_inherit…` → **RED** (behavioural pin load-bearing).  
  • `clear_compile_findings_is_the_hydrate_reset_seam` → **GREEN**.  
(b) `publish_compile_findings(Vec::new())` occurrences in the file: **2** before gut (production +
the assert’s `src.contains("publish_compile_findings(Vec::new())")` string), **1** after (assert
only). The pin `include_str!`s the whole file, including the test module, so the assert feeds
itself.  
(c) Same GREEN when the production body is replaced by
`let _ = "publish_compile_findings(Vec::new())";` (string decoy; no `live_code` scrub).

**Impact.** The seam Class-R pin does not enforce a real empty publish. Behavioural pin +
mission_editor call-site pin still catch the live defect and the missing hydrate call. Hollow is
the *named seam* pin specifically.

**Disposition — fix this wave.** Scope the scan to production only (`split("#[cfg(test)]").next()`,
or `live_code` / `only_body("pub fn clear_compile_findings(")`), and require `Vec::new()` inside
that body — not in the test’s own assert string. Perturbation (a)/(c) must RED.

---

### F4 — MINOR | T-763 `marker_attributes_selects_by_faction_id_and_id` — **selection addr construction unpinned**

**Evidence.**  
(a) ORIGINAL id-alone `.find(|r| r.id == id)` → pin **RED**.  
(b) Comment decoy of the pair find + live id-alone → **RED**.  
(c) Pair find planted only inside `marker_attributes` (outside the panel window) + id-alone in
panel → **RED**.  
(d) NEW shape: keep pair find + `None::<(String, String)>`, but change the click site
`let addr = (m.faction_id.clone(), m.id.clone())` → `(String::new(), m.id.clone())` → pin
**GREEN**.

**Impact.** Attributes lookup and signal type are pinned. The only production write that fills
`marker_selected` can drop the faction half without a RED. Latent — today’s source still clones
both fields — but the ticket’s foreign-payload story depends on the addr being honest at set
time, not only at find time.

**Disposition — fix this wave.** Pin the panel’s `addr = (m.faction_id.clone(), m.id.clone())`
(or equivalent) inside the same `markers_panel` window. Perturbation (d) must RED.

---

## Claim attacks (by ticket)

### T-745 — attrs_update_slot all-None / missing-id no-op

| claim | result |
|---|---|
| All-None early return present (HEAD source) | **HELD** in production |
| Missing-id raw guard present (HEAD source) | **HELD** in production |
| Lasting Class-R pin | **MISSING** → **F1 MAJOR** |
| Strip guards → suite pin RED | **FAILED** — related pin stays GREEN |

### T-761 — COMPILE_FINDINGS cleared on hydrate

| claim | result |
|---|---|
| Remove hydrate `clear_compile_findings` call | **HELD** — mission_editor pin **RED** |
| Clear *before* `hydrate_from_server` (order swap) | **HELD** — order assert **RED** |
| Clear only on IDB path (before hydrate in file order) | **HELD** — order assert **RED** |
| String decoy of the call + live clear removed | **HELD** — `live_code` pin **RED** |
| Gut `clear_compile_findings` body | **HELD** — behavioural pin **RED**; seam pin hollow → **F3** |
| Clear republishes `compile_findings()` (no empty) | **HELD** — behavioural **RED** |
| thread_local actually emptied | **HELD** — `publish_compile_findings(Vec::new())` whole-list replace; behavioural sees empty |
| `subject_id` stale rows after clear | **HELD** — `evaluate_now` assert rejects `mission-a-squad-1` |
| Mount vs hydrate | Clear sits on the canvas `on_load` boot path **after** `hydrate_from_server.await` (same destiny as hydrate; smoke still runs clear after hydrate returns). Not a mount-only-miss on the inspected path. |
| `PANEL_SINK` / `Rc::ptr_eq` | Clear routes through `publish_compile_findings` (repaints sink). No change to `install_seam` identity cleanup. |

### T-763 — decoy narrative / clean count / marker pair / store prose

| claim | result |
|---|---|
| (5) Contiguous stub `Marker placement lands in T-069.` gone | **HELD** — absent on HEAD |
| Reintroduce contiguous stub in a comment | **HELD** — `favourites_tab_is_wired_not_stubbed` **RED** (comments included) |
| (6) Clean response carries count `0` via helper | **HELD** against helper gut; **HOLLOW vs route bypass** → **F2 MAJOR** |
| (7) Attributes find pairs `(factionId, id)` | **HELD** against id-alone / comment decoy / wrong-fn plant |
| (7) Selection addr carries both halves | **UNPINNED** → **F4 MINOR** |
| (8) Store prose no longer claims “no root key whatsoever” | **HELD** — phrase absent; `no root \`markers\` key` present (3×). Prose-only; no new MEC pin (642==642). |
| API path intentionally `handlers/missions.rs` (not frontend) | **HELD** — frontend `missions.rs` untouched; justified |

---

## Main safe to build the next wave on?

**no** — not until F1–F4 are fixed in-wave (orchestrator NO-DEFERRAL).  
Production behaviour for T-745 / T-761 / T-763 looks correct on HEAD under source + behavioural
checks, but two MAJORs leave the wave’s own claims regression-blind (unpinned T-745 guards;
`/compiled` count pin that greens while the route omits the header), and two MINORs are hollow
shapes on pins this wave just added.

---

## Verified-clean register (claims re-proved)

- T-745 production guards mirror `_multi` all-None shape and add raw missing-id → `did = false`.  
- T-761: `clear_compile_findings` + MissionEditorPage call after hydrate `.await`; behavioural
  clear drops `COMPILE_FINDINGS` and `subject_id` leakage through `evaluate_now`.  
- T-763: stub sentence stays gone (comment reintroduction RED); marker Attributes find pairs
  `(faction_id, id)`; store prose corrected to “no root `markers` key”.  
- Suites: frontend **996 == 996**; api lib **268 == 268** (+1 t763); map-engine-core **642**
  unchanged.  
- Main: `3521d0bc`, tracked tree untouched aside from this report.

---

## Attacked and FAILED to break

1. **T-761 hydrate call-site / order** — remove clear, clear-before-hydrate, IDB-only clear, string
   decoy all RED the mission_editor pin.  
2. **T-761 behavioural clear** — gut body / republish-without-empty RED
   `a_second_mission_does_not_inherit…`; `subject_id` assert holds.  
3. **T-763 stub absence** — contiguous reintroduction in a comment RED
   `favourites_tab_is_wired_not_stubbed`.  
4. **T-763 Attributes pair find** — id-alone, comment decoy, wrong-fn plant all RED
   `marker_attributes_selects_by_faction_id_and_id`.  
5. **T-763 helper empty-count omit** — skip insert when empty RED `t763_…count_zero`.  
6. **T-763 store “no root key whatsoever” overclaim** — gone on HEAD; markers-key prose present.  
7. **Frontend / API list↔run mismatch** — none (996==996, 268==268).  
8. **map-engine-core census drift from prose edit** — none (642==642).  
9. **PANEL_SINK / ptr_eq cleanup regression from T-761** — clear uses existing publish path; no
   new seam identity bug found.  
10. **Affordance / `subject_id_routes` family** — not broken by this wave’s owns; validation_panel
    behavioural pin still exercises `subject_id` post-clear.

---

## Focused re-verify (F1–F4 post-fix)

**Re-verify HEAD:** `20cb00095ffbb6c8e9ab8357082f6c76363586ca`  
(`20cb0009` T-763 route helper + marker addr; ancestors `d83b97c3` F3, `e0a7b442` F1)

**Gate (orchestrator post-fix):** PASS.  
**Method.** HOST cargo (`~/.cargo/bin/cargo`). Private target
`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target-w136-reverify`. Mutations only in detached
worktree `~/.cache/tbd-verify136-reverify` at HEAD (deleted after). Main checkout left
byte-identical except this append. No fix / commit / ticket filing. No wave close.

**Fix commits under test**

| finding | fix |
|---|---|
| F1 | `e0a7b442` — `attrs_update_slot_noops_when_all_none_or_id_missing` |
| F3 | `d83b97c3` — seam pin scoped to production body |
| F2+F4 | `20cb0009` — route helper call + marker addr pin |

### Suite reconciliation (post-fix)

| measurement | value |
|---|---|
| frontend `--list` | **997** |
| frontend run | **997 passed / 0 failed** (`--list` == run) |
| `website-api --lib` `--list` | **268** |
| `website-api --lib` run | **268 passed / 0 failed** (`--list` == run) |

Net vs original wave-136 verify (996 frontend): **+1** = F1 lasting pin
`attributes::tests::attrs_update_slot_noops_when_all_none_or_id_missing`.

---

### F1 — T-745 `attrs_update_slot` no-op guards — **CLOSED**

**Disposition re-read.** Original: no lasting Class-R pin; strip both guards → sibling route pin stayed GREEN. Fix: lasting pin requiring all-None early `return` before `let did` + raw missing-id → false.

**Perturbation.** Strip both T-745 guards from `pub fn attrs_update_slot` in `editor_ops.rs` (all-None early return + `!raw_slot_rows(core).contains_key(id) → false`).

**RED** — `attributes::tests::attrs_update_slot_noops_when_all_none_or_id_missing`:

```
thread 'attributes::tests::attrs_update_slot_noops_when_all_none_or_id_missing' (2171541) panicked at apps/website/frontend/src/attributes.rs:1290:13:
T-745: all-None guard must check `role.is_none()` before `let did`; prelude was:


    
test attributes::tests::attrs_update_slot_noops_when_all_none_or_id_missing ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 996 filtered out; finished in 0.30s
```

**GREEN** after `git checkout -- apps/website/frontend/src/editor_ops.rs`:
`test attributes::tests::attrs_update_slot_noops_when_all_none_or_id_missing ... ok`

---

### F2 — T-763 `/compiled` route helper call — **CLOSED**

**Disposition re-read.** Original: pin greens while `get_compiled_mission` bypasses helper with inline HeaderMap that omits count on empty. Fix: narrow t690/t763 window to exclude helper def; require `compiled_diagnostics_response_headers(&diagnostics)` inside route body.

**Perturbation.** Keep helper pristine; replace route
`let mut headers = compiled_diagnostics_response_headers(&diagnostics);` with inline
`HeaderMap::new()` + `if !diagnostics.is_empty() { … insert count/rules … }`.

**RED** — both strengthened pins:

```
thread 'handlers::missions::tests::t690_compiled_route_surfaces_the_structured_diagnostics' (2176305) panicked at apps/website/api/src/handlers/missions.rs:2422:9:
/compiled route body must call compiled_diagnostics_response_headers(&diagnostics); got:
```

(route dump then shows `let mut headers = axum::http::HeaderMap::new();` / `if !diagnostics.is_empty() {` — no helper call)

```
thread 'handlers::missions::tests::t763_compiled_clean_mission_response_carries_diagnostics_count_zero' (2176319) panicked at apps/website/api/src/handlers/missions.rs:2496:9:
clean /compiled count pin requires the route to call the helper; got:
```

(same bypassed route body)

`test result: FAILED. 0 passed; 1 failed; … 267 filtered out` for each.

**GREEN** after restore:
- `t690_compiled_route_surfaces_the_structured_diagnostics ... ok`
- `t763_compiled_clean_mission_response_carries_diagnostics_count_zero ... ok`

---

### F3 — T-761 `clear_compile_findings_is_the_hydrate_reset_seam` — **CLOSED**

**Disposition re-read.** Original: whole-file `src.contains` self-fed off assert string; gut body / string decoy left seam pin GREEN. Fix: `live_code` + `only_body("pub fn clear_compile_findings(")` requiring `Vec::new()` + `publish_compile_findings`.

**Perturbation (a).** Gut production body to empty `pub fn clear_compile_findings() {}`.

**RED:**

```
thread 'validation_panel::t761_compile_findings_do_not_survive_mission_switch::clear_compile_findings_is_the_hydrate_reset_seam' (2172895) panicked at apps/website/frontend/src/validation_panel.rs:2235:9:
T-761: clear_compile_findings must empty via Vec::new() in the production body; got:


test …clear_compile_findings_is_the_hydrate_reset_seam ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 996 filtered out; finished in 0.10s
```

**Perturbation (c).** String decoy `let _ = "publish_compile_findings(Vec::new())";` (no live publish).

**RED** (`live_code` scrubs the string literal):

```
thread 'validation_panel::t761_compile_findings_do_not_survive_mission_switch::clear_compile_findings_is_the_hydrate_reset_seam' (2173263) panicked at apps/website/frontend/src/validation_panel.rs:2236:9:
T-761: clear_compile_findings must empty via Vec::new() in the production body; got:

    let _ =                                       ;

test …clear_compile_findings_is_the_hydrate_reset_seam ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 996 filtered out; finished in 0.10s
```

**GREEN** after restore: `…clear_compile_findings_is_the_hydrate_reset_seam ... ok`

---

### F4 — T-763 marker selection addr — **CLOSED**

**Disposition re-read.** Original: pair-find pin greened when click wrote `(String::new(), m.id.clone())`. Fix: require `let addr = (m.faction_id.clone(), m.id.clone())` inside `markers_panel` window.

**Perturbation (d).** Production click site only:
`let addr = (m.faction_id.clone(), m.id.clone())` → `let addr = (String::new(), m.id.clone())`
(pair find + `None::<(String, String)>` left intact).

**RED:**

```
thread 'eden_dock_right::tests::marker_attributes_selects_by_faction_id_and_id' (2178535) panicked at apps/website/frontend/src/eden_dock_right.rs:4417:9:
markers_panel click must build addr from both faction_id and id
test eden_dock_right::tests::marker_attributes_selects_by_faction_id_and_id ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 996 filtered out; finished in 0.00s
```

**GREEN** after restore:
`test eden_dock_right::tests::marker_attributes_selects_by_faction_id_and_id ... ok`

---

## Focused re-verify verdict

| finding | severity (original) | post-fix pin | hollow → RED | restore → GREEN | status |
|---|---|---|---|---|---|
| F1 | MAJOR | `attrs_update_slot_noops_when_all_none_or_id_missing` | yes | yes | **CLOSED** |
| F2 | MAJOR | t690 call needle + t763 route assert | yes (both) | yes | **CLOSED** |
| F3 | MINOR | seam `live_code`/`only_body` | yes (a)+(c) | yes | **CLOSED** |
| F4 | MINOR | panel `addr = (faction_id, id)` | yes | yes | **CLOSED** |

**findings CLOSED:** **4 / 4**  
**safe to build the next wave on?** **yes**  
**list/run totals:** frontend **997 == 997**; `website-api --lib` **268 == 268**

