# Editor command decisions

The decidable half of the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
document commands: the bytes an export writes, the wording of the compile, merge and save reports,
and the selection digests a clipboard receives. Every function is pure over its arguments; how the
result reaches a disk, a clipboard or a server belongs to the host.

## Contents

```text
apps/website/map-engine/src/editing/commands/
├── export_text.rs       compiled export bytes, compile summary, export latch, row metadata
├── merge_report.rs      the merge report and the duplicate-slot-id save refusal, as author text
├── mod.rs               the module tree
├── selection_digest.rs  selected ids resolved to entities; grid, classname, summary clipboard texts
└── tests/               unit tests pinning each wording and projection by value
```

## How it works

- **Export.** `compiled_export_text` hands the compiled bytes through unchanged, because a
  re-parsed "pretty" copy is not byte-identical to the compiled document.
  `compile_diagnostics_summary` turns the compile's findings into one line counted by severity
  (`The compile reported 1 error · 2 warnings — see the validation panel.`) and returns `None` for a
  clean compile. `row_meta_missing_message` tells a signed-out author to sign in, not to save a
  version. `export_gesture_is_duplicate` drops a second activation that carries the same
  `Event.timeStamp`, and never latches a `0.0` stamp. `apply_row_metadata_to_export` sets the JSON
  envelope's `maxPlayers` and `gameMode` from the mission row, and `live_doc_title` reads the
  trimmed, non-blank `meta.title` of the live document for the compiled export's title.
- **Reports.** `format_merge_report` phrases a merge report as a summary naming only the non-zero
  counts plus one `kind id — reason` line per skipped row; a report that does not parse yields a
  line saying so. `duplicate_slot_id_report` phrases the save refusal, naming the squad callsign and
  the [slot](/documentation_v2/glossary.md#slot) id on each line.
- **Selection digests.** `resolve_selected_entities` looks each selected id up in the slots, then
  the vehicles, then the objects of the document's JSON views, and drops ids the document does not
  hold. `grid_position_text`, `classnames_text` and `selection_summary_text` build the three
  clipboard texts; every grid comes from `format_grid_ref`, which calls
  `crate::camera::grid_reference::grid_ref_3digit`, the formatter behind the map-edge labels, so a
  pasted grid matches the one on screen. A single selection copies the bare grid (`032 048`).

## Boundaries

- Depends on: `crate::camera::grid_reference` for the three-digit grid reference,
  `crate::data::scenario::validate` (`Finding`, `Severity`) for the compile summary, and
  `serde_json`.
- Used by: the Mission Creator's document commands
  (`apps/website/frontend/src/v2/apps/editor/shell/document_commands.rs`, which re-exports all
  three modules) and its exporter test
  (`apps/website/frontend/src/v2/apps/editor/shell/tests/exporter_grid_reference.rs`, which checks
  `format_grid_ref` against the map-edge labels).
- Rules: the compiled export is byte-identical to its input
  (`class_r_compiled_export_is_byte_identical_to_wire` in `tests/export_text.rs`); a clean compile
  has no summary (`a_clean_compile_produces_no_diagnostics_summary`); a zero stamp never latches
  (`t799_zero_stamp_never_latches`); an unparseable merge report degrades to a named line
  (`class_r_merge_report_unparseable_degrades` in `tests/merge_report.rs`); nothing here names a
  browser, clipboard or network type.

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the export, merge and clipboard features these decisions back.
