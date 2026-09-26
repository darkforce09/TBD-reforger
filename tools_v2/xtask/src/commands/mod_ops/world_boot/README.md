# Headless world boot driver

The boot half of `cargo xtask mod world-boot`: it stages a throwaway run folder, optionally seeds
a compiled [mission](/documentation_v2/glossary/g_to_m.md#mission) through the platform, boots the
dedicated server headless, and hands the console log to the verdict in
`tools_v2/xtask/src/commands/mod_ops/world_boot_verdict.rs`.

## Contents

```text
tools_v2/xtask/src/commands/mod_ops/world_boot/
├── boot_environment.rs  host bridge, addon GUID and scenarioId reads, temp folders, numeric settings
├── compiled_lane.rs     `--compiled`: the seeded fixture mission, server launch, log poll, kill, sweep
└── execution.rs         flag parsing, the boot sequence and the environment and API failure reports
```

## How it works

`tools_v2/xtask/src/commands/mod_ops/world_boot.rs` holds the options and the `RunState` whose
drop kills the server, deletes the fixture missions and removes the run folder unless
`--keep-logs` is given.

```text
execution::run ─ --selftest ─▶ world_boot_verdict::cmd_selftest + four-weapon equip selftest
  └─ boot
       ├─ --mission=<file|name>: a path, or <name>[.json] under contracts_v2/fixtures/missions/valid/
       ├─ host bridge, server binary, dev profile, addon GUID, scenarioId ─ else exit 3 (or 1)
       ├─ run folder under $TMPDIR: addons/tbd-framework -> checkout
       ├─ --compiled[=uuid]: dev login as mission_maker, seed or reuse a mission, take its
       │    approved, pending or newly submitted artifact, write compiled.json
       ├─ a mission: stage it in profile/profile/TBD_MissionArtifactCache/ with an empty
       │    backend config and Data/registry.json as TBD_Registry.json
       ├─ server.json from the dev profile with per-process ports (21000+ and 26000+)
       ├─ ArmaReforgerServer -addonsDir … -config … -profile … -maxFPS 15, under timeout
       ├─ poll for console.log up to TBD_WORLDBOOT_TIMEOUT (240 s), settle TBD_WORLDBOOT_SETTLE (4 s)
       └─ assess_log (+ four-weapon equip with --compiled) ─▶ WORLD BOOT: PASS (0) / FAIL (1)
```

- `--compiled` and `--mission` are mutually exclusive; `TBD_API_BASE` sets the API (default
  `http://127.0.0.1:8080`). An API that gives no answer is an environment failure (exit 3); a
  mission the API cannot compile is a code failure (exit 1).
- With a mission, the verdict also checks the mission validated and compares validator warnings
  with the per-mission budget in `.world-boot-warning-baseline` at the repository root.

## Boundaries

- Depends on: `crate::commands::mod_ops::website_api_client` (dev login, mission create, submit,
  artifact document, artifact cache, sweep), `crate::commands::mod_ops::world_boot_verdict`
  (`assess_log`, `cmd_selftest`, `MissionCtx`), `crate::core::repository_layout`
  (`DEV_SERVER_PROFILE`), `developer_tools::repository_layout` (the mission fixture folder), and
  the dedicated server under `$HOME/.local/share/Steam/steamapps/common/Arma Reforger Server`.
- Used by: `tools_v2/xtask/src/commands/mod_ops/world_boot.rs`, which re-exports `run` to the
  `mod` dispatch.
- Rules: an environment fault exits 3 with the harness named, never 1; the equip assertion needs
  exactly four `result=ok` lines for slots 0 to 3 (`EXPECTED_EQUIP_OK` in `world_boot.rs`,
  proven by the `--selftest` arm); the verdict tests are in
  `tools_v2/xtask/src/commands/mod_ops/tests/world_boot_verdict/tests.rs`, and this folder has no
  unit tests of its own.
