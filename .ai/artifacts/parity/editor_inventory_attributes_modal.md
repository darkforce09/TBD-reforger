# Live editor inventory — the Attributes modal and every per-entity attribute

**Provenance:** produced 2026-08-01 by a sub-agent of the attributes sweep. The parent hit the
session limit before writing its artifact; **this was recovered from the sub-agent's report, not
from a file it wrote.** That is weaker than a normal artifact — the `file:line` citations should be
spot-checked before anything load-bearing rests on them. Preserved because the alternative was
losing it.

**Why it matters:** this is the ground truth for how much of Eden's 93 `ATTR-FIELD-*` surface TBD
actually has. Short answer: **nine fields on one entity type.**

---

## 1. Tabs — exactly four

`const TABS: [&str; 4] = ["Transform", "Identity", "States", "Arsenal"];` — `attributes.rs:16`.
Strip rendered `:114-137`; body dispatch `:138-154`.

Default open tab is **Identity (index 1)**, not Transform — `mission_editor.rs:724-725`.
`open_arsenal` forces index 3 — `editor_ops.rs:614`.

## 2. Every field, per tab

### Transform (`attributes.rs:255-319`)

| Label | Editable | Widget | Writes |
|---|---|---|---|
| X | yes | number (`number_field` `:167-228`) | `position.x` (`store.rs:1345-1347`) |
| Y | yes | number | `position.y` (`:1348-1350`) |
| Z | yes | number | `position.z` (`:1354-1355`) |
| Rotation | yes | number, `°` suffix `:292` | `position.rotation`, normalised `[0,360)` (`:1351-1353`) |
| Stance | yes | `<select>` `:297-312` — stand/crouch/prone | slot `stance` (`:1246`) |

Commits on **blur or Enter** (`:176-183`, `:198-210`).

**~~A real behaviour worth a ticket:~~ CORRECTED — this is deliberate.** Editing X or Y sets Z to
0.0, at `store.rs:1356-1358`. The original version of this file called it a defect. It is not: the
line carries `// terrain-follow; DEM z is sampled on the JS side`. Moving a slot horizontally drops
it back onto the terrain, which is the intended behaviour, and a ticket "fixing" it would have
broken working code.

Recorded rather than silently edited because the mis-framing propagated — it was reported to the
operator as a bug found in passing before the `attributes_sweep` caught it. **The lesson is the one
this program keeps relearning: the code was read, the adjacent comment was not.**

### Identity (`attributes.rs:321-348`)

| Label | Editable | Notes |
|---|---|---|
| Role | yes | text, placeholder `"Rifleman"` → slot `role` (`store.rs:1239-1241`) |
| Tag | yes | text, placeholder `"MED · ENG · SL…"` → slot `tag` (`:1242-1244`) |
| Squad | **read-only** | plain `<div>` `:342-344`; `"—"` when empty |

Text fields commit **per keystroke** (`on:input` `:246`) — **one undo step per keystroke**, stated
at `:230-231`. That is a defect worth its own ticket.

**Precision point:** "Squad" displays the raw **`squadId` string**, not the squad name —
`store.rs:467-468` interns `read_str(&txn, &slot, "squadId")`.

### States — a stub, and structurally incapable

`states_tab()` at `attributes.rs:350-367`. Three static elements, no inputs, no doc reads, no doc
writes. Exact strings: `"Unit traits — wired to the compiler in a later phase."` (`:355`),
`"Medic (soon)"` / `"—"` (`:358-359`), `"Engineer (soon)"` / `"—"` (`:362-363`).

**The function takes no arguments** — `fn states_tab() -> impl IntoView` (`:351`). It cannot read
the slot even in principle.

### Arsenal (`arsenal.rs:633-642`)

Live loadout editor. **14 pick rows** from `arsenal_rules.rs:49-162`: primary, optic, magazine,
launcher, handgun, throwable, headCover, jacket, pants, boots, vest, armoredVest, backpack,
handwear. Plus a cargo panel (`arsenal.rs:1131`) over `["vest","pants","jacket","backpack"]`
(`arsenal_rules.rs:606`) and a filter box (`:699`).

All Arsenal edits write **one** document key — slot `loadout` (`arsenal.rs:682` →
`editor_ops::set_loadout` `:777-793` → `store.rs:1319`).

## 3. `read_attrs` — nine fields

`editor_ops.rs:631-663`, struct `SlotAttrs` at `:122-132`. Reads from `core.materialize()` (SoA):
`id`, `x`, `y`, `z`, `rotation`, `stance`, `role`, `tag`, `squad`.

**It does NOT read** `assetId`, `index`, `callsign`, `rank`, `loadout`, or `loadoutId`. Its only
other caller is the bottom-toolbelt SEL X/Y/Z readout (`eden_chrome.rs:3691`).

## 4. Full set of keys a slot can carry

From `add_slot` (`store.rs:526-558`) plus the mutators:

`id` · `squadId` · `index` · `role` · `tag`\* · `assetId`\* · `position{x,y,z,rotation}` ·
`stance` · `loadoutId` (always `Any::Null`, legacy, unused — `:1312-1313`) · `loadout` ·
`callsign` (`:1293`) · `rank` (`:1301`)   (\* omitted when empty)

`PASTE_KNOWN_SLOT_KEYS` (`store.rs:2571-2582`) pins the same set **minus `callsign`/`rank`**, which
fall through the "extras" branch (`:1496`) on paste.

`INFERRED:` there is no closed Rust struct pinning slot shape — `hydrate` loads `editor.slots`
opaquely (`store.rs:1832-1838`) and paste re-merges unknown keys, so a slot can carry arbitrary
extras from a saved payload.

## 5. Presence / absence — the Eden-parity answer

**Present:** rotation (editable), stance, role, tag, X/Y/Z, loadout.
**Present but NOT in this modal:** `callsign` and `rank` — both live only in the **ORBAT Manager**
inspector (`orbat_manager.rs:1314-1333`, `:1336-1355`).

**NOT FOUND anywhere in the frontend** — each verified by word-boundary grep, zero hits:

> Variable Name · init/script/expression/condition · size/scale · placement radius · skill ·
> health/damage · fuel · ammo (as an attribute) · lock · playable/isPlayer toggle · role description ·
> face · unit name · enable simulation · dynamic simulation · simple object · hide object ·
> allow damage · stamina · revive · local only · door states

**Multi-select edit with per-field checkboxes — the opposite is implemented.** Multi-selection
**suppresses the modal entirely**: `editor_ops.rs:583-585` (`if ctx.selection.borrow().len() > 1 {
return; }`) in `open_attributes`, and identically at `:605-607` in `open_arsenal`. `attributes.rs`
contains **zero** checkbox inputs.

## 6. Scope limits that shape the tickets

- **Slots only.** The dbl-click pick is `select_tool::pick` over the **slot** SoA
  (`mission_editor.rs:1963` → `select_tool.rs:128-130`). Placed **vehicles** (`vehiclesById`),
  **objects** (`entitiesById`) and **zones** never open the Attributes modal.
- **Vehicles have a separate, thinner surface** in the Vehicles dock: a `Heading°` number input
  (`eden_chrome.rs:2088-2104`) and cargo rows (`:2126-2186`). **No X/Y/Z, no stance, no identity.**
- **No slider anywhere per-entity** — the only two `type="range"` are mission-level (time scrubber
  `eden_chrome.rs:1084`, hillshade `:4143`).
- Modal auto-closes if the slot is undone away (`attributes.rs:53-57`).

---

## Implication

Eden defines **93 `ATTR-FIELD-*` ids**. TBD has **nine per-slot fields plus a loadout**, on one
entity type, with no multi-edit and no per-entity checkbox anywhere. The parity table's claim to
cover this surface with **3 rows** is the under-scoping this sweep exists to fix.

**One defect found in passing** that deserves its own ticket regardless of parity work: modal text
fields commit **one undo step per keystroke** (`attributes.rs:246`, `on:input`). Note the ORBAT
Manager's `callsign`/`rank` inputs commit `on:change` instead — two identity surfaces with
different semantics, so a fix must not flatten them into one.

The Z-reset originally listed here as a second defect **was a misreading** — see §Transform.
