# Scenario Creation Wizard (`create_dialog`)

## Responsibilities
- Dialog modal host.
- Scenario metadata inputs (Title, Terrain selector, Author attribution, Game mode preset).
- Initializes empty scenario and redirects to `/missions/:id/edit`.

## Files
- **`dialog.rs`**: the whole dialog — fields, submit and reset.

**Not present in the legacy dialog:** author attribution. The author is the signed-in session, not
a field. The form is one step, not a multi-step wizard, and it also carries weather, time of day,
a player cap and an optional library blurb.
