# Parity sweep — partial. What survived the session limit, and what must be re-run.

**2026-08-01.** The full parity re-scope was dispatched as three Opus lanes. The session token limit
hit mid-flight and **all three lead agents died before writing their artifacts.** Three of their
sub-agents had already finished and reported; those reports are recovered here.

Read this before trusting anything in this directory.

## What is here, and how good it is

| File | Subject | Evidence grade |
|---|---|---|
| [`editor_inventory_attributes_modal.md`](editor_inventory_attributes_modal.md) | The Attributes modal, all 4 tabs, every per-entity attribute, the full slot key set | **Recovered from a chat report** |
| [`editor_inventory_mission_settings.md`](editor_inventory_mission_settings.md) | Mission Settings, top strip, `meta.*`, the `author_env` gate, create-vs-editor split | **Recovered from a chat report** |
| [`editor_inventory_absent_entities.md`](editor_inventory_absent_entities.md) | Markers, triggers, waypoints, comments, compositions — the five absent families | **Recovered from a chat report** |

**"Recovered from a chat report" is a real caveat, not a formality.** This program has already
recorded four instances of a claim entering the record from an agent's chat summary and being
treated as sourced — including one fabricated quotation and two phantom statistics. These three
files were never written by their authors to disk; they are transcriptions of what those agents said
they found. Every claim carries a `file:line`, which makes them **checkable** — but they have not
been checked. **Spot-check before resting a ticket on any single line.**

## What must be re-run

| Deliverable | Target file | Status |
|---|---|---|
| **Attributes sweep** — all 93 `ATTR-FIELD-*` ids triaged vs live source | `attributes_sweep.md` | **NOT WRITTEN** |
| **Interactions sweep** — all 83 `interactions.md` ids + keyboard map + collisions | `interactions_sweep.md` | **NOT WRITTEN** |
| **`owns` derivation + wave packing** — file paths per ticket, hot files, collision matrix | `owns_and_waves.md` | **NOT WRITTEN** |

The three recovered files are **inputs** to the first of those, not substitutes for it. They
establish what the editor *has*; the sweep still has to walk every Eden id and assign a parity value.

## Findings established before the limit — verified by the main thread

These were checked directly and do **not** need re-deriving:

1. **The id count is 93, not 96.** `grep -oE '\bATTR-FIELD-[A-Z0-9-]+\b' attributes.md | sort -u |
   wc -l` → 93 (OBJ 31, TRG 13, SCN 11, GRP 10, MRK 10, WP 9, CMT 3, LYR 3, COMP 3). My earlier
   figure of 96 came from a looser pattern that caught the bare template `ATTR-FIELD`, the glob
   `ATTR-TAB-*` and a cross-ref. **The parity table covers 3 of 93.**

2. **The compiled schema is closed.** `mission.schema.json` carries **25** `"additionalProperties":
   false`, including on `$defs/slot`, `$defs/group`, `meta` and `environment`. So most Eden state
   attributes are **contract-blocked, not merely unbuilt** — they need a schema widening plus
   mod-side support, which makes them `executor: workbench` work in the second program, not
   factory-dispatchable UI work. **This is the single most important input to program sizing and it
   has not yet been quantified per-id.**

3. **Tickets already exist for several families**, and must be mapped onto rather than duplicated:

   | Ticket | Status | Title |
   |---|---|---|
   | T-082 | deferred | Full attribute fields |
   | T-079 | idea | Triggers + waypoints + systems |
   | T-069 | deferred | Markers on map |
   | T-213 | idea | Marker placement |
   | T-078 | deferred | Custom compositions |
   | T-076 | idea | Vehicle crew UI |
   | T-077 | idea | Alt + empty vehicle |
   | T-074 | **cancelled** | Faction submode / catalog filter |

   This corrects the adversarial pass's "10 parity rows have no ticket anywhere" — it searched the
   drafts and the plan, not the registry. The rows are uncovered **by my drafts**; the registry has
   `idea`/`deferred` tickets for most of them. The real work is promoting and scoping those, not
   writing new ones.

4. **`ENV-SETTINGS-002` in the parity table is stale.** T-193 (`b30f5490`) *removed* View Distance
   and Thermals rather than wiring them; `eden_chrome.rs:4624` now actively refuses to author
   `viewDistance`, `thermals`, `windDirDeg`, `fog`, `wind`.

5. **The T-216 drop ledger.** `crates/map-engine-core/src/mission/flatten.rs:2584-2649` enumerates
   six author-facing values the compile silently drops — a squad's `leaderSlotId`, a slot's `tag` /
   `callsign` / `rank` / `stance`, and the entire vehicle roster — and records that
   `make verify-t180` stayed green throughout **because not one of its 22 tests checked**. Any
   attribute marked `match` or `partial` must be checked against this: *"the editor authors it"* and
   *"it reaches the game"* are different claims, and this ledger exists because they were confused
   before.

## A methodology note worth keeping

The attributes agent recorded a trap it avoided: `grep -rl stance apps/mod` returns **39 files**,
which would have supported "the mod has a stance concept". A **word-boundary** search returns
**zero** — every hit was the substring inside `instance`.

That is the same failure mode as the three statistical errors already on record in this program. It
is now the fourth independent instance of *a grep answering a different question than the one
asked*. Word-boundary by default; state the command with the number.

## Re-run guidance

The three dead lanes should be re-dispatched with their original briefs, plus:

- Use **93**, not 96, as the attribute id count.
- Map onto the eight existing tickets above; do not propose duplicates.
- **Quantify the schema split per id** — (a) buildable in the SPA today, (b) blocked on a schema
  widening, (c) blocked on mod support, (d) genuinely `na` for a 2D web editor. That ratio decides
  how large the factory-safe program is.
- Treat the three recovered files here as **starting evidence to verify**, not as settled fact.
- Instruct the lead agents **not to fan out** — the attributes lead spent its budget dispatching four
  sub-agents and died before writing, while its sub-agents' work survived only by accident.
