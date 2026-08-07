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
Oracle 2 cannot corroborate markers 83+ (plan rows carry the 100+ labels), so wave gates after 100
pass `TBD_GATE_BASE_CONFIRM=<prev close sha>` after verifying the sha by hand. This is the
documented hatch, not a bypass: membership is confirmed by the operator-side reading the tooling
demands.

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
| 113 | assign at close | T-082 T-669 T-694 | — | — | pending |
| 114 | assign at close | T-633 T-651 T-695 | — | — | pending |
| 115 | assign at close | T-634 T-670 T-688 | — | — | pending |
| 116 | assign at close | T-069 T-690 T-696 | — | — | pending |
| 117 | assign at close | T-084 T-671 T-672 | — | — | pending |
| 118 | assign at close | T-637 T-698 T-699 | — | — | pending |
| 119 | assign at close | T-697 T-700 T-703 | — | — | pending |
| 120 | 95 | T-701 T-706 | PASS (run 3; run 1 red on the ledger tripwire firing as designed, run 2 = fixup) | 0B/4M/5m/4N → all hardened pre-close | SHIPPED — per-entity hide + the one-pass 1.3 contract; ledger learned DeclaredPendingEmit; 9 reader-ambiguities + 3 prose residues fixed BEFORE any reader dispatched; unread gate live at 53 fields |
| 121 | assign at close | T-702 T-212 T-654 | — | — | pending |
| 122 | assign at close | T-673 T-674 T-675 | — | — | pending |
| 123 | assign at close | T-676 T-677 T-678 | — | — | pending |
| 124 | assign at close | T-679 T-680 T-681 | — | — | pending |
| 125 | assign at close | T-682 T-684 T-685 | — | — | pending |
| 126 | assign at close | T-689 T-705 | — | — | pending |

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

## Deferred tickets filed by verifiers

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
