# Deploy

Deployment, backup, restore drills, installation, and staging service configuration. Staging separates agent templates, configuration, transport, and operations.

Source modules: `cli.rs`, `database_backup.rs`, `database_operations.rs`, `database_restore.rs`, `database_restore_drill.rs`, `dispatch.rs`, `mod.rs`, `staging.rs`, `website.rs`. The website lane keeps its pure parts under `website/`: `rsync_argv.rs` (the exclude list, which is also the `--delete` guard), `asset_preflight.rs` (the remote map-asset probe that runs before the rsync), `systemd_unit.rs` (the hand-installed unit's install command).
