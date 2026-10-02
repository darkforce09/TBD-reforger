# Staging environment identity

The identities a staging receipt records as `environment:` lines: the staging host, what it runs,
and for the load check the load generator. `collect` reads them all once per recorded run.

## Contents

```text
tools/xtask/src/commands/staging/environment_identity/
├── build_identity.rs           binary digests, main guild id, and the Workshop version from `console.log`
├── load_generator_identity.rs  this workstation's CPU, memory, interface and round trip to the host
├── mod.rs                      `collect`, which reads and journals every identity
└── staging_host_identity.rs    the host's name, kernel, CPUs and memory
```

## How it works

`collect` reads the host identity, the API and host agent binary SHA-256 and the main guild id
(`DISCORD_GUILD_ID` from the API env file, by key), the migration head and Postgres version (a
committed SELECT), the API build (`tbd_build_info` from `/metrics`), the app 1890870 build id
(the server install's app manifest), the Workshop version (the first `console.log` line of
instance 1 naming `TBD_Framework` with a dotted version), and the partner guild id from
`deploy.env`; for the load check it adds `load_generator_hardware` and `load_generator_network`.
Each raw answer is journaled as `identity.<name>`. An identity that cannot be read is recorded as
`unavailable (<why>)`.

## Boundaries

- Depends on: `remote_observers/`, `remote_actions/game_server_update.rs`, the observation journal;
  `ip` and `ping` on this workstation.
- Used by: `tools/xtask/src/commands/staging/procedure_runner/recording.rs`; the load
  procedure's observations.
- Rules: only reads; no entry names a secret (the recorder refuses such a key).
