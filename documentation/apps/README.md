**Status:** live

# Application documentation

The documentation of the one product left in `apps/`: the Arma Reforger
[mod](/documentation/glossary/g_to_m.md#mod), its three Enfusion addons, the game framework, the
Workbench export and the MCP bridge. Developers and AI agents read it below the mod's code
READMEs, for behaviour, design, open work and decisions. The documents of the library crates, the
API server, the single-page app and the
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) among them, are
in [crates/](/documentation/crates/README.md), and those of the tool crates, the
[ticketboard](/documentation/glossary/n_to_z.md#ticketboard) among them, in
[tools/](/documentation/tools/README.md).

## Contents

```text
documentation/apps/
└── mod/               the Enfusion mod suite: framework design and screen specs, export evidence, the MCP bridge
```

## How it works

The folder sits at the mod's path with the leading `apps/` replaced by `documentation/apps/`; the
mod's scripts have no `src/`, so the mirror leaves out `Scripts/Game/TBD/` instead. Each folder
opens with a README index; the documents inside follow the templates in
`documentation/standards/templates/`: feature docs for a screen, a system or a cross-cutting
subject, and `decisions.md` logs for the decisions behind them.

| Product | Code | Documentation |
|---|---|---|
| mod | [`apps/mod/`](/apps/mod/README.md): the three Enfusion addons, the game framework, the Workbench export and the MCP bridge | [mod documentation](/documentation/apps/mod/README.md) |

The mod calls the API's `/api/v1/game-runtime/` routes from inside a dedicated server; the
[API documentation](/documentation/crates/api/api_server/README.md) describes the server side, and
the [applications README](/apps/README.md) gives the commands that build and check the mod.

## Code

- [Applications](/apps/README.md) — the folder of the mod.
- [Mod suite](/apps/mod/README.md) — described under `mod/`.

## Boundaries

- Depends on: the code under `apps/mod/`, which every document is checked against; the
  [README standard](/documentation/standards/readme_standard.md) and the templates in
  `documentation/standards/templates/`; the glossary for its terms.
- Used by: the [applications README](/apps/README.md) and the mod's READMEs, which link these
  documents under Related documentation; the runbooks, which link the mod's documents.
- Rules: a folder here mirrors a code folder under `apps/mod/` and keeps its spelling; a document
  describes the committed code, and a disagreement between a document and the code is recorded
  with both places and resolved in the code's favour.

## Related documentation

- [Library crate documentation](/documentation/crates/README.md) — the API server, the single-page
  app, the game server host agent and the other library crates.
- [Tool documentation](/documentation/tools/README.md) — the developer tools, the ticketboard
  among them.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — running the mod
  on the staging fleet.
- [Glossary](/documentation/glossary/README.md) — the platform's terms.
