# Vanilla reference fetch commands

The `cargo xtask fetch` group: mirrors of two public references for the vanilla Arma Reforger
scripts that the [mod](/documentation/glossary/g_to_m.md#mod) builds on, cached in the checkout for the
`enf` index and lookup commands. Mod developers run them by hand.

## Contents

```text
tools/xtask/src/commands/fetch/
├── cli.rs       the `FetchCmd` clap enum: `vanilla-source` and `vanilla-api`, arguments taken raw
├── dispatch.rs  picks the checkout root for each command and calls its mirror
└── mod.rs       the module tree
```

## How it works

`dispatch.rs` picks the checkout root and hands the raw arguments to the mirror in the
[`enfusion_script_index`](/tools/enfusion/enfusion_script_index/README.md) crate's
`vanilla_page_fetch` module, which caches the pages in the `vanilla_reference` lane of the
[reference lanes](/mod/References/README.md) and returns the exit code; the caching rules,
the upstream hosts and the curl recipe are in that module's README. `TBD_FETCH_ROOT` points
either command at another root, which the crate's tests use; otherwise `vanilla-source` finds the
checkout root from the working directory, and `vanilla-api` takes `PWD` when it is the checkout
root.

## Commands

Clap's help is turned off on both, so every argument, `--help` included, reaches the command.

### vanilla-source

- Synopsis: `cargo xtask fetch vanilla-source [--all | --grep <pattern> | <File.c>...]`
- Does: with no argument, fetches a curated set of the game mode, respawn, spawn point, player
  controller, menu, faction, group and damage classes; `--all` fetches every file in the index;
  `--grep` every file whose name contains the pattern, ignoring case; names fetch those files.
  An unknown name or an HTTP failure is reported (`MISS`, `FAIL`) and counted, not fatal. The
  pause defaults to 0.4 s.
- Exit codes: 0 done, misses included; 1 the index held no source links, after `map.tsv` is
  emptied; 2 `--grep` without a pattern; 1 with `xtask: <error>` when `mod/References/` is
  missing or the index cannot be downloaded.
- Example: `cargo xtask fetch vanilla-source --grep Respawn`

### vanilla-api

- Synopsis: `cargo xtask fetch vanilla-api [<Class>... | --from-file <path>]`
- Does: fetches the class index, then each named class page; `--from-file` reads class names one
  per line, skipping blank lines and `#` comments (a missing file prints grep's message and
  fetches none). A missing class page prints `MISS` and does not fail the run. The pause defaults
  to 0.3 s; `TBD_FETCH_VANILLA_API_CURL` names a curl binary to use instead of the one on `PATH`.
- Exit codes: 0 done; 1 the class index could not be fetched; 2 `--from-file` without a path;
  1 with `xtask: <error>` when `mod/References/` is missing.
- Example: `cargo xtask fetch vanilla-api SCR_BaseGameMode`

## Boundaries

- Depends on: the `enfusion_script_index` crate (`vanilla_page_fetch::vanilla_api` and
  `vanilla_page_fetch::vanilla_source`); `find_repository_root` and `is_repository_root` from
  `repository_layout::prelude` (the `repository_root` finder); curl and network access to the two upstream sites, through the crate.
- Used by: people; `tools/xtask/src/cli/dispatch.rs` routes the group. The `enf apidoc` and
  `enf source` commands of `tools/developer_tools/src/bin/enf.rs` read the caches, and
  `apidoc.rs` in `tools/enfusion/enfusion_script_index/src/` names `vanilla-api` when its
  cache is missing.
- Rules: this folder holds the command line only; both commands stay polite to single-operator
  hosts (cache first, one request at a time, a pause between requests); an empty index is a
  refusal, never an empty mirror; the caches stay gitignored.

## Related documentation

- [Vanilla source coverage](/documentation/mod/tbd-framework/vanilla_source_coverage.md) —
  which vanilla classes the mod relies on and how to fetch them.
- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — where the fetch fits in
  mod work.
