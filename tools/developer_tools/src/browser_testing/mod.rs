//! The `gate` and `capture` command lines and the two gate suites that wait for the ballistics
//! crates: the ballistics agreement and the offline mortar page.
//!
//! **Role:** parses the `gate` and `capture` arguments and dispatches each subcommand to its suite
//! in `browser_gate_suites`, or to the ballistics agreement and offline mortar suites here.
//! **Position:** the library half of the `gate` and `capture` binaries; the suites here drive
//! Chromium through `chrome_devtools_protocol` and serve the app through `browser_gate_suites`.
//! **Signals & state:** none at this level; each suite owns its own run.
//! **Invariants:** the command lines alone decide the exit codes.

pub mod capture_cli;

pub mod cli;

pub mod mortar_offline;

pub mod ballistics_agreement;
