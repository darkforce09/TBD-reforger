# Mission Overview Page (`/missions/:id`)

*Replaces the 1,412-line `editor/library/mission_overview.rs` file.*

## Architecture
- **`page.rs`**: Main layout container.
- **`header.rs`**: Scenario title, author, version, and primary [ Launch Editor ] button.
- **`intel_briefing.rs`**: Tabbed operational briefing (Situation, Mission, Execution, Admin & Logistics).
- **`map_preview.rs`**: Interactive terrain minimap showing initial spawn zones.
- **`slot_census.rs`**: BLUFOR / OPFOR / INDFOR role breakdown table.
