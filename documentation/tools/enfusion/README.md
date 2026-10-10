**Status:** live

# Enfusion crates documentation

The documents on the Enfusion crates in `tools/enfusion/`, the libraries that read and drive the
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) engine's own formats and tools: the `.pak`
archive reader (`enfusion_pak`), the script oracle behind the `enf` binary (`enfusion_script_index`)
and the broker behind `mcpd` (`enfusion_mcp_broker`). Developers and AI agents read them below the
crates' code READMEs, for the flows that cross crates, the reasons and the open work.

## Contents

```text
documentation/tools/enfusion/
└── enfusion_script_index.md  `enf`: the Enfusion symbol indexes, lookups, citation and capability checks
```

## How it works

Each document follows the [feature doc template](/documentation/standards/templates/feature_doc.md)
and covers one crate's subsystem end to end. The code READMEs are exact about each folder and are
linked, not repeated:

| Crate | What it does | Document | Code |
|---|---|---|---|
| `enfusion_script_index` | indexes Enfusion scripts, extracts vanilla scripts from the paks, checks citations and capability verdicts, mirrors the vanilla reference pages | [Enfusion script oracle](/documentation/tools/enfusion/enfusion_script_index.md) | [`tools/enfusion/enfusion_script_index/`](/tools/enfusion/enfusion_script_index/README.md) |
| `enfusion_pak` | reads the game's `.pak` archives and loose extracted folders behind one virtual file system | its code README | [`tools/enfusion/enfusion_pak/`](/tools/enfusion/enfusion_pak/README.md) |
| `enfusion_mcp_broker` | the Unix-socket broker over one enfusion-mcp server for the [Workbench](/documentation/glossary/n_to_z.md#workbench) NetAPI | [Enfusion MCP tooling runbook](/documentation/runbooks/enfusion_mcp_tooling.md) | [`tools/enfusion/enfusion_mcp_broker/`](/tools/enfusion/enfusion_mcp_broker/README.md) |

A crate whose behaviour outgrows its README gets a document here and a Contents line.

## Code

- [Enfusion crates](/tools/enfusion/) — the crates these documents cover.
- [Enfusion script oracle](/tools/enfusion/enfusion_script_index/) — described in
  `enfusion_script_index.md`.

## Boundaries

- Depends on: the crates' code, the `enf` binary in `tools/developer_tools/src/bin/` and the xtask
  commands that call them, which every claim is checked against; the feature doc template; the
  ticket manager (`ttm`) for open work.
- Used by: the crates' READMEs, which link these documents under Related documentation; the
  [tooling documentation](/documentation/tools/README.md) index.
- Rules: a document describes the committed code, and a disagreement goes under Known
  discrepancies with both places; no document here writes an Enfusion citation marker that the
  oracle cannot resolve, since `enf citations` reads every Markdown file under `documentation/`.

## Related documentation

- [Developer tools documentation](/documentation/tools/developer_tools/README.md) — the
  executables, `enf` among them, that run these crates.
