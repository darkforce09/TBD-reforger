# Enfusion script oracle

The `enfusion_script_index` crate, the library behind the `enf` binary: it turns
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) script sources (the gitignored upstream
framework and vanilla lanes of the [reference lanes](/mod/References/README.md)) into TSV
symbol indexes of names and `file:line` coordinates, answers lookups against them, checks the
`@idx` citations in `documentation/` and the framework capability verdicts, extracts, carves and
reconstructs vanilla sources, and mirrors the vanilla reference pages behind `cargo xtask fetch`.

## Contents

```text
tools/enfusion/enfusion_script_index/
├── Cargo.toml  the `enfusion_script_index` library package: `clap`, `content_digest`, `enfusion_pak`, `process_runner`, `regex`, `repository_layout`, `thiserror`, `verification_core`; layout tier 2
└── src/        the scanner, the index writer and readers, the gates, the vanilla extraction, the page mirrors, the `enf` command line
```

## How it works

```text
reference lanes ─▶ enf index ─▶ .ai/artifacts/enf-index/<lane>_*.tsv ─▶ enf lookup | dirs | citations | capability
game paks (enfusion_pak) ─▶ enf extract | carve | dump-entry ─▶ mod/References/vanilla_reference/
cargo xtask fetch vanilla-api | vanilla-source (vanilla_page_fetch) ─▶ cached HTML ─▶ enf apidoc | source
```

`run_command_line` parses `enf`, runs the command and returns its exit code: 0 done, 1 a check
failed or a command produced nothing, 2 a command could not run (its error printed after `enf: `
with every cause). Every lane shares one scanner, a committed index is never replaced by an empty
one, and a lane output lands only inside `mod/References/`. `src/README.md` describes each
module.

## Getting started

Run from the repository root:

```bash
cargo test -p enfusion_script_index                          # scanner and parsers
cargo run -q -p developer_tools --bin enf -- index crf --root mod/References/crf_framework
cargo run -q -p developer_tools --bin enf -- citations       # every @idx marker in documentation/
cargo xtask fetch vanilla-api                                # the Script API index into the vanilla lane
```

## Configuration

| Variable | Default | Effect |
|---|---|---|
| `ENFUSION_GAME_PATH` | `$HOME/.cache/enfusion-mcp-root` | the game folder `enf extract` and `enf dump-entry` read |
| `TBD_FETCH_DELAY` | 0.3 s (`vanilla-api`), 0.4 s (`vanilla-source`) | the pause after each page fetched from the network |
| `TBD_FETCH_VANILLA_API_CURL` | `curl` on `PATH` | the curl binary `vanilla-api` runs |

## Boundaries

- Depends on: `enfusion_pak`, `repository_layout`, `content_digest`, `process_runner`,
  `verification_core`, `clap`, `regex`, `thiserror`.
- Used by: the `enf` binary in `tools/developer_tools/src/bin/enf.rs`; xtask's `fetch` command
  line (`tools/xtask/src/commands/fetch/`); the mod wave gate, which runs this crate's tests.
- Rules: tier 2 of `tools/enfusion`; an index carries names and coordinates, never code bodies;
  no `std::process::exit` in the library (only `run_command_line` returns an exit code); every
  `curl` runs through `process_runner`.

## Related documentation

- [Enfusion script oracle](/documentation/tools/enfusion/enfusion_script_index.md) — the oracle
  tables, their lookups and the citation gate in depth.
- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — the `@idx` citations and
  the oracle gates in mod work.
