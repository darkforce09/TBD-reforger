# Fleet crates

The programs that carry out the platform's [fleet commands](/documentation/glossary/a_to_f.md#fleet-command)
on the game hosts, beside the Arma Reforger dedicated servers the API controls.

The one fleet crate is the
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent): one agent runs beside
each game server instance, polls the API outbound over HTTPS for the fleet commands addressed to
its server, performs each through `systemctl --user`, BattlEye RCon or a `game.scenarioId` switch
in the server's JSON config, and reports every step to the API's command ledger.

Its binary `game_server_host_agent` is installed by `cargo xtask deploy staging` as
`~/.local/bin/game_server_host_agent`, configured per instance by
`~/.config/game_server_host_agent/instance-N/agent.toml` and run as
`game_server_host_agent@N.service`; the
[staging deploy runbook](/documentation/runbooks/game_server_staging/staging_deploy.md) walks the
procedure, and the [crate README](game_server_host_agent/README.md) covers a manual install.

## Contents

```text
crates/fleet/
└── game_server_host_agent/  `game_server_host_agent`: the agent beside each game server instance that claims its fleet commands from the API and carries them out
```

## Boundaries

- Depends on: foundation crates built for every target and contracts crates; the host agent
  names `fleet_wire_contract` (the fleet command shapes and the machine credential format) and
  `newtype_ids` (its typed ids), and external crates.
- Used by: no workspace member depends on a fleet crate; `cargo xtask deploy staging` builds and
  installs one game server host agent per fleet instance.
- Rules: a fleet crate declares `category = "crates/fleet"` (`cargo xtask verify crate-tiers`)
  and speaks to the API over HTTP only, never through an API crate.
