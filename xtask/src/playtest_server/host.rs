//! The container↔host bridge — **moved**. This is a re-export, not an implementation.
//!
//! T-853 lifted the bridge to [`crate::hostrun`] so the playtest server and the `wave.sh` port share
//! ONE implementation instead of two copies of `scripts/lib/hostrun.sh`. Everything that used to
//! live here — `Host::detect`, `capture`, `capture_trimmed`, `signal_quietly`, `spawn_background`,
//! `require_host`, `instruction_name`, `broken` — is unchanged and lives there now, along with the
//! tests that pinned it and the full record of WHY the bridge still exists (short version: the C
//! toolchain half of the shim is obsolete, the host-glibc half is not).
//!
//! This file survives as a one-line re-export so `super::host::Host` keeps resolving for `boot.rs`
//! and `lifecycle.rs`. It holds no behaviour and nothing should be added to it — new bridge work
//! belongs in [`crate::hostrun`], which is the point of the lift.

pub use crate::hostrun::Host;
