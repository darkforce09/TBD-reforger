# Vanilla reference fetch commands

The `cargo xtask fetch` group: mirrors of two public references for the vanilla Arma Reforger
scripts that the [mod](/documentation_v2/glossary.md#mod) builds on, cached in the checkout for the
`enf` index and lookup commands. Mod developers run them by hand.

## Contents

```text
tools_v2/xtask/src/commands/fetch/
├── cli.rs             the `FetchCmd` clap enum: `vanilla-source` and `vanilla-api`, arguments taken raw
├── dispatch.rs        picks the checkout root for each command and calls its `run`
├── mod.rs             the module tree
├── tests/             unit tests for both mirrors on scratch checkouts, offline
├── vanilla_api.rs     `vanilla-api`: the Script API reference in Doxygen HTML
└── vanilla_source.rs  `vanilla-source`: vanilla script source pages, one per `.c` file
```

## How it works

Both commands cache under `apps/mod/vanilla_reference/`, which `apps/mod/.gitignore` keeps out of
git, and never fetch a page whose cached copy is non-empty. Each fetch is a `curl` run through
`verification_core::proc::Run` with a browser user agent, because the upstream hosts refuse curl's
default, and a pause of `TBD_FETCH_DELAY` seconds after each page fetched from the network. Both
end by printing the `enf` command that indexes what they cached. `TBD_FETCH_ROOT` points either
command at another root, which the tests use; otherwise `vanilla-source` finds the checkout root
from the working directory, and `vanilla-api` takes `PWD` when it is the checkout root.

- `vanilla-source` fetches the file index `files.html` from arexplorer.zeroy.com when it is not
  cached, builds `map.tsv` (file name to page) from it, and fetches the pages it is asked for into
  `source_html/`.
- `vanilla-api` fetches the class index `annotated.html` from the Bohemia community wiki's Script
  API reference, then any class pages it is asked for into `apidoc/`, under Doxygen's names, where
  `_` doubles (`SCR_BaseGameMode` is `interfaceSCR__BaseGameMode.html`).

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
  emptied; 2 `--grep` without a pattern; 1 with `xtask: <error>` when the index cannot be
  downloaded.
- Example: `cargo xtask fetch vanilla-source --grep Respawn`

### vanilla-api

- Synopsis: `cargo xtask fetch vanilla-api [<Class>... | --from-file <path>]`
- Does: fetches the class index, then each named class page; `--from-file` reads class names one
  per line, skipping blank lines and `#` comments (a missing file prints grep's message and
  fetches none). A missing class page prints `MISS` and does not fail the run. The pause defaults
  to 0.3 s; `TBD_FETCH_VANILLA_API_CURL` names a curl binary to use instead of the one on `PATH`.
- Exit codes: 0 done; 1 the class index could not be fetched; 2 `--from-file` without a path.
- Example: `cargo xtask fetch vanilla-api SCR_BaseGameMode`

## Boundaries

- Depends on: `verification_core::proc` for curl; `find_repo_root` in
  `tools_v2/xtask/src/core/repository_root.rs` and `is_repo_root` from `ticket_engine::repository`;
  curl and network access to the two upstream sites.
- Used by: people; `tools_v2/xtask/src/cli/dispatch.rs` routes the group. The `enf apidoc` and
  `enf source` commands of `tools_v2/developer-tools/src/bin/enf.rs` read the caches, and
  `apidoc.rs` in `tools_v2/developer-tools/src/enfusion_tooling/` names `vanilla-api` when its
  cache is missing.
- Rules: both commands stay polite to single-operator hosts: cache first, one request at a time,
  a pause between requests; an empty index is a refusal, never an empty mirror; the caches stay
  gitignored.

## Related documentation

- [Vanilla source coverage](/documentation_v2/mod/tbd-framework/vanilla_source_coverage.md) —
  which vanilla classes the mod relies on and how to fetch them.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — where the fetch fits in
  mod work.
