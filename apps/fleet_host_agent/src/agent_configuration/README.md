# Agent configuration

Loads and validates the [fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent)'s TOML
configuration file and reads the two secrets it names, the
[machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential) and the
[RCON](/documentation_v2/glossary/n_to_z.md#rcon) password, so a misconfigured agent stops at startup
with a named error instead of failing its first command.

## Contents

```text
apps/fleet_host_agent/src/agent_configuration/
├── configuration_error.rs  `ConfigurationError` and its file problems; every error names its key or file
├── configuration_file.rs   the TOML shape before validation, with the defaults and no unknown key
├── mod.rs                  the module tree; `AgentConfiguration::load` and the per-key validation
├── secret_files.rs         reading a secret file, and the credential and RCON password formats
└── tests/                  unit tests for every key's validation and the secret file checks
```

## How it works

`AgentConfiguration::load` reads the file and `AgentConfiguration::parse` deserialises it into
`ConfigurationFile` (`#[serde(deny_unknown_fields)]`, so a misspelt key is an error rather than a
silent default), then validates each key into the settings the other modules take:

| Key | Default | Accepted | Becomes |
|---|---|---|---|
| `api_base_url` | required | an absolute `https` URL, or `http` on `localhost` or a loopback IP; no user, password, query or fragment | the API origin, normalised to end in `/` |
| `credential_file` | required | a secret file holding `tbdm_<32 lowercase hex>_<64 lowercase hex>` | the machine credential |
| `poll_interval_seconds` | 5 | 1 to 300 | the wait between claims while nothing is queued |
| `game_server.systemd_user_unit` | required | a `.service` unit name, checked by `SystemdUnitName::parse` | the unit process control acts on |
| `game_server.server_config_path` | required | an absolute path to a regular file the agent can open for writing | the dedicated server config `restart_with_mission` rewrites |
| `game_server.start_dwell_seconds` | 8 | 0 to 30 | the wait after start and restart before the state read |
| `game_server.systemctl_program` | `/usr/bin/systemctl` | an absolute path | the program process control runs |
| `rcon.address` | required | an IP address, not a host name | the RCON server address |
| `rcon.port` | 19999 | 1 to 65535 | the RCON server port |
| `rcon.password_file` | required | a secret file holding 3 to 256 bytes (at least 3 characters) with no whitespace or control character | the RCON password |

A secret file must be an absolute path to a regular file of at most 4 KiB with no group or other
permission bits (mode 600, for example), holding UTF-8 text; surrounding whitespace such as a
trailing newline is ignored. Both secrets become `SecretText`, which never prints. The dwell is
capped at 30 s so the verb timeout, the dwell and two state reads fit the ledger's 180 s execution
window for start and restart. The verb and state-read timeouts and the RCON timings are fixed in
code, not configured.

## Boundaries

- Depends on: `crate::process_control` (`ProcessControlSettings`, `SystemdUnitName`),
  `crate::rcon` (`RconSettings`, `RconTimings`), `crate::dedicated_server_config`
  (`DedicatedServerConfig`) and `crate::secret_text`; the `toml`, `serde`, `reqwest` (`Url`) and
  `thiserror` crates.
- Used by: `apps/fleet_host_agent/src/main.rs`, which loads the file named on the command line and
  exits 78 on a `ConfigurationError`.
- Rules: no error message quotes a secret; every key is validated at load time, never at first
  use; the credential format matches `contracts_v2/definitions/machine-credential.schema.json`
  (the tests in `tests/agent_configuration.rs` hold each rule).
