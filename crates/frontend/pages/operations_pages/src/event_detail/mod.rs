//! The operation dossier and everything it is built from.
//!
//! **Role:** declares the route component, the hub body shared with the schedule, the mission
//! dossier card, the faction cards, the slotting selector with its squad pane, seat rows, footer
//! actions and assign picker, and the viewer's registration access; re-exports the page for the
//! router, and the selector with the mission standing it takes for the standalone slotting route.
//! **Position:** the `/events/:id` route, in the operations hub.
//! **Signals & state:** none at this level; the route owns the operation fetch and the selector
//! owns the slotting state.
//! **Invariants:** the hub body is the one renderer of an operation, so the schedule's detail
//! column and this route can never drift apart. Everything about what the viewer may register for
//! is derived from what the backend returned to that viewer.

pub mod assign_picker;
mod faction_armory;
mod hero_countdown;
mod mission_dossier;
pub mod page;
pub mod registration_access;
mod reservation_actions;
mod seat_row;
pub mod slotting_selector;
mod squad_pane;

#[cfg(target_arch = "wasm32")]
pub(crate) use hero_countdown::event_hub_view;
#[cfg(target_arch = "wasm32")]
pub use page::EventHubPage;
#[cfg(target_arch = "wasm32")]
pub(crate) use registration_access::mission_standing::MissionStanding;
#[cfg(target_arch = "wasm32")]
pub(crate) use registration_access::mission_standing::standing_notices;
#[cfg(target_arch = "wasm32")]
pub(crate) use slotting_selector::OrbatSelector;

// The guard battery spans every shard of this page, so the names it reaches for through
// `use super::*` are gathered here.
#[cfg(test)]
use frontend_api_dtos::{EventHub, EventMissionDossier};
#[cfg(test)]
use frontend_ui::DEFAULT_AVATAR;
#[cfg(test)]
use mission_dossier::{HubModpackFetch, briefing_text, hub_modpack_fetch, meta_badges};

#[cfg(test)]
#[path = "tests/event_hub.rs"]
mod tests;
