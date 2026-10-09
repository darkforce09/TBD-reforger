# Staging boot verdict

The judgement over a dedicated server's `console.log` that `cargo xtask deploy staging` passes or
fails a deploy on, and the self-test that proves each check can fail. Both run offline, through
`deploy staging --verify-boot <log>` and `--verify-boot-selftest`, and the deploy runs the same
verdict over the log it pulls back. `tools/commands/deployment/src/staging/boot.rs` declares
both files, holds the `Out` sink that prints or captures, and re-exports the functions.

## Contents

```text
tools/commands/deployment/src/staging/boot/
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
`read_addon_guid` reads the GUID from `mod/tbd-framework/addon.gproj`; a readable gproj without
a GUID line yields an empty GUID, which the deploy's settings check then reports as a mismatch.

`verify_boot_cli` (`--verify-boot`) reads no `deploy.env`: it takes `TBD_ADDON_GUID` (else the
gproj's), `TBD_ADDONS_STAGING` (required, exit 2 without it), `TBD_ADMIN_COUNT` (default 0) and
`TBD_PROFILE_DIR` from the environment, and exits 0 on `BOOT VERDICT: PASS` and 1 on `FAILED`.

## Boundaries

- Depends on: `super::Paths` and the `Out` sink in
  `tools/commands/deployment/src/staging/boot.rs`; the `regex` crate; the gproj of
  `mod/tbd-framework/`.
- Used by: `tools/commands/deployment/src/staging.rs` (the two offline modes);
  `tools/commands/deployment/src/staging/config.rs` (the GUID cross-check);
  `tools/commands/deployment/src/staging/remote/instance_boot_verdict.rs` (each fleet
  instance's log check after a live restart).
- Rules: the addon check tells the checkout from the Workshop copy by path, not GUID; the last `Loaded addons:` block
  wins; a missing log is never a pass; the self-test exits 0 only when every fixture gives its expected
  answer.
