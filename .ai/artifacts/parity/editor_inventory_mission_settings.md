# Live editor inventory — Mission Settings, top strip, and the scenario-attribute surface

**Provenance:** produced 2026-08-01 by a sub-agent of the attributes sweep. The parent hit the
session limit before writing its artifact; **recovered from the sub-agent's report, not from a file
it wrote.** Spot-check the `file:line` citations before resting anything load-bearing on them.

**Headline:** the scenario-level surface is **small and deliberately gated** — 1 display-only field,
10 editable controls, 12 world-layer checkboxes. Of Eden's scenario attributes only **time**,
**weather** and **title** exist, and most of the rest are absent **by an enforced code gate, not by
oversight.** That distinction decides whether a parity row is a ticket or a closed question.

---

## The gate — read this before proposing any scenario-attribute ticket

Every document write in Mission Settings goes through `author_env` (`eden_chrome.rs:271`), which
**refuses any key not in `CARRIED_ENV_KEYS` (`:231`) or `AUTHORED_FLOW_KEYS` (`:332`)** and logs an
error instead (`:272-277`).

`CARRIED_ENV_KEYS` — pinned at exactly 5 by test `:4648-4652`: `time` · `weather` ·
`showHillshade` · `hillshadeOpacity` · `showGrid`.
`AUTHORED_FLOW_KEYS` — pinned to exactly 4 by test `:4700-4711`: `briefingSeconds` ·
`safeStartSeconds` · `timeLimitSeconds` · `jip`.

A test named `fields_with_no_mod_reader_get_no_control` (`:4686-4694`) asserts `nightVision` is not
authorable. `eden_chrome.rs:4624` actively refuses `viewDistance`, `thermals`, `windDirDeg`, `fog`,
`wind`. The rationale is at `:4619-4621`: *"the schema HAS a slot for it… A schema field is not a
reader."*

**So "Eden has it and we don't" is not automatically a gap here.** Several were built, then
deliberately removed.

---

## Mission Settings dialog — complete inventory

Component at `eden_chrome.rs:3788`, mounted `mission_editor.rs:2096`.

### Environment
| # | Label | Widget | Key | Editable |
|---|---|---|---|---|
| 1 | **Terrain** | plain div `:3841-3843` | reads `meta.terrain` | **display-only** |
| 2 | **Time** | `<input type="time">` `:3850-3868` | `environment.time` | yes |
| 3 | **Weather** | `<select>` `:3877-3895` — Clear/Overcast/Heavy Rain/Dense Fog | `environment.weather` | yes |

`ENV_UNCARRIED_NOTE` (`:289`): *"View distance and thermals are not part of a compiled mission — it
carries time and weather only."*

### Mission flow (`render_flow_section` `:3927`)
| # | Label | Key | Default |
|---|---|---|---|
| 4 | Mission duration | `timeLimitSeconds` | 5400 (`:381`) |
| 5 | Briefing | `briefingSeconds` | 600 (`:377`) |
| 6 | Safe start | `safeStartSeconds` | 300 (`:379`) |
| 7 | Join in progress | `jip` — Disabled / Until safe start ends / Always | `:383` |

All commit on `on:change` (blur/Enter), not `on:input` — rationale `:3919-3924`. Invalid duration is
**refused and the box reverted** (`:3980-3999`).

### Render prefs (`render_prefs_section` `:4057`)
| # | Control | Storage |
|---|---|---|
| 8 | Basemap — Satellite / Map | **localStorage** `tbd-mc-basemap-view` |
| 9 | Show hillshade | document |
| 10 | Hillshade strength 0-100% | document |
| 11 | Grid | document |
| 12-23 | **12 world-layer checkboxes** | **localStorage** `tbd-mc-world-layers` |

The 12 layers (`world_layer_prefs.rs:61-76`): Roads, Buildings, Forest mass, Trees, Props,
**Contours**, Sea, Fences, Airfield, **Height labels**, Town labels, Road names. `props` defaults
off, rest on (`:39-56`).

> Note for the contour tickets: **`Contours` and `Height labels` are already toggleable layers.**
> The contour work (T-639/640/641) modifies existing layers, not new ones.

---

## Presence / absence — scenario attributes

| Attribute | Verdict |
|---|---|
| **Title** | present — **top strip only**, not in Mission Settings (`eden_chrome.rs:1047-1063`) |
| **Time / Weather** | present, **two surfaces each** (dialog + top strip), both mirror to the row |
| **Author** | not in the editor — server-assigned `author_id` (`dto.rs:828`) |
| **Description / briefing** | **not editable anywhere in the SPA.** Reaches the doc only via hydrate (`store.rs:1658-1663`). `PATCH /missions/:id` accepts `briefing` (`handlers/missions.rs:657`) but **no frontend caller sends it** |
| **Picture / thumbnail** | not in the editor; DTO + API exist (`dto.rs:850`, `handlers/missions.rs:658`) |
| **Fog** | not independent — only the `dense_fog` weather preset. Gated `:4624` |
| **Wind** | gated `:4624` — schema declares `windDirDeg`, nothing reads it |
| **View distance** | **removed (T-193)**, was working. Residual dead DTO field `dto.rs:991`, still parsed `editor_ops.rs:205-208` |
| **Thermals** | **removed (T-193)**. Residual dead field `dto.rs:992` |
| **Night vision** | never built, refused by design (test `:4686-4694`) |
| **DLC / addon requirement** | not found |
| **Respawn / spectator / tickets** | **absent by design** — `SETTINGS_UNREAD_NOTE` `:370`, rationale `:4672-4685` (*"TBD events are one life"*) |
| **Player count** | create-dialog only (`create_mission_dialog.rs:199-215`); compiled from the **row** (`dto.rs:895`) |
| **Side selection** | present but **entity-scoped** — `EDEN_SIDE_CHIPS` (`:2871`) sets *placement* side in `DockRight`, not a mission attribute |
| **Garbage collection** | not found |
| **Terrain selection** | display-only in editor; chosen at create time (`create_mission_dialog.rs:126-151`) |

**Two dead DTO fields worth a cleanup ticket:** `MissionEnv.view_distance` and `.thermals` have no
writer since T-193 but are still parsed at `editor_ops.rs:205-212`.

---

## `meta.*` — the complete key set

From every `self.meta.insert` in `store.rs`:

| Key | Editor-reachable? |
|---|---|
| `meta.title` | **yes** — strip input |
| `meta.terrain` | no — display-only |
| `meta.environment` | **yes** — via `author_env` only |
| `meta.briefing` | no — hydrate-from-row only |
| `meta.schemaVersion`, `meta.map` | no |
| `meta.id` | **no live writer** — `seed_meta` (`store.rs:1667`) has no production caller |

---

## An inaccuracy in the existing source comments

`eden_chrome.rs:310-314` claims *"`compile_payload` … builds the saved version out of exactly two
meta keys — `meta.terrain` and `meta.environment` — and `hydrate` restores exactly those two."*

**Stale.** `compile_payload` reads **five**: `terrain` (`mission/compile.rs:156`), `map` (`:169-173`),
`schemaVersion` (`:180-183`), `environment` (`:187-191`), `title` (`:251`). `hydrate` restores all
five (`store.rs:1750`, `1753`, `1759`, `1764`, `1768`). The comment's *conclusion* still holds; its
count does not.

---

## Create-time vs editor — which surface owns which field

| Field | Create dialog | Editor | Verdict |
|---|---|---|---|
| Title | yes | yes | both — but the editor title does **not** PATCH the row |
| Terrain | yes | read-only | create-time only |
| Game mode | yes | **not found** | create-time only |
| Time / Weather | yes | yes | both — editor mirrors back to the row |
| Max players | yes | **not found** | create-time only |
| Description | **no** | **no** | **neither surface** — API-only |
| Thumbnail | **no** | **no** | **neither surface** — API-only |

`RowMirror` PATCHes only two columns — `time_of_day` and `weather` (`eden_chrome.rs:501-508`),
400 ms debounce (`:517`).

---

## Implication for ticketing

The gap here is **not** "add Eden's scenario attributes". Most were considered and refused, with
tests pinning the refusal. The real gaps are narrower and better-founded:

1. **Description/briefing is uneditable in the entire SPA** despite the API accepting it — the
   single clearest hole, and it blocks FNF-style derived briefings.
2. **Game mode and max players are create-time-only**, so a mission's shape cannot be changed after
   creation.
3. **Two dead DTO fields** (`view_distance`, `thermals`) still parsed after T-193 removed them.
4. **`meta.id` has no live writer**, so `compile_export` falls back to the route id
   (`mission/compile.rs:323-327`).
