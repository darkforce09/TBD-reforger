# Editor wave 107 — adversarial verify (T-638 · T-657 · T-659)

**VERDICT: 0 BLOCKER · 1 MAJOR · 3 MINOR · 8 NOTE — wave passes; the T-638×T-667 deferred ticket must be re-scoped MAJOR (grid-ref labels are position-frozen by the keyed `<For>`, so the collapse slide strands them off their lines).**

Range `8ffac4e3..f1c31273` (three slices, all cut from `8ffac4e3`, merged T-657 → T-659 → T-638).
Suites at HEAD: `website-frontend` native **519/519**; `map-engine-core --features doc,mission` **385 pass / 1 ignored** (validate filter = **36/36**); `--features mission` (bare) validate **36/36**. Working tree left clean; every re-fire mutation restored and re-verified.

---

## MAJOR-1 — T-638×T-667 disclosed gap is not cosmetic: grid-ref labels are POSITION-FROZEN, so the collapse slide strands every retained label off its line

**Evidence.** The T-667 overlay renders labels via `<For each=move || labels().0 key=|l| l.text.clone()>` with `style=move || format!("left:{:.1}px…", l.pos_px)` (eden_toolbelt.rs:587-624). The item is plain data — the style closure has no reactive dependency, so it runs once at row creation. Tachys' keyed diff (`~/.cargo/registry/src/*/tachys-0.2.18/src/view/keyed.rs`, `apply_diff`) calls `view_fn` **only in the `DiffOpAdd` arm**; retained keys are moved or left as-is with their original built view. Net: a label whose 3-digit text stays inside the pane span keeps its **creation-time** `pos_px` forever, no matter how often `labels()` re-runs. T-638's centre-hold then guarantees the trigger: a left-dock toggle nudges the camera by `(256−24)/2 = 116 px` (right dock: 148 px), the wgpu grid lines slide with the camera, the retained labels do not. At default zoom −2 a 1 km line is 250 px, so a grid **reference label sits ~46 % of a cell off its line, persistently** (until that key exits and re-enters the span). The same mechanism fires on every pan (pre-existing since T-667 — labels lag the map by however far the camera moved since each was created; the native `labels_match_grid_lines` invariant proves the pure math and never sees the DOM).

**Impact.** Wrong grid references over the map — misinformation, not decoration, in a milsim grid tool. T-638's disclosure ("labels anchor to expanded geometry while the map reflows; cosmetic at the freed strip") materially understates it: the freed-strip coverage gap and the northings column floating at `left:258px` mid-map ARE cosmetic, but the frozen-label drift is not, and it is triggered by every collapse toggle and every pan. Root cause is out-of-range (T-667, wave 106); T-638's camera nudge is what makes it fire on a bare keypress with no pointer motion.

**Disposition.** Document only (per wave rules). Scope the already-planned T-638-residue/eden_toolbelt ticket as **MAJOR** and make it two-part: (a) grid-refs read the live accessors; (b) fix the `<For>` so position updates apply — key on the world line AND re-render on position change (or signal-ize `pos_px`). One 2-minute browser pan will visually confirm the frozen-label behaviour before anyone starts; the framework-source evidence above is unambiguous but cheap to double-check live.

---

## MINOR-1 — T-657: ORBAT-CALLSIGN-UNIQUE emits an id-keyed `subject` pointer into an ARRAY; the other four rules are positional — and a test pins the wrong shape

**Evidence.** validate.rs:878 — `format!("/editor/squads/{member_id}/callsign")` where `member_id` is the squad **id** (`"sq2"`); `editor.squads` is an array (`compile.rs:239` `values_of_ordered(...)`), so the pointer does not resolve. SLOT-RESOLVES / IDENTITY-FILLED / HAS-LEADER / TEMPLATE-COVERAGE all emit positional `/editor/squads/{i}` (:774, :821, :967). The test at :1499 asserts `"/editor/squads/sq2/callsign"`, freezing the malformed form. The `Finding` doc (:110-119) promises "the JSON-pointer-ish path … so the panel can focus the offender" and says the pointer is for display/focus while `subject_id` is for selection.

**Impact.** T-655's focus-by-pointer breaks (or needs a special case) for exactly one rule; the contract the wave-104 verifier forced (`subject_id`) is coherent everywhere else. No runtime consumer today.

**Disposition.** One-line fix + pin update in the T-655 wave (the rules land there anyway). Not worth a standalone ticket.

## MINOR-2 — T-657: ORBAT-TEMPLATE-COVERAGE is production-dead — no writer in the repo produces `squad.template.requiredRoles`

**Evidence.** Squad rows are written with exactly `id / factionId / callsign / name / slotIds / vehicleIds` (+`leaderSlotId` via `set_leader`) — store.rs:610-617, :2806; repo-wide grep finds **zero** writers of a squad `template` key or `requiredRoles` (the `OrbatSquadTemplate` in orbat.rs is the unrelated event-ORBAT row struct). The rule therefore never fires on any payload the editor can author; it fires only on its own trip fixture. The module charter it ships inside says the seed "does not seed a rule whose subject data the editor cannot yet produce" (validate.rs:26-27).

**Impact.** This is the FNF dead-check shape the engine was built against — with the one honest difference that `self_check` mechanically proves the rule CAN fire (my neuter-mutation took five tests down, incl. self_check, so decay will be loud). Disclosed in the commit as forward-compat. Still: a warning row the panel will never show, justified by a D3-D8 "revival" of rules that are equally unreachable until a template writer exists.

**Disposition.** Tie it to the writer: either the compositions/faction-template ticket (T-650 lane) explicitly takes "write `template.requiredRoles` on instantiate", or move this rule into that ticket and drop it from the registry until then. Operator's call; flagging the honesty question as asked.

## MINOR-3 — T-638: the reflow Effect's "does NOT reallocate the device buffer" comment is false at HEAD — `e.resize` unconditionally reconfigures the surface

**Evidence.** mission_editor.rs:1314-1318 ("a collapse does NOT reallocate the device buffer … We still route the change through `e.resize` (identical dims) to mark damage"); engine.rs:1591-1604 — `resize` always runs `self.surface.configure(&self.device, &self.config)`, which tears down and recreates the swapchain even at identical dims. So the ticket's "device buffer really is reallocated" claim is the correct half **at HEAD**, and the code comment documents the opposite of what its own call does. `mark_dirty()` (engine.rs:1637) was available for the damage-only intent.

**Impact.** One needless swapchain reconfigure per E/R/Backspace keypress — bounded by typing rate, no user-visible cost measured or expected. The defect is the comment: the next person reasoning about resize semantics inherits a false premise the ticket itself contradicts.

**Disposition.** Polish-slice: either switch to `mark_dirty()` (if "layout change goes through resize" isn't actually load-bearing) or rewrite the comment to "canvas dims unchanged; surface is still reconfigured".

---

## NOTE-1 — T-638 silently RESOLVES wave-101 N-3 (hidden-chrome drop bands)

Wave-101 N-3 recorded that an armed place released in the static dock bands while `chrome_hidden` was **cancelled**, contradicting T-662's "every px is a map gesture" comment. The accessor conversion (mission_editor.rs:2062-2065, insets 0 while hidden) makes it **land** — the direction N-3 said the docs promised. Reachable only via arm → Backspace mid-gesture → release (palette is unmounted while hidden). No T-662 pin guards drop semantics (the t662 module gates mounts and buttons only — verified). Record N-3 as resolved so it stops being carried. Similarly intended: drops now land in a collapsed dock's freed strip — that strip IS map, and the expanded-dock cancel behaviour is unchanged (accessors return 256/320 when shown+expanded).

## NOTE-2 — "marquee self-check probe grid" is a prose mislabel for `farthest_empty_px`

The commit message and eden_layout comments repeatedly bill `select_tool::farthest_empty_px` as "the marquee self-check probe grid". It is the **click-smoke's guaranteed-empty-px prober** (feeds `probe()`'s deselect gate, select_tool.rs:440-497); `marquee_selfcheck` (:333) is world-space and reads no insets. Substance unaffected: at 0-inset (hidden) the probe property holds — T-662 unmounts all six chrome mounts, so every px of the full-window grid is clickable; with a collapsed dock the 24 px inset conservatively shrinks the search space (the doc's "shrinks the search space; does not weaken the property" argument stands); the degenerate fallback is unchanged. Answering A directly: no configuration makes the probe or any self-check fail.

## NOTE-3 — collapse-latch edges: E/R toggle invisibly while hidden; the stub column is a rectangular over-approximation

(a) The E/R arms are not gated on `chrome_hidden` (mission_editor.rs:1285-1292), so a keypress while hidden flips state that only materializes on unhide — consistent with the pinned "orthogonal/persist" design, mildly surprising UX, session-local. (b) While collapsed the accessor reports 24 for the **full-height** column, but the stub occupies only y∈[48,72]; the bare 24 px column below it is real map that still cancels drops / is excluded from the probe grid. Keeps the pane rectangular; conservative; matches the documented "the dock becomes exactly this stub" model.

## NOTE-4 — T-659 census cost rides every `doc_tick`, and `doc_tick` bumps on selection too

`census_input()` → `orbat_manager_snapshot()` (editor_ops.rs:1622) does three JSON serialize+parse passes (factions/squads/slots) per recompute; pre-T-659 that ran only from the ORBAT Manager dialog (orbat_manager.rs:778, :1270). The census memo tracks `doc_tick`, which `refresh_docks` bumps from **every** funnel site including click/marquee selection refreshes (mission_history.rs:448-452 → editor_ops.rs:1055). So every click now costs a full ORBAT-snapshot parse (+`read_env_value("mode")` = one `small_maps_json` parse in the summary memo). Fine at ORBAT scale (hundreds of slots); revisit if slot counts grow. Cross-slice storm check (E) is otherwise clean: E/R collapse toggles bump **no** census/strip signal (verified — the strip's gate closure tracks only `chrome_hidden`; the census memo only `doc_tick`; the Effect writes only the layout Cells + engine), and a Backspace unhide costs one remount-recompute.

## NOTE-5 — side-vocabulary seam: census pins 3 sides; T-657/schema admit a 4th (CIV) and arbitrary faction rows

`CENSUS_SIDES` = BLUFOR/OPFOR/INDFOR (eden_top_strip.rs:1137), agreeing with `orbat_add_squad`'s guard (editor_ops.rs:1755 `matches!(side, "BLUFOR"|"OPFOR"|"INDFOR")`) and `asset_catalog::EDEN_SIDES` — the editor cannot author anything else, so the two slices **cannot disagree on editor-authored docs**. But a hydrated foreign payload with a CIV or custom-keyed (`TBD-Blue`) faction shows ALL its slots as `UNA` in the badge/summary while T-657 stays green (CALLSIGN-UNIQUE scopes by faction row, any key; V2's own comment cites four canonical sides incl. CIV). Latent, imported-payloads-only; the pinned `count_for_key("CIV") == 0` test shows T-659 chose this deliberately.

## NOTE-6 — `assert_self_check` still has no runtime caller

Its doc sells it as "the form a service calls once at startup"; only tests call `self_check` (repo grep — the mission_editor hits are the unrelated GPU self-checks). Pre-existing from T-656; wire it when T-655 mounts the panel (or at API boot).

## NOTE-7 — stale comments: carried, not worsened; fixes are ticketed

(a) The false T-159.22 invariant comment ("`left`/`pan_px` are both None here…") now sits at mission_editor.rs:2044-2046, directly above T-638's own on_canvas edit (:2057-2065) — T-638 changed the gate 12 lines below it and left it standing. The three wave-106 MAJORs behind it are tracked in **T-723** (deferred: "Armed-placement pointerup: one root, three defects"); the comment should fall with T-723. T-638's changes did not touch gesture-state handling — not worsened. (b) mission_editor.rs:2661-2662 still says the T-667 furniture slot is "reserved, not built" (wave-106 (c); T-667 built it). (c) `chrome_hidden_signal_gates_the_five_mounts` still counts six (name stale since wave 106). All pre-recorded; none regressed.

## NOTE-8 — claim ledger: everything else checked TRUE

- **T-638 accessor seam**: four consts frozen at expanded values; four accessors fold `chrome_hidden` (wins, → 0) over per-dock latches (persist) — eden_layout.rs:119-162; state machine + orthogonality + persistence pinned in `t638_collapse` (7 tests) and fired against a real `map_engine_core::camera::OrthoCamera` with perturb/fail/restore (un-held drift == pane-delta/scale asserted). Owned readers moved (select_tool.rs:464-471, mission_editor.rs:2062-2065); `t636_band_readers_agree` evolved to require accessor-by-name and forbid the frozen consts + bare literals in both readers.
- **Cell mirror timing (A)**: single writer verified (both setter call sites are inside the one Effect, :1350-1352/:1359-1361; no other callers). No one-frame-stale window: Leptos 0.8 effects are microtask-scheduled (`any_spawner` → `spawn_local`); microtasks flush before the next paint AND before any subsequent input macrotask, so no pointer event can observe pre-mirror insets. Reasoned, not measured; a measurement would need instrumented wasm.
- **Centre-hold scope (A)**: nudge gated on `!was_hidden && !hidden` — Backspace hide **and** unhide never slide (T-662 preserved); dock toggles while hidden change nothing visually (insets both 0 ⇒ before==after). Chevron: shared `collapse_chevron`, 24×24 (`size-6`) hit-box, outer corners, glyph mirror pinned; stub views 24×24 with `aria-expanded`.
- **E/R keys**: same keydown closure, behind `in_editable_field()` (:1242), bare-key guards keep Ctrl+R reload.
- **T-657**: five V3 rules query the authored `editor` graph; vocabulary confirmed against store.rs writers; evals total via `str_field`/`str_array` (11-payload garbage test); `subject_id` Some on all five, None on seeds; clean fixture honestly completed (leaderSlotId + name, validate.rs:1210-1231); case-insensitivity is `str::to_lowercase` — full Unicode lowercase, not casefold ('Ⅱ'→'ⅱ' matches; `straße` vs `STRASSE` does NOT clash; fine for milsim callsigns, worth knowing it's not casefold. Same-id-listed-twice in one `squadIds` self-clashes with a nonsense message — malformed-input corner only).
- **T-659**: `census_from_rows` pure join with dangling-hop → unassigned; `west+east+ind+unassigned==total` holds by construction (`total` = row count, every row lands in exactly one bucket — and is asserted); UNA chip nonzero-only (view :800-812); golden `'COOP 160 on Everon — WEST 78 v EAST 74 (+8 IND)'` byte-exact; **suffix ORDER test exists** (`summary_suffixes_are_append_only_and_ordered`: both suffixes, IND before unassigned, core prefix byte-identical); rustdoc stability pin present (:1216-1239); one snapshot read reused from the ORBAT manager path; vehicles excluded with rationale; `mode` via `read_env_value` (meta.environment bag; no writer → usually absent, as claimed); tests+derivation native in eden_top_strip since editor_ops is `#[cfg(wasm32)]` (main.rs:31).
- **Counts reconcile**: base `8ffac4e3` = 501 native ⇒ T-638 tip 501+7=**508** ✓, T-659 tip 501+11=**512** ✓, HEAD 501+7+11=**519** ✓ (ran). `#[test]`-marker sweep 1063→1097 = +16/+11/+7 across the three slices, nothing lost in the merges. Validate = 36 under `doc,mission` AND bare `mission` ✓. All 9 rules registered; registry self_check green at HEAD ✓.
- **Hygiene**: zero new `#[allow]` in the range (the two `#![allow(dead_code)]` are pre-existing context); one new `.unwrap()`, test-only; dock_right's 637-line diffstat is the full/stub closure re-indent — real changes are the chevron import + stub + swap only; select_tool's diff is exactly the four accessor reads; no out-of-owns edits (T-657 validate.rs only; T-659 top_strip+editor_ops; T-638 its five). Registry rows for the three tickets still `queued` — wave-close sync pending, per process.

## Re-fire (F) — one fired-once per slice, by mutation

| Slice | Mutation | Result |
|---|---|---|
| T-638 | `centre_hold_target`: `(c0x−c1x)` → `(c1x−c0x)` | `centre_hold_keeps_the_pane_centre_world_point` **FAILED** (6 passed / 1 failed) — restored |
| T-657 | `has_leader` forced `true` | **5 tests FAILED** incl. `t657_rules_are_registered_and_self_check_passes` and `engine_self_check_passes_for_the_seed_registry` — the dead-rule guard is loud, exactly as designed — restored |
| T-659 | `Some("OPFOR") => c.west += 1` | `census_counts_each_side_and_totals` + `census_single_side_leaves_others_zero` **FAILED** — restored |

Tree verified clean after each restore (`git status` empty).

## Priority-question index

**A** — probe sound at 0-inset (NOTE-2); drops land = N-3 resolved, no pin guards it (NOTE-1); no stale-inset frame (NOTE-8, microtask argument); resize realloc: the ticket is right, the comment is wrong (MINOR-3). **B** — labels cannot drift from lines *in the pure math* (live-camera re-projection) but the DOM freezes them (MAJOR-1): collapse slide = 116/148 px systematic label-off-line, plus cosmetic coverage gap in the freed strip and the northings column at 258 px — deferred ticket is **MAJOR**. **C** — TEMPLATE-COVERAGE is a production-dead rule with honest self_check mechanics (MINOR-2); casefold nuance + duplicate-id corner (NOTE-8); subject-pointer contract broken for one rule (MINOR-1). **D** — vocabularies agree wherever the editor can author (guard verified); CIV/custom-key divergence is import-only (NOTE-5); suffix-order test verified (NOTE-8). **E** — no signal storms (NOTE-4); T-725's northings-under-bar unchanged by 0-inset accessors (grid-refs read frozen consts; span bottom was already `vh`); registry self_check green with all 9 (NOTE-8). **F** — counts reconciled + three re-fires bit (table). **G** — no allow growth, no drive-bys, stale comments carried with fixes ticketed (NOTE-7, T-723).
