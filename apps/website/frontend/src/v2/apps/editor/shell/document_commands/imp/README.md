# Document command transport

The browser half of the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s document
commands: Save Version, the two exports, the merge of another
[mission](/documentation_v2/glossary.md#mission) and the clipboard exporters, each as the request,
download, clipboard write and toast around a decision the map engine makes. The parent module,
`apps/website/frontend/src/v2/apps/editor/shell/document_commands.rs`, declares these files inside
its wasm-only `imp` module and re-exports them.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/shell/document_commands/imp/
├── clipboard.rs       the clipboard exporters of the selection, and the previews the harness reads
├── compilation.rs     the compiled mod document of the live mission, with its findings
├── exports.rs         the "Export JSON" and "Export Compiled" downloads, the double-activation guard
├── mission_merge.rs   the list of the author's other missions, and the merge of one into this one
└── mission_saving.rs  the "Save Version" POST, its refusals and outcome; the JSON download helper
```

## How it works

Every command reads the document through the parent's `EDITOR_CTX` (the shared `DocHandle`, the auth
store, the mission id and the current-semver signal), which the canvas mount installs with
`set_ctx`.

- `save_now` refuses in review mode and refuses a document with duplicate
  [slot](/documentation_v2/glossary.md#slot) ids before it compiles anything, then compiles the
  save-shaped payload (the `orbat` left out, since the server derives it) and sends
  `POST /api/v1/missions/{id}/versions` with `{semver, editor_notes, payload}`. On success it clears
  the dirty flag, drops the hydrate's conflict backups and moves the current semver; the status
  reads "Saved v…", "Version … already exists" (409), "Payload too large" (413), "Sign in to save"
  (401), or the error's headline with one finding per problem.
- `export_now` downloads `mission-<id>.json`, the `MissionExport` envelope: the editor superset with
  the `orbat`, which re-imports into the editor and which the
  [mod](/documentation_v2/glossary.md#mod) cannot load.
- `export_compiled_now` downloads `mission-<id>.compiled.json`, the compact mod document that
  `flatten_mod_document_json_with_diagnostics` compiles from the mission row (under the live
  document's title) and the save-shaped payload, as the
  [artifact](/documentation_v2/glossary.md#artifact) compile does; it refuses while the row never
  arrived, naming an expired sign-in apart from a missing row, publishes the findings to the
  validation panel, and says in the toast when unsaved edits make it differ from any saved version.
  `begin_export_gesture` drops a second activation carrying the same DOM event timestamp.
- `merge_mission_now` reads `GET /api/v1/missions/{id}`, merges that payload into the open document
  as one undo step and runs `after_local_edit`; `other_missions` lists
  `GET /api/v1/missions?scope=mine` without the open mission. The clipboard exporters copy the
  selection's grid position, classnames or summary through
  `crate::v2::core::utils::clipboard::write_clipboard`, and `export_preview_json` returns what each
  would copy. No surface of the editor calls the merge, the mission list or the clipboard exporters;
  the harness reaches the merge and the previews through `window.__editorCommands`.

## Boundaries

- Depends on: the parent's context cells and its re-exports of
  `website_map_engine::editing::commands` (export text, merge report, selection digest);
  `website_map_engine::data::scenario` (`compile`, `flatten`, `validate`); the `DocHandle` and the
  undo driver in `apps/website/frontend/src/v2/apps/editor/bridge/document_host/`;
  `shell::review_mode` and `shell::hydrate::clear_local_backups`; the validation panel's
  `publish_compile_findings`; `crate::v2::core` (`api_get`, `api_post`, the toasts, the clipboard
  helper, `MissionDetail`); `web_sys` for the Blob download. Over HTTP:
  `POST /api/v1/missions/{id}/versions`, `GET /api/v1/missions/{id}` and
  `GET /api/v1/missions?scope=mine`.
- Used by: through the parent's re-exports, the top strip's save dialog and export buttons in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/`; the
  [Arsenal](/documentation_v2/glossary.md#arsenal) tab's loadout download (`download_json`) in
  `apps/website/frontend/src/v2/apps/editor/arsenal/tab_content.rs`; the parent's `__editorCommands`
  bridge; the source contracts in
  `apps/website/frontend/src/v2/apps/editor/shell/tests/document_commands/`.
- Rules: the tests in
  `apps/website/frontend/src/v2/apps/editor/shell/tests/document_commands/` pin these: the save
  checks duplicate slot ids before it compiles or posts
  (`save_now_checks_duplicates_before_it_compiles_or_posts` in `duplicate_slot_guard.rs`); the
  compiled export is never pretty-printed, so it stays byte-comparable with the artifact document
  (`class_r_source_forbids_value_pretty_on_compiled_export` in `source_contracts.rs`); the merge
  runs the post-edit tail (`class_r_merge_mission_now_runs_the_after_local_edit_tail`, same file).

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the Save Version dialog and the exports.
