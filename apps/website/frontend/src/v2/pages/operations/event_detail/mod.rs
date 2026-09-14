//! The operation dossier and everything it is built from.
//!
//! **Role:** declares the route component, the hub body shared with the schedule, the mission
//! dossier card, the faction cards, the slotting selector with its squad pane and assign picker,
//! and re-exports the page for the router and the selector for the standalone slotting route.
//! **Position:** the `/events/:id` route, in the operations hub.
//! **Signals & state:** none at this level; the route owns the operation fetch and the selector
//! owns the slotting state.
//! **Invariants:** the hub body is the one renderer of an operation, so the schedule's detail
//! column and this route can never drift apart.
#![allow(dead_code)]

mod assign_picker;
mod faction_armory;
mod hero_countdown;
mod mission_dossier;
mod page;
mod slotting_selector;
mod squad_pane;

pub(crate) use hero_countdown::event_hub_view;
pub use page::EventHubPage;
pub use slotting_selector::OrbatSelector;

// The guard battery spans every shard of this page, so the names it reaches for through
// `use super::*` are gathered here.
#[cfg(test)]
use crate::v2::core::api::dto::{EventHub, EventMissionDossier};
#[cfg(test)]
use crate::v2::core::ui::DEFAULT_AVATAR;
#[cfg(test)]
use mission_dossier::{briefing_text, hub_modpack_fetch, meta_badges, HubModpackFetch};

#[cfg(test)]
#[path = "tests/event_hub.rs"]
mod tests;
