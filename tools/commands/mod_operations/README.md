# Mod operations

The `mod_operations` crate: the game [mod](/documentation/glossary/g_to_m.md#mod)'s operations
behind `cargo xtask mod`. It compiles the mod's scripts on the headless dedicated server, boots the
world and judges its log, runs a local playtest server, prepares a
[Workbench](/documentation/glossary/n_to_z.md#workbench) session, validates and publishes the
Workbench equipment and vehicle export. Mod developers, mod slice agents, the ticket manager's
mod wave gate and the `mod-gates` CI workflow run it.

## Contents

```text
tools/commands/mod_operations/
├── Cargo.toml  the `mod_operations` library package: the command crates it delegates to and the tool foundations, layout tier 10
└── src/        the compile gate, the world boot, the playtest server, the equipment export, the website API client and the errors
```

## How it works

The xtask binary parses the command line and calls the crate: `mod <subcommand>` reaches `run`
with the parsed `ModCmd`, which maps it to one entry function and returns that entry's exit code.
`dev-server`, `playtest`, `compile` and `world-boot` take their arguments raw and parse
them in their own modules, so their usage text answers `--help`.

The game-facing gates share one exit contract (0 pass, 1 a code failure in the mod, 2 usage or
no verdict, 3 environment), so a machine fault never reads as broken mod code. The dedicated
server runs on the real machine through the container-to-host bridge, under `setsid`, and is
always stopped by its whole process group; its launcher's output is drained on a thread of its
own. Every child process runs through `process_runner`; an error that escapes an entry prints
`xtask: <cause>` and exits 1.

The commands themselves are described in the
[source README](/tools/commands/mod_operations/src/README.md).

## Boundaries

- Depends on: `workstation_setup`, `enfusion_mcp`, `remote_debugging`, `mod_script_checks`,
  `process_runner`, `repository_root`, `repository_layout` (the mod folder and the three addon folders, from its
  `enfusion_mod_folders`), `verification_core`, `content_digest`, `fleet_wire_contract`, `clap`,
  `flate2`, `heck`, `jsonschema`, `libc`, `regex`, `serde`, `serde_json`, `tar`, `thiserror`,
  `walkdir`; the Arma Reforger dedicated server and Workbench, `curl`, `git`, `npm` and `cargo`
  as subprocesses.
- Used by: the xtask binary's `mod` group.
- Rules: tier 10 of `tools/commands` (`cargo xtask verify crate-tiers`); an environment fault
  never exits 1.

## Related documentation

- [Two-client playtest](/documentation/runbooks/two_client_playtest/README.md) — a playtest a
  second client joins, with `mod playtest`.
- [Mod suite](/mod/README.md) — the addons these commands compile, boot and export from.
