# Coverage audit — every research artifact against the ticket registry

**2026-08-01.** The check that should have existed. Coverage in this program was repeatedly verified
*from an authority document to tickets* and never *from every artifact to tickets*; three artifacts
were found at zero coverage by the operator rather than by a check. This file sweeps **every** file
under `.ai/artifacts/` that feeds the editor-UI program, files the nine ranked
`framework_synthesis.md` items that had no ticket, and gives an explicit disposition — file / fold /
rule out — for everything else that is unreached.

**Headline.** The framework research is now filed. **The two artifacts the brief describes as "now
swept" are swept but NOT filed**: `parity/screenshots_sweep.md` and `parity/3den_sweep.md` were
written **after** the registry write (`git log`: registry write `6a84ba9f`, then `bf5f276b`
3den sweep, then `fbd90be2` screenshots sweep; mtimes 22:51 → 23:16 → 23:23), and between them they
propose **20 new slices and 34 existing-ticket scope extensions, none of which reached the
registry**. A sweep is not a filing. That is the same failure one level up.

---

# 1. The nine synthesis items — what was filed

`framework_synthesis.md` Part C ranks 16 items. Seven were already filed (B1→T-656, B2→T-659,
B3→T-651, B4→T-671 *(partial — see §3.1)*, B6→T-650, B8→T-654, U5→T-659 *(mis-attributed — see
§3.1)*). The nine below had no ticket. All nine are now filed.

| Item | Id | Status | Executor | Wave | Why that executor |
|---|---|---|---|---|---|
| **B5** objectives as typed per-side entities | **T-212** *(extended, id kept)* | queued | `workbench` | — | Whichever shape wins is a `mission.schema.json` change against a root that is `additionalProperties: false`, plus Enfusion work in `apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/`. `framework_synthesis.md` D.5 puts B5 in the mission-content program for exactly this reason. |
| **U3** loadout templates + inheritance | **T-687** | queued | `workbench` | — | `faction-library.schema.json` is `additionalProperties: false` at the root (:7) and on `$defs/role` (:46); a `default` block plus a per-role template reference is a `packages/tbd-schema/` widening plus `make schema-codegen`. **The alternative is recorded in the body** — see §3.3; `workbench` is the executor that cannot mis-dispatch. |
| **U1** zone volume + force counts + starting owner | **T-685** | queued | `workbench` | — | `$defs/zoneRules` is `additionalProperties: false` and its own schema description states why: Enfusion's `JsonLoadContext` makes an undeclared key *invisible*, not rejected. Every new key needs a field in `TBD_MissionLoader` / `TBD_ObjectiveRules.c` and a resolve branch in `TBD_ObjectiveRegistry.c`. |
| **U6** aircraft exemption on boundary zones | **T-689** | queued | `workbench` | — | Same `$defs`, same closed vocabulary, same reader. `depends_on: T-685` — shipping the pair costs one widening instead of two. |
| **B9** mission parameters | **T-684** | **idea** | `workbench` | — | Document-shape change + a mod-side launch-time reader. Same shape and reason as T-654. Ranked **16 of 16**; filed as `idea` deliberately so the research is not lost and no one queues it. |
| **B7** default-override instrumentation | **T-683** | queued | `claude-code` | 5 (plan 105) | Read-only aggregation over `mission_versions.json_payload` in `apps/website/api/` — the Rust workspace, not `packages/` or `apps/mod/`. Reads the schema's declared `default`s; **must not widen the schema**. |
| **U2** loadout import | **T-686** | queued | `claude-code` | 12 (plan 112) | SPA-only. No new format: the document is the already-shipped `loadout-export.schema.json` v2. |
| **U4** aggregated settings view | **T-688** | queued | `claude-code` | 15 (plan 115) | Read-only aggregation over the doc; the schema already declares every `default`. |
| **U7** compile diagnostics | **T-690** | queued | `claude-code` | 16 (plan 116) | `crates/map-engine-core/src/mission/compile.rs` + `mission_commands.rs`. Rust workspace. |

Split: **5 workbench · 4 claude-code.** Every `claude-code` row got a `wave_plan.tsv` row with
`owns`; no `workbench` row did.

## 1.1 B5 — T-212 extended, not superseded, and two corrections to the old row

T-212's previous body was written before the research and carried a premise that is **half wrong**.
Checked against the live tree today:

- The container is `crates/map-engine-core/src/doc/store.rs`**:351** and the emit is
  `crates/map-engine-core/src/mission/compile.rs`**:228** — not `:174` / `:91` as the old summary
  said. Restating a stale line number is the exact failure this program has on record eight times.
- More importantly, **:228 sits inside `compile_payload`, the editor superset.** `objectives` is
  **not** a top-level property of `packages/tbd-schema/schema/mission.schema.json` — the fourteen are
  `schemaVersion / meta / environment / factions / orbat / slots / radioPlan / zones / entities /
  layers / flow / winConditions / briefings / settings`, and the root is `additionalProperties:
  false`. `crates/map-engine-core/src/mission/flatten.rs` contains **zero** occurrences of the word.
  The container does not reach the mod at all.
- What *does* work is objectives-as-zones: `TBD_ObjectiveRegistry.c:36-42` keys on `zone.type` in
  `{objective_capture, objective_destroy, objective_hold_until}` and drives `winConditions.endOn`.

So B5 is a decision about which of two shapes wins, not a UI hung on the dead one. The ticket carries
WOG's `WMT_Task_Point` spine with its observed distributions (`wog.md:382-402`), FNF v4's per-side
framing as **one entity with two framings** (`fnf_v4.md:943` steal-list #6), v4's four documented
pairing defects (`fnf_v4.md §14.4`), and `_Condition = true` in 165/166 as the reason the scripting
hatch goes behind advanced disclosure.

**The caveat is carried, not laundered.** `wog.md:379-381` and `:403-406` mark the *semantics*
`INFERRED:` because `wmt_main` — the addon supplying every `WMT_*` module — is **entirely absent from
the corpus** (`wog.md:88-95`: not in `extracted/`, not in `pbo/`). Parameter names and observed values
are hard evidence; the reading of what they do is inference and is stated in the ticket as
inadmissible in an acceptance criterion.

**Row changes made, stated as required:** `status` idea→queued · `executor` claude-code→**workbench**
· `stream` mission-creator→shared-schema · `targets` `[website]`→`[shared, mod]` · `order` 2300→4430
(renumbered into the program-2 range, the same treatment the write log gave the five promoted rows) ·
`surfaces` and `impact` widened · `depends_on: [T-685]` added. **T-212's pre-existing
`wave_plan.tsv` row at plan label 9 was deleted** — it is now a `workbench` row and a workbench row
must not appear in the plan. The row's `owns` (`store.rs; compile.rs`) collided with nothing, so
removing it changed the whole-file colliding-pair count by zero (355 → 355).

**Residual, deliberately not filed:** the SPA authoring UI for objectives cannot be scoped until the
shape is decided, and pre-filing it would be the duplicate the brief forbids. Split it as a
`claude-code` follow-on with its own `owns` once T-212 lands — the house pattern is T-076/T-675 and
T-079/T-676.

## 1.2 U3 — the executor is a product call, and it is recorded in the ticket

The synthesis disagrees with itself here and the disagreement is load-bearing. C.2 U3 lists only
`arsenal_rules.rs` and "faction library" as the surfaces and, unlike U1, carries **no** "the mod must
read it" note. D.5 then sweeps U3 into the mission-content program with a blanket "every one of these
changes `mission.schema.json`". Checked:

- `packages/tbd-schema/schema/mission-editor-payload.schema.json` carries **zero**
  `"additionalProperties": false` — the editor payload constrains nothing.
- `mission.schema.json` `$defs/slot.loadout` is closed and flat.
- OFCRA's own model **compiles the inheritance away** — "compiles into `mission.sqm` and leaves zero
  runtime footprint; the authoring artefact is deleted before the mission ships"
  (`ofcra_omtk.md:1806`).

So if templates live in the editor payload and the deep merge resolves **at compile** into the
existing flat `slot.loadout`, nothing under `packages/` changes, no mod work exists, and U3 is
`claude-code`. That is the better-evidenced architecture. It is filed `workbench` because that is the
executor that cannot mis-dispatch, and **the whole argument is in the ticket body** so the operator
can flip it in one edit. Same disclosure pattern as T-650 (composition storage) and T-671 (where the
briefing is authored).

One checkable find worth keeping: **the remove-inherited verb is already representable and nobody has
used it.** `faction-library.schema.json:42` declares `"slot": { "type": ["string", "null"] }`, so
every `wear` / `equipment` entry and every weapon `optic` / `magazine` can already carry an explicit
null distinct from absent — OFCRA's `explicit-null 579×` mechanism (`ofcra_omtk.md:506, :733, :1374`)
needs no new type.

---

# 2. Artifact-by-artifact coverage

`reached` = the finding is inside a filed ticket's scope. `scope-note only` = a ticket exists for the
surface but the artifact's specific contribution is not in any ticket body.

| Artifact | Lines | What it contains | Distinct findings | Reached | Not reached |
|---|---:|---|---|---|---|
| `frameworks/README.md` | 204 | Provenance, evidence weighting, the `INFERRED:` discipline, the mixed-corpus caveat | Method, not findings | n/a — consumed by `framework_synthesis.md` §"How to read the citations" and §A.2 | — (one stale line: its OFCRA "skips §4→§6" note is wrong, `ofcra_omtk.md` has a §5 at :432) |
| `frameworks/fnf_v3.md` | 1,198 | FNF v3.6.9 on its own terms, 14 sections, zero `INFERRED:` markers | Feeds synthesis A.1–A.14, B, C | **All C-ranked items now filed** (B3←§13.2, B8←§3 step 3, B2←§3 step 12) | Its §13.1 "copy semantic layers wholesale" recommendation is **deliberately rejected** in T-654 — a filed rejection, not a gap |
| `frameworks/fnf_v4.md` | 945 | FNF v4.7.0 + the v3→v4 delta | Feeds A.1–A.14, B.1–B.6, C | B5←§7/§14.4 (T-212) · U4←:906 (T-688) · U6←:469/:477 (T-689) · U7←:713 (T-690) · B6←§13.3 (T-650) | — |
| `frameworks/fnf_tooling.md` | 923 | MissionAnalyzer (27 checks, 14 live) + DTAS | Feeds B1, B2, B9, C.4 | B1←§3.2 (T-656) + the group (T-655/657/658/659/660) · B9←§2.5 (T-684) | — |
| `frameworks/ofcra_omtk.md` | 1,949 | OFCRA `omtk` v2.13.7, largest analysis in the corpus | Feeds A.1–A.14; the loadout §5 is 369 lines | U3←§5.2/§5.7/§13.2 (T-687) · B9←§10.1 (T-684) | §12 (referee ladder, lonewolf detector, radio-theft blocking) — **explicitly out of scope**, synthesis §"What this does not cover" |
| `frameworks/wog.md` | 1,084 | WOG, reverse-engineered from 50 addons + 171 missions | Feeds A.1–A.14; the richest parameter evidence | B5/U1←§7 (T-212/T-685) · U2←:814 (T-686) · B7←:603/:1078 (T-683) · U5←§4b *(see §3.1)* | — |
| **`framework_synthesis.md`** | 1,286 | The 16-item ranked build list + Part D feedback | **16 ranked items** | **16 / 16** — 7 already filed, **9 filed here** | — |
| `eden_screenshots/README.md` + `batch01`–`08` | 4,395 | 75 operator screenshots of Eden, 8 batch analyses + cross-batch reconciliation | Consumed wholesale by `parity/screenshots_sweep.md` (its §1.1 precedence list) | Via the sweep — see next row | Via the sweep — see next row |
| **`parity/screenshots_sweep.md`** | 1,059 | The 75-screenshot corpus triaged into **374 rows** | 374 (na 148 · missing 132 · partial 59 · match 32 · UNKNOWN 3) | 148 `na` closed with reasons · 32 `match` · **143 open rows map onto 34 already-filed tickets** — but as **scope-note only**, none applied to a ticket body | **19 rows / 7 proposed factory slices (NEW-F1…F7) — UNFILED** · **5 rows / 1 workbench slice (NEW-W1) — UNFILED** · 10 deliberately unowned · 3 UNKNOWN |
| `3den_enhanced_feature_catalogue.md` | 869 | The 3den Enhanced mod catalogue | Consumed wholesale by `parity/3den_sweep.md` | Via the sweep | Via the sweep |
| **`parity/3den_sweep.md`** | 826 | The catalogue triaged into **245 rows** | 245 (no 134 · want 60 · maybe 40 · have 11) | 134 `no` + 11 `have` closed with reasons · **21 `want` inside T-645's scope** · §4.3's two rows (U2, U3) **filed here as T-686/T-687** | **11 factory slices (E1–E11) + 1 workbench (E12) — UNFILED** · **9 existing-ticket scope extensions — UNAPPLIED** |
| `parity/attributes_sweep.md` | 540 | All 93 `ATTR-FIELD-*` ids triaged | 93 | **93 / 93.** 22 `na` closed · 22 factory ids → T-069/T-651/T-665/T-671/T-082/T-650/T-663 · **49 workbench ids → T-673…T-682** (count matches the write log exactly) | — |
| `parity/interactions_sweep.md` | 613 | All 83 `interactions.md` ids + the full keyboard map | 83 + 5 key collisions | **83 / 83.** 41 absorbed by existing tickets · 11 shipped · 9 `na` · 6 no owner · 16 → P-1…P-12 → T-647/T-648/T-649/T-662/T-664/T-666/T-669/T-670/T-672 | The 5 keyboard collisions are named in ticket bodies but **no test asserts them** — that is 3den_sweep's E10 |
| `parity/README.md` | 146 | The session-limit post-mortem; what must be re-run | 5 established findings + 2 process hazards | All 3 re-runs it demanded were done (attributes / interactions / owns sweeps all exist) | — |
| `parity/editor_inventory_attributes_modal.md` | 144 | Recovered chat report — Attributes modal, 4 tabs | Input, spot-checked by `attributes_sweep.md` §1.2/§4.6 | Consumed | — |
| `parity/editor_inventory_mission_settings.md` | 159 | Recovered chat report — Mission Settings, `meta.*`, `author_env` | Input, consumed | Consumed | — |
| `parity/editor_inventory_absent_entities.md` | 175 | Recovered chat report — the five absent entity families | Input, consumed | Consumed | — |
| `parity/gap_analysis_rewrite_log.md` | 199 | `gap_analysis.md` rewritten sample→census: 59 rows → 191 | 132 new rows + 7 corrections | Applied to `gap_analysis.md` in place | The 22 `na` reasons `attributes_sweep.md` §6 asked to be written in are **not** there yet — cheap doc work |
| `parity/owns_and_waves.md` | 968 | `owns` derivation for T-631…T-660 | Process artifact | Consumed by `owns_parity.md` §5 → the registry write | — |
| `parity/owns_parity.md` | 797 | `owns` for the parity tickets + the authoritative 43-row packing | Process artifact | Consumed — §5 is the write log's primary source | — |
| `parity/owns_correction_chrome3.md` | 116 | Supersedes T-636/637/638 `owns`; moves T-637 to its own wave | Process artifact | Applied | — |
| `parity/registry_write_log.md` | 303 | The filing record: 43 factory + 10 workbench + 4 non-dispatchable | Process artifact | Is the record | Its §7 residual **T-146 (no plan row)** is still open |
| `parity/camset_panic_finding.md` | 98 | `__editorCamSet` panics wgpu; 8 gate smokes drive the editor with it | 1 finding + 1 consequence | — | **Held by the artifact's own instruction** — "no ticket should be filed against the gate until someone checks" in a real browser. See §3.5 |
| `adversarial/verify_claims.md` | 324 | Claims re-derived against primary source | ~30 verdicts, 2 `FAILS` | The WOG `/g` `FAILS` is carried into **T-656**'s body verbatim (write log §6) | The other PARTIALs are artifact-accuracy corrections, not buildable work — see §3.6 |
| `adversarial/verify_consistency.md` | 348 | Internal consistency + inferential soundness | 5 reasoning failures (F1–F5) | — | **F1 (three communities, not four) is not corrected anywhere.** See §3.6 |
| `adversarial/verify_coverage.md` | 320 | What was never asked / quietly dropped | 5 GAPs + 31-row walk + 10 dropped threads | GAP-4 (unreconciled ticket sets, stale registry) **closed by the registry write**; dropped thread 7 (`settings.respawn` dead enum) **is** covered by T-291, contra the doc | **GAP-3 (LoS occlusion)** · **GAP-5 (collab / review / versioning / E2E)** partly · dropped threads 6, 9, 10. See §3 |
| `editor_ui_program_plan.md` | 266 | The program framing + 4 open decisions | 13 ids, all resolve in the registry | **13 / 13** | Open decision #3's **north arrow** — neither adopted nor rejected. See §3.4 |
| `editor_ui_ticket_drafts.md` | 318 | The 23 drafts T-631…T-653 | 18 ids, all resolve | **18 / 18**; all 23 drafts filed, cancelled or deferred | — |
| `editor_chrome_direction.md` | 120 | Eden's layout in Aegis's colours | 1 new ticket + 3 rescopes | T-668 filed; T-634/636/637 rescoped; T-632 absorbed | — |
| `eden-feds-draft.jsonl` | 159 rows | Wiki-derived Eden feature entries, `status: draft` | 159 rows | **Zero** — referenced by no current artifact | Orphan. See §3.7 |
| `backlog-braindump-2026-07-25.md` | — | The operator's verbal backlog, `NEW` / `STALE?` statuses | Items 1–25 | Partly, via other programs (item 14–16 → T-295 `idea`) | Items 24–25 and the promised `[DERIVED]` sections. See §3.8 |

**Totals.** 39 artifact files audited (6 frameworks · 1 synthesis · 9 screenshot · 1 3den catalogue ·
14 parity · 3 adversarial · 3 program docs · the JSONL · the braindump). Two artifacts are at
**materially incomplete** coverage
(`screenshots_sweep.md`, `3den_sweep.md`); one is at **zero** (`eden-feds-draft.jsonl`); everything
else is either filed, consumed by a filed artifact, or explicitly closed with a reason.

---

# 3. Still unreached — every item, with a recommendation

## 3.1 Two mis-attributions in the brief's "filed" column

**U5 (derived trait badges) is NOT covered by T-659.** T-659 is the slot-census badge — per-side
counts (`WEST 78 · EAST 74 · IND 8`) and a generated summary line. Its body contains no trait, medic,
engineer or badge content, and a registry-wide search for `trait badge` / `medic` / `engineer` in a
badge sense returns nothing. U5 is *"medic / engineer / radio operator / leader badges computed at
render, never stored"* (`wog.md §4b`; WOG's stored-mutation version is broken three ways and produced
**zero** output across 171 missions, `wog.md §14.1`).

> **Recommendation: fold into T-659, do not file separately.** Same widget family, same wave-7 slot,
> and the data is already resident. `3den_sweep.md` §4.1 rank 9 independently asks T-659 for
> per-*type* counters alongside per-*side*, and rank 9 of the same table routes **Unit Traits** to
> **T-674** (`attributes.rs:358-363` literally advertises `"Medic (soon)"` / `"Engineer (soon)"`).
> So the *typed trait* is T-674 (workbench) and the *derived badge* is T-659 (claude-code). One
> sentence added to each body closes U5. Not filed as a tenth ticket because it would collide with
> T-659 on `eden_top_strip.rs` and buy nothing.

**B4 (briefing authoring) is only half covered by T-671.** T-671 is the mission-row `briefing` column
plus the thumbnail. B4 is *per-faction structured briefings with marker links and a
uniform-recognition section*. Of those: per-faction prose is **shipped** (T-214, T-344); marker links
route through T-069 (queued, wave 16) and are unowned by any ticket; the **uniform-recognition
section** — invented independently by two communities (`wog.md §6` 44 records with shipped `.paa`
images; `ofcra_omtk.md §3 step 4` placeholder silhouettes the maker is *required* to replace, because
uniform theft is a bannable rule) — is in **no ticket at all**.

> **Recommendation: fold both residuals into T-671's body as scope lines**, not new tickets. Marker
> links are a rendering concern of T-069's markers; the uniform section is one more field on a
> `briefings[faction]` object that already exists in the schema. `3den_sweep.md` §4.1 rank 5 also
> routes briefing **templates** to T-671. Three scope lines on one queued ticket beats three tickets.

## 3.2 `screenshots_sweep.md` — 24 rows in 8 proposed slices, unfiled

Written after the registry write. Its §4.2 proposes seven factory slices and one workbench slice, and
its §4.3 already did the wave-packing arithmetic against the real `wave_plan.tsv`.

| Slice | Rows | Recommendation |
|---|---:|---|
| **NEW-F1** `Ctrl+S` saves a version | 2 | **Fold into T-669** — the sweep's own §4.3 says so: `mission_editor.rs` is claimed in 18 of 19 waves, NEW-F1 has no usable slot, and T-669 is already the keyboard-arm ticket owning that file alone. One extra match arm. |
| **NEW-F2** editor preferences, separated from mission settings | 8 | **File.** The largest and the only one with a structural argument: editor prefs (basemap view, 12 world layers, autosave, camera speed) are `localStorage` state filed under *Mission* Settings. Also the natural home for `DLG-SHELL-003` — TBD has **no cancel path on any settings dialog**. `3den_sweep.md` E6 is the same ticket from a second source, and E1/E3/E9 all depend on it. |
| **NEW-F3** editor help surface | 3 | **File.** TBD's 10 keyboard bindings are documented nowhere in the UI. |
| **NEW-F4** merge another mission into this one | 1 | **File**, scoped to `mission_commands.rs` + `store.rs` (the sweep notes that scoping opens every wave). |
| **NEW-F5** asset-browser favourites | 1 | **Merge with `3den_sweep` E3** — same feature, two sources. One ticket. |
| **NEW-F6** mission shape editable after creation (game mode, min/max players) | 3 | **File.** A mission's lobby shape is frozen at birth (`create_mission_dialog.rs:156-160`, `:197-213`). Distinct from T-671. |
| **NEW-F7** locations list — browse and fly to a named place | 1 | **Merge with `3den_sweep` E1** (bookmarks + locations index) — same feature, two sources. |
| **NEW-W1** player gadget availability flags (map/compass/watch/GPS/HUD) | 5 | **File as `workbench`.** Same shape as T-681/T-682. |

Plus **143 open rows that thicken 34 already-filed tickets as scope notes that are not in any ticket
body** — most consequentially T-648 (Eden's grid family is six controls, not one), T-668 (22 rows;
the ticket was the thinnest-specified in the program and is now the best-sourced), T-682 (15 rows;
every refused weather channel with its Eden control shape), T-646 (14), T-664 (12; verbatim two-state
item lists).

> **Recommendation:** file 5 new tickets (NEW-F2, F3, F4, F6, W1), fold NEW-F1 into T-669, and merge
> NEW-F5/F7 into 3den's E3/E1. **Not filed here** because 5–7 new factory rows is a program-sizing
> decision with wave-packing consequences the operator owns, and the brief scoped this pass to the
> synthesis's nine. The scope-note application to 34 bodies is a separate, mechanical pass.

## 3.3 `3den_sweep.md` — 12 proposed slices + 9 scope extensions, unfiled

Also written after the registry write. Its §4.3's two rows (U2, U3) are **filed here** as T-686 /
T-687 — this audit reaches them from the synthesis, the sweep reached them independently from the
mod, and the two sources agree.

Unfiled: **E1** map bookmarks + locations index (the sweep calls it "the cheapest large QoL win in the
catalogue") · **E2** command palette (the standard web answer to *"the current UI is very
inconsistent and very disjointed"*, which is the complaint T-668 exists to fix) · **E3** asset-browser
favourites · **E4** selection filter + document search (what makes T-649's Select All safe) · **E5**
clipboard exporters (grid position is the milsim-relevant one) · **E6** editor preferences store ·
**E7** loadout buffer (copy/apply loadouts between slots — TBD has no path at all) · **E8** small-UI
idioms (numeric nudge, shared `Ctrl+F`, tree-row tooltips) · **E9** editor-only visibility override ·
**E10** keybinding collision test · **E11** whole-terrain zone button · **E12** briefing Signal
section (`workbench`).

Unapplied scope extensions: T-645 (**21 want rows** — the itemised contents plus three corrections,
including *drop Align Z± / Space Z, they fight T-092.1's ground-everything contract*), T-642, T-666,
T-649, T-671, T-665, T-659, T-673, T-674.

> **Recommendation: file E8, E11, E10, E1 and E5 (all XS–S, high ratio), merge E3 with NEW-F5 and E6
> with NEW-F2, and apply the 9 scope extensions to the existing bodies.** E2 (command palette) is a
> product decision, not a filing decision — it is the largest new build in either sweep and it
> overlaps T-668's remit. **Do not** file E12 until T-671 lands. Same reason as §3.2 for not filing
> them in this pass.

## 3.4 Small, cheap, unreached

| Item | Source | Recommendation |
|---|---|---|
| **North arrow** | `editor_ui_program_plan.md:264` open decision #3 | **Rule out, in T-667's body.** T-667 adopted the scale bar and edge grid labels and says "skip the legend"; the north arrow is named in the preamble and never dispositioned. A north-up-locked 2D orthographic map does not need one. One sentence, not a ticket. |
| **22 `na` reasons missing from `gap_analysis.md`** | `attributes_sweep.md` §6 | **Fold into the next `gap_analysis.md` revision** (`cursor-docs`). The sweep's own warning is the argument: "an unexplained `missing` invites a ticket, and 22 of them would invite 22." |
| **5 keyboard collisions with no test** | `interactions_sweep.md` §4.4 | **Covered by 3den E10** if E10 is filed; otherwise a two-line unit test. |
| **T-146 has no wave plan row** | `registry_write_log.md` §7.1 | Still open, unchanged by this pass. Needs an `owns` and a wave, or a supersede into T-646/T-084. |
| **T-687 appears on no queue view** | found while verifying this write | `generate_ticket_mod_queue_md` (`xtask/src/sync.rs:254-266`) filters on `targets` **containing `"mod"`**, so a `workbench` ticket whose work is a `packages/tbd-schema` widening with no Enfusion change shows up in `docs/TICKET_REGISTRY.md` **and nowhere else** — not the mod queue, not the dev queue, not the lead view. T-687 is the first such row. **Deliberately not fixed by adding `"mod"` to its targets**, because that would assert Enfusion work the evidence says may not exist (§3.1 of the ticket body). The real fix is the filter: `executor == workbench` should be sufficient for the mod queue, `targets` should not gate it. One-line `xtask` change, worth a `cursor-docs`/tools row rather than silently mislabelling a ticket. |

## 3.5 `camset_panic_finding.md` — held, not unreached

The artifact instructs that **no ticket be filed** until someone runs `window.__editorCamSet(6400,
6400, 0)` in a real browser, because headless Chrome already produced one false "broken engine" in
the same session. Two outcomes: headless artifact (note it in the capture README, use wheel events)
or a real bug that invalidates whatever 8 gate smokes in `tools/tbd-tools/src/smokes.rs` have been
asserting.

> **Recommendation: leave unfiled; this is a 30-second operator check, and it is the correct
> disposition.** But **T-641 (queued, wave 3) is blocked on it** — its re-scope was never done
> because the zoom driver crashes the thing being photographed, and the artifact says T-641 "should
> not be filed as either greenfield or zoom-band defect until the map is looked at." T-641 is
> currently filed as neither and scheduled in wave 3. That is the highest-priority residual in this
> audit.

## 3.6 The adversarial passes — mostly corrections, one that still bites

`verify_claims.md`'s one severe finding (the WOG `/g` regex claim FAILS; SQF regex flags are legal
trailing syntax) is already carried into **T-656**'s body, which withdraws the "both validators are
silently dead" framing to a one-framework argument. Good.

`verify_consistency.md` **F1 is not corrected anywhere**: `frameworks/README.md:28` states the rule
*"'Two FNF frameworks' is one repo at two eras, not two projects"*, and the synthesis then counts
"five artefacts, four communities" and stamps "4/4" on every Appendix row by counting FNF twice. The
convergences are mostly real at **N=3**; the arithmetic that sells them is not.

> **Recommendation: not worth a ticket — worth a one-line correction in `framework_synthesis.md`'s
> Appendix header.** No ticket in the program rests on the difference between 3/3 and 4/4: every
> item this audit filed cites the specific framework and section, not the tally. Filing a ticket to
> fix an adjective would be exactly the low-value work the brief warns against. But leaving it
> uncorrected means the next reader inherits it, which is this program's signature failure.

`verify_coverage.md` **GAP-3 remains fully open**: T-643 (ray) and T-644 (viewshed) both sample the
bare-earth DEM, and neither body contains the word *occlusion*, *building* or *canopy* — while the
same engine already holds 4,131 building OBBs and 501,861 trees plus a canopy-density texture. A
milsim LoS that reports "clear" through a forest block is the tool confidently wrong.

> **Recommendation: add the occlusion decision to T-643's "DECIDE INSIDE THE TICKET" list**
> (it already has one, for eye height and profile rendering). Not a new ticket — it is a scope line
> on a queued ticket, and T-643 explicitly exists to prove the sampling path T-644 reuses.

`verify_coverage.md` **dropped thread 7 is stale**: it says no ticket carries the dead
`settings.respawn: "tickets"` enum. **T-291** (`idea`) covers
`settings.{respawn,spectatorPolicy,nightVision}` as declared-with-zero-implementation. No action.

## 3.7 `eden-feds-draft.jsonl` — zero coverage, and the right answer is to delete or archive it

159 wiki-derived Eden feature rows, all `status: draft`, referenced by nothing. It is an earlier
attempt at the same coverage problem that `docs/specs/.../eden/attributes.md` +
`interactions.md` + `gap_analysis.md` (now a 191-row census) solved properly.

> **Recommendation: not worth a ticket to mine it — worth one grep to prove it is subsumed.** If its
> `ui_surface` values map onto the 176 Eden ids the census now covers, delete it or move it under an
> `archive/` prefix so no future audit has to ask this question again. That is a `cursor-docs`
> chore, not a program item. **This audit did not mine its 159 rows**; that is stated so the boundary
> is visible rather than implied.

## 3.8 `backlog-braindump-2026-07-25.md` — operator-named wants, outside this program

`verify_coverage.md` GAP-5 names items **14–16** (git-diff-style mission versioning, realtime
collaborative editing, commenting + reviewer workflow) and **25** ("never tested: place unit → export
JSON → load in mod — the single highest-information test available"). Checked: item 15 has **T-295**
(`idea`, "Realtime collaborative editing"). Items 14, 16 and 25 have no ticket. The promised
`[DERIVED]` companion sections never landed.

> **Recommendation: rule out of the editor-UI program, explicitly.** The 14-section framework schema
> had no section for collaboration, review or the author's test loop — reasonable, because all four
> studied frameworks are single-file and single-author — so no amount of framework research was ever
> going to reach them. They are a **separate program**, and folding them into a UI/UX program would
> stall it on a different discipline exactly as D.5 argues for the mission-content split. Item 25 in
> particular is already the `executor: human` slice that has both T-068 and T-181 parked as
> `deferred`; it does not need a third row.

---

# 4. Verification — verbatim

## `./scripts/ticket check` (via `distrobox-host-exec`; `xtask` is a host binary)

```
check OK
```

## `./scripts/ticket sync`

```
sync complete
```

Regenerated with changes: `docs/TICKET_REGISTRY.md`, `docs/TICKET_MOD_QUEUE.md` (the five new
`workbench` rows land here, which is correct), `docs/TICKET_BRAINSTORM.md` (T-684 in, T-212 out).

## `bash scripts/verify-no-python.sh`

```
==> find *.py (excl .git / node_modules / target / worktrees)
  OK (none)
==> python interpreter invocations in scripts/ + Makefile
  OK — 12 file(s) invoke python3, all inventoried, none new
verify-no-python: PASS
```

The collision checker written for this pass is **Node, in the session scratchpad, deliberately not in
the repo** — for the same reason the previous pass kept its checker out (`verify-no-python` is a hard
gate at `Makefile:466`, and a repo-resident checker is a maintenance liability nobody asked for).

## The collision check — written for this pass, calibrated against the previous one

It asserts: 4 tab-separated columns per row with a bare-integer wave label · every plan id resolves in
the registry · **no two tickets in the same wave share an `owns` path** · no duplicate ids in the
block · no `workbench`/`human`/`ci` executor anywhere in the file · every dispatchable `eden` row has
a plan row · plus a whole-file baseline so nothing can hide in the platform factory's pre-existing
state.

**Baseline, before any write — reproduces `registry_write_log.md` §4 exactly, which is how the
checker was validated:**

```
1. column count      : 43 rows in labels 100-118 (file total 466 rows, 0 malformed)
2. ids in registry   : 43/43 resolve
3. owns collisions   : 19 waves, 33 pairs compared, 0 COLLIDING
4. duplicate ids     : none
5. factory safety    : block executors = ['claude-code'] ; non-dispatchable executors anywhere in file: [T-205@0, T-206@0, T-404@5]
6. coverage          : 44 dispatchable eden rows, 1 without a plan row -> T-146
7. whole file        : 466 rows, 355 colliding pairs of 9411 compared, 9 duplicate ids

PLAN OK — editor-UI block has zero intra-wave owns collisions
```

**After the write:**

```
1. column count      : 47 rows in labels 100-118 (file total 469 rows, 0 malformed)
2. ids in registry   : 47/47 resolve
3. owns collisions   : 19 waves, 39 pairs compared, 0 COLLIDING
4. duplicate ids     : none
5. factory safety    : block executors = ['claude-code'] ; non-dispatchable executors anywhere in file: [T-205@0, T-206@0, T-404@5]
6. coverage          : 48 dispatchable eden rows, 1 without a plan row -> T-146
7. whole file        : 469 rows, 355 colliding pairs of 9410 compared, 9 duplicate ids

PLAN OK — editor-UI block has zero intra-wave owns collisions
```

**Read the delta honestly.** Rows 466 → 469 is +4 new rows and −1 (T-212's row at label 9, removed
because the row became `workbench`). Pairs compared 9411 → 9410: the four additions contribute +6
intra-wave pairs, T-212's removal drops 7. **Colliding pairs stayed at 355, so T-212's row was
colliding with nothing and its removal hid nothing.** Duplicate ids, malformed rows and the
non-dispatchable-executor list are all unchanged from baseline. Line 5 is the check the brief
required: **no `workbench` ticket appears anywhere in the plan** — the three that do are the
pre-existing platform-factory rows T-205, T-206 and T-404, untouched by this pass.

## Registry totals after the write

```
664 tickets · next_id 691 · highest id T-690
status   : shipped 520 · queued 63 · idea 42 · cancelled 20 · deferred 17 · ready 2
executor : claude-code 586 · cursor-docs 46 · workbench 25 · human 4 · - 3
```

(`- 3` is three legacy rows that carry no `executor` field at all — pre-existing, untouched.)

Was 656 / `next_id` 683 / T-682 · queued 55 · idea 42 · workbench 20 · claude-code 583.
Delta: **+8 rows** (T-683…T-690)
and **T-212 moved** idea→queued and claude-code→workbench. No other existing row was touched.
