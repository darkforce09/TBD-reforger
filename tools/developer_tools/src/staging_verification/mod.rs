//! Engines the staging verification harness runs against the staging host: the member load from
//! the workstation, and the acknowledgement-dropping relay on the staging host itself.
//!
//! - **Role:** holds the self-contained engines behind the xtask `staging` command group; each
//!   submodule owns one engine, its inputs and its report.
//! - **Position:** developer_tools library. The xtask staging procedures build an engine's inputs
//!   from the committed staging data and the run's bindings, call it, and journal what it
//!   reports; the engines speak to the staging host over the network and never touch the
//!   repository tree. The relay runs on the staging host as the `acknowledgement-dropping-relay`
//!   executable, and the fleet procedure drives it there over its control socket.
//! - **Signals & state:** none at this level; an engine builds and owns its own tokio runtime.
//! - **Invariants:** an engine never writes a credential to disk, a log or its report.

pub mod acknowledgement_relay;
pub mod load_generation;
