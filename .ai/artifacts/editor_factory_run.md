# Editor factory — run log

Program: waves 100–126 (77 tickets), authority `docs/platform/EDITOR_FACTORY_START.md`.
Run started 2026-08-02 from 852f17a4 (plan adversarially verified: 20/20 invariants).

**Close-marker numbering — DERIVE IT FROM GIT, NEVER FROM A FORMULA:**

```sh
git log --grep='^wave [0-9]\+ CLOSED' --format='%s' -1     # → the highest marker in the ledger
```

The next close is **that number + 1**. Nothing else. The ledger (wave.sh oracle 1) requires marker
numbers to advance by exactly one and hard-refuses `n > high + 1` (`scripts/platform/wave.sh:2098`).

> **The old `M = L − 17` formula was WRONG and was removed on 2026-08-07.** It held for waves
> 100→111 (markers 83→94), then broke the moment waves 112–119 were deferred: editor wave **120**
> closed at marker **95**, not 103, because markers advance sequentially regardless of which editor
> waves run or in what packing width. A fresh orchestrator applying the formula to wave 121 would
> write `wave 104 CLOSED`, and oracle 1 would refuse the gate — *"claims a wave that never opened."*
> The marker and the editor-wave label are independent counters; only the git ledger knows the
> marker.
>
> **At the last close (`1de39175`, editor wave 120 @ marker 95) the next marker is 96.**

The editor label rides in the free text: `wave <marker> CLOSED — editor wave L: …`.

**ORACLE 2 STOPS DEMANDING THE HATCH AT MARKER 99 — measured, not guessed (2026-08-07).**
`wave_plan.tsv` is SILENT for waves 83–98, which is why every gate from editor wave 100 to 115 had
to pass `TBD_GATE_BASE_CONFIRM=<prev close sha>`. But the plan DOES carry rows for waves 99–103
(wave 99 = the 78-row platform backlog; 100–103 = the editor waves of the same number). So once a
close marker reaches 99, the next gate corroborates against those coincidental rows and the hatch
is no longer requested — **verified live at the wave-116 gate, which passed 30/30 with no
`TBD_GATE_BASE_CONFIRM` at all.** This is a NUMBER COLLISION, not agreement about your wave, and it
is benign: the one thing that could have hard-refused is wave 99's only non-`shipped` ticket,
T-449, and it is `cancelled`, which `wave_ledger_unshipped_at` (wave.sh:2044) accepts alongside
`shipped`. **If a gate demands the hatch when this note says it should not, stop and read it — do
not reflexively confirm.**

Derive the base FROM THE LEDGER, never by typing a sha from memory:
`BASE=$(git rev-list --extended-regexp --grep='^wave [0-9]+ CLOSED' -1 HEAD)`. The gate refuses an
abbreviated or misremembered sha, and it refused one of this run's on exactly that ground.

**Standing env on every wave.sh / cargo invocation:**
`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target` (never /tmp) ·
`TBD_WAVE_GENERATION_FLOOR=100` (aim current_wave at this program, not the legacy backlog).

**Shape per wave:** worktrees → ≤3 slice agents (Opus; Fable for `.c` under
apps/mod/tbd-framework/) → BARRIER all report → merge all → `wave.sh gate` → ONE Fable
adversarial verifier on merged main → triage (BLOCKERs fixed in-wave, rest deferred with
diagnosis) → registry flip + `distrobox-host-exec sh -c './scripts/ticket sync'` → close commit.

| Wave | Marker | Tickets | Gate | Verifier | Outcome |
|---|---|---|---|---|---|
| 100 | 83 | T-661 | PASS 30/30 (run 3; runs 1–2 red on non-ticket causes, see Incidents) | 0B/0M/5m/5N | SHIPPED — split proven pure (101 symbols 1:1, 419/419 tests); + capture harness ported to Rust (43a3f170); T-707..T-710 filed |
| 101 | 84 | T-639 T-662 T-663 | PASS (run 1) | 0B/1M/3m/5N | SHIPPED — contour band + RMB/Backspace freed + dead DTO cut; T-711 (doc sync) T-712 (annotations) filed |
| 102 | 85 | T-640 T-664 T-665 | PASS (run 1) | 0B/4M/3m/10N | SHIPPED — two-tone contours + context menu + layer flags; 4 MAJOR deferred: T-713 (crater rings), T-714 (re-march hitch), T-715 (dock vanish), T-716 (menu honesty) |
| 103 | 86 | T-076 T-631 T-641 | PASS (runs 1+2; re-run over blocker fix) | 1B/2M/2m/4N → fix CLEARED | SHIPPED — crew UI + boot failure path + spot heights; BLOCKER (hydrated-crew wipe) fixed in-wave 5f92cc4a; T-717 T-718 deferred |
| 104 | 87 | T-635 T-656 T-666 | PASS (run 1) | 0B/3M/2m/5N | SHIPPED — HUD slot+toggle, validation engine (trip-fixture discipline), layer authoring; T-719 T-720 filed, T-715 amended (hidden-selection lane) |
| 105 | 88 | T-636 T-646 T-683 | PASS (run 3; run 1 red on unswept route → fixed a7d91fca; run 2 = capture rerun) | 1B/1M/3m/5N → fix CLEARED | SHIPPED — status-bar split + class:/chips + defaults endpoint; BLOCKER (multibyte panic) fixed in-wave c2a902dd; T-721 T-722 filed, T-719 amended |
| 106 | 89 | T-647 T-667 T-691 | PASS (run 2; run 1 red on a stale cross-file pin → fixup 74154ba0) | 0B/4M/2m/4N | SHIPPED — T-647 partial (PLACE-001 undelivered, ATTR-OPEN vehicles-only); furniture + prefs clean; T-723..T-726 filed |
| 107 | 90 | T-638 T-657 T-659 | PASS (run 1) | 0B/1M/3m/8N | SHIPPED — dock collapse (accessor seam), 5 ORBAT rules, census+summary; T-727 filed (grid-ref strand, MAJOR); W101 N-3 recorded closed |
| 108 | 91 | T-642 T-650 T-658 | PASS (run 1) | 0B/2M/4m/8N | SHIPPED — ruler end-to-end, doc-side compositions, EvalContext seam; MAJORs routed into T-723/T-726 (widened), T-728 filed |
| 109 | 92 | T-079 T-643 T-660 | PASS (run 1) | 0B/1M/5m/12N | SHIPPED — triggers editor half, LoS ray, cargo/loadout rules; T-729 filed (owner-line perf MAJOR), T-723/T-726 widened again |
| 110 | 93 | T-644 T-648 T-668 | PASS (run 2; run 1 pre-fix) | 1B/1M/5m/13N → flip fixed in-wave | SHIPPED — viewshed (mirror BLOCKER fixed + wired end-to-end), transform gestures, state vocabulary; T-730..T-732 filed, T-729/T-726 widened |
| 111 | 94 | T-645 T-655 T-693 | PASS (runs 1+2; re-run over blocker fix) | 1B/2M/4m/7N → fix CLEARED | SHIPPED — placement helpers (garrison split honestly), always-on validation panel, merge mission; BLOCKER (repeat-merge id collision) fixed in-wave 9ebcb8a9; T-733 T-734 filed |
| — | — | **WAVES 112–119 UN-DEFERRED 2026-08-07** by the operator and run in plan order (24 Rust tickets, 3-wide; they serialize on mission_editor.rs — 8 of 8 waves touch it — so the packing was NOT widened). The 2026-08-02 budget-pivot deferral below is closed out. | | | |
| 112 | 96 | T-649 T-686 T-692 | PASS 30/30 (run 1) | 1M/4m/3N | SHIPPED — select-all in view + multi-edit checkboxes (both suppress-on-multi guards inverted, T-716 rows honest), loadout import round-trip, Help menu + Controls Hint; T-735..T-742 filed |
| 113 | 97 | T-082 T-669 T-694 | PASS 30/30 (run 1) | 1M/1m/3N | SHIPPED — entity type + role description (F-7 closed as a disabled affordance; needed a new core mutator OUTSIDE owns), cut + paste-at-source, mission shape reinterpreted by the operator (derived slot count, no min_players); T-743..T-747 filed |
| 114 | 98 | T-633 T-651 T-695 | PASS 30/30 (run 1) | 1M/2m/4N | SHIPPED — Aegis slider+select primitives in ui.rs, editor comments (own non-compiling core root, rule proven by firing it), catalogue favourites; **PLACE-COMMENT-001 does NOT fully close** — no map glyph, not composable (T-748); T-748..T-752 filed |
| 115 | 99 | T-634 T-670 T-688 | PASS 30/30 (run 1) | 2M/5m/3N | SHIPPED — two-row strip inside the same 48px (no pin weakened), m/px readout unified with T-667's scale bar, aggregated settings with schema-sourced defaults; **T-753 confirmed by experiment** (flow defaults drift ships green); T-753..T-758 filed |
| 116 | 100 | T-069 T-690 T-696 | PASS 30/30 (run 1, **no BASE_CONFIRM needed** — see below) | 2M/2m/3N | SHIPPED — markers authored on briefings[] (the ticket's markersById premise was WRONG and was refused; premise corrected in the registry), compile emits structured diagnostics (six T-216 silent drops now speak), locations + bookmarks; T-759..T-763 filed |
| 117 | 101 | T-084 T-671 T-672 | PASS 30/30 (run 1) | 1M/2m/1N | SHIPPED — search grammar (classname-TAIL decision made and pinned), mission presentation (**three live bugs found and fixed**), connection graph (see+check in full, one edge verb short by design); T-764..T-768 filed |
| 118 | 102 | T-637 T-698 T-699 | PASS 30/30 (run 1) | 1M/3m/1N | SHIPPED — 240px dock equalisation (input-critical geometry, unprojection pinned), clipboard exporters, loadout buffer; the T-634 tripwire fired and was INVERTED not weakened; **verifier REFUTED two slice claims** (T-769, and T-759's attempted widening); T-769..T-773 filed |
| 119 | 103 | T-697 T-700 T-703 | PASS 30/30 (run 1) | 2M/1m/4N | SHIPPED — document search + selection filter (one query language across four surfaces), numeric nudge + shared SearchBox, **keybinding collision test that found bindings shipped undocumented for the whole programme**; T-774..T-776 filed. **RUN COMPLETE: 24/24 tickets, 8/8 gates PASS on run 1, zero quarantines, zero BLOCKERs.** |
| 120 | 95 | T-701 T-706 | PASS (run 3; run 1 red on the ledger tripwire firing as designed, run 2 = fixup) | 0B/4M/5m/4N → all hardened pre-close | SHIPPED — per-entity hide + the one-pass 1.3 contract; ledger learned DeclaredPendingEmit; 9 reader-ambiguities + 3 prose residues fixed BEFORE any reader dispatched; unread gate live at 53 fields |
| 127 | 104 | T-775 T-753 T-773 | PASS 30/30 (run 1; re-gated PASS 30/30 over the fix pass) | 0B/2M/1m/1N + focused re-verify 0B/0M/1m/1N | SHIPPED — **NO-DEFERRAL REGIME (operator, 2026-08-07): every verifier finding at every severity is fixed in-wave, not filed.** All 6 findings fixed across 3 fix rounds. The wave's real yield was a FOUR-INSTANCE LIVE DEFECT FAMILY: `update_slot_position` stores `pz = 0.0` on any x/y write with z=None, excused everywhere by a "DEM re-sampled JS-side" comment for a sampler that **died with the React deletion** — fixed at attrs (F-2), Align/Distribute/pattern (F-5) and the marquee drag (F-6, which passed a literal `vec![0.0; n]`). Family proven CLOSED by the re-verifier. T-777 filed and deliberately NOT fixed — it is T-743's reserved decision surface |
| 128 | 105 | T-764 T-735 T-774 | PASS 30/30 (run 1; re-gated twice over the fix pass, PASS both) | 1M/1m/1N + focused re-verify 0B/0M/1m | SHIPPED — 4/4 findings fixed. **Every ticket this wave was a guard that could not see what it claimed to check**, and the pattern recurred one level down: T-735 shipped an audit so unimplemented keywords fail CLOSED, and that audit walked `$ref` CHAINS to the far end and pronounced the schema fully supported while the validator checked nothing (`modpackId: 123` accepted against a chained `minLength: 3`) — refused rather than followed, so cycle detection never enters the one path whose job is not to lie. Also: escaped RFC-6901 pointers silently resolved to the WRONG node. T-764's depth cap held under every attack; T-774 settled the binding count by measurement (34/32 — T-703's "39" wrong by seven). **The verifier RETRACTED its own stack measurement in favour of the fix agent's.** |
| 129 | 106 | T-754 T-759 T-737 | PASS 30/30 (run 1; re-gated 3× over the fix pass, PASS each) | 2M/1m/1N + re-verify 1M/1N | SHIPPED — dead-click router widening, hollow pins scoped to production, import refusals named. **7 fixes over 5 rounds, and TWO WERE DEFECTS THE WAVE'S OWN FIXES CREATED**: F2 made `route_select_zone` honest, after which F1's affordance probe disagreed with it (F6); and F1 ADDED an unguarded `thread_local` seam while F5 was fixing the identical shape twelve lines away. The availability question was answered in three independent places; F7 collapsed it to one (`validation_panel::subject_id_routes`). **PROCESS NOTE — the fix pass overran its own 3-round cap and RV-1 (an inert-but-would-work row) should have been FILED, not fixed: the two affordance polarities are not equally severe.** T-778 filed (6 unguarded seams in 4 files) |
| 130 | 107 | T-723 T-760 T-769 | PASS 30/30 (run 1 fmt red → rustfmt fixup; re-gated PASS over F1/F3/F4) | 0B/1M/2m/1N + focused re-verify 0B/0M/0m/0N | SHIPPED — armed pointerup (button0/KeepArmed/Esc-RMB), MissionMarkers lane fed from after_doc_change+rebind, measured dock tree h-full. **F1 MAJOR: markers_bind feeds were unpinned hollow wiring — Class-R pin now RED if either feed is deleted.** Suite 967 after pin (966 pre-fix; T-723 claim 965 was wrong by one). |
| 131 | 108 | T-762 T-767 T-746 | PASS 30/30 (run 1 + rustfmt fixup; re-gated PASS over F1–F3) | 0B/2M/0m/1N + focused re-verify 0 | SHIPPED — world_assets fly_to/named_locations, connection re-exports + formation prose, ShapeMirror single-flight. **Two hollow-pin MAJORs fixed**: F1 body pins on fly_to/named_locations; F2 ROW_HYDRATE detail.game_mode. Suite 972. ConnKind frontend copy remains found_not_fixed (owns). |
| 132 | 109 | T-771 T-758 T-765 | PASS 30/30 (run 1; re-gated PASS over F1–F3) | 0B/0M/0m/2N+standing + re-verify 0 | SHIPPED — dual-scope loadout banner, inert settings/validation non-focusable rows, Unicode glob fold. Suite 981. |
| 133 | 110 | T-750 T-766 T-756 | PASS 30/30 (run 1; re-gated PASS over F1–F2) | 0B/1M/0m/1N + re-verify 0 | SHIPPED — Favourites failure+Retry, clear_meta_briefing, scale formatter corners. **F1 MAJOR: blank clear reachability at call site was hollow — pinned.** Suite 986. |
| 134 | 111 | T-736 T-755 T-776 | PASS 30/30 (run 1; re-gated PASS over F1–F3) | 0B/1M/1m/1N + re-verify 0 | SHIPPED — hollow-pin wave: one-commit/one-tail, narrated schema/scale/ControlsHint pins, census/search guards. **Three new hollow shapes found+fixed**: Escape predicate live-state, alias ::Schema, STRIP div-balance. Suite 987. |
| 135 | 112 | T-744 T-751 T-757 | PASS 30/30 (run 1; re-gated PASS over F1–H2) | 0B/0M/2m/1N+H + re-verify 0 | SHIPPED — Attributes stay open on hide, Select/favourites/compile honesty, one MISSION_SCHEMA embed. Suite 992. |
| 136 | 113 | T-745 T-761 T-763 | PASS 30/30 (run 1; re-gated PASS over F1–F4) | 0B/2M/2m/0N + focused re-verify 0 | SHIPPED — attrs_update_slot no-op + lasting Class-R pin; compile findings cleared on hydrate; decoy narrative struck, marker Attributes (factionId,id), /compiled route pins count header (API handlers/missions.rs). Suite 997. |
| 137 | 114 | T-741 T-749 T-772 | PASS 30/30 (run 1; re-gated PASS over F1/F2/NIT) | 0B/1M/1m/1N + focused re-verify 0 | SHIPPED — multi-edit slot-scope honesty + selection_n flow pin; settle-only scrubber prose/pin rename + Slider rustdoc; ControlsHint close call-site hit-box. Suite 1003. |
| 138 | 115 | T-742 T-752 T-739 | PASS 30/30 (run 1; re-gated PASS over F1–F6) | 0B/4M/2m/1N + focused re-verify 0 | SHIPPED — approach C: wave.sh test --slice private dirs + brief ban; clippy --all-targets Makefile+CI+gate; gap_analysis/top_strip cite honesty. Suite 1006. |
| 139 | 116 | T-732 T-747 T-726 | PASS 30/30 (run 1; re-gated PASS over F1–F3 + cite) | 0B/3M/0m/1N + focused re-verify 0 | SHIPPED — attrs multi atomic undo; wave.sh map-engine --all-features + tripwire; Esc→modal_stack (ORBAT/Faction/top-strip). Suite 1014. |
| 140 | 117 | T-770 T-768 T-740 | PASS 30/30 (run 1) | 0B/0M/2m/1N | SHIPPED — loadout sink acks; connect LMB→complete_connect; redo Cmd chord. Suite 1023. |
| 141 | 118 | T-748 T-738 | PASS 30/30 (run 1) | 0B/0M/1m/1N | SHIPPED — MissionComments map glyphs + rebind feeds; Escape help shared-channel honesty. Suite 1025. |
| 142 | 119 | T-778 T-779 T-780 | PASS 30/30 (run 1; re-gated PASS over the fix) | 1M/1m/2N | SHIPPED — the residues the 130–141 run NAMED rather than dropped. T-778 guarded 5 of 6 seams and correctly REFUSED the 6th (a `window` bridge with `Closure::forget()` — no disposable state, nothing to guard); T-779 found 2 MORE discarded acks while auditing 159 call sites, and its refusal surface exists because gating the tail correctly would otherwise have shown GREEN "no unsaved changes" over a pick that never landed; T-780 ported the marker/comment lane recipe but had to feed it from a different channel (owns), then PINNED the chain proving every history path reaches it — the verifier attacked all 59 sites, both engine-mount orderings and IDB restore success AND failure, and could not break it. **VERIFIER MAJOR: T-780 reintroduced T-779's own defect in the same wave** — `delete_connection` guarded on `connection_count()==0` rather than id presence, so a stale edge id reported success and dirtied an unchanged document. Fixed by construction: the reconcile now lives inside the single writer every selection write funnels through. Also found: `everon_peaks_max_above_350` needs `--features png` or a bare run SILENTLY FILTERS IT OUT — the T-747 vacuous-pass family on a new flag |
| 143 | 120 | T-743 T-777 T-782 | PASS 30/30 (run 1; re-gated PASS over the fix) | 0B/0M/2m/1N | SHIPPED — **the operator's two reserved decisions, executed.** Paste-at-original now lands on the SOURCE coordinates and keeps the AUTHORED z; the editor's player figure is the derived placed-slot count. **The ticket's premise was wrong: there was NO golden-test churn** — `PASTE_NUDGE` had one consumer and zero appearances in any test, fixture or golden, so the "blast radius" that made this an operator decision for months was an assumption about an uncovered arm. The constant was DELETED, not zeroed (a `0.0` named "nudge" invites re-inflation), and the shared no-anchor arm was split: plain Ctrl+V resolves its own anchor and falls back to the VIEW CENTRE, because the cursor reads off-map whenever the pointer sits over chrome — the normal state after click-then-paste. T-777 read z from the CLIPBOARD not the live document (a live read pairs the copy's old x/y with the source's CURRENT elevation, and resolves to nothing across missions or after the source is deleted). **The combined behaviour — which neither slice could test alone — was verified empirically by the verifier: exact x/y AND per-slot z.** T-782 left the compile path untouched: the ruling settled what is DISPLAYED, not what is COMPILED |
| 144 | (disavowed, folded into 121) | T-781 T-783 | PASS 30/30 | 1M/1m | SHIPPED — comments become composable (PLACE-COMMENT-001's last clause) and a placed composition keeps its authored elevation; the seam lifecycle mechanism collapses from two copies to one. **The verifier found a LIVE DATA-LOSS PATH predating the wave:** `mint_id`/`mint_ids` proved id uniqueness against `materialize()`, which DROPS hidden-layer (T-665) and editorHidden (T-701) slots; `next_id` resets per mount and inserts are UPSERTS, so after an IDB restore a place could re-mint a hidden slot's id and silently overwrite it. **Same blind spot wave 127 hit with elevations.** Fixed by sourcing both minters from `slots_json`. **This wave's own close (marker 121) was later DISAVOWED — see the renumber note above — and its span re-gated and re-closed together with wave 145.** |
| 145 | 121 | T-784 | PASS 30/30 (gated jointly with 144 after the disavowal) | 0B/0M/0m/4N + orphan-commit audit | SHIPPED — a comment can be selected by clicking it, in the outliner and on the map glyph, so T-781's composable comments are finally reachable (the wave-144 verifier proved the gap was TOTAL: the selection filter only NARROWS, so it could not introduce a comment). Exclusive-vs-composes was decided FROM THE CODE — the composition capture takes one selection slice, so a separate lane would have made T-781's capture arm unreachable. **The reconcile needed no new code**: the comment id lands in `ctx.selection` itself, and the existing reconcile keys on emptiness rather than kind. Delete partitions by asking the document, not by a `cmt-` prefix (a minting convention, wrong for hydrated missions). **Also fixed a pre-existing selection-loss bug**: both prune sites checked survivors against the slot SoA, so ANY document change silently dropped every vehicle, object and comment id — and deselected hidden slots for being INVISIBLE rather than GONE, which made `show_selection` unreachable. **The fix agent DIED mid-run after committing**; a separate audit confirmed the commit complete and correct, perturbing its pin six ways including needles-in-comments. |
| 200 | 122 | T-785 T-786 T-787 | PASS 30/30 (run 1; a2i smoke fixup bef0a071 pre-gate — the T-785 slice disclosed the smoke encoded the old per-keystroke contract) | 4M/3m/2N | SHIPPED — text_field focused/draft split (core F-01 CLOSED — verifier re-proved with real per-char CDP, ~60 chars, zero focus losses), modal z from modal_stack + strip transient exclusivity (O-3/O-5 acceptance pairs re-proved STACK-DRIVEN, every z-50 tie scrim-unreachable), docks end on the bar (O-1 re-proved with equality at 3 viewports; 36px inset vindicated over the spec's 96px by measurement). **OPERATOR DECISION 2026-08-09: the verifier's 9 findings are NOT fixed in-wave — filed as T-811..T-815 (5 tickets, findings merged by file), wave 209, executed by Grok/Cursor factory (`docs/platform/WAVE209_GROK_KICKOFF.md`); supersedes the no-deferral regime for this wave. Wave 209 MUST land before 201 — owns overlap (attributes/eden_top_strip/orbat_manager vs T-807).** Headline findings: F-02 was fixed on the WRONG WIDGET (bookmark rename; the real layer rename is eden_tree.rs, OUTSIDE owns — a green source-pin on the wrong widget from the one slice that shipped with no browser); the fixed bookmark rename now truncates to last char; multi-edit differing-field focus+blur with zero typing wipes the field across the selection; strip transients survive non-strip dialog opens + one-Esc-two-layers. A mid-wave harness restart killed the first in-place fix pass (2 agents, uncommitted); partial diff parked in session scratchpad, tree verified reset to bef0a071 — nothing unverified landed. |
| 209 | 123 | T-811 T-812 T-813 T-814 T-815 | PASS 30/30 (Grok-run, then first-party re-run; re-gated PASS over the pin fixup 27343f82) | 2M/0m/0N → both fixed in-wave + focused re-verify CLEAN | SHIPPED — **the wave-200 verifier findings, implemented by Grok/Cursor per WAVE209_GROK_KICKOFF.md, verified and closed from this command center.** All five acceptances re-proved live with real per-char CDP, including the COMPOSED Esc ladder (T-813 field-consume + T-814 open-order/consume-aware — one layer per press, field-draft → visual-top → next → transient, no pile-up under rapid Esc). Both MAJORs were broken REGRESSION GUARDS, not features: T-814's O-3 wiring pin disarmed by a stray second #[test] (fn orphaned, duplicate ran twice — the gate counted the wrong test), and T-811's layer pin hollow (raw include_str self-match, the T-759 class, fresh in the file whose wave-200 defect was a green pin on the wrong widget). Fixed 27343f82 (test-module-only, cmp-proven scope-pure), perturbation-proven both directions, re-gate PASS, focused re-verify clean. Grok boundary discipline held: no close, no registry writes, no repack, parked worktrees untouched; two unpoison re-lands were its own catch. T787 probe missions deleted (T-815 NIT-2). **Wave 209 unblocks wave 201 pending the operator eye-pass of 200+209.** |
| 201 | — | T-807 T-791 T-793 | PENDING | PENDING | PENDING — UX remediation band (source `.ai/artifacts/editor_hostile_ux_review.md`, briefs in registry summaries; band rules in `EDITOR_FACTORY_START.md` §UX remediation band) |
| 201 | 124 | T-807 T-791 T-793 | PASS 30/30 (run 1) | 0B/0M/2N → filed T-816 T-817 | SHIPPED — copy sweep landed whole (units+rounding with the no-op discipline intact, '1 slot', DEM hint gone, hero gated on the REAL enum value 'live', yrs-persist quiet with BLOCKED_EMPTY preserved, why-tooltips on disabled rows, SNAP chip); **T-791's premise was STALE — the composition stamp already shipped** (T-650/T-723/T-781 lineage; the review's live dead-end did not reproduce; verifier re-proved the full F-30 acceptance incl. stored-elevation survival) so the slice shipped the missing live arm hint + un-hollowed two pins; **T-793 root-caused O-2 as text-keyed <For> DOM reuse** (T-727 class, not a stale transform) — 250px spacing exact, 0.00px vs the CUR oracle post-240m-pan, per-frame mid-pan. F-13's status-bar half was found in T-793's file by T-807 and routed there MID-WAVE by orchestrator addendum (no cross-owns edit). Verifier's two NITs are pre-existing and filed: Esc hint/arm double-consume (T-816), wheel-zoom label heartbeat lag (T-817). Operator override 2026-08-09: waves 200→209→201 run in ONE session (eye-pass verdicts recorded between); the one-wave-per-session amendment resumes at wave 202. |
| 202 | — | T-788 T-797 T-800 | PENDING | PENDING | PENDING — UX remediation band (source `.ai/artifacts/editor_hostile_ux_review.md`, briefs in registry summaries; band rules in `EDITOR_FACTORY_START.md` §UX remediation band) |
| 202 | 125 | T-788 T-797 T-800 | PASS 30/30 (run 1 pre-fix; post-fix re-gate red ONCE on a dead Postgres — environment, `make db-up` + api restart, then PASS; two harness restarts mid-wave, no work lost — everything was committed) | 1M → fixed in-wave 95ee8cb7 + focused re-verify RESOLVED | SHIPPED — **multi-edit costs ONE undo step** (premise split: position path already atomic per T-732; the identity path the review measured got the new core `update_slots_attr_batch`, added T-732-style in the in-wave COMPLETION PASS because it sat outside T-788's owns), keep-multi dblclick both arms, panel follows the selection; **row-2 toolbar + Edit menu are LIVE** (slice shipped disabled-first — the dispatch lived in sibling-owned mission_editor.rs; completion added the register bridge; the verifier's 1 MAJOR was the completion's own false reactivity claim — plates frozen by a mount-order subscription gap — fixed with a generation-signal subscription + the missing pin), ORBAT in the menu row, View menu gone (F-14+F-15 emptied it); **catalog failures name their cause + Retry**, Add Vehicle explains an empty catalog, dev seed finally has vehicles (4, idempotent). Slice honesty note: BOTH structural gaps were disclosed refusals with recipes, not silent deferrals — the completion-pass pattern (checkpoint commits on merged main) closed them pre-gate. Recorded, not filed: a second in-session engine boot (route-leave → return) crashes under HEADLESS SOFTWARE WebGPU only — byte-identical on base, zero engine code in the wave; the operator eye-pass probes it on real GPU. |
| 203 | — | T-792 T-789 T-809 | PENDING | PENDING | PENDING — UX remediation band (source `.ai/artifacts/editor_hostile_ux_review.md`, briefs in registry summaries; band rules in `EDITOR_FACTORY_START.md` §UX remediation band) |
| 204 | — | T-790 T-798 T-803 | PENDING | PENDING | PENDING — UX remediation band (source `.ai/artifacts/editor_hostile_ux_review.md`, briefs in registry summaries; band rules in `EDITOR_FACTORY_START.md` §UX remediation band) |
| 205 | — | T-795 T-799 T-806 | PENDING | PENDING | PENDING — UX remediation band (source `.ai/artifacts/editor_hostile_ux_review.md`, briefs in registry summaries; band rules in `EDITOR_FACTORY_START.md` §UX remediation band) |
| 206 | — | T-796 T-804 T-810 | PENDING | PENDING | PENDING — UX remediation band (source `.ai/artifacts/editor_hostile_ux_review.md`, briefs in registry summaries; band rules in `EDITOR_FACTORY_START.md` §UX remediation band) |
| 207 | — | T-802 T-808 T-794 | PENDING | PENDING | PENDING — UX remediation band (source `.ai/artifacts/editor_hostile_ux_review.md`, briefs in registry summaries; band rules in `EDITOR_FACTORY_START.md` §UX remediation band) |
| 208 | — | T-801 T-805 | PENDING | PENDING | PENDING — UX remediation band (source `.ai/artifacts/editor_hostile_ux_review.md`, briefs in registry summaries; band rules in `EDITOR_FACTORY_START.md` §UX remediation band) |
| 150 | assign at close | T-702 T-212 T-654 | — | — | **PARKED** (was wave 121 — RENUMBERED 2026-08-08, see below) |
| 151 | assign at close | T-673 T-674 T-675 | — | — | **PARKED** (was 122) |
| 152 | assign at close | T-676 T-677 T-678 | — | — | **PARKED** (was 123) |
| 153 | assign at close | T-679 T-680 T-681 | — | — | **PARKED** (was 124) |
| 154 | assign at close | T-682 T-684 T-685 | — | — | **PARKED** (was 125) |
| 155 | assign at close | T-689 T-705 | — | — | **PARKED** (was 126) |

> ## ⚠️ THE MOD HALF WAS RENUMBERED 121–126 → 150–155 ON 2026-08-08. READ THIS BEFORE RESUMING IT.
>
> **Nothing about the mod work changed** — not a line of `.c`, not a branch, not a worktree, not a
> ticket. **Only the wave-number column in `wave_plan.tsv` moved**, and this table with it. The six
> parked worktrees (T-702, T-212, T-654, T-673, T-674, T-675) are untouched and still parked
> mid-barrier. Verified at the time: the plan's ticket set is byte-identical before and after, 574
> rows both ways.
>
> **WHY.** Close markers and editor-wave labels are independent counters — the top of this file says
> so — but `wave.sh`'s ledger cross-check looks a close marker up in the plan's WAVE column. That is
> harmless while the numbers do not collide. At **marker 121** it collided with mod wave 121, whose
> three tickets are legitimately `queued` because the half is parked, so the gate concluded that
> wave 144's close "assigns wave 121 ticket(s) the registry does not call shipped" and **refused
> every base, derived or explicit.** There is no override env on that path. Markers 122–126 would
> have hit mod waves 122–126 the same way — seventeen queued tickets across six waves, i.e. every
> remaining remediation wave.
>
> This file already predicted the class ("a NUMBER COLLISION, not agreement about your wave") and
> called it benign at marker 99, correctly: wave 99's only non-shipped ticket was `cancelled`, which
> `wave_ledger_unshipped_at` accepts. It is **not** benign when the colliding tickets are genuinely
> open.
>
> **The real defect is the cross-check conflating two counters the design calls independent.** The
> operator chose the renumber over editing `wave.sh` mid-run — smaller, reversible, and it does not
> touch the gate itself. **If the mod half is ever renumbered back into a live marker range, or if
> markers ever reach 150, this returns.** The durable fix is to make that check compare a marker
> against the MARKER ledger, not against plan rows that happen to share its number.
>
> **CONSEQUENCE FOR THE NEXT GATE:** markers 121+ now have no plan rows at all, so oracle 2 falls
> silent and the gate asks for `TBD_GATE_BASE_CONFIRM=<prev close sha>` again — the behaviour every
> gate from editor wave 100 to 115 had. That is expected here, not a signal. Verify the sha is the
> previous `wave M-1 CLOSED` commit from the git ledger, then pass it.

## Continuation recipe (compaction-proof — execute mechanically from any fresh context)

State lives in the table above: the first non-`SHIPPED` row is the current wave. **The 112–119
deferral was LIFTED on 2026-08-07 — those waves are being run in plan order, so read the table
normally and do not skip them.** (Waves 121–126, the mod half, remain PARKED mid-barrier: five
slice branches committed-and-unmerged — T-702, T-212, T-654, T-673, T-674 — plus T-675's worktree
stacked on T-674 with no work in it. Do not reap, drop or merge those six; preflight WARNs about
them and that warn is expected. Their pending unread-gate row flips are a mod-half close step.)
The close marker is **not** derived from the wave number: read it
from git (`git log --grep='^wave [0-9]\+ CLOSED' --format='%s' -1`, then +1 — see the top of this
file). Per wave L, tickets from `awk -F'\t' '$1==L' docs/platform/wave_plan.tsv`:
1. `bash scripts/mod/slice-worktree.sh new T-xxx` per ticket.
2. Dispatch ≤3 slice agents (Agent tool, background): model **opus** for ALL coders — Rust and
   Enfusion `.c` alike (operator cost amendment 2026-08-07; supersedes the Enfusion→Fable rule —
   do not resurrect it from older copies). Verifiers stay **fable**. Brief = registry summary
   verbatim + the standing HARD RULES block and the required report schema, both recorded in
   **[`docs/platform/EDITOR_SLICE_BRIEF.md`](../../docs/platform/EDITOR_SLICE_BRIEF.md)** — paste
   the block inline (it is 0.12% of the budget; making agents read it costs more than it saves).
3. BARRIER: all agents report. Then merge each: `git merge --no-ff slice/T-xxx -m "T-xxx: <title>"`.
4. Wave gate: `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target TBD_WAVE_GENERATION_FLOOR=100
   TBD_GATE_WAVE=<L> bash scripts/platform/wave.sh gate` — when oracle 2 falls silent (markers 83+
   have no matching plan rows) it demands `TBD_GATE_BASE_CONFIRM=<prev close sha>`: verify the sha
   is the previous `wave M-1 CLOSED` commit (git log), then pass it. Red → triage: fix in-wave
   once; a second red on the SAME ticket → QUARANTINE, DON'T STOP (operator amendment 2026-08-02):
   revert that slice off main (`wave.sh revert` keeps the branch alive), defer its ticket with the
   full diagnosis, close the wave with the rest, continue. Owns collision at dispatch → serialize
   the colliding tickets within the wave. Environment blocks → fix the environment. The ONLY stop
   is real data loss, where continuing could destroy work.
5. One Fable adversarial verifier over merged main (documents, never fixes; BLOCKERs fixed
   in-wave, else deferred tickets + diagnosis). Report → .ai/artifacts/editor_verify/waveL.md.
   Brief + severity/triage table: **[`docs/platform/EDITOR_VERIFY_BRIEF.md`](../../docs/platform/EDITOR_VERIFY_BRIEF.md)**.
   **Every wave, full adversarial, Fable 5 — operator decision 2026-08-07; not tiered, not traded.**
6. `bash scripts/platform/wave.sh verified $(git rev-parse HEAD)` · registry: wave tickets →
   shipped, verifier tickets filed (next free T-7xx) · `distrobox-host-exec sh -c 'cd
   /home/Samuel/Projects/TBD-Reforger && ./scripts/ticket sync'` · update this file's table ·
   `echo L > docs/platform/factory_pack_wave`.
7. Close commit staging EXPLICIT paths (registry, generated TICKET_* docs, CLAUDE.md, ROADMAP.md,
   this file, editor_verify/waveL.md, factory_pack_wave), subject
   `wave M CLOSED — editor wave L: <one-liner>; GATE PASS n/n`, Co-Authored-By trailer. Then
   `bash scripts/mod/slice-worktree.sh drop T-xxx` per slice and
   `git -c core.hooksPath=/dev/null push origin main` (plain push dies on the absent-git-lfs
   pre-push hook; verify range LFS-free first: `git diff --name-only origin/main..HEAD | grep
   map-assets` → must be empty).
8. Next wave. After wave 126: make ci-local + make leptos-gates, update
   docs/platform/EDITOR_FACTORY_START.md per-wave outcomes, final summary. NO playtest.

Known traps already hit: rustfmt needs the file's real edition (tools/* are 2024); `make
schema-validate` broken in worktrees; agent chat summaries are not artifacts — trust gates and
the verifier; wave-101 agents running concurrent slice gates queue on the gate lock (WAITING is
serialisation, not a hang).

## T-742 (w138) — THE SHARED TARGET DIR HAS LIED SIX TIMES IN TWO WAVES

Standing instruction for every remaining wave, and the evidence w138 should fix against.
`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target` lets concurrent worktrees execute each other's
test binary. Observed live, 2026-08-07/08:

| # | Wave | Direction | What happened |
|---|---|---|---|
| 1 | w127 T-753 | noise | 907 / 905 / 906+1-unreachable-failure / 907 on an UNCHANGED tree |
| 2 | w127 T-775 | **FALSE PASS** | `14 passed` with BOTH new tests absent from the binary; `--list` confirmed |
| 3 | w128 T-764 | **FALSE PASS** | run said `915 passed`; `--list` on the SAME dir showed 917 tests, neither of its names present |
| 4 | w128 T-735 | false fail | a red in a SIBLING's file, mid-edit |
| 5 | w128 T-774 | **FALSE PASS THAT HID REAL DEFECTS** | prose pin GREEN while the asserted phrase was split across a line break and could not match. On a private dir it was RED — and fixing it exposed TWO further real defects |
| 6 | w128 verifier | phantom flake | a reported "load-dependent flake" was proven to be a foreign binary: the failing assertion (`count > 95` over a fixed 103-char literal) is unfalsifiable from shipped source. 40/40 green under 6-way load |

**`touch` DOES NOT PREVENT IT — that mitigation is retired (sighting 5 had touched first).**

**STANDING RULE, in every brief from wave 129:** do ad-hoc verification in a private
`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target-<TICKET>` (**never /tmp** — 16 GB tmpfs), delete it
before reporting, run the mandated slice gate on the shared dir, and **cross-check the `--list`
total against the run total every time, reporting both**. Disagreement means the binary is not yours.

**MEASURED DISK FIGURES — T-742's summary is wrong and this changes its approach decision.**
The ticket says "~4 GB shared vs ~44 GB each". Measured on the live box mid-run: shared dir **57 GB**;
a slice-private *frontend test* dir **2.7 GB**; in-repo gate dirs ~41 GB combined (`target-gate-api`
alone 27 GB). The ~44 GB figure is a full WORKSPACE build — a per-slice **test** dir is 2.7 GB, so
three concurrent slices cost ~8 GB, not ~132 GB. **Per-slice test dirs are affordable**, which the
ticket assumed they were not. The operator reserved this approach decision; give them these numbers.

## THE z = 0.0 FAMILY — discovered and closed in wave 127; bake into every later brief

`MissionDocCore::update_slot_position` (store.rs:2779-2783) writes `pz = 0.0` whenever `z` is None
and x or y is Some. Every call site excused this with a comment saying the DEM is re-sampled
JS-side. **That sampler does not exist** — `terrainZ` did not survive the React deletion; a grep for
`terrainZ|terrain_z|sample_z|sampleZ|dem_z|elevation_at` across `apps/website/frontend/src` returns
only comments saying so. The 0.0 was final, and vehicles were unaffected (`set_vehicle_position`
passes `e.z`) — **the slot/vehicle asymmetry is the reliable tell**.

Four instances, all shipped green, all fixed in wave 127:
`attrs_update_position`/`_multi` (F-2) · `commit_positions`, reached by
`apply_pattern_to_selection`/`align_selection`/`space_selection` (F-5) · the `LG::Move` marquee drag,
which passed a literal `vec![0.0; n]` (F-6) · `move_entities` — no frontend caller.

**Rules for every later wave:**
- **NEVER fix this in `crates/map-engine-core`.** store.rs:2747-2749 claims byte-parity with the JS
  oracle `ydoc.updateSlotPosition`. Fix at the frontend caller, every time.
- **Reuse `keep_z_rows()` / `slot_z()`** (`editor_ops.rs:1216-1257`, `pub(crate)`). A third
  z-resolution path is its own defect class. Read z off `slots_json` (exact f64) — **NOT** the SoA,
  whose `zs` is f32 and omits hidden-layer slots (T-665).
- **Any new `update_slot_position` / `move_entities_and_vehicles` caller must pass a real z.**
  Rotation-only calls (x=None, y=None) cannot trip the arm and are safe.
- Batch callers: hoist the document read; `raw_slot_rows` is an O(document) JSON parse.
- If you build a `zs` vector, `zs[i]` must be `ids[i]` — a mismatched zip hands one slot another's
  elevation, which is WORSE than the zeroing and looks green.

## Forward constraints for later-wave briefs (from verifiers — bake into dispatch prompts)

- **T-664 (W102):** mount the context menu beside the UNGATED dialog mounts (survives chrome_hidden);
  do not treat `defaultPrevented` on contextmenu as "already handled" — prevent_default is only
  suppressing the browser menu (mission_editor.rs:1868). [wave101 N-5]
- **T-640 (W102):** t090_render_lod_contract.md §N3 is STALE (T-711 pending) — trust
  lod_gates.rs/dem_vectors.rs as shipped, not the doc. [wave101 F-1]
- **T-636 (W105):** the toolbelt/status-bar split must keep BOTH new mounts behind the
  `chrome_hidden` gate (the rf HUD already inherits it). [wave101 N-5]
- **T-706 (W120):** eden_env.rs:309 `keys_nothing_reads_are_not_authored` names viewDistance/
  thermals as never-authored — widening the schema must consciously edit that list or stay clear
  of those keys; mission.schema.json itself never carried them. [wave101 N-5] ALSO: T-650's
  `compositions` rides the open editor-payload root undeclared (the schema's own zones-note
  anti-pattern; downstream verified safe) — declare it while widening. [wave108 MINOR-3]
- **T-660 (W109):** include the one-line EvalContext doc fix — the doc instructs cross-crate
  construction via `..Default::default()`, which is E0639 on a #[non_exhaustive] struct; point
  callers at with_known_asset_ids. Your cargo rules extend EvalContext — that's the invited
  seam. [wave108 MINOR-4]
- **T-655 (W111):** construct EvalContext via with_known_asset_ids (not struct syntax). [wave108]
- **T-666 (W104):** T-715 is known-pending on the same files (hidden slots vanish from docks via
  filtered slot_rows) — do not double-fix, do not regress dimming further. [wave102 F-3]
- **T-649 (W112):** coordinate the multi-select edit path with T-716's enabled-but-dead menu rows
  (Attributes/Edit Loadout at len()>1). [wave102 F-5]
- **T-636 (W105):** the bottom-band re-layout should give the debug HUD a legitimate visible slot
  (T-719: it's currently painted over by DockRight z-20) — if it does, T-719 collapses to its
  AltGr line; pin it with a test either way. [wave104 MAJOR-1]
- **T-657 (W107):** Finding.subject is a positional JSON pointer and V3's message omits the slot
  id — add a stable subject id to Finding while writing the ORBAT rules (T-655's click-to-select
  needs it); rules must not panic (eval panics propagate — wasm trap once the panel wires
  always-on eval). [wave104 MINOR-1 + engine probe]
- **T-655 (W111):** consume Finding.subject via retained-snapshot index→id mapping OR rely on the
  id T-657 adds; never gate diagnostics behind a key (doctrine chain §D.4#7). [wave104 MINOR-1]
- **T-667 (W106):** mount the scale bar in the status bar's CLEAR CENTRE span (data-status-furniture
  slot); grid-reference labels anchor to the MAP PANE edges (between the docks), not viewport
  edges — correct Eden geometry AND avoids the T-721 occluded zones (bar's left 256px / right
  320px sit under the docks until T-721 lands). [wave105 MAJOR-1]
- **T-642 (W108) + T-648 (W110):** both build on mission_editor's gesture machine — T-723 is
  known-pending there (armed pointerup: no button filter, stranded LG::Pending, no Esc disarm;
  the ':1940 left/pan_px None' invariant comment is FALSE). Do not trust that comment; do not
  build ruler/transform arms on the armed-branch behaviour without reading wave106.md MAJOR-2/3.
  Regression tests must be event-sequence tests, not source pins. [wave106 MAJOR-1..3]
- **T-650 (W108):** if compositions introduce a squad-template shape, wire it to
  T-657's TEMPLATE-COVERAGE field (squad.template.requiredRoles — currently production-dead,
  no writer exists); otherwise leave the rule forward-compat and say so. [wave107 MINOR-2]
- **T-655 (W111):** ORBAT-CALLSIGN-UNIQUE emits an unresolvable JSON pointer
  (/editor/squads/{id}/callsign keys an id into an array; the others are positional) — one-line
  shape fix while wiring the panel; subject_id already rescues selection. [wave107 MINOR-1]
- **T-644 (W110):** T-643's occlusion() anchors the observer eye at the first COVERED sample —
  an off-coverage head + descending sight yields a false BLOCKED at the profile's first sample
  (unreachable on full-coverage Everon today; guard it in the viewshed). Also: live LoS reads the
  8 m box-average grid — systematically optimistic on knife crests vs the raw raster; documented,
  untested at the seam. [wave109 MINOR-2 + NOTE]
- **T-648 (W110):** INCLUDED one-line fix — the false T-159.22 comment in mission_editor's
  pointerdown (':2150-ish, left/pan_px are both None here') is four waves stale with its own
  refutation stacked beneath it; delete/correct it while you're in the file. [wave109 NOTE]
- **T-706 (W120):** flatten.rs already emits a win-condition "triggers" vocabulary — T-079's
  editor triggersById is a DIFFERENT object; the schema widening must name them apart (editor
  triggers vs win triggers) or the wire collides. [wave109 NOTE]
- **T-645 (W111):** the ticket mandates one-step-undoable bulk ops but no atomic batch API exists
  in store.rs (T-732 filed — three-instance family). Use the existing one-txn batch shapes
  (paste_slots / place_composition / move_entities_and_vehicles) where they fit; where they
  don't, document N-step honestly and cite T-732 — do not fake atomicity. [wave110 NOTE-B]
- **T-084 (W117):** `class:` matches full resource_name prefixes only — a bare classname
  (class:B_Soldier) silently empties the tree on GUID-headed Reforger ids; decide classname-TAIL
  matching semantics as part of the grammar rewrite. [wave105 MINOR-2]
- **T-082 (W113):** the Attributes modal shows refused locked-slot Transform edits as accepted
  (one-shot snapshot, no re-read) — add re-read or disabled affordance. [wave102 F-7]

## Operator decisions taken in-flight (review at wake-up)

- **BUDGET PIVOT (operator-approved live, 2026-08-02):** ~9M tokens left on the weekly; full
  remaining program needs ~16M. Approved: close 111 → run 120 as planned → repack the Enfusion
  half (121–126, 17 tickets) into ~3 wide waves (5–6 coders; file-disjoint .c files, T-706 stays
  ahead of all readers) → finalization. **Waves 112–119 (24 Rust tickets) deferred to next week**
  — they serialize on mission_editor.rs regardless of wave width, so widening didn't help there.
  Close markers continue sequentially regardless of packing width.

- **T-650 composition storage (W108):** the ticket's open question — doc-side vs user-scoped API
  rows — was routed DOC-SIDE per the planned owns (store.rs + editor_ops + eden_dock_right). The
  ticket's own analysis favours user-scoped rows, but that path adds an unplanned DB migration +
  API surface; not invented overnight. The doc model is shaped for a mechanical lift (self-
  contained JSON rows, no cross-doc references). If you accept the user-scoped framing, file the
  lift ticket; the data exports cleanly.
- **T-642 ruler open decisions (W108):** labels on-the-line + total in status bar; per-leg slope
  shown (DEM free); Esc clears in-progress then placed, dbl-click ends chain; rulers are session
  overlay state, NOT saved. Each documented in-code.
- **T-638 centre-hold (W107):** camera holds the world point under the map-pane centre across
  dock reflows (Eden slides); Backspace hide/show never slides.

## Mod-wave dispatch notes (wave A/B/C repack)

- **flatten emits:** T-706 opened contracts whose flatten emits belong to the reader tickets.
  T-674 (slot identity: leaderSlotId/tag/callsign/rank/stance) and T-675 (vehicles[]) each get
  flatten.rs as dispatcher-extended owns — they COLLIDE there, so within wave A T-675 dispatches
  only after T-674 reports (mini-serialization inside the wide wave; barrier unchanged). Each
  emit must flip its DeclaredPendingEmit ledger rows to Reaches (the ledger enforces this).
- Other wave-A tickets (T-702 Rust/Opus; T-212, T-654, T-673 Fable) are file-disjoint and
  dispatch immediately. Gate per wave: bash scripts/mod/compile.sh (exit 0) + the platform wave
  gate; compile.sh --selftest once per wave proves the gate can fail.

## REMEDIATION PROGRAM — waves 127–141, AUTHORED AND NOT DISPATCHED

**Filed 2026-08-07 on operator instruction: "set up waves to fix the bugs… but don't start that yet."**
Rows are in `docs/platform/wave_plan.tsv`. **44 tickets, 15 waves, 3-wide, ZERO intra-wave `owns`
collisions — verified mechanically, not by eye.** `cargo xtask slice-collisions` parses the plan.

**Numbering does not force execution order.** 127+ keeps these clear of the **PARKED mod half at
121–126**; whether remediation runs before or after that half is the operator's call.

Filing these rows BEFORE the waves close also gives gate oracle 2 something to corroborate — the
gate's own error text recommends exactly this, and it is why editor waves 100–115 all needed
`TBD_GATE_BASE_CONFIRM`.

| Wave | Theme | Tickets |
|---|---|---|
| **127** | **Silent corruption and false success — run this first** | T-775 T-753 T-773 |
| 128 | The two hand-rolled engines that fail open, and the half-blind census | T-764 T-735 T-774 |
| 129 | Affordances that lie: dead clicks, hollow pins, unnamed refusals | T-754 T-759 T-737 |
| 130 | **Prerequisite T-723** + the marker render lane + the windowing box | T-723 T-760 T-769 |
| 131 | Seams whose absence forced duplicated vocabularies | T-762 T-767 T-746 |
| 132 | What the UI claims about itself | T-771 T-758 T-765 |
| 133 | Failure arms, clearing mutators, formatter corners | T-750 T-766 T-756 |
| 134 | Pins that do not constrain what they narrate | T-736 T-755 T-776 |
| 135 | Modal lifecycle, shared primitives, duplicate schema embeds | T-744 T-751 T-757 |
| 136 | History tails, panel lifecycle, residue | T-745 T-761 T-763 |
| 137 | Overclaims and small honesty fixes | T-741 T-749 T-772 |
| 138 | **Factory infrastructure** and the stale record | T-742 T-752 T-739 |
| 139 | **Prerequisite T-732** + the vacuous-pass tripwire + T-726 | T-732 T-747 T-726 |
| 140 | Work that was blocked until 130/139 landed | T-770 T-768 T-740 |
| 141 | The two scope gaps needing 130's lane | T-748 T-738 |

**Sequencing that is load-bearing, not cosmetic:**
- **T-723 (w130) must precede T-768 (w140)** — T-768 is the connect gesture's pointer half, and
  building it on the un-fixed armed-pointerup path inherits every one of that path's defects.
- **T-732 (w139) must precede T-770 (w140)** — T-770's receipt cannot measure the document until
  the write path returns an acknowledgement.
- **T-760 (w130) must precede T-748 (w141)** — both need the same `draw_order.rs` lane and the same
  `mission_history.rs` rebind tail. T-069 and T-672 independently established that a lane fed only
  from a slice's own owns goes stale after undo/redo/restore, which is why the feed comes first.
- **T-754 (w129) before T-758 (w132)** — fixing the a11y of rows that are still dead clicks makes
  the view honest but no more useful.

**NOT PACKED — operator decisions, not defects with a known fix:**
- **T-743** — paste-at-original lands 20 m off. Three paths: re-scope the row, add an
  exact-coordinate variant, or change `PASTE_NUDGE` and accept the golden-test churn (the nudge is
  byte-parity with the JS oracle). Three artifacts state the wrong fact either way.
- **T-742** — the shared `CARGO_TARGET_DIR` is load-bearing for disk (~4 GB shared vs ~44 GB each).
  Packed at w138 as a fix, but the *approach* is yours.
- **T-752** — clean the five clippy findings and add `--all-targets`, or record deliberately that
  test-target lints are out of scope for the frontend.
- **The player-cap question** from T-694: slot count vs filled players vs the 128 server cap.

## Deferred tickets filed by verifiers

- **T-774** (W119/MAJOR-1) ⚠️ **The census T-703 built to fix "2 of 11 listeners" itself sees
  11 of 13.** `faction_manager.rs:70` and `orbat_manager.rs:249` install window keydowns, both
  mounted from `mission_editor.rs` (one via an `eden_chrome` re-export, which is why a symbol search
  missed it). Harmless today — both bind Escape only, gated on open state — but both sit outside the
  growth tripwire AND the coverage pins, so a key added to either ships undocumented and
  collision-unchecked with everything green. One line per file. **Also: the slice's "39 bindings"
  is unsubstantiated — the verifier's parser counts 32.**
- **T-775** (W119/MAJOR-2, PRE-EXISTING): **focusing a coordinate field and clicking away without
  typing overwrites the document's precise value with a rounded one** (412.37 → 412), with a real
  write and a real undo step — no equality check anywhere in the path. T-700's nudge inherits it
  through `seed()`, so nudging 412.37 commits 413. One-line fix (skip `on_commit` when the parsed
  draft equals the last settled value) kills the rounding, the no-op write and the extra undo step.
- **T-776** (W119/NIT-1..4): three census guards weaker than they read (substring live-state check,
  extractor-uniqueness pin scoped to seven files, a closure reading neither `key()` nor `code()` is
  silently dropped) plus a mislabelled hit row.

> **⚠️ TWO SLICE CLAIMS FROM WAVE 118 WERE REFUTED BY THE VERIFIER. Do not scope remediation off
> either.** (1) T-637's `CONTAINER_H` rationale — see T-769; the smoke would have failed LOUDLY,
> not silently. (2) T-699's report that the repo-wide `class_r_scrub` machinery is hollow — see the
> SCOPE CORRECTION on T-759; `scrub()`'s first pass cuts the test module, and disabling it turns 18
> pins red across 7 files. **The orchestrator relayed both as fact before verification. An agent's
> chat summary is not an artifact — state slice claims as claims until the Fable pass has been through them.**

- **T-769** (W118/MAJOR-1): the dock tree still scrolls in a **fixed 420px box inside a ~958px
  region**, so up to ~538px of the void this ticket absorbed returns on >50-row missions — and the
  shipped comment excusing it is FALSE (the windowing smoke also pins `rendered <= 60`, which an
  h-full scroller violates loudly at both 1080p and the gate's 1440×900). Fix the height and the
  smoke's cap together, and correct the comment.
- **T-770** (W118/MINOR-1): `commit_writes` counts closure calls, not sink acknowledgements —
  `update_slot_loadout` returns `()` and no-ops silently, so the receipt's WARNING arm is unreachable
  in production. T-732's fix should make the write path return an acknowledgement.
- **T-771** (W118/MINOR-2): the Attributes multi-select banner now contradicts the three loadout
  verbs sitting beneath it. **Its text is pinned at `mission_editor.rs:8745` — amend wording and pin together.**
- **T-772** (W118/NIT-1): the ControlsHint close button's hit box shrank ~36→20px with the shared
  `BTN_ICON` change. Fix at the call site, not by widening the recipe the dense rows need.
- **T-773** (W118, pre-existing, verified): **`server_intel.rs` reports "copied" over a clipboard
  write it never confirmed** — the promise is dropped. The fix already exists in-repo: promote
  T-698's `write_clipboard` (awaits the JsFuture, toasts only on the resolve arm) and repoint it.
- **T-764** (W117/MAJOR-1) ⚠️ **Proved by building a rig, not by reading.** The verifier extracted
  T-084's regex engine verbatim into a native harness on a 1 MiB thread (the wasm32 stack) and
  found the **200k budget bounds STEPS, NOT STACK DEPTH** — one native frame per matcher step
  through boxed continuations. `((( … )))` aborts at ~2500 parens, `^^^…` at ~20000,
  `(x+x+)+y` at ~2700 haystack chars: a **wasm trap that kills Leptos and unsaved placements**, not
  the "returns no-match" the slice claimed. NOT reachable from catalogue data (names are 100-150
  chars) — every vector needs a pasted multi-thousand-char pattern. **Fix is a DEPTH CAP, not a
  bigger budget.** Everything else held: 37 correctness cases, 0 wrong answers; 2,000,000 randomised
  glob cases, 0 mismatches; no multibyte panic.
- **T-765** (W117/MINOR-1): glob folds the pattern with `to_ascii_lowercase` and the haystack with
  full `to_lowercase`, so `CAFÉ*` misses `café_x`. Safe direction (a miss, not a false positive).
- **T-766** (W117/MINOR-2): clearing a briefing clears the ROW but not `meta.briefing`, so a
  same-session Export still ships the deleted text. Needs a core mutator that distinguishes
  "set to empty" from "not supplied".
- **T-767** (W117/NIT + T-672): stale `formation` enum prose; and `ConnectionKind` et al are `pub`
  inside a **private** `store` module, so the frontend cannot name them and T-672 carries a
  duplicate vocabulary. A one-line re-export in `doc/mod.rs` retires it.
- **T-768** (W117, disclosed partial): **CONN-START-001's pointer half is deferred behind T-723.**
  A third arming source on the armed-pointerup path inherits its missing button filter, strandable
  `LG::Pending` and absent Esc disarm. Eden also starts at RMB ▸ Connect, so only the final LMB pick
  is missing — after T-723 it changes the caller and nothing else. Also covers CONN-DEL-001 having
  no line to select. **Schedule any connections lane WITH T-760's marker lane** — same rebind tail,
  same `draw_order.rs` edit.
- **T-759** (W116/1) ⚠️ **HOLLOW PINS — the signature defect aimed at the pins themselves.**
  T-696's headline source pins `include_str!` the WHOLE file including their own test module, so
  every positive needle matches the assertion searching for it. **Delete the production usage and
  they all stay green.** The facts are true today; the guarantee is not. The same file already
  contains the correct pattern three tests down.
- **T-760** (W116/2): markers **draw nothing** — no render lane. The slice's refusal was correct
  (the sole rebind tail is outside its owns; the SoA route fails because `slots_bind_soa` is the
  pick bridge). Filed because T-651's identical gap got T-748 and markers had no equivalent.
  Recipe left in the ticket. **Schedule with or after T-672 to share the `draw_order.rs` edit.**
- **T-761** (W116/3): compile findings survive a mission switch — export A, navigate to B, and B's
  panel shows A's build report with subject_ids that resolve to nothing. Client-side routing, same
  wasm instance, no reset on hydrate.
- **T-762** (W116/4): `fly_to` rides the **T-166 smoke hook** in production. Behaviourally sound
  and there is still only one camera mover, but rename the hook and fly-to dies silently — and the
  only pin tying them together is one of T-759's hollow ones.
- **T-763** (W116/5-8): **strikes a false claim from the record** — the T-069 slice's "a pin was
  passing on a decoy" narrative does NOT reproduce; the pin inversions were correct, the story was
  not. Plus: /compiled headers pinned by source scan only, marker attribute selection by bare id,
  and overstated test prose. *An agent's chat summary is not an artifact — this one was relayed
  onward before it was checked.*
- **T-753** (W115/MAJOR-1) ⚠️ **CONFIRMED BY EXPERIMENT.** `FLOW_DEFAULT_*` are `pub const` in BOTH
  `flatten.rs:1491` and `eden_env.rs:186` with **no cross-crate pin**; the only guard restates the
  literals against the frontend's own copy. The verifier edited flatten.rs 600→900, ran the frontend
  suite → **800 passed / 0 failed** while the compiler emits 900s and the editor shows 600. Reverted,
  tree clean. This is the defect class **T-688 was filed to prevent, one layer beneath it**. The fix
  compiles today (the frontend already has the `mission` feature) and was simply never written.
- **T-754** (W115/MAJOR-2): T-688's rows **click through to nothing** — the T-655 router resolves
  slots and vehicles, the only entity-owned settings are zones, so 100% of owned rows fall to a
  text-only toast while still wearing `cursor-pointer`. A dead click dressed as an affordance.
  Widening the router also fixes T-655's own zone-subject blind spot.
- **T-755** (W115/MINOR-2,3 + NIT-1): three pins narrower than the claims they narrate (T-688's
  constructor needle, T-670's scrub check, and **T-692's ControlsHint pin, which checks presence
  rather than gated-subtree position** — the mechanism that makes Backspace hide it).
- **T-756** (W115/MINOR-4 + NIT-3): a non-finite zoom prints a confident `1.00 m/px` instead of the
  em-dash; band-top carry gives 4 sig figs. Both outside the live clamp.
- **T-757** (W115/MINOR-5): `mission.schema.json` (91 KB) is embedded twice in the wasm bundle — same
  path so no drift, but present twice in the dev artifact; `eden_zones.rs:626` still calls it "~40 KB".
- **T-758** (W115/MINOR-6): mission-owned rows are inert focusable buttons. Sequence **after** T-754,
  or the view becomes honest but still unhelpful.
- **T-748** (W114/MAJOR-1) ⚠️ **PLACE-COMMENT-001 does not fully close.** The spec row wants
  "Draggable, copy/paste, layerable, **composable**" + "Comment icon at position". Shipped comments
  have **no map glyph** (deliberately absent from the render SoA — the same property that keeps
  them off the compiled mission) and **cannot be composed** (compositions capture from the
  selection; comments sit in no selection lane). T-651 disclosed the drag/copy narrowings honestly;
  **"composable" was a silent drop**, which is what the HARD GATE bars. Scope accounting, not
  broken code.
- **T-749** (W114/MINOR-1): T-633's settle-only rationale cites live HH:MM drag feedback that does
  not exist (the readout is a `doc_tick` memo, frozen mid-drag), and a test comment claims an
  absence its assertion does not check. Behaviour is correct; the prose is not.
- **T-750** (W114/MINOR-2): the Favourites tab spins on "Resolving…" forever if the registry fetch
  fails — no error signal, no retry. The fetch omission predates T-695; the spinner surface is new.
- **T-751** (W114/NIT-1..4): disabled Select's chevron does not dim · T-215 pin distinctness is
  prose-enforced (nothing pins the favourites arm clone-free) · the comment undo test pins row
  restore but not layer-filing restore · **`compile.rs:37-38` still says keep the twin key lists
  "in lockstep" when four keys now diverge deliberately — someone will "fix" that and either drop
  every comment on save or compile annotations into missions.**
- **T-752** (W112–114, three slice agents + orchestrator): `clippy --all-targets` is red on main and
  **neither the wave gate nor `ci-local-leptos` passes `--all-targets`**, so the debt is invisible to
  both. Nothing is breaking. Counts in the slice reports are inflated by T-742; T-633's five are the
  credible residue. `map-engine-core` already uses `--all-targets --all-features`; the frontend is the outlier.
- **T-743** (W113/F-1,F-4) ⚠️ **NEEDS OPERATOR DECISION**: `paste_at_cursor(None, None)` is NOT
  paste-at-original — `paste_slots`' no-anchor arm unconditionally applies `PASTE_NUDGE` = 20m
  (store.rs:2175-2178, :4225), so **ACTION-PASTE-ORIG-001 does not close as literally named**. The
  nudge is byte-parity with the JS oracle, so it is not a slice-sized fix. Three artifacts state
  the opposite and are wrong: the T-669 registry summary, `gap_analysis.md:265`,
  `interactions_sweep.md:233`.
- **T-744** (W113/F-2): hiding a slot closes its open Attributes modal via the same `None` path
  written for "undone away" — `materialize()` drops hidden rows and `read_attrs` reads through it.
- **T-745** (W113/F-3): `attrs_update_slot` (single) fires the history tail unconditionally — an
  all-`None` call dirties the mission. Latent (no caller reaches it); the `_multi` variant has the guard.
- **T-746** (W113/F-5): `ShapeMirror` has no single-flight between the open-GET and an in-flight
  PATCH; plus the duplicated `is_row_id` and the private `ROW_META` that forces the extra GET.
- **T-747** (W113, near-miss): **a bare `cargo test -p map-engine-core` is a VACUOUS PASS** — the
  `doc` module is feature-gated, so 139 of 600 tests compile and every store.rs pin this wave added
  is silently skipped. The wave gate is sound (it uses `--all-features`), but a hand-run lies.
  Same family as T-742: an ad-hoc cargo invocation misleading an agent.
- **T-735** (W112/MAJOR-1): T-686's import schema checker fails **OPEN** in three forms — `oneOf`
  discards refusals from non-passing branches, schema-form `additionalProperties` is a no-op,
  tuple-form `items` is dropped — and the pin meant to catch that is blind to all three (its
  `is_schema` heuristic only inspects nodes already carrying a supported keyword). Proved in a
  scratch crate. NOT live: every node of the shipped schema is currently examined. One `$defs`
  edit from green tests over unvalidated documents.
- **T-736** (W112/MINOR-1,2): two pins weaker than their own messages — the one-commit blacklist is
  literal-spelling based (`.into_iter()` evades it), and the one-tail pin counts 1 whether the tail
  is inside the loop or outside. Both live implementations are correct; neither pin would catch the
  regression it exists for.
- **T-737** (W112/MINOR-3): import refusals drop `RowError.key`, so two stranded rows are
  indistinguishable. Includes the confirmed-reachable export/import asymmetry (exportable but not
  re-importable when the compat feed is Ready).
- **T-738** (W112/MINOR-4): the help census scrapes only the two `ev.code()` match blocks, so the
  three-plus `ev.key()` Escape listeners — including T-692's own hint close path — are invisible to
  it. **T-703 (W119) must consume this extractor and widen it, not write a third copy.**
- **T-739** (W112/NIT-1): drifted line cites, one stating now-inverted behaviour
  (`gap_analysis.md:244` and `editor_ops.rs:1236` still claim suppress-on-multi). **Registry summary
  line numbers are themselves drifting — treat cites in dispatch briefs as hints, not as load-bearing.**
- **T-740** (W112/NIT-2,3): help prose says "sixteen" bindings, real count is 17; redo row says
  Ctrl+Y but the guard is `ctrl||meta`.
- **T-741** (W112/NIT-4): mixed slot+vehicle selections overclaim ("N entities selected" /
  "every selected entity") because `attrs_multi_ids` filters vehicles out — reachable via the
  Ctrl+A this wave shipped.
- **T-742** (W112, from the T-649 slice agent, NOT the verifier): concurrent slice worktrees share
  `CARGO_TARGET_DIR` and can execute each other's test binary — T-649 observed a failure in a
  T-686 test that does not exist in its tree, and the reported line moved between runs as the
  sibling edited. The private-dir slice gate is unaffected; ad-hoc `cargo test` is not.
  **NEEDS OPERATOR DECISION** — the shared dir is load-bearing for disk.
- **T-711** (W101/F-1,N-2): §N3 LOD contract superseded by T-639; t152_7 stale call convention. cursor-docs.
- **T-712** (W101/F-2,F-3,N-1): contour constant annotations honest; sample default view in acceptance.
- **T-707** (W100/F-2): wave.sh test-split comment transposes its own measurement — one-line fix.
- **T-708** (W100/F-3): capture shot hang-fallback 25s→130s latency regression — send_with_timeout.
- **T-709** (W100/F-4): capture zoomsweep lost the per-zoom console error tap (hides __editorCamSet).
- **T-710** (W100/F-5): capture port pure fns (ztag/canvas_path/crop math/step parse) unpinned by tests.

## Incidents

- **W100 gate red on planning-session files, not T-661.** First full wave gate (base c2dac546)
  failed `no-node` (tools/editor-capture/cdp2.mjs, zoomsweep.mjs) and `no-shell` (crop.sh,
  run_shot_gpu.sh unlisted). The capture harness was committed 2026-08-01 (d1df67fb, d9fe8243)
  without running the language gates — they are repo-wide scans, so the red predates the wave and
  would fail on 852f17a4 itself. All 24 other steps PASS; T-661's split is clean. Triage: port the
  two .mjs onto tbd-tools' existing cdp.rs (same precedent as smokes.rs, itself a port of 19 Node
  drivers), absorb or inventory the two .sh, keep the README's KB-002 knowledge. Remediation
  commit lands inside wave 100's gate range so the re-run gate and the wave verifier cover it.
