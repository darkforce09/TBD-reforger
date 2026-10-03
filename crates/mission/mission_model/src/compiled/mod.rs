//! The compiled rows of a mission document.
//!
//! **Role:** the typed rows the game-document compiler emits: entities, vehicles with their crew
//! seats, slots with their loadouts, the ORBAT groups and radio nets ([`entities`]), and the
//! mission-level sections, zones, factions, meta, environment, flow, win rule, settings, markers
//! and briefings ([`mission`]).
//! **Position:** part of `mission_model`; the map engine's game-document compiler builds these
//! rows from the authored payload and serialises them as the compiled mission document.
//! **Signals & state:** none; plain data types.
//! **Invariants:** each row serialises to exactly the `mission.schema.json` `$defs` shape it
//! names; optional keys are skipped rather than emitted empty where the schema says so.

pub mod entities;
pub mod mission;
