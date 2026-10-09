**Status:** live

# Game server host agent documentation

The documents on the [game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent),
the program in `crates/fleet/game_server_host_agent/` that performs server commands on a
self-hosted game host.
Operators and developers read them below the crate's code READMEs, for the design, its limits and
the open work.

## Contents

```text
documentation/crates/fleet/game_server_host_agent/
└── fleet_command_execution.md  who executes which command, the claim-to-result flow, safety, RCON, design
```

## How it works

The [fleet command execution](/documentation/crates/fleet/game_server_host_agent/fleet_command_execution.md)
document follows the [feature doc template](/documentation/standards/templates/feature_doc.md)
and holds the design across the API, the agent and the game runtime. The code READMEs are exact
about the agent and are linked, not repeated: the [crate README](/crates/fleet/game_server_host_agent/README.md)
holds the command loop, the safety model, the installation and the configuration file; the
[command execution](/crates/fleet/game_server_host_agent/src/command_execution/README.md) and
[RCON](/crates/fleet/game_server_host_agent/src/rcon/README.md) READMEs hold each action's success rule and the
RCON wire protocol. The folder mirrors `crates/fleet/game_server_host_agent/`.

## Code

- [Game server host agent](/crates/fleet/game_server_host_agent/) — the crate the document covers.
- [Server infrastructure](/crates/api/api_server_infrastructure/src/) — the API's fleet
  command ledger and machine credentials the agent talks to.

## Boundaries

- Depends on: the agent's code, the API's fleet executor routes, the game runtime's fleet command
  scripts and `tools/commands/deployment/src/staging/host_agent.rs`, which every claim is
  checked against; the feature doc template; the ticket registry for open work.
- Used by: the `crates/fleet/game_server_host_agent/` README, which links this folder under Related
  documentation.
- Rules: the document describes the committed code, and a disagreement goes under Known
  discrepancies with both places; installation steps stay in the crate README and the staging
  runbook.

## Related documentation

- [Fleet command ledger](/documentation/crates/api/api_server/verification_evidence/fleet_command_ledger.md)
  — the API side: states, leases, fencing and execution windows.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — deploying the
  staging game server with the agent.
