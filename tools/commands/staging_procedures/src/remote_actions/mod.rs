//! The staging harness's host actions: every command that changes the staging host.
//!
//! **Role:** declares one builder module per kind of change: the host tool's fixture commands,
//! the Discord outage drop-in, the relay's control, the database backup and the game server
//! update.
//!
//! **Position:** used by `staging_dispatch.rs` for the confirmed actions and by the procedures' host
//! action steps and recovery lists; every command runs through
//! `remote_observers/host_shell.rs`.
//!
//! **Signals & state:** none; pure builders.
//!
//! **Invariants:** a builder here returns a [`crate::remote_observers::remote_command::CommandPurpose::Change`]
//! command except the read it pairs with (the drop-in's state, the relay's status, the installed
//! build, the bot's member read), and none carries a secret.

pub(crate) mod database_backup;
pub(crate) mod game_server_update;
pub(crate) mod host_fixture_commands;
pub(crate) mod outage_dropin;
pub(crate) mod relay_control;
