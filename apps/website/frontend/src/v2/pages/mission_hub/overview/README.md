# Mission Overview Page (`/missions/:id`)

*Replaces the 1,412-line `editor/library/mission_overview.rs` file.*

## Architecture
- **`page.rs`**: Main layout container.
- **`header.rs`**: Scenario title, author, version, and primary [ Launch Editor ] button.
- **`intel_briefing.rs`**: Tabbed operational briefing (Situation, Mission, Execution, Admin & Logistics).
- **`map_preview.rs`**: Interactive terrain minimap showing initial spawn zones.
- **`slot_census.rs`**: BLUFOR / OPFOR / INDFOR role breakdown table.

## Files
- **`page.rs`**: the route — the fetch, the edit predicate and the layout, with the review record
  (`mission_review/review_record.rs`) under the dossier for the author and administrators: every
  review with its decision, conditions or rejection reason, the approved artifact, the thread with
  a reply box, and each artifact's read-only review workspace link.
- **`header.rs`**: the title, the attribution line and the Edit Armory button.
- **`dossier_body.rs`**: the shared read-only dossier — badges, detail grid, faction armory — plus
  the formatters the mission library reads it through.
- **`intel_briefing.rs`**: the tactical briefing section.
- **`armory_editor.rs`** / **`armory_dialog.rs`**: the Edit Armory dialog, which is the page's one
  authoring surface; it lives outside the shared body deliberately.

**Not present in the legacy page:** a map preview (no minimap is rendered, so there is no
`map_preview.rs`) and a slot census (no per-side role breakdown is rendered, so there is no
`slot_census.rs`). The briefing is a single section, not a tabbed one.
