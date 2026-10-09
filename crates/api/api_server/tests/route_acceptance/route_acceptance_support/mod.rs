//! The route acceptance framework of the `route_acceptance` binary.
//!
//! **Role:** proves every route of each part against two dimensions — the authorized caller
//! gets the documented success and contract, the unauthorized caller is refused — from compact
//! per-route specs; the framework derives every probe that needs no domain knowledge.
//!
//! **Position:** compiled into the `route_acceptance` binary beside `common` and
//! `contract_support`, which it uses. Parts: [`specs`] holds every part's specs, [`world`] every
//! part's world.
//!
//! **Signals & state:** see each module; none is process-wide.
//!
//! **Invariants:** the shared core ([`spec`], [`actors`], [`requests`], [`world`], [`specs`]) is
//! what every part builds on, so a part changes only its own spec and world files.

pub(crate) mod actors;
pub(crate) mod contracts;
pub(crate) mod derived_probes;
pub(crate) mod dimension_runner;
pub(crate) mod probe;
pub(crate) mod requests;
pub(crate) mod spec;
pub(crate) mod specs;
pub(crate) mod world;
