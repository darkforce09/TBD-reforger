//! Operations: the event calendar and each event's hub, the event access control and reservation
//! pools, ORBAT slotting with the squad reservations and the waiting list, the member directory,
//! the game-runtime roster and player deployments, the derived attendance, the caller's service
//! record and leave requests, the saved mortar fire missions and the ballistics catalog uploads.
//!
//! **Role:** the API's operations domain: its `/api/v1` route table ([`routes()`]), the handlers
//! behind it, the event access, reservation, authoring, lifecycle, live occupancy, fire-mission
//! and ballistics catalog services, and the event, access, reservation, fire-mission, catalog and
//! leave request models.
//! **Position:** above `api_state` (the application state every handler extracts),
//! `api_caller_identity` (the lock order, the authority recheck, the machine caller),
//! `api_http_layer` (the caller extractors), `api_member_activity` (the statistics recompute and
//! the re-evaluation queue), the four domains it names (`api_identity_and_access`,
//! `api_match_telemetry`, `api_missions`, `api_server_infrastructure`), `api_audit_log`,
//! `api_configuration`, `api_database`, `api_foundation`, `api_identifiers`,
//! `api_mission_vocabulary`, `mission_model`, the ballistics crates (`ballistics_model`,
//! `ballistics_solver`, `fire_mission_planning`, `ballistics_calibration`) and
//! `fleet_wire_contract`. The API's router merges [`routes()`]; the event lifecycle sweeper and
//! the reservation re-evaluator workers, the command center domain, the staging fixtures tool and
//! the integration suites call its services and read its models.
//! **Signals & state:** none in memory; events, attachments, seats, reservations, policies,
//! pools, fire missions and catalog versions live in the database, reached through the caller's
//! pool or transaction.
//! **Invariants:** every reservation writer takes the one lock order of
//! [`services::event_reservations::reservation_scope`]; an event's status is derived from its
//! schedule inside Postgres; a stored fire mission carries the server's own solution against the
//! catalog version it pins.

mod error;
pub mod handlers;
pub mod models;
pub mod prelude;
pub mod routes;
pub mod services;

pub use error::{Error, Result};
pub use routes::routes;
