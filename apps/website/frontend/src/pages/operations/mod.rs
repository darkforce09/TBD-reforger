//! T-934.3 — operations & ORBAT surfaces.

pub mod event_hub;
// events.rs renamed at the T-934.3 move (audit table 3.5): the module is the
// operations calendar, not a generic "events" bag.
pub mod event_schedule;
pub mod faction_manager;
pub mod orbat_manager;
pub mod orbat_selection;
