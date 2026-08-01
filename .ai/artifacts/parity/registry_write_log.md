# Registry write log — the editor-UI program is filed and the factory can start

**2026-08-01.** `.ai/tickets/registry.json` and `docs/platform/wave_plan.tsv` now carry the
editor-UI program (43 factory-dispatchable tickets, 19 waves) and the workbench program (10
tickets, deliberately not dispatchable). `ticket check` is green and `ticket sync` has regenerated
every view.

Sources applied, in precedence order:
[`owns_parity.md`](owns_parity.md) §5 (the authoritative 43-row packing) ->
[`owns_correction_chrome3.md`](owns_correction_chrome3.md) (supersedes the T-636/T-637/T-638 `owns`
rows and moves T-637 to its own wave) -> [`../editor_chrome_direction.md`](../editor_chrome_direction.md)
(rescopes T-634/636/637, absorbs T-632, adds the state-vocabulary ticket).

---

## 1. Placeholder -> id mapping

Every placeholder name in the packing now has a real `T-` id. Ids `T-631`…`T-660` went to the
sibling's drafted set as drafted; the twelve genuinely new rows continue from **T-661**.

| Packing name | Filed as | Wave | Why this id |
|---|---|---|---|
| `T-630.5` — the `eden_chrome.rs` split | **T-661** | 0 | A dotted `T-630.5` would have made it a *slice of* shipped T-630 (a rate-limiter fix), which it is not. Fresh top-level id. |
| `W1-UNBLOCK` — RMB + Backspace | **T-662** | 1 | — |
| `N9` — dead `view_distance`/`thermals` DTO fields | **T-663** | 1 | — |
| `P-3` — right-click context menu | **T-664** | 2 | — |
| `N3` — editor layer visibility + transform lock | **T-665** | 2 | — |
| `P-6` — outliner layer authoring | **T-666** | 4 | — |
| `T-641b` — scale bar + grid labels | **T-667** | 6 | `T-641a`/`T-641b` are not valid ids (`^T-[0-9]{3}(\.[0-9]+)*$`). The render half keeps **T-641**; the furniture half gets a fresh id. |
| *(new)* — one state vocabulary across the chrome | **T-668** | 10 | New ticket required by `editor_chrome_direction.md`; see §5.1. |
| `P-10` — `Ctrl+X` / `Ctrl+Shift+V` | **T-669** | 13 | — |
| `P-11` — scale readout | **T-670** | 15 | — |
| `N1` — briefing text + thumbnail | **T-671** | 17 | — |
| `T-079d` — connection graph | **T-672** | 17 | `T-079a` keeps **T-079**; the graph quarter needs its own id. |

Promoted existing rows — **ids kept, status/summary/owns rewritten, no duplicates created**:

| Packing name | Filed as | Wave |
|---|---|---|
| `T-069 ⊕ T-213` — markers | **T-069** (T-213 cancelled into it) | 16 |
| `T-076a` — vehicle crew UI | **T-076** | 3 |
| `T-079a` — triggers, editor half | **T-079** | 9 |
| `T-082` — attributes modal fields | **T-082** | 13 |
| `T-084` — classname / prefix search | **T-084** | 17 |

`T-641a` -> **T-641** (spot heights). `T-641b` -> **T-667**.

## 2. Counts

### Program 1 — `eden`, factory-dispatchable — **43 rows**

| Status | Executor | n |
|---|---|---|
| `ready` | `claude-code` | 1 (T-661, the wave-0 split) |
| `queued` | `claude-code` | 42 |

Composition: 26 drafted ids (T-631, T-633…T-651, T-655…T-660 — T-632/652/653/654 are not in the
packing) + 12 new ids (T-661…T-672) + 5 promoted existing ids (T-069, T-076, T-079, T-082, T-084)
= **43**, matching `owns_parity.md` §5.1 exactly. T-632's slot in the count is taken by T-668.

### Program 2 — workbench — **10 rows**, all `status: queued`, `executor: workbench`

| Id | Title | Attribute ids |
|---|---|---|
| T-673 | Marker style and Area markers — the `$defs/marker` widening | 6 |
| T-674 | T-216 follow-on: slot identity reaches the wire | 4 (`OBJ-UNIT-NAME` folded in per §6's own gate) |
| T-675 | Vehicle roster reaches the game — the compile half of T-076 | 0 (closes the sixth T-216 drop) |
| T-676 | Trigger activation and effects — the Enfusion runtime | 12 |
| T-677 | Waypoints — blocked on AI units existing at all | 9 (+6 interaction ids) |
| T-678 | Group AI state: combat mode, behaviour, formation, speed | 4 |
| T-679 | Placement scatter: radius and area shape | 3 |
| T-680 | Vehicle states: lock, fuel, ammo | 3 |
| T-681 | Entity states: health, allow-damage, show-model, size, stamina | 5 |
| T-682 | Environment readers: fog, wind, view distance | 3 |

**6+4+0+12+9+4+3+3+5+3 = 49** — the exact count `attributes_sweep.md` §6 gives for
`executor: workbench`. No `owns`, no wave rows, and the wave-plan checker asserts none of them is in
the plan.

Ten rather than the ~7 the brief expected: §6's table has nine id-bearing sub-programs and I merged
only the one merge §6 itself directs (`OBJ-UNIT-NAME` "rides the T-216 follow-on schema slice"),
then added T-675 for the T-076b split. Merging further (vehicle states + entity states; waypoints +
group AI) is defensible and would give 8, but each pair has a *different* stated gate in §6, and
collapsing them would hide one gate behind another.

### Non-dispatchable `eden` rows — 4, no `owns`, no wave rows

| Id | Status | Executor | Why |
|---|---|---|---|
| T-632 | `cancelled` | claude-code | Absorbed by T-637 (`editor_chrome_direction.md`): the clipped MANAGE tab is a symptom of the dock width, not an independent defect. Row kept so the id is not reused. |
| T-652 | `deferred` | claude-code | Rocks — operator decision, "doesn't matter if they're rocks". |
| T-653 | `idea` | **cursor-docs** | See §5.3 — the two source artifacts disagree on disposition, and the deliverable is documentation. |
| T-654 | `idea` | **workbench** | Conditional inclusion — a design ticket; `owns_and_waves.md` §6 puts it in program 2 the moment it gains an `owns`. |

### Superseded existing rows — 6, all `cancelled` with a pointer

`T-072` -> T-647 · `T-073` -> T-648 · `T-075` -> T-648 · `T-077` -> T-647 · `T-078` -> T-650 ·
`T-213` -> T-069. `framework_synthesis.md` §D.6 explicitly directs *"supersede them in the registry
rather than leaving duplicates"*; the house pattern is T-074's cancellation note.

### Registry totals after the write

```
656 tickets (was 604) · next_id 683 · highest id T-682
status   : shipped 520 · queued 55 · idea 42 · cancelled 20 · deferred 17 · ready 2
executor : claude-code 583 · cursor-docs 46 · workbench 20 · human 4
```

## 3. The wave plan

43 rows appended under a commented block header. **Plan labels are 100-118, not 0-18**: labels
0-11, 43-68, 76-82 and 99 already belong to the platform factory in the same file, and column 1 must
stay a bare integer (`check_wave_labels()` in `xtask/src/slice_collisions.rs` hard-fails otherwise).
The offset is documented in the file. Program wave *N* = plan label *N* + 100.

| Plan | Program wave | n | Tickets |
|---|---|---|---|
| 100 | 0 | 1 | T-661 — **alone by construction** |
| 101 | 1 | 3 | T-662 · T-663 · T-639 |
| 102 | 2 | 3 | T-664 · T-665 · T-640 |
| 103 | 3 | 3 | T-631 · T-076 · T-641 |
| 104 | 4 | 3 | T-635 · T-666 · T-656 |
| 105 | 5 | 2 | T-636 · T-646 |
| 106 | 6 | 2 | T-647 · T-667 |
| 107 | 7 | 3 | T-638 · T-659 · T-657 |
| 108 | 8 | 3 | T-642 · T-650 · T-658 |
| 109 | 9 | 3 | T-643 · T-079 · T-660 |
| 110 | 10 | 3 | T-648 · T-644 · **T-668** |
| 111 | 11 | 2 | T-655 · T-645 |
| 112 | 12 | 1 | T-649 *(T-632 was its partner; absorbed)* |
| 113 | 13 | 2 | T-669 · T-082 |
| 114 | 14 | 2 | T-651 · T-633 |
| 115 | 15 | 2 | T-670 · T-634 |
| 116 | 16 | 1 | T-069 *(T-637 evicted by the chrome3 correction)* |
| 117 | 17 | 3 | T-672 · T-671 · T-084 |
| 118 | 18 | 1 | **T-637 — alone** |

19 waves, 43 tickets, mean 2.26 — matching `owns_correction_chrome3.md` exactly.

## 4. Verification

### `./scripts/ticket check` (via `distrobox-host-exec`; `xtask` is a host binary)

```
check OK
```

### `./scripts/ticket sync`

```
sync complete
```
Regenerated: `CLAUDE.md` status block, `docs/TICKET_REGISTRY.md`, `docs/TICKET_LEAD.md`,
`docs/TICKET_DEV_QUEUE.md`, `docs/TICKET_MOD_QUEUE.md` (all 10 workbench rows land here, which is
the correct destination for them), `docs/TICKET_BRAINSTORM.md`,
`docs/specs/Mission_Creator_Architecture/ROADMAP.md`.

### The collision check — written for this and run

The important verification is the one the whole exercise exists to prevent: **two tickets in one
wave editing the same file.** No repo tool does exactly this (`cargo xtask slice-collisions` packs
and reports, and `--repack` would have *destroyed* this hand-derived plan), so a checker was written
and run. It lives in the session scratchpad, deliberately **not** in the repo — `verify-no-python`
is a hard gate (`Makefile:466`) and the checker is Python; `./scripts/verify-no-python.sh` re-run
after the write returns `verify-no-python: PASS`.

It asserts: 4 tab-separated columns per row · every plan id resolves in the registry · **no two
tickets in the same wave share an `owns` path** · no duplicate ids · no `workbench`/`human`/`ci`
executor has a plan row · every dispatchable `eden` row has a plan row. Scope A is the new block;
Scope B diffs the whole file against the pre-write baseline so nothing can be hidden in the
platform factory's pre-existing state.

```
==============================================================================
SCOPE A — the editor-UI block (plan labels 100-118)
==============================================================================
1. column count      : 43 rows in the block, all 4 tab-separated columns  (file total 466 rows, 0 malformed)
2. ids in registry   : 43/43 resolve
3. owns collisions   : 19 waves, 33 pairs compared, 0 COLLIDING
4. duplicate ids     : none
5. factory safety    : block executors = ['claude-code'] ; 10 new workbench tickets, 0 of them in the plan
6. coverage          : 44 dispatchable eden rows, 1 without a plan row -> T-146

==============================================================================
SCOPE B — whole file vs the pre-write baseline (proving nothing new was introduced)
==============================================================================
   colliding pairs: baseline 355 -> now 355  (added by this write: 0)
   duplicate ids  : baseline 9 -> now 9  (added: none)
   non-dispatchable executors in the plan: baseline ['T-205', 'T-206', 'T-404'] -> now ['T-205', 'T-206', 'T-404']

PLAN OK — editor-UI block has zero intra-wave owns collisions, every id resolves,
          every row is executor claude-code, and the write added nothing to the baseline.
```

**Read Scope B honestly.** The file already contained 355 colliding pairs, 9 duplicated ids and
3 `workbench` rows in the plan before this write — almost all in label 0 (shipped rows, parked
there by `--repack`) and label 99 (the unscheduled-backlog bucket). Those are **pre-existing
platform-factory state, not introduced here, and not fixed here**. The delta this write contributes
to every one of those numbers is **zero**.

## 5. Decisions the artifacts did not settle

### 5.1 Where the new state-vocabulary ticket goes — **wave 10, and it is forced**

`editor_chrome_direction.md` requires a new ticket and says it *"should land early so every later
wave builds against the finished vocabulary rather than retrofitting it."* It gives it no wave and
no `owns`. Three constraints collide:

- `owns_correction_chrome3.md` fixes the program at **19 waves**, so a 20th wave was not available.
- The packing's waves are 2-3 tickets; waves 1-4 and 7-9 are already at 3.
- Its `owns` is wide by construction — "one state language, whole chrome" changes classes in every
  module that renders a control.

The earliest wave with a free slot **and** no `eden_*` / `ui.rs` claimant is **wave 10**
(T-648 owns `mission_editor`+`select_tool`+`editor_ops`; T-644 owns `los_tool`+`engine`+`dem/sample`).
Waves 5 and 6 have free slots but both have an `eden_toolbelt.rs` claimant. So wave 10 is not a
preference, it is the only answer that keeps 19 waves and zero collisions — and it is recorded in
the ticket body that **T-636, T-638 and T-667 land before it and will need a retrofit pass.** That
is the honest cost and the operator should see it.

`owns` assigned: `eden_layout` · `eden_top_strip` · `eden_toolbelt` · `eden_dock_left` ·
`eden_dock_right` · `eden_tree` · `eden_settings` · `eden_zones` · `eden_vehicles_panel` · `ui.rs`
(10 files). I did **not** narrow it to the "hot" chrome modules: the ticket's entire value is that
the language is applied *everywhere*, and a narrowed `owns` would have been a silent deferral.

### 5.2 T-632 is `cancelled`, not deleted

`editor_chrome_direction.md` says "absorbed". Deleting the row would free the id for reuse and lose
the reasoning; leaving it `queued` would put a duplicate in the factory's path. `cancelled` with a
summary naming T-637 is the house pattern (T-074).

### 5.3 T-653 — the two artifacts disagree, and I did not pick a winner

`owns_and_waves.md` §2 Group G/H calls it **superseded** (the capture scripts are Python and
`verify-no-python` is a hard gate, so they cannot be promoted into `tools/`; the findings belong in
`docs/website/EDITOR_GATE_RUNBOOK.md`, which makes it a `cursor-docs` ticket).
`framework_synthesis.md` §D.4 #11 says **leave as `idea`**.

These are reconcilable rather than contradictory, so I filed the reconciliation rather than
escalating: `status: idea` (both the draft and §D.4 agree on that), `executor: cursor-docs` (the
deliverable is documentation, per the no-Python evidence), and the three findings written into the
summary verbatim so they survive regardless of what happens to the ticket. It carries no `owns` and
takes no wave slot under either reading. **Flagged here because it is the one place I merged two
sources rather than following one.**

### 5.4 T-651's `owns` — an intra-document disagreement, resolved by the designated authority

`owns_parity.md` §2.1 (prose) gives T-651 `…eden_dock_right.rs; …mission_editor.rs`; its own
Appendix and §5.3 packing table both give `…mission_editor.rs; …context_menu.rs`. The brief names
**§5 as authoritative**, §5.3 and the Appendix agree with each other, and the choice does not affect
wave 14's disjointness either way. Filed with `context_menu.rs`.

### 5.5 Other calls

- **`order`** — 3800-4220 in wave order for program 1, 4300-4390 for program 2, 4230-4240 for the
  non-dispatchable rows. The five promoted tickets were **renumbered** out of their old 690-840
  slots so the queue views read in program order; T-213 keeps its old order (cancelled).
- **T-069's `depends_on`** was `["T-151"]`. T-151 is `shipped`, so that edge is discharged; replaced
  with `["T-661"]`, the live gate.
- **`notes`** on every new row carries its program wave and plan-row label, so a developer can find
  the schedule from the ticket.
- **Promoted rows keep** any pre-existing `spec` / `branch` / `priority`. T-069 therefore still
  points at `t069_markers_on_map.md` — **whose premise is confirmed dead.** The summary says so in
  capitals; the spec rewrite is the first task in the ticket.

## 6. Corrections carried into the ticket bodies

Every one of these is a claim that was true once, or true elsewhere, and would have been inherited:

| Ticket | Correction |
|---|---|
| T-641 | The draft's *"we render zero height annotations"* is **false** — `dem/peaks.rs`, `world_assets/labels.rs` and the live `Height labels` layer toggle all exist. `HEIGHT_LABEL_MIN_ZOOM = -2.0` and the default zoom is `-2`: probably a band defect, not greenfield. |
| T-635 | The draft's `Ctrl+Alt+D` HUD toggle **does not exist**; it died with React's `FpsCounter.tsx` at T-159.29.3. The ticket must create the binding. |
| T-656 | The *"both validators are silently dead, neither community knows"* framing is withdrawn — the WOG `/g` claim failed verification. Only the FNF leg holds; it is a one-framework argument now. |
| T-666 | `attributes_sweep.md:212` grades `LYR-NAME` as `match`. Four of the five layer mutators have **zero callers**; T-037 shipped the core, not the UI. |
| T-648 | `GRID_CELL_M` is a spatial-index cell size, **not** a snap step, and all 37 word-boundary `snap` hits are `let snap = read_snapshot()`. Nothing snap-related exists. |
| T-663 | `ENV-SETTINGS-002` is stale: T-193 *removed* view distance and thermals rather than wiring them. |
| T-069 / T-213 | T-213's spec cites a `state/schema.ts` deleted at T-159.29.3; T-069's spec premise is confirmed dead. |
| T-674 / T-675 | The T-216 ledger: six author-facing values are silently dropped at compile, and `make verify-t180` stayed green because **not one of its 22 tests checked**. |
| T-646 | Folding `RIGHT-SUBMODE-001` in **revives a deliberately cancelled ticket** (T-074). Stated in the body rather than done silently. |

## 7. Residuals for the operator

1. **T-146** (Asset Browser Data Wiring) is `queued`/`claude-code`/`eden` and has **no plan row** —
   pre-existing, and the only such row. The interactions sweep maps `RIGHT-MODE-001` to it and it
   overlaps T-646/T-084's territory. It needs an `owns` and a wave, or a supersede.
2. **T-079c (systems/modules) was NOT filed**, per the brief. The `SYS` family declares **no ids at
   all** in `attributes.md:223-225`; its entire derivable content is one `PaletteKind` enum variant.
   That is a lower bound with no upper bound — unsized, not small — and it would have consumed an
   `eden_dock_right.rs` wave slot that four real tickets are queued for. It needs its own catalogue
   pass before it can be a ticket.
3. **Two single-agent waves at the tail** (16 and 18) are the honest cost of the chrome direction.
   `owns_correction_chrome3.md` records this as the clearest evidence yet for the *second* split
   (`editor_ops.rs` then `mission_editor.rs`, floor 17 -> ~8), which the operator has declined on
   risk grounds. Unchanged; recorded so the trade stays visible.
4. **Product calls still open inside filed tickets**, each stated in its own summary: where the
   briefing is authored (T-671 — the library-dossier version is collision-free against the entire
   program); where composition storage lives (T-650 — in-document vs server-side); whether the
   validation panel ships earlier than wave 11 (T-655 — the file graph permits wave 3 and it costs
   zero waves either way); and what fills the new bottom-right primary-action slot (T-636).
5. **The marker render path is the `medium`-graded claim most worth checking first.** If markers can
   ride the slot SoA that `editor_ops.rs` already uploads via `core.materialize()`, T-069 drops both
   `map-engine-render` paths. Stated in the ticket.
