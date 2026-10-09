//! Integration tests of the operations domain's events and reservations: event access and
//! visibility, administration, eligibility policies, ORBAT attachment, registration and seats,
//! the roster, reservation guards, quotas, waitlist promotion, live slot occupancy and
//! attendance.
//!
//! One test binary on one database: every module shares the database
//! [`common::require_test_database_url`] provisions once and isolates its tests through rows
//! and ids it mints for itself.

#[path = "../common/mod.rs"]
mod common;
#[path = "../event_eligibility_support/mod.rs"]
mod event_eligibility_support;
#[path = "../events_support/mod.rs"]
mod events_support;
#[path = "../fleet_support/mod.rs"]
mod fleet_support;
#[path = "../reservation_guard_support/mod.rs"]
mod reservation_guard_support;
#[path = "../telemetry_support/mod.rs"]
mod telemetry_support;

mod attendance_no_show_derivation;
mod eligibility_release_transactions;
mod event_access_context;
mod event_administration_transactions;
mod event_eligibility_policies;
mod event_visibility_projection;
mod events_field_contracts;
mod events_orbat_attachment;
mod events_registration_and_seats;
mod events_roster_and_members;
mod live_slot_occupancy;
mod reservation_attendance_transactions;
mod reservation_guard_transactions;
mod reservation_quota_allocations;
mod waitlist_promotion_transactions;
