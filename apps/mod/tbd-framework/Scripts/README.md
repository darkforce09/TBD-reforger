# Framework scripts

The addon's script root. It holds the one script module the framework ships, `Game/`; the
[Workbench](/documentation_v2/glossary.md#workbench) export plugins and the
[Enfusion](/documentation_v2/glossary.md#enfusion) MCP handlers live in their own addons,
`apps/mod/tbd-export/` and `apps/mod/tbd-emcp/`.

## Contents

```text
apps/mod/tbd-framework/Scripts/
└── Game/  the game script module, compiled into servers and clients
```

## Authority

- Server: as the game module decides; see `Game/`.
- Client: as the game module decides; see `Game/`.
- Owner: as the game module decides; see `Game/`.
- RPCs: declared in the game module.
- Replicated properties: declared in the game module.

## Boundaries

- Depends on: vanilla Arma Reforger's script modules.
- Used by: the Enfusion script compiler, when a server, client or Workbench loads
  `apps/mod/tbd-framework/addon.gproj`, and `cargo xtask mod compile`.
- Rules: the framework carries no `WorkbenchGame/` module, and `cargo xtask mod compile` exits 1
  when one appears.
