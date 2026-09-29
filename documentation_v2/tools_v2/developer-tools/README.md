**Status:** live

# Developer tools documentation

The documents on `developer-tools`, the crate of heavy offline tools: its library and the seven
executables `enf`, `gate`, `mcpd`, `world`, `map`, `capture` and `acknowledgement-dropping-relay`.
Developers and AI agents read them below the crate's code READMEs, for the flows that cross
modules, the reasons and the open work.

## Contents

```text
documentation_v2/tools_v2/developer-tools/
├── enfusion_script_oracle.md         `enf`: the Enfusion symbol indexes, lookups, citation and capability checks
├── map_raster_pipeline.md            `map`: satellite, Map view, labels, water archives and the glyph atlas
└── staging_verification_engines.md   the member load engine and `acknowledgement-dropping-relay`, end to end
```

## How it works

Each document follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md)
and covers one executable's subsystem end to end. The code READMEs are exact about each folder
and are linked, not repeated:

| Executable | What it does | Document | Code |
|---|---|---|---|
| `enf` | indexes [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) scripts, extracts vanilla scripts from the paks, checks citations and capability verdicts | [Enfusion script oracle](/documentation_v2/tools_v2/developer-tools/enfusion_script_oracle.md) | [`src/enfusion_tooling/`](/tools_v2/developer-tools/src/enfusion_tooling/README.md) |
| `mcpd` | the Unix-socket broker over one enfusion-mcp server for the [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) NetAPI | [Enfusion MCP tooling runbook](/documentation_v2/runbooks/enfusion_mcp_tooling.md) | [`src/enfusion_tooling/`](/tools_v2/developer-tools/src/enfusion_tooling/README.md) |
| `gate` | headless Chromium gates: `gate doctor`, the editor smokes, the DOM oracle `v-suite` | [Editor gates runbook](/documentation_v2/runbooks/editor_gates.md) | [`src/browser_testing/`](/tools_v2/developer-tools/src/browser_testing/README.md) |
| `capture` | screenshots, zoom sweeps and crops of a running [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) | [Editor capture runbook](/documentation_v2/runbooks/editor_capture.md) | [`src/browser_testing/screen_capture/`](/tools_v2/developer-tools/src/browser_testing/screen_capture/README.md) |
| `world` | a Workbench world export to committed terrain artifacts, and their gates | [Terrain export and map assets](/documentation_v2/assets_v2/terrain_export_and_map_assets.md) | [`src/world_export_pipeline/`](/tools_v2/developer-tools/src/world_export_pipeline/README.md) |
| `map` | the raster, label, water and glyph assets | [Map raster pipeline](/documentation_v2/tools_v2/developer-tools/map_raster_pipeline.md) | [`src/map_raster_pipeline/`](/tools_v2/developer-tools/src/map_raster_pipeline/README.md) |
| `acknowledgement-dropping-relay` | on the staging host, relays one fleet host agent's calls and, when armed, withholds one claim or result answer | [Staging verification engines](/documentation_v2/tools_v2/developer-tools/staging_verification_engines.md) | [`src/staging_verification/acknowledgement_relay/`](/tools_v2/developer-tools/src/staging_verification/acknowledgement_relay/README.md) |

The building-blueprint compiler and the engine-backed map verifications have no binary of their
own; `cargo xtask map` and the xtask schema gates call them, and their READMEs
([`src/blueprint/`](/tools_v2/developer-tools/src/blueprint/README.md),
[`src/map_verification/`](/tools_v2/developer-tools/src/map_verification/README.md)) are their
documentation. The member load generator has no binary either: the xtask staging load procedure
is the caller it is built for, and the staging verification engines document covers it beside the
relay. A subsystem whose behaviour outgrows its README gets a document here and a Contents line.

## Code

- [Developer tools](/tools_v2/developer-tools/) — the crate these documents cover.
- [Enfusion tooling](/tools_v2/developer-tools/src/enfusion_tooling/) — described in
  `enfusion_script_oracle.md`.
- [Map raster pipeline](/tools_v2/developer-tools/src/map_raster_pipeline/) — described in
  `map_raster_pipeline.md`.
- [Staging verification engines](/tools_v2/developer-tools/src/staging_verification/) — described
  in `staging_verification_engines.md`.

## Boundaries

- Depends on: the crate's code, the xtask tasks that call it and the committed assets it writes,
  which every claim is checked against; the feature doc template; the ticket registry in
  `.ai/tickets/` for open work.
- Used by: the `tools_v2/developer-tools/` README, which links these documents under Related
  documentation; the [tooling documentation](/documentation_v2/tools_v2/README.md) index; the
  [terrain export and map assets](/documentation_v2/assets_v2/terrain_export_and_map_assets.md)
  document.
- Rules: a document describes the committed code, and a disagreement goes under Known
  discrepancies with both places; no document here writes an Enfusion citation marker that the
  oracle cannot resolve, since `enf citations` reads every Markdown file under `documentation_v2/`.
