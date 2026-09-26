# Enfusion script oracle and MCP broker

The library behind two binaries. For `enf`, it turns
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) script sources (the gitignored upstream framework
in `apps/mod/crf_framework/` and the vanilla game scripts) into committed TSV symbol indexes,
answers lookups against them, and checks the `@idx` citations in `documentation_v2/` and the
framework capability verdicts. For `mcpd`, it runs the persistent enfusion-mcp broker that `cargo
xtask mcp` talks to, and it resolves which enfusion-mcp server every caller starts.

## Contents

```text
tools_v2/developer-tools/src/enfusion_tooling/
├── apidoc.rs                   `enf apidoc`: the cached Script API HTML pages to `vanilla_api_*.tsv`
├── capability.rs               `enf capability`: the framework index joined with the verdict table
├── carve.rs                    `enf carve`: script-shaped text recovered by scanning the raw paks
├── citations.rs                `enf citations`: every `@idx lane#Symbol` marker checked against an index
├── cli.rs                      the `enf` clap command tree, its dispatch and exit codes
├── enfusion_mcp_entrypoint.rs  which enfusion-mcp server command a caller starts, in four tiers
├── index.rs                    `enf index`, `lookup` and `dirs`: the TSV index writer and its readers
├── mcp_broker.rs               `mcpd`: the Unix-socket broker over one enfusion-mcp child, and its stub
├── mod.rs                      the module tree and `refuse_empty_write`, the empty-index guard
├── source.rs                   `enf source`: vanilla `.c` files rebuilt from cached source HTML pages
├── symbols.rs                  the `.c` scanner: declarations, methods, `modded` classes, `RplProp` fields
└── tests/                      unit tests for the scanner, the parsers, citations and the server resolver
```

## How it works

```text
apps/mod/crf_framework/ ─┐
vanilla .c tree         ─┴▶ enf index <crf|vanilla> ─▶ symbols::scan ─▶ .ai/artifacts/enf-index/<lane>_*.tsv
game paks ─▶ enf extract (PakVfs) ─▶ apps/mod/vanilla_reference/Scripts/
cached HTML ─▶ enf apidoc ─▶ vanilla_api_classes.tsv, vanilla_api_members.tsv
            ─▶ enf source ─▶ apps/mod/vanilla_reference/Source/
index TSVs ─▶ enf lookup | enf dirs | enf citations | enf capability ─▶ capability_matrix.tsv
```

- `index::build` scans every `.c` file under `--root` with the one scanner in `symbols.rs` and
  writes four TSVs per lane (`_symbols`, `_files`, `_modded`, `_rplprops`) that hold names and
  `file:line` coordinates, never code bodies, so they are committed while the sources stay
  gitignored. `lookup` and `dirs` read `crf_symbols.tsv` by default.
- `citations::verify` walks every Markdown file under `documentation_v2/` and resolves each `@idx
  crf#`, `@idx vanilla#` and `@idx api#` marker against `crf_symbols.tsv`, `vanilla_symbols.tsv` and
  `vanilla_api_classes.tsv`.
- `capability::build` joins `crf_files.tsv` and `crf_symbols.tsv` with the rules in
  `documentation_v2/mod/tbd-framework/capability_verdicts.tsv`, writes `capability_matrix.tsv`, and
  fails when any framework file matches no rule.
- `extract` reads scripts by name from the pak file table through `crate::enfusion_pak::PakVfs`;
  `carve` scans raw pak bytes instead, and `dump-entry` writes one entry's stored bytes for codec
  work.
- `index`, `apidoc`, `source` and `carve` refuse an empty result through `refuse_empty_write` before
  they write.
- `enfusion_mcp_entrypoint::resolve` picks the server command: `ENFUSION_MCP_BIN` when it names a
  file, then the module the pinned npm package installs
  (`crate::repository_layout::ENFUSION_MCP_ENTRYPOINT`), then a copy in the npx cache, then `npx -y
  enfusion-mcp`.
- `mcp_broker::run` starts that server once, initialises it, and serves tool calls over a Unix
  socket one at a time, because the [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) NetAPI
  takes one stream; it relabels each reply's id to 2 for `cargo xtask mcp consume`. It stops on
  SIGTERM or SIGINT, after `MCP_DAEMON_IDLE` seconds idle (1800) or `MCP_DAEMON_MAX_LIFE` seconds of
  life (14400), killing the child and removing the socket and pid file. `--stub` or `MCP_STUB=1`
  without `--socket` runs an offline stand-in for the server, shaped by `STUB_MODE`, `STUB_DAEMON`
  and `STUB_LINGER`, for `cargo xtask mcp selftest`.

## Boundaries

- Depends on: `crate::enfusion_pak::PakVfs` for `extract` and `dump-entry`;
  `crate::repository_layout` for the index folder, the documentation root, the verdict table and the
  installed MCP module; `tokio` for the broker; `clap` and `regex`.
- Used by:
  - `tools_v2/developer-tools/src/bin/enf.rs` (`cli::entrypoint`) and
    `tools_v2/developer-tools/src/bin/mcpd.rs` (`mcp_broker::run`);
  - `tools_v2/xtask/src/commands/mcp/call.rs` and `tools_v2/xtask/src/commands/mcp/daemon.rs`,
    through `enfusion_mcp_entrypoint::resolve` and `process_pattern`;
  - `cargo xtask fetch vanilla-api` and `cargo xtask fetch vanilla-source`, which print the `enf
    apidoc` and `enf source` step that follows them;
  - the mod wave gate in `tools_v2/xtask/src/commands/mod_ops/wave_execution/execution.rs`, whose
    unit-test step filters on `enf::`, a path no test in this module has.
- Rules: every lane shares the one scanner in `symbols.rs` (`does_not_invent_apis` and the other
  tests in `tests/symbols/tests.rs`); a committed index is never overwritten with an empty one
  (`refuse_empty_write_reds_on_empty`); `enf citations` exits 1 on any unresolved marker and `enf
  capability` on any untriaged framework file; the server command is resolved only here, so `cargo
  xtask mcp call`, `cargo xtask mcp daemon` and `mcpd` start the same server
  (`an_installed_package_resolves_to_the_pinned_module`).

## Related documentation

- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — the `@idx` citations and
  the oracle gates in mod work.
- [MCP command group](/tools_v2/xtask/src/commands/mcp/README.md) — the broker's launcher and its
  selftest.
- [Enfusion script oracle](/documentation_v2/tools_v2/developer-tools/enfusion_script_oracle.md) —
  the oracle tables, their lookups and the citation gate in depth.
