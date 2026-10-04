//! Source pins: the production text of the operations schedule, the operation dossier and the
//! service record, for the guard tests that read it.
//!
//! **Role:** each function returns the full text of one logical source file of this area. Where
//! that file is split into shards, the function joins them in declaration order, so a guard that
//! reads the whole file still sees every definition exactly once.
//! **Position:** test-only, inside the area it reads; called from the operations pages' guard
//! tests.
//! **Signals & state:** none; every shard is embedded at compile time.
//! **Invariants:** every `include_str!` is relative to this file and names a file of this area, so
//! the pins keep resolving wherever the area moves; a shard is never listed twice.

use frontend_test_support::source_shards::production_source;

/// The operations schedule, as one text: the route component with its split pane, and the
/// operation card the master column repeats.
pub(crate) fn event_schedule_source() -> String {
    production_source(&[
        include_str!("../schedule/mod.rs"),
        include_str!("../schedule/page.rs"),
        include_str!("../schedule/upcoming_ops.rs"),
    ])
}

/// The operation dossier, as one text: the route component, the hub body, the mission dossier
/// card, the faction cards, the slotting selector with its squad pane, seat rows, footer actions
/// and assign picker, and the viewer's registration access.
pub(crate) fn event_hub_source() -> String {
    production_source(&[
        include_str!("../event_detail/mod.rs"),
        include_str!("../event_detail/page.rs"),
        include_str!("../event_detail/hero_countdown.rs"),
        include_str!("../event_detail/mission_dossier.rs"),
        include_str!("../event_detail/faction_armory.rs"),
        include_str!("../event_detail/slotting_selector.rs"),
        include_str!("../event_detail/squad_pane.rs"),
        include_str!("../event_detail/seat_row.rs"),
        include_str!("../event_detail/reservation_actions.rs"),
        include_str!("../event_detail/assign_picker.rs"),
        include_str!("../event_detail/registration_access/mod.rs"),
        include_str!("../event_detail/registration_access/mission_standing.rs"),
        include_str!("../event_detail/registration_access/place_outlook.rs"),
        include_str!("../event_detail/registration_access/places_panel.rs"),
        include_str!("../event_detail/registration_access/refusal_notices.rs"),
        include_str!("../event_detail/registration_access/seat_eligibility.rs"),
        include_str!("../event_detail/registration_access/waitlist_promotion.rs"),
    ])
}

/// The service record, as one text: the route component, the active-orders banner, the combat
/// history table, both leave panels, and the column heading they share.
pub(crate) fn deployments_source() -> String {
    production_source(&[
        include_str!("../deployments/mod.rs"),
        include_str!("../deployments/page.rs"),
        include_str!("../deployments/active_orders.rs"),
        include_str!("../deployments/service_record.rs"),
        include_str!("../deployments/leave_of_absence.rs"),
        include_str!("../deployments/leave_review_queue.rs"),
        include_str!("../deployments/table_head.rs"),
    ])
}
