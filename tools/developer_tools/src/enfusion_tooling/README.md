# Enfusion script oracle and MCP broker

The library behind two binaries. For `enf`, it turns
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) script sources (the gitignored
`crf_framework` and `vanilla_reference` lanes of the
[reference lanes](/apps/mod/References/README.md)) into committed TSV symbol indexes,
answers lookups against them, and checks the `@idx` citations in `documentation/` and the
framework capability verdicts. For `mcpd`, it runs the persistent enfusion-mcp broker that `cargo
xtask mcp` talks to, and it resolves which enfusion-mcp server every caller starts.

## Contents

```text
tools/developer_tools/src/enfusion_tooling/
├── apidoc.rs                   `enf apidoc`: the cached Script API HTML pages to `vanilla_api_*.tsv`
├── capability.rs               `enf capability`: the framework index joined with the verdict table
├── carve.rs                    `enf carve`: script-shaped text recovered by scanning the raw paks
├── citations.rs                `enf citations`: every `@idx lane#Symbol` marker checked against an index
├── cli.rs                      the `enf` clap command tree, its dispatch and exit codes
├── enfusion_mcp_entrypoint.rs  which enfusion-mcp server command a caller starts, in four tiers
├── index.rs                    `enf index`, `lookup` and `dirs`: the TSV index writer and its readers
├── mcp_broker.rs               `mcpd`: the Unix-socket broker over one enfusion-mcp child, and its stub
├── mod.rs                      the module tree and `refuse_empty_write`, the empty-index guard
├── reference_output.rs         the output guard of `carve`, `extract` and `source`: inside the references folder, `--replace`
├── source.rs                   `enf source`: vanilla `.c` files rebuilt from cached source HTML pages
├── symbols.rs                  the `.c` scanner: declarations, methods, `modded` classes, `RplProp` fields
└── tests/                      unit tests for the scanner, the parsers, citations, the server resolver and the output guard
```

## How it works

```text
apps/mod/References/crf_framework/ ─┐
vanilla .c tree                    ─┴▶ enf index <crf|vanilla> ─▶ symbols::scan ─▶ .ai/artifacts/enf-index/<lane>_*.tsv
game paks ─▶ enf extract (PakVfs) ─▶ apps/mod/References/vanilla_reference/Scripts/
          ─▶ enf carve            ─▶ apps/mod/References/vanilla_reference/Carved/
cached HTML ─▶ enf apidoc ─▶ vanilla_api_classes.tsv, vanilla_api_members.tsv
            ─▶ enf source ─▶ apps/mod/References/vanilla_reference/Source/
index TSVs ─▶ enf lookup | enf dirs | enf citations | enf capability ─▶ capability_matrix.tsv
```

- `index::build` scans every `.c` file under `--root` with the one scanner in `symbols.rs` and
  writes four TSVs per lane (`_symbols`, `_files`, `_modded`, `_rplprops`) that hold names and
  `file:line` coordinates, never code bodies, so they are committed while the sources stay
  gitignored. `lookup` and `dirs` read `crf_symbols.tsv` by default.
- `citations::verify` walks every Markdown file under `documentation/` and resolves each `@idx
  crf#`, `@idx vanilla#` and `@idx api#` marker against `crf_symbols.tsv`, `vanilla_symbols.tsv` and
  `vanilla_api_classes.tsv`.
- `capability::build` joins `crf_files.tsv` and `crf_symbols.tsv` with the rules in
  `documentation/mod/tbd-framework/capability_verdicts.tsv`, writes `capability_matrix.tsv`, and
  fails when any framework file matches no rule.
- `extract` reads scripts by name from the pak file table through `crate::enfusion_pak::PakVfs`;
  `carve` scans raw pak bytes instead, and `dump-entry` writes one entry's stored bytes for codec
  work.
- `index`, `apidoc`, `source` and `carve` refuse an empty result through `refuse_empty_write` before
  they write.
- `carve`, `extract` and `source` write only inside `apps/mod/References/`:
  `reference_output::checked_reference_output` refuses an output folder outside it, the folder
  itself, a path holding `..`, and a checkout without the folder. `carve` and `extract` remove a
  previous output only when given `--replace` (`reference_output::clear_previous_output`);
  without it they refuse. The defaults of every vanilla output are the lane paths in
  `crate::repository_layout`.
- `enfusion_mcp_entrypoint::resolve` picks the server command: `ENFUSION_MCP_BIN` when it names a
  file, then the module the pinned npm package installs
  (`crate::repository_layout::ENFUSION_MCP_ENTRYPOINT`), then a copy in the npx cache, then `npx -y
  enfusion-mcp`.
- `mcp_broker::run` starts that server once, initialises it, and serves tool calls over a Unix
  socket one at a time, because the [Workbench](/documentation/glossary/n_to_z.md#workbench) NetAPI
  takes one stream; it relabels each reply's id to 2 for `cargo xtask mcp consume`. It stops on
  SIGTERM or SIGINT, after `MCP_DAEMON_IDLE` seconds idle (1800) or `MCP_DAEMON_MAX_LIFE` seconds of
  life (14400), killing the child and removing the socket and pid file. `--stub` or `MCP_STUB=1`
  without `--socket` runs an offline stand-in for the server, shaped by `STUB_MODE`, `STUB_DAEMON`
  and `STUB_LINGER`, for `cargo xtask mcp selftest`.

## Boundaries

- Depends on: `crate::enfusion_pak::PakVfs` for `extract` and `dump-entry`;
  `crate::repository_layout` for the index folder, the references folder and its vanilla lane
  paths, the documentation root, the verdict table and the installed MCP module;
  `crate::repository_paths::find_repo_root` for the output guard; `tokio` for the broker; `clap` and `regex`.
- Used by:
  - `tools/developer_tools/src/bin/enf.rs` (`cli::entrypoint`) and
    `tools/developer_tools/src/bin/mcpd.rs` (`mcp_broker::run`);
  - `tools/xtask/src/commands/mcp/call.rs` and `tools/xtask/src/commands/mcp/daemon.rs`,
    through `enfusion_mcp_entrypoint::resolve` and `process_pattern`;
  - `cargo xtask fetch vanilla-api` and `cargo xtask fetch vanilla-source`, which print the `enf
    apidoc` and `enf source` step that follows them;
  - the mod wave gate in `tools/xtask/src/commands/mod_ops/wave_execution/execution.rs`, whose
    unit-test step filters on `enf::`, a path no test in this module has.
- Rules: every lane shares the one scanner in `symbols.rs` (`does_not_invent_apis` and the other
  tests in `tests/symbols/tests.rs`); a committed index is never overwritten with an empty one
  (`refuse_empty_write_reds_on_empty`); a lane output lands only inside the references folder and
  replaces a previous one only on `--replace` (`an_output_outside_the_references_folder_is_refused`,
  `a_previous_output_is_removed_only_on_request`); `enf citations` exits 1 on any unresolved marker and `enf
  capability` on any untriaged framework file; the server command is resolved only here, so `cargo
  xtask mcp call`, `cargo xtask mcp daemon` and `mcpd` start the same server
  (`an_installed_package_resolves_to_the_pinned_module`).

## Related documentation

- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — the `@idx` citations and
  the oracle gates in mod work.
- [MCP command group](/tools/xtask/src/commands/mcp/README.md) — the broker's launcher and its
  selftest.
- [Enfusion script oracle](/documentation/tools/developer_tools/enfusion_script_oracle.md) —
  the oracle tables, their lookups and the citation gate in depth.
