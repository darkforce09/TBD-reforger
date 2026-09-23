//! Reservation allocation shared by interactive and background transactions: one lock order,
//! pure planning, persisted quota allocations, claims, releases and promotion.

pub mod claim_refusals;
pub mod eligibility_reevaluation;
pub mod event_administration;
pub mod mission_restoration;
pub mod mutation_authority;
pub mod participant_allocations;
pub mod quota_availability;
pub mod quota_selection;
pub mod reevaluation_queue;
pub mod reservation_planning;
pub mod reservation_release;
pub mod reservation_scope;
pub mod scope_snapshot;
pub mod seat_claims;
pub mod seat_matching;
pub mod waitlist_promotion;
