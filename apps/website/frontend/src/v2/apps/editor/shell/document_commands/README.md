# Document commands

The files of the browser-only half of the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s document commands (Save Version,
the exports, the merge and the clipboard exporters). The parent module,
`apps/website/frontend/src/v2/apps/editor/shell/document_commands.rs`, declares the inline module
`imp` whose child files live here.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/shell/document_commands/
└── imp/  the transport of Save Version, the exports, the merge and the clipboard exporters
```

## How it works

`document_commands.rs` re-exports the map engine's pure command helpers
(`website_map_engine::editing::commands`: the export text, the merge report, the selection digest)
in every build, and declares `mod imp` for the `wasm32` build only; Rust reads the children of that
inline module from `imp/`, so this folder holds nothing else. The parent's `imp`
keeps the command context (`EDITOR_CTX`), the [mission](/documentation_v2/glossary/g_to_m.md#mission) row
the compiled export needs (`ROW_META`, `ROW_HYDRATE`) with the setters the hydrate and the review
restore call, and `register_editor_commands`, which publishes `window.__editorCommands`
(`compile_save_json`, `compile_export_json`, `compiled_document_json`, `compiled_diagnostics_json`,
`merge_mission_json` and the three `clipboard_*_json` previews) for the headless harness.

## Public surface

None: the items of `imp/` cross the boundary only through the parent's `pub use imp::*`.

## Boundaries

- Depends on: what `imp/` uses (see its README): the map engine's compile, the document host, the
  session's review mode and hydrate, and the frontend core's
  [API](/documentation_v2/glossary/a_to_f.md#api) client and toasts.
- Used by: the parent module `apps/website/frontend/src/v2/apps/editor/shell/document_commands.rs`.
- Rules: a command file joins `imp/` and is declared inside the parent's `mod imp`, which stays
  `#[cfg(target_arch = "wasm32")]`; what a command decides belongs to
  `website_map_engine::editing::commands`, never to this folder.

## Related documentation

- [Mission Creator feature inventory: data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md) — the compile behind Save Version and the exports.
