# Operation Dossier & Slotting (`/events/:id`)

*Replaces the 2,042-line monolithic `pages/operations/event_hub.rs` file.*

## Architecture
- **`page.rs`**: Main layout container (~250 LOC).
- **`hero_countdown.rs`**: Operation title, T-minus clock, TS3 & modpack links.
- **`mission_dossier.rs`**: Tactical mission briefing, AO thumbnail, and commander intent.
- **`faction_armory.rs`**: Side uniforms, equipment loadout preview, and vehicle allocations.
- **`slotting_selector.rs`**: The interactive ORBAT hierarchy (Factions -> Squads -> Slots) with 1-click slotting and reservations.
