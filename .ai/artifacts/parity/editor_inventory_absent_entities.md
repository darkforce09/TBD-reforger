# Live editor inventory — markers, triggers, waypoints, comments, compositions

**Provenance:** produced 2026-08-01 by a sub-agent of the attributes sweep. The parent agent hit the
session limit before writing its own artifact, so **this was recovered from the sub-agent's report
rather than from a file it wrote.** Per this program's own rule, that makes it *weaker evidence than
a normal artifact* — every claim below carries a `file:line`, and those should be spot-checked
before anything load-bearing rests on them. It is preserved because the alternative was losing it.

**Scope:** what the live editor actually has for the five Eden entity families that
`eden/gap_analysis.md` lists as `missing`. Ground truth is the source, not the docs.

---

## 1. Markers — UI stub, an unreachable doc mutator, and a shipped *mod* consumer

Three separate things are called "markers" in this repo and only one of them works. The distinction
is the finding.

### Editor UI — a stub, and pinned as one

| | |
|---|---|
| Tab button | `eden_chrome.rs:3046` — `tab_btn(2, "Markers")` |
| Match arm | **none** — falls through to `_ =>` at `eden_chrome.rs:3335-3340` |
| Entire body | `eden_chrome.rs:3337` — the text `"Marker placement lands in T-069."` |
| Pinned by test | `eden_chrome.rs:4499-4502` asserts `SRC.contains(&stub("Marker", "T-069"))`, message `"the Markers stub is out of scope and must be untouched"` |

`PaletteKind` (`eden_chrome.rs:1833`) has only `Character` / `Vehicle` / `Object` (arms 1944-1950).
**No marker kind, no place path, no tool, no render.**

### Document entity — exists, is a *briefing* marker, and nothing can call it

- `MissionDocCore::set_faction_briefing_marker` — `crates/map-engine-core/src/doc/store.rs:2030`
- `remove_faction_briefing_marker` — `store.rs:2070`
- Both write `factionsById[fid].briefing.markers[]` (`store.rs:2053`, `2082`)

**Zero product callers.** A repo-wide grep for `set_faction_briefing_marker` /
`remove_faction_briefing_marker` / `briefing_marker` hits **only `store.rs`** — the definitions plus
its own tests. `editor_ops.rs`, the frontend's entire doc-mutation surface (~75 `pub fn`), contains
**no marker function of any kind**. No wasm binding in `js.rs` or `lib.rs`. **The mutator is
reachable only from native test code.**

A separate root `markers` map also exists (`store.rs:355` emits `markersById`) and `store.rs:2007-2018`
documents it as **authoritative for nothing** — `flatten_to_mod_document` declares no root key, so
it is a closed hydrate→emit loop. Its own comment: *"Author here, not there."*

### Schema — nested under briefing only, far thinner than Eden

`$defs/marker` — `mission.schema.json:584-663`. Shape is `{x, z, icon, label}`, all four `required`,
`additionalProperties: false`. Reachable only via `$defs/briefing.markers` (`:578-581`). **There is
no top-level `markers`.** `icon` is a closed 64-alias enum (`:594-659`).

**The Eden gap in one line:** no shape, brush, colour, alpha, size or rotation, and no
area/ellipse/rectangle marker. Icon + text + position only.

### Mod side (T-181) is real, and is a different feature

`apps/mod/tbd-framework/Scripts/Game/TBD/Markers/` — 5 files, 1307 lines.
`TBD_MarkerService.BuildForPlayer` (`TBD_MarkerData.c:82`) reads `briefing.markers` out of the
**already-compiled** mission (`:131-138`) and sends the caller's side's rows.
`TBD_MarkerClient.c:4-10` hands rows to Reforger's own `SCR_MapMarkerManagerComponent`.

**The distinction that matters:** the mod is the **consumer**; the editor is the **producer**, and
the producer does not exist. Today a marker can enter a document only by hand-editing JSON or from
Rust test code. **T-181 shipping "markers" does not mean the editor can author one.**

### Ticket status, and a stale spec

`T-069` is `deferred`. Its spec `t069_markers_on_map.md:14` says *"`MapMarker` exists in
`state/schema.ts`"* — **that premise is dead**: `grep -rn "MapMarker"` returns 0 hits repo-wide and
no `schema.ts` exists (the React app was deleted at T-159.29.3). The spec needs rewriting, not just
promoting.

---

## 2. Triggers — absent

- `grep -rin "trigger" apps/website/frontend/src/ --include="*.rs"` → 9 hits, **all unrelated**:
  re-read ticks (`attributes.rs:19`, `editor_ops.rs:68`), an auth-refresh test string
  (`auth.rs:514`), JS-bridge prose (`yrs_persist.rs:8,94`), file-download triggers
  (`mission_commands.rs:5,227,258`), an encode trigger (`mission_editor.rs:1170`).
- `mission.schema.json` → 2 hits (`:429`, `:488`), both **prose inside `description` strings**.
  No trigger in `$defs` (30 defs, none named trigger).

**Nearest analogue, and it is not a trigger:** `$defs/zone` (`id, type, label, faction, shape,
rules`; type enum `spawn, objective_capture, objective_destroy, objective_hold_until, boundary,
base_protection`) + `$defs/zoneRules` (16 keys). `INFERRED:` zones give an area with typed rules,
but there is **no activation/condition/timer/effects model** — no author-written condition, no
effect list. `$defs/flow` and `$defs/winConditions` are mission-global, not per-area.

---

## 3. Waypoints — absent, totally

`grep -rin "waypoint"` across `apps/website/frontend/src/`, `crates/`, `packages/tbd-schema/schema/`
→ **exactly 2 hits**, `mission.schema.json:608` and `:609` — the strings `"waypoint"` /
`"waypoint2"` inside the marker **icon alias enum**. Glyph names, not entities.

Mod side: one file, `TBD_MarkerIcons.c` — the same icon aliases.

**No waypoint entity, no group attachment, no type / order / behaviour / completion field
anywhere.**

---

## 4. Comments (Eden annotation) — absent

Every `Comment` hit in `apps/website/frontend/src/` and `crates/` is either prose about
source-code comments (`mission_title_prefer.rs:467,582`, `event_manager.rs:1611`, `arsenal.rs:4011`,
`store.rs:3752`) or the social feature below.

**The social feature is not an Eden comment.** `missions.rs:2394-2400` renders a "Comments" button
under a `"Collaboration"` heading on the mission *detail page*; `missions.rs:2501-2513` is a Sheet
reading `"Comments coming soon."`, sourced `// Comments — empty-state shell (no API yet)`. That is a
per-mission discussion thread stub, **not a canvas annotation with title/tooltip/position**.

`grep -in "comment" mission.schema.json` → **0 hits**.

---

## 5. Compositions — not found as a feature

All 0 hits across frontend, crates and api for: `save.*[Cc]omposition`, `[Cc]omposition.*save`,
`save_group`, `save_as`, `SaveComposition`, `user_composition`, `custom_composition`.

The only frontend `composition` occurrences are `asset_catalog.rs:345,354,362,782,784,837,844` — a
**path-string classifier**. `derive_object_alias` (`:352`) prefixes an alias `comp:` vs `prop:`
based on whether the Arma resource path contains `"Composition"` (`:362-367`). **That consumes
Bohemia's shipped composition prefabs by name; it does not author one.**

No title/author/category metadata. No schema def — `mission.schema.json:72` mentions compositions
only in prose describing `entities[]`, and that same description states `entities[]` is read by
nothing: *"NOTHING READS THIS ON ANY BUILD SHIPPED TODAY… This array is a CONTRACT AHEAD OF ITS
CONSUMER… Do not cite it as a feature."*

---

## 6. Asset palette tabs — the real state of `DockRight`

Component at `eden_chrome.rs:2960`; buttons rendered `3041-3046`. Note the tab **indices** are
0,1,3,2 while **visual order** is Factions, Vehicles, Zones, Markers — Zones was deliberately placed
ahead of the Markers stub (comment at `:3043-3044`).

| Pos | Label | Button | Arm | State |
|---|---|---|---|---|
| 1 | Factions | `:3041` | `0 =>` `:3058` | **LIVE** — asset browser, side chips, search, character placement |
| 2 | Vehicles | `:3042` | `1 =>` `:3247` | **LIVE** (T-215) — `editor_ops::begin_place_vehicle` (`editor_ops.rs:1044`) |
| 3 | Zones | `:3045` | `3 =>` `:3334` | **LIVE** (T-582) — `zones_panel` wasm `:2228`, native stub `:2790` |
| 4 | Markers | `:3046` | *(none)* `:3335` | **STUB** — one sentence, `:3337` |

Plus a fifth mode that is a **chip, not a tab**: `EdenChip::Objects` (enum `:2875-2880`) sets
`objects_mode` (`:2925`), re-skinning tab 0 into an objects palette (`:3114-3139`, `:3160-3178`).
**LIVE.**

---

## Correction recorded by the sub-agent

`feed_cluster_markers` / `set_cluster_markers` at `crates/map-engine-render/src/engine.rs:3736,
3741, 3906` are **not** Eden markers — they are slot LOD cluster discs (T-173 H2), confirmed by
reading `3739-3760` (`slots_gpu::cluster_mode`, `ClusterIndex::build` over slot positions). A grep
for "marker" in the render crate will hit these and must not be read as marker rendering.

---

## What this means for ticketing

Five Eden families, and **all five are absent from the editor as an authoring surface.** Markers are
the subtlest and the most dangerous to mis-plan: the schema, the mod consumer and a doc mutator all
exist, so a grep-level survey reads as "mostly built". The producer — the thing a mission maker
touches — does not exist at all.

Existing tickets: `T-069` (markers, `deferred`, stale spec) · `T-213` (marker placement, `idea`) ·
`T-079` (triggers + waypoints + systems, `idea`) · `T-078` (custom compositions, `deferred`).
Editor comments have **no ticket** — they were T-651 in the draft set only.
