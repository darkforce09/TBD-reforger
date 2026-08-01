# `eden/gap_analysis.md` rewrite — change log

**2026-08-01.** Rewrote `docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md` in place:
sample → census. Transcription of [`attributes_sweep.md`](attributes_sweep.md) and
[`interactions_sweep.md`](interactions_sweep.md) plus seven corrections. **The triage was not
re-derived** — only the corrections, the coverage arithmetic and the ticket-column verification were
done here.

---

## Row counts — before / after

| | Before | After |
|---|---:|---:|
| Rows with a parity value | **59** | **191** |
| `interactions.md` ids covered (of 83) | 41 | **83** |
| `attributes.md` ids covered (of 93) | 3 | **93** |
| Rows outside both catalogues | 15 | **15** (preserved) |

Coverage is exact in both directions — no catalogue id lacks a row, and no row cites an id that is
not in a catalogue or in the preserved legacy set:

```bash
cd docs/specs/Mission_Creator_Architecture/eden
grep -oE '\bATTR-FIELD-[A-Z0-9-]+\b' attributes.md | sort -u | wc -l                    # 93
grep -oE '\b[A-Z][A-Z0-9]*(-[A-Z0-9]+)*-[0-9]{3}\b' interactions.md | sort -u | wc -l   # 83
# set-diff of catalogue ids vs gap_analysis row ids, both directions → empty
```

### Parity distribution

| parity | before | after | delta |
|---|---:|---:|---:|
| match | 12 | 24 | +12 |
| partial | 8 | 17 | +9 |
| missing | 31 | 113 | +82 |
| deferred | 1 | 4 | +3 |
| na | 3 | 31 | +28 |
| tbd_only | 3 | 2 | −1 |
| `working` *(not in the legend)* | 1 | **0** | retired |
| **total** | **59** | **191** | **+132** |

`tbd_only` drops by one because `LAYER-CREATE-001` was mis-scored `tbd_only` (correction 5). The two
surviving `tbd_only` rows are the original TBD-only pair (`TBD-LAYER-001`, `TBD-CONFLICT-001`).

### Build class — new column, attribute rows only (93)

| class | count | share |
|---|---:|---:|
| **a** SPA-buildable today | 22 | 23.7 % |
| **b** schema-blocked | 20 | 21.5 % |
| **c** mod-blocked | 29 | 31.2 % |
| **d** `na` | 22 | 23.7 % |

**22 factory · 49 workbench · 22 closed.** Six of the 22 factory ids are already shipped, so new
factory attribute work is **16 ids**. Interaction rows are deliberately **not** build-classed — the
interactions sweep did not triage that axis and inventing it here would launder an unmeasured claim.

---

## The seven corrections, as applied

| # | Row(s) | Before | After | Basis |
|---|---|---|---|---|
| 1 | `ATTR-FIELD-OBJ-SKILL` | `missing` | **`na`** (class d) | Bodies spawn AI-disabled (`TBD_SpawnManager.c:963,1166`); `skill` word-boundary in `apps/mod` = 0. `missing` implied buildable work with no subject |
| 2 | `ENV-SETTINGS-002` | `partial`, *"Thermals + view dist in dialog"* | **`missing`**, notes rewritten | T-193 (`b30f5490`) removed both; `eden_chrome.rs:4622-4629` now asserts they are not authorable. Same subject as `ATTR-FIELD-SCN-VIEW-DIST`, also `missing`. Noted that `feature_inventory.md:1732` still records Status `working` and is stale for the same reason |
| 3 | `ATTR-FIELD-LYR-NAME` | `match` | **`missing`** (class a) | `rename_editor_layer` has one mention repo-wide — its own definition (`store.rs:1886`). Verified independently in this pass, see the disagreement note below |
| 4 | `OBJ-CALLSIGN` · `OBJ-RANK` · `OBJ-STANCE` | no rows existed | **`partial`** (b/b/c) | T-216 ledger `flatten.rs:2584-2649` records the compile silently drops them; `TBD_MissionSlotStruct.c:59-69` has no field for any. *"The editor authors it"* ≠ *"it reaches the game"* |
| 5a | `RIGHT-MODE-001` | `match` | **`partial`** | Eden's single Object mode is three TBD surfaces (Factions tab / Vehicles tab / Objects chip); no `F1` |
| 5b | `RIGHT-SUBMODE-001` | `missing \| T-074` | **`partial`**, ticket → shipped T-180.5 | Side chips filter the tree (`eden_chrome.rs:2871`, `:2919`). **T-074 is `cancelled`** — no row in the file now cites it as live |
| 5c | `PLACE-001` | `partial` | **`missing`** | Click-then-click is structurally impossible — the arm dies on the click that creates it (`editor_ops.rs:1037`, `mission_editor.rs:1668-1671`). The old note was right, the value was not |
| 5d | `SEL-GROUP-ICON-001` | `partial \| T-071` | **`missing`** | T-071 shipped and this did not change: squad rows are non-interactive `<div>`s (`eden_chrome.rs:1578-1611`) |
| 5e | `LAYER-CREATE-001` | `tbd_only` | **`missing`** | A claim about *semantics* was standing in for a claim about *existence*. Editor Layers ≠ Eden layers is still true and is kept in the note; there is no create control at all |
| 6 | `CONN-GROUP-001` | `missing \| T-071` | **`partial`**, T-071 ✅ + new P-4 | Grouping shipped via T-071/T-180 off the map; only Eden's map-surface Ctrl+drag is absent |
| 7 | `RIGHT-SEARCH-001` | `working` | **`match`** | See below |

### Correction 7 — what I did with `working`, and why

**Retired it; mapped the one row onto `match`.** Not added to the legend.

- `working` is a **Status** value from `feature_inventory.md`, not a parity value — that is where it
  leaked in (`ENV-SETTINGS-002` still carries Status `working` there).
- The row it was on already has a defined value with independent evidence:
  `interactions_sweep.md` scores `RIGHT-SEARCH-001` **`match`** on `asset_catalog.rs:396-414` with
  T-055 shipped.
- Adding a sixth parity value for a single row would have grown the vocabulary instead of
  reconciling it, and would have left a value with no definition of how it differs from `match`.

The legend now states explicitly that `working` is not a parity value.

---

## Disagreement found between the sweeps — reported, not silently resolved

**`ATTR-FIELD-LYR-NAME` / the editor-layer mutator family.**

| Source | Verdict | Evidence used |
|---|---|---|
| `attributes_sweep.md` §2 (LYR table) | `LYR-NAME` = **`match`**, T-037 shipped | *Existence*: `rename_editor_layer` (`store.rs:1886`) "with create/reparent/remove/refile alongside" |
| `interactions_sweep.md` §2.9 + §6.1 | `LAYER-CREATE-001` / `LAYER-DEL-001` = **`missing`**; `gap_analysis`'s `tbd_only` is wrong | *Reachability*: `grep add_editor_layer\|rename_editor_layer\|reparent_editor_layer` → 3 hits, all comments plus one auto-seed |

Both greps are correct. They answer **different questions** — "does the mutator exist" vs "does any
UI call it" — and the two sweeps did not read each other, so neither caught the split.

Re-verified directly in this pass, repo-wide, `--include='*.rs'`:

| Symbol | Definition | Product callers | Test callers |
|---|---|---|---|
| `rename_editor_layer` | `store.rs:1886` | **0** | 0 |
| `reparent_editor_layer` | `store.rs:1895` | **0** | 0 (one comment at `outliner.rs:131`) |
| `move_slot_to_layer` | `store.rs:1915` | **0** | 0 (two doc comments) |
| `remove_editor_layer` | `store.rs:1527` | **0** | 2 (`store.rs:4432-4443`) |
| `add_editor_layer` | `store.rs:1872` | **1** — the auto-seed `editor_ops.rs:1136` | 8 |

**Applied the reachability reading** (`missing`, class a, → slice P-6), consistent with the
brief's correction 3 and with how the interactions sweep treats the rest of the family. The
disagreement is recorded in the document's provenance section as well as here, so nobody re-derives
it from the attributes sweep alone.

**No other row disagreed.** One *within*-sweep inconsistency was found and is flagged in the row
itself rather than resolved silently: `attributes_sweep.md`'s family table files
`ATTR-FIELD-OBJ-UNIT-NAME` under T-082, while its own §6 and `owns_parity.md` §3/§4 route it to the
workbench T-216 follow-on (it is class **b**). The table cites the build-class-correct home and says
so in the note.

---

## Ticket column — verification

Every `T-0xx` in the file was checked against `.ai/tickets/registry.json` (read-only; the registry
was **not** modified, per the concurrency instruction).

**Verified present — 41 ids**, as registry rows or as slices of one:

`T-037 · T-048 · T-049 · T-050 · T-052 · T-053 · T-054 · T-055 · T-056 · T-057 · T-061 · T-067 ·
T-068 · T-068.4 · T-068.10 · T-069 · T-071 · T-072 · T-073 · T-074 · T-075 · T-076 · T-077 · T-078 ·
T-079 · T-082 · T-084 · T-090 · T-091 · T-091.2 · T-092 · T-092.1 · T-110 · T-146 · T-175 · T-180 ·
T-180.5 · T-193 · T-213 · T-215 · T-216 · T-242 · T-582`

**Could not verify — 3, none of them in a ticket column:**

| Id | Where | Status |
|---|---|---|
| `T-631` … `T-660` | Ticket-column **legend** only, naming the editor-UI draft range | **Absent from the registry** — it maxes at T-630. A concurrent agent is filing these. The legend says so explicitly; no row cites one |
| `T-159.29.3` | Prose in the "carried forward" table (the React deletion that killed `state/schema.ts`) | Not a registry row and **not in T-159's `slices` array**, which has 17 entries and none under `.29`. It is a historical commit-tagged sub-slice used the same way by `CLAUDE.md` and both sweeps. Left as-is |

**`T-074` (cancelled) appears exactly twice** — once in the ticket-column legend stating it is
cancelled and absorbed by T-180.5, once in the `RIGHT-SUBMODE-001` row recording that it *was* cited
there and no longer is. **No row cites it as live work.**

**Slices proposed but not yet filed** are cited as `new — Px` / `new — Nx` / `new — G2`, defined in
`interactions_sweep.md` §5.2 and `attributes_sweep.md` §5. Used: P-1 · P-2 · P-3 · P-4 · P-5 · P-6 ·
P-7 · P-8 · P-9 · P-10 · P-11 · P-12 · N1 · N2 · N3 · N4 · N5 · N6 · N7 · N8 · N9 · G2. This keeps
every `T-` id in the file registry-backed while still naming the work.

---

## Structural changes

- **Kept:** the header block, the parity legend, category sections, the summary at the end.
- **Added:** a coverage table + reproduction commands in the header (so the census claim is
  checkable); a provenance section; a **build-class legend**; a **ticket-column legend**;
  a `build_class` column on all 93 attribute rows.
- **Reorganised** into three parts — interactions (83), attributes (93), shell & data (15) — because
  the old six coarse sections cannot hold 191 rows. The old section names are preserved as far as
  they map (*Asset browser & placement*, *Transform, widget, toolbar*, *Compositions*,
  *Connections … & groups*, *Layers, selection …*, *Shell & data*).
- **Preserved** all 15 non-catalogue rows. The four `TOOLBAR-*` ids among them are now labelled
  *(minted)* — `interactions.md:371` literalises only `TOOLBAR-NEW-001` and `TOOLBAR-TUTORIAL-001`,
  so those four were invented by the old sample and the other ~13 toolbar buttons remain `UNKNOWN`
  as ids.
- **Summary rewritten** from the new body: parity totals, interactions by domain, attributes by
  build class / parity×class / family, disposition, the five keyboard-pointer collisions, and a
  *"carried forward unresolved"* table.
- **Marked the execution-order line historical** rather than deleting it — T-071/T-091/T-092/T-180
  have shipped since it was written, so it is no longer the queue.

### `UNKNOWN` / `INFERRED` carried forward verbatim, not converted to parity values

`OBJ-STAMINA` (`UNKNOWN:` whether Reforger exposes the toggle) · the ~13 unnamed toolbar buttons
(`UNKNOWN` as ids) · the `SYS` family (declares no ids) · `$defs/zoneRules`' 16 untriaged keys ·
`INFERRED:` zones are not trigger areas · `INFERRED:` `Delete` with a vehicle selected removes
nothing for it · both T-069/T-213 spec premises are dead and need rewriting before promotion.

---

## Verification run

| Check | Result |
|---|---|
| `distrobox-host-exec sh -c './scripts/ticket check'` — baseline, before the edit | `check OK` |
| `distrobox-host-exec sh -c './scripts/ticket check'` — after the rewrite | `check OK` |
| 191 parity rows parsed from the file; distribution matches the summary table | pass |
| Every `attributes.md` + `interactions.md` id has exactly one row (set-diff both ways) | pass — empty both ways |
| Every relative link target resolves on disk (14 targets) | pass — one fixed: `../eden_screenshots/` → `../../../../.ai/artifacts/eden_screenshots/` |
| Every cited `T-` id present in the registry | 41/44; the 3 exceptions are prose, listed above |

**Files not touched, per the concurrency instruction:** `.ai/tickets/registry.json`,
`docs/platform/wave_plan.tsv`.
