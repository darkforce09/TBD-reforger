//! The container↔host bridge the playtest server runs its host commands through.
//!
//! **Role:** names [`process_runner::host_execution::Host`] for the playtest's modules: container
//! detection and the `distrobox-host-exec`/`host-spawn` bridge.
//! **Position:** under [`crate::playtest_server`]; `boot.rs`, `lifecycle.rs` and the run order
//! import `super::host::Host`; the implementation, its tests and the reason the bridge exists live
//! in [`process_runner::host_execution`], shared with the wave drivers.
//! **Signals & state:** none.
//! **Invariants:** holds no behaviour; new bridge work belongs in `process_runner`.

pub(super) use process_runner::host_execution::Host;
