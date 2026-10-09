# Enfusion script oracle source

The modules of the `enfusion_script_index` crate, the library behind the `enf` binary. It turns
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) script sources (the gitignored
`crf_framework` and `vanilla_reference` lanes of the
[reference lanes](/mod/References/README.md)) into committed TSV symbol indexes,
answers lookups against them, and checks the `@idx` citations in `documentation/` and the
framework capability verdicts; `vanilla_page_fetch/` mirrors the vanilla reference pages that
`enf apidoc` and `enf source` parse. The enfusion-mcp broker behind `mcpd` is the
[`enfusion_mcp_broker`](/tools/enfusion/enfusion_mcp_broker/README.md) crate.

## Contents

```text
tools/enfusion/enfusion_script_index/src/
├── apidoc.rs                   `enf apidoc`: the cached Script API HTML pages to `vanilla_api_*.tsv`
├── capability.rs               `enf capability`: the framework index joined with the verdict table
├── carve.rs                    `enf carve`: script-shaped text recovered by scanning the raw paks
├── citations.rs                `enf citations`: every `@idx lane#Symbol` marker checked against an index
├── command_line.rs             the `enf` clap command tree, its dispatch and exit codes (`run_command_line`)
├── empty_write_guard.rs        `refuse_empty_write`, the empty-index guard
├── error.rs                    `Error` and `Result`: why a command, an index build or a page mirror stopped
├── index.rs                    `enf index`, `lookup` and `dirs`: the TSV index writer and its readers
├── lib.rs                      the crate root: module header, `mod` lines and the re-exports
├── prelude.rs                  `run_command_line` and the index layout constants for glob import
├── reference_output.rs         the output guard of `carve`, `extract` and `source`: inside the references folder, `--replace`
├── script_index_layout.rs      the index folder, the upstream symbol table and the capability verdict table `enf` defaults to
├── source.rs                   `enf source`: vanilla `.c` files rebuilt from cached source HTML pages
├── symbols.rs                  the `.c` scanner: declarations, methods, `modded` classes, `RplProp` fields
├── tests/                      unit tests for the scanner and the parsers
├── vanilla_page_fetch/         the Script API and source page mirrors behind `cargo xtask fetch`
└── vanilla_page_fetch.rs       the mirrors' module tree and contract
```

## How it works

```text
mod/References/crf_framework/ ─┐
vanilla .c tree                    ─┴▶ enf index <crf|vanilla> ─▶ symbols::scan ─▶ .ai/artifacts/enf-index/<lane>_*.tsv
game paks ─▶ enf extract (PakVfs) ─▶ mod/References/vanilla_reference/Scripts/
          ─▶ enf carve            ─▶ mod/References/vanilla_reference/Carved/
cached HTML ─▶ enf apidoc ─▶ vanilla_api_classes.tsv, vanilla_api_members.tsv
            ─▶ enf source ─▶ mod/References/vanilla_reference/Source/
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
- `extract` reads scripts by name from the pak file table through `enfusion_pak::PakVfs`;
  `carve` scans raw pak bytes instead, and `dump-entry` writes one entry's stored bytes for codec
  work.
- `index`, `apidoc`, `source` and `carve` refuse an empty result through `refuse_empty_write` before
  they write.
- `carve`, `extract` and `source` write only inside `mod/References/`:
  `reference_output::checked_reference_output` refuses an output folder outside it, the folder
  itself, a path holding `..`, and a checkout without the folder. `carve` and `extract` remove a
  previous output only when given `--replace` (`reference_output::clear_previous_output`);
  without it they refuse. The defaults of every vanilla output are the lane paths in
  the `repository_layout` crate.

## Boundaries

- Depends on: the `enfusion_pak` crate (`PakVfs`) for `extract` and `dump-entry`;
  `script_index_layout.rs` for the index folder and the verdict table; the `repository_layout`
  crate for the references folder and its vanilla lane paths and the documentation root, and
  `REFERENCES_DIR` for the output guard; `repository_root` (`find_repository_root`) for the
  checkout root; `content_digest` for the file
  and blob digests; `process_runner` and `verification_core` for the mirrors' `curl`; `clap`,
  `regex` and `thiserror`.
- Used by:
  - `tools/developer_tools/src/bin/enf.rs` (`run_command_line`);
  - `cargo xtask fetch vanilla-api` and `cargo xtask fetch vanilla-source`, which run
    `vanilla_page_fetch` and print the `enf apidoc` and `enf source` step that follows them;
  - the mod wave gate in `tools/commands/mod_operations/src/wave_execution/execution.rs`, whose
    unit-test step runs this crate's tests.
- Rules: every lane shares the one scanner in `symbols.rs` (`does_not_invent_apis` and the other
  tests in `tests/symbols/tests.rs`); a committed index is never overwritten with an empty one;
  a lane output lands only inside the references folder and replaces a previous one only on
  `--replace`; `enf citations` exits 1 on any unresolved marker and `enf
  capability` on any untriaged framework file.

## Related documentation

- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — the `@idx` citations and
  the oracle gates in mod work.
- [Enfusion script oracle](/documentation/tools/enfusion/enfusion_script_index.md) —
  the oracle tables, their lookups and the citation gate in depth.
