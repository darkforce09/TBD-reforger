**Status:** live

# Developer tools documentation

The documents on `developer_tools`, the crate of heavy offline tools: its library and the eight
executables `enf`, `gate`, `mcpd`, `world`, `map`, `capture`, `acknowledgement-dropping-relay` and
`staging-load`.
Developers and AI agents read them below the crate's code READMEs, for the flows that cross
modules, the reasons and the open work.

## Contents

```text
documentation/tools/developer_tools/
└── map_raster_pipeline.md            `map`: satellite, Map view, labels, water archives and the glyph atlas
```

## How it works

Each document follows the [feature doc template](/documentation/standards/templates/feature_doc.md)
and covers one executable's subsystem end to end. The code READMEs are exact about each folder
and are linked, not repeated:

| Executable | What it does | Document | Code |
|---|---|---|---|
| `enf` | indexes [Enfusion](/documentation/glossary/a_to_f.md#enfusion) scripts, extracts vanilla scripts from the paks, checks citations and capability verdicts | [Enfusion script oracle](/documentation/tools/enfusion/enfusion_script_index.md) | [`tools/enfusion/enfusion_script_index/`](/tools/enfusion/enfusion_script_index/README.md) |
| `mcpd` | the Unix-socket broker over one enfusion-mcp server for the [Workbench](/documentation/glossary/n_to_z.md#workbench) NetAPI | [Enfusion MCP tooling runbook](/documentation/runbooks/enfusion_mcp_tooling.md) | [`tools/enfusion/enfusion_mcp_broker/`](/tools/enfusion/enfusion_mcp_broker/README.md) |
| `gate` | headless Chromium gates: `gate doctor`, the editor smokes, the DOM oracle `v-suite` | [Editor gates runbook](/documentation/runbooks/editor_gates.md) | [`browser_gate_suites`](/tools/browser_testing/browser_gate_suites/README.md), [`src/browser_testing/`](/tools/developer_tools/src/browser_testing/README.md) |
| `capture` | screenshots, zoom sweeps and crops of a running [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) | [Editor capture runbook](/documentation/runbooks/editor_capture.md) | [`browser_gate_suites/src/screen_capture/`](/tools/browser_testing/browser_gate_suites/src/screen_capture/README.md) |
| `world` | a Workbench world export to committed terrain artifacts, and their gates | [Terrain export and map assets](/documentation/assets/terrain_export_and_map_assets.md) | [`src/world_export_pipeline/`](/tools/developer_tools/src/world_export_pipeline/README.md) |
| `map` | the raster, label, water and glyph assets | [Map raster pipeline](/documentation/tools/developer_tools/map_raster_pipeline.md) | [`src/map_raster_pipeline/`](/tools/developer_tools/src/map_raster_pipeline/README.md) |
| `acknowledgement-dropping-relay` | on the staging host, relays one fleet host agent's calls and, when armed, withholds one claim or result answer | [Staging verification engines](/documentation/tools/staging/staging_verification_engines.md) | [`tools/staging/acknowledgement_dropping_relay/`](/tools/staging/acknowledgement_dropping_relay/README.md) |
| `staging-load` | runs the member load a plan describes and prints its report, as the xtask staging load procedure's child process | [Staging verification engines](/documentation/tools/staging/staging_verification_engines.md) | [`tools/staging/staging_load_generator/`](/tools/staging/staging_load_generator/README.md) |

The building-blueprint compiler and the engine-backed map verifications have no binary of their
own; `cargo xtask map` and the xtask schema gates call them, and their READMEs
([`src/blueprint/`](/tools/developer_tools/src/blueprint/README.md),
[`src/map_verification/`](/tools/developer_tools/src/map_verification/README.md)) are their
documentation. A subsystem whose behaviour outgrows its README gets a document here and a Contents line.

## Code

- [Developer tools](/tools/developer_tools/) — the crate these documents cover.
- [Map raster pipeline](/tools/developer_tools/src/map_raster_pipeline/) — described in
  `map_raster_pipeline.md`.

## Boundaries

- Depends on: the crate's code, the xtask tasks that call it and the committed assets it writes,
  which every claim is checked against; the feature doc template; the ticket registry in
  `.ai/tickets/` for open work.
- Used by: the `tools/developer_tools/` README, which links these documents under Related
  documentation; the [tooling documentation](/documentation/tools/README.md) index; the
  [terrain export and map assets](/documentation/assets/terrain_export_and_map_assets.md)
  document.
- Rules: a document describes the committed code, and a disagreement goes under Known
  discrepancies with both places; no document here writes an Enfusion citation marker that the
  oracle cannot resolve, since `enf citations` reads every Markdown file under `documentation/`.
