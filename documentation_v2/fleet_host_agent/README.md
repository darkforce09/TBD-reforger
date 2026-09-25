**Status:** live

# Fleet host agent documentation

The documents on the [fleet host agent](/documentation_v2/glossary.md#fleet-host-agent), the
program in `apps/fleet_host_agent/` that performs server commands on a self-hosted game host.
Operators and developers read them below the crate's code READMEs, for the design, its limits and
the open work.

## Contents

```text
documentation_v2/fleet_host_agent/
└── fleet_command_execution.md  who executes which command, the claim-to-result flow, safety, RCON, design
```

## How it works

The [fleet command execution](/documentation_v2/fleet_host_agent/fleet_command_execution.md)
document follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md)
and holds the design across the API, the agent and the game runtime. The code READMEs are exact
about the agent and are linked, not repeated: the [crate README](/apps/fleet_host_agent/README.md)
holds the command loop, the safety model, the installation and the configuration file; the
[command execution](/apps/fleet_host_agent/src/command_execution/README.md) and
[RCON](/apps/fleet_host_agent/src/rcon/README.md) READMEs hold each action's success rule and the
RCON wire protocol. The folder mirrors `apps/fleet_host_agent/` with `apps/` left out.

## Code

- [Fleet host agent](/apps/fleet_host_agent/) — the crate the document covers.
- [Server infrastructure](/apps/website/api_v2/src/server_infrastructure/) — the API's fleet
  command ledger and machine credentials the agent talks to.

## Boundaries

- Depends on: the agent's code, the API's fleet executor routes, the game runtime's fleet command
  scripts and `tools_v2/xtask/src/commands/deploy/staging/host_agent.rs`, which every claim is
  checked against; the feature doc template; the ticket registry for open work.
- Used by: the `apps/fleet_host_agent/` README, which links this folder under Related
  documentation.
- Rules: the document describes the committed code, and a disagreement goes under Known
  discrepancies with both places; installation steps stay in the crate README and the staging
  runbook.

## Related documentation

- [Fleet command ledger](/documentation_v2/website/api_v2/verification_evidence/fleet_command_ledger.md)
  — the API side: states, leases, fencing and execution windows.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — deploying the
  staging game server with the agent.
