# Staging boot verdict

The judgement over a dedicated server's `console.log` that `cargo xtask deploy staging` passes or
fails a deploy on, and the self-test that proves each check can fail. Both run offline, through
`deploy staging --verify-boot <log>` and `--verify-boot-selftest`, and the deploy runs the same
verdict over the log it pulls back. `tools_v2/xtask/src/commands/deploy/staging/boot.rs` declares
both files, holds the `Out` sink that prints or captures, and re-exports the functions.

## Contents

```text
tools_v2/xtask/src/commands/deploy/staging/boot/
├── read_addon_guid.rs  the addon GUID from the gproj, the three assertions, the verdict, `--verify-boot`
└── selftest.rs         `--verify-boot-selftest`: fifteen checks over fixture logs the engine writes
```

## How it works

`verify_boot_log` runs three assertions and passes only when all three hold:

- `assert_local_addon_won`: the last `Loaded addons:` block names
  `<addons dir>/tbd-framework/addon.gproj` for the addon GUID. The Workshop copy of the addon
  carries the same GUID under the profile's `addons/TBDFramework_<GUID>/`, so only the path tells
  the deployed checkout from it.
- `assert_room_registered`: the log holds `Server registered with address:`, the engine's own line
  for a joinable backend room.
- `assert_admins_configured`: the engine logged `Server config loaded.` and `JSON is Valid`, so
  `game.admins[]` exists; an admin count of 0 passes with a warning that every `#tbd` command will
  answer `TBD: admin only.`.

It then says how strong the addon verdict is: a Workshop copy mounted or downloaded this boot, or
present on disk, makes it a real contest; no rival anywhere is reported as weak evidence.
`read_addon_guid` reads the GUID from `apps/mod/tbd-framework/addon.gproj`; a readable gproj without
a GUID line yields an empty GUID, which the deploy's settings check then reports as a mismatch.

`verify_boot_cli` (`--verify-boot`) reads no `deploy.env`: it takes `TBD_ADDON_GUID` (else the
gproj's), `TBD_ADDONS_STAGING` (required, exit 2 without it), `TBD_ADMIN_COUNT` (default 0) and
`TBD_PROFILE_DIR` from the environment, and exits 0 on `BOOT VERDICT: PASS` and 1 on `FAILED`.

## Boundaries

- Depends on: `super::Paths` and the `Out` sink in
  `tools_v2/xtask/src/commands/deploy/staging/boot.rs`; the `regex` crate; the gproj of
  `apps/mod/tbd-framework/`.
- Used by: `tools_v2/xtask/src/commands/deploy/staging.rs` (the two offline modes);
  `tools_v2/xtask/src/commands/deploy/staging/config.rs` (the GUID cross-check);
  `tools_v2/xtask/src/commands/deploy/staging/remote/verify_boot_remote.rs` (the verdict after a
  live restart).
- Rules: the addon check tells the checkout from the Workshop copy by path, not GUID
  (`addon_check_discriminates_on_path_not_guid` in
  `tools_v2/xtask/src/commands/deploy/staging/tests/boot/tests.rs`); the last `Loaded addons:` block
  wins (`last_loaded_addons_block_wins`); a missing log is never a pass
  (`missing_log_is_not_a_pass`); the self-test exits 0 only when every fixture gives its expected
  answer.
