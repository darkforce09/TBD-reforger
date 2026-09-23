//! HTTP handlers owned by the operations domain. The routes that reach them are registered in
//! [`super::routes`].

pub mod event_access_administration;
pub mod event_create_update;
pub mod event_group_administration;
pub mod event_hub;
pub mod event_listing;
pub mod event_mission_attachment;
pub mod fire_missions;
pub mod game_runtime_deployments;
pub mod game_runtime_roster;
pub mod leave_requests;
pub mod member_service_record;
pub mod orbat_view;
pub mod slot_assignment;
pub mod slot_registration;
pub mod waitlist_promotion;
