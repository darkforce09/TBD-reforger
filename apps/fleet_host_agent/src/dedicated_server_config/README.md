# Dedicated server config

The Arma Reforger dedicated server's JSON config (the file its `-config` parameter names) and the
one change the [fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent) makes to it:
pointing `game.scenarioId` at another [mission header](/documentation_v2/glossary/g_to_m.md#mission-header).

## Contents

```text
apps/fleet_host_agent/src/dedicated_server_config/
├── atomic_replacement.rs    replaces a file's contents by write, fsync and rename in its own directory
├── mod.rs                   the module tree; `DedicatedServerConfig::switch_scenario` and `ServerConfigError`
├── scenario_id_location.rs  finds the byte range of the `game.scenarioId` string without re-serialising
└── tests/                   unit tests for the location scan, the surgical rewrite and the replacement
```

## How it works

`DedicatedServerConfig::switch_scenario` changes the file in three steps, and the file is
unchanged whenever one fails:

1. Read: the file must be at most 1 MiB of UTF-8 JSON.
2. Rewrite surgically: `scenario_id_location` walks the parsed document's tokens to the root
   object's `game` member (an object) and its `scenarioId` member (a string), decoding member names
   before comparing them, and refuses a `game` or `scenarioId` named twice in one object. Only that
   string's byte range is replaced, so every other key and value, the key order, the spelling of
   every number and the whitespace stay byte for byte. The result is parsed again and must equal
   the original with `game.scenarioId` alone changed, or the switch fails as
   `RewriteNotSurgical`.
3. Replace atomically: `atomic_replacement` follows a symbolic link to the file it names, writes a
   new file in the same directory (mode 600 while written), gives it the original's permission
   bits, fsyncs it and renames it over the original, then fsyncs the directory. The path names the
   complete old file until it names the complete new one; on any failure before the rename the new
   file is removed. A failed directory fsync after the rename is logged, not reported.

`ServerConfigError` says which step failed: `Unreadable`, `TooLarge`, `NotJson`, `NoScenarioId`
(with the reason, such as "game has no scenarioId member"), `RewriteNotSurgical` or
`NotReplaced`.

## Boundaries

- Depends on: the `serde_json`, `rand`, `thiserror` and `tracing` crates, and the Unix file
  permission and open-mode extensions of `std`.
- Used by: `crate::agent_configuration`, which builds the `DedicatedServerConfig` from
  `game_server.server_config_path`; `crate::command_execution::mission_restart`, which calls
  `switch_scenario` before restarting the unit; and
  `apps/fleet_host_agent/tests/process_control.rs` and
  `apps/fleet_host_agent/tests/host_agent_ledger.rs`.
- Rules: only the value of `game.scenarioId` changes, and the file mode is kept
  (`tests/dedicated_server_config.rs`, `tests/atomic_replacement.rs`); a failed switch leaves the
  original file and no partial file behind; the new value is written only as a JSON string, never
  interpolated into other text.
