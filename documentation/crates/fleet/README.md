**Status:** live

# Fleet crate documentation

The feature documentation of the fleet crates under `crates/fleet/`: the game server host agent
that carries out the [fleet commands](/documentation/glossary/a_to_f.md#fleet-command) on each game
host. Operators and developers read it below the crates' code READMEs.

## Contents

```text
documentation/crates/fleet/
└── game_server_host_agent/  the game server host agent: how it carries out fleet commands
```

## How it works

The folder mirrors `crates/fleet/`. A document about one crate sits in a folder named after that
crate, as the [game server host agent](/documentation/crates/fleet/game_server_host_agent/README.md)
documentation does. Each document follows the
[feature doc template](/documentation/standards/templates/feature_doc.md).

## Code

- [Fleet crates](/crates/fleet/README.md) — the crates the documents describe.

## Boundaries

- Depends on: the code of `crates/fleet/`; the
  [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md).
- Used by: the [library crate documentation](/documentation/crates/README.md) index.
- Rules: a document of one crate sits in that crate's folder here, with a README at each level.

## Related documentation

- [Library crate documentation](/documentation/crates/README.md) — the other crate categories.
