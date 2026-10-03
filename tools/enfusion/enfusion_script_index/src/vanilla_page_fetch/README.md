# Vanilla reference page mirrors

The two mirrors behind `cargo xtask fetch`: the public references for the vanilla Arma Reforger
scripts that the [mod](/documentation/glossary/g_to_m.md#mod) builds on, cached in the checkout for
the `enf apidoc` and `enf source` commands. xtask's
[`fetch` command line](/tools/xtask/src/commands/fetch/README.md) picks the checkout root and calls
them; mod developers run them by hand.

## Contents

```text
tools/enfusion/enfusion_script_index/src/vanilla_page_fetch/
├── reference_cache.rs  the cache folder each mirror fills in the vanilla lane, refused without `apps/mod/References/`
├── tests/              unit tests for both mirrors on scratch checkouts, offline
├── vanilla_api.rs      `vanilla-api`: the Script API reference in Doxygen HTML
└── vanilla_source.rs   `vanilla-source`: vanilla script source pages, one per `.c` file
```

## How it works

Both mirrors cache in the `vanilla_reference` lane of the
[reference lanes](/apps/mod/References/README.md), which `.gitignore` keeps out of git, and never
fetch a page whose cached copy is non-empty. `reference_cache::prepare_vanilla_cache` creates the
mirror's cache folder (`repository_layout::VANILLA_SCRIPT_API_PAGES` or
`repository_layout::VANILLA_SOURCE_PAGES`) and refuses, before any fetch, when the checkout has no
`apps/mod/References/` folder; it never creates that folder. Each fetch is a `curl` run through
`process_runner::Run` with a browser user agent, because the upstream hosts refuse curl's
default, and a pause of `TBD_FETCH_DELAY` seconds after each page fetched from the network. Both
end by printing the `enf` command that indexes what they cached, and return the exit code.

- `vanilla_source::run` fetches the file index `files.html` from arexplorer.zeroy.com when it is
  not cached, builds `map.tsv` (file name to page) from it, and fetches the pages it is asked for
  into `source_html/`.
- `vanilla_api::run` fetches the class index `annotated.html` from the Bohemia community wiki's
  Script API reference, then any class pages it is asked for into `apidoc/`, under Doxygen's names,
  where `_` doubles (`SCR_BaseGameMode` is `interfaceSCR__BaseGameMode.html`).

## Boundaries

- Depends on: `process_runner` (`Run`, `which`) and `verification_core` (`NotRun`) for curl;
  `repository_layout` for the references folder and the two cache folders; `regex` for the
  source index's links.
- Used by: `tools/xtask/src/commands/fetch/dispatch.rs`.
- Rules: a cache hit never touches the network; an index with no page links is a refusal (exit
  1), never an empty mirror; the references folder is never created here.
