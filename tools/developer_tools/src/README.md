# Developer tools source tree

The source of the `developer_tools` package's eight executables: one entry point each for the
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) script oracle, the headless browser gates
of the single-page app, the enfusion-mcp broker, the pipelines that build and verify the terrain
and map assets under `assets/`, and the staging verification engines. The package has no library.

## Contents

```text
tools/developer_tools/src/
└── bin/                    the eight entry points, one `main` per executable
```

## How it works

Each binary in `bin/` is a `main` that calls one tool crate's command-line entry and returns its
exit code; the crates own the argument parsing, the work, the paths and the tests.

```text
bin/world              ──▶ the world_export_pipeline crate (tools/map_assets/world_export_pipeline)
bin/map                ──▶ the map_raster_pipeline crate (tools/map_assets/map_raster_pipeline)
bin/acknowledgement_dropping_relay ──▶ the acknowledgement_dropping_relay crate (tools/staging/acknowledgement_dropping_relay)
bin/staging_load       ──▶ the staging_load_generator crate (tools/staging/staging_load_generator)
bin/gate, bin/capture  ──▶ the browser_gate_suites crate (tools/browser_testing/browser_gate_suites)
bin/mcpd               ──▶ the enfusion_mcp_broker crate (tools/enfusion/enfusion_mcp_broker)
bin/enf                ──▶ the enfusion_script_index crate (tools/enfusion/enfusion_script_index)
```

## Public surface

- The eight binaries, whose commands `bin/` lists.

## Boundaries

- Depends on: the seven tool crates in the diagram, nothing else.
- Used by: people and the `cargo xtask` recipes and CI tasks that run a binary by
  `--bin <name>`; no crate depends on the package.
- Rules: an entry file holds only `main` and its one call and stays under 250 lines; the package
  declares no library and depends only on tool crates.
