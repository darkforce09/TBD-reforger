# Document command transport

The browser half of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s document
commands: Save Version of the [mission](/documentation/glossary/g_to_m.md#mission), the two exports
and the clipboard previews, each as the request, download or harness reading around a decision the
map engine makes. The parent module,
`apps/frontend/src/workspaces/editor/session/document_commands.rs`, declares these files inside
its wasm-only `imp` module and re-exports them.

## Contents

```text
apps/frontend/src/workspaces/editor/session/document_commands/imp/
├── clipboard.rs       the previews of what a copy of the selection would hold, for the harness
├── compilation.rs     the compiled mod document of the live mission, with its findings
├── exports.rs         the "Export JSON" and "Export Compiled" downloads, the double-activation guard
└── mission_saving.rs  the "Save Version" POST, its refusals and outcome; the JSON download helper
```

## How it works

Every command reads the document through the parent's `EDITOR_CTX` (the shared `DocHandle`, the auth
store, the mission id and the current-semver signal), which the canvas mount installs with
`set_ctx`.

- `save_now` refuses in review mode and refuses a document with duplicate
  [slot](/documentation/glossary/n_to_z.md#slot) ids before it compiles anything, then compiles the
  save-shaped payload (the `orbat` left out, since the server derives it) and sends
  `POST /api/v1/missions/{id}/versions` with `{semver, editor_notes, payload}`. On success it clears
  the dirty flag, drops the hydrate's conflict backups and moves the current semver; the status
  reads "Saved v…", "Version … already exists" (409), "Payload too large" (413), "Sign in to save"
  (401), or the error's headline with one finding per problem.
- `export_now` downloads `mission-<id>.json`, the `MissionExport` envelope: the editor superset with
  the `orbat`, which re-imports into the editor and which the
  [mod](/documentation/glossary/g_to_m.md#mod) cannot load.
- `export_compiled_now` downloads `mission-<id>.compiled.json`, the compact mod document that
  `flatten_mod_document_json_with_diagnostics` compiles from the mission row (under the live
  document's title) and the save-shaped payload, as the
  [artifact](/documentation/glossary/a_to_f.md#artifact) compile does; it refuses while the row never
  arrived, naming an expired sign-in apart from a missing row, publishes the findings to the
  validation panel, and says in the toast when unsaved edits make it differ from any saved version.
  `begin_export_gesture` drops a second activation carrying the same DOM event timestamp.
- `export_preview_json` returns what a copy of the selection's grid position, classnames or
  summary would hold, refusing an empty selection. No surface of the editor copies the selection
  yet; the harness reads the previews, and merges a payload straight through the hosted document,
  via `window.__editorCommands`.

## Boundaries

- Depends on: the parent's context cells and its re-exports of
  `mission_editing_commands::document_text` (export text, merge report, selection digest);
  `mission_payload`, `mission_compiler` and `mission_validation`; the `DocHandle` and the
  undo driver in `apps/frontend/src/workspaces/editor/bridge/document_host/`;
  `session::review_mode` and `session::hydrate::clear_local_backups`; the validation panel's
  `publish_compile_findings`; `crate::foundation` (`api_post`, the toasts, `MissionDetail`);
  `web_sys` for the Blob download. Over HTTP: `POST /api/v1/missions/{id}/versions`.
- Used by: through the parent's re-exports, the top strip's save dialog and export buttons in
  `apps/frontend/src/workspaces/editor/ui/docks/top_strip/`; the
  [Arsenal](/documentation/glossary/a_to_f.md#arsenal) tab's loadout download (`download_json`) in
  `apps/frontend/src/workspaces/editor/arsenal/tab_content.rs`; the parent's `__editorCommands`
  bridge; the source contracts in
  `apps/frontend/src/workspaces/editor/session/tests/document_commands/`.
- Rules: the tests in
  `apps/frontend/src/workspaces/editor/session/tests/document_commands/` pin these: the save
  checks duplicate slot ids before it compiles or posts
  (`save_now_checks_duplicates_before_it_compiles_or_posts` in `duplicate_slot_guard.rs`); the
  compiled export is never pretty-printed, so it stays byte-comparable with the artifact document
  (`class_r_source_forbids_value_pretty_on_compiled_export` in `source_contracts.rs`).

## Related documentation

- [Mission Creator feature inventory](/documentation/apps/frontend/workspaces/editor/feature_inventory/README.md)
  — the Save Version dialog and the exports.
- [Mission Creator feature inventory: top command strip](/documentation/apps/frontend/workspaces/editor/feature_inventory/top_command_strip.md) — Save Version, the exports and the merge that has no surface.
