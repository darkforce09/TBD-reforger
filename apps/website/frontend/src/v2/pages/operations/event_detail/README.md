# Operation Dossier & Slotting (`/events/:id`)

One operation: when it starts, what its missions are, and which slot the caller takes in each.

## Architecture
- **`page.rs`**: the route component. Reads the operation id from the path, fetches
  `GET /events/:id`, and wraps the hub body in this route's chrome — the topographic backdrop,
  the scroll surface and the link back to the schedule.
- **`hero_countdown.rs`**: the hub body, shared with the schedule's detail column. The hero
  section — operation name, T-minus clock, local start time, briefing, voice-server chip and the
  modpack link — and the list of mission dossiers under it.
- **`mission_dossier.rs`**: one mission card — heading, terrain/mode/time line, meta badges,
  registration state and fill counts, briefing, faction grid and the slotting selector — plus the
  label rules, the briefing empty rule and the modpack-fetch choice the hero reads with.
- **`faction_armory.rs`**: one faction's card — the uniform silhouettes and the armory the
  mission serves for that faction — and the side ordering every faction list on the page uses.
- **`slotting_selector.rs`**: the order-of-battle selector. Fetches
  `GET /event-missions/:emid/orbat`, resolves the active faction and squad, and renders the
  faction tabs, the squad list, the slot pane and the footer action bar, with register and
  withdraw wired to the backend.
- **`squad_pane.rs`**: the selected squad — its reservation header with reserve and release, its
  slot rows with occupant or availability marker, and the reservation rules the footer line and
  the register button both read.
- **`assign_picker.rs`**: the inline member typeahead a squad manager fills an empty slot with.
- **`tests/event_hub.rs`**: the briefing empty rule and where it is bound, the meta badges, the
  modpack chip's fetch choice, the tier checks behind the slotting affordances, and the avatar
  sink in the member picker.

## Not present in the legacy page
- **AO satellite preview and weather forecast**: the dossier header carries terrain, game mode
  and start time as text. There is no map thumbnail and no weather on this page.
- **Commander intent**: the mission dossier renders one authored briefing, and an explicit
  "no briefing provided" affordance when it is blank. There is no separate intent field.
- **Equipment loadout preview and vehicle allocations**: the faction card shows uniform
  silhouettes and the armory item list the mission serves. The dossier carries no loadout
  preview and no vehicle roster, so neither is rendered — not even as an empty state, which
  would advertise an authoring path that does not exist.
- **Objective lists**: same reason. There is no objectives field on the wire and no writer for
  one.
