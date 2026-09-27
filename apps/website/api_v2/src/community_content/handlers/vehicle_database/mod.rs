//! The vehicle database: the identification table members read and administrators maintain.
//!
//! **Role:** the vehicle database handlers, one submodule per route family: the reads, the
//! whole-row writes (create and replace), the partial write, the removal, the one validator the
//! three writes share and the statements they all run.
//! **Position:** the domain's route table [`crate::community_content::routes::routes`] registers
//! the re-exported handlers under `/vehicle-database`, which `core::http_router` nests under
//! `/api/v1`; the handlers read and write `vehicle_databases` and answer
//! [`crate::community_content::models::VehicleDatabase`] rows.
//! **Signals & state:** none; each handler takes the pool from
//! [`AppState`](crate::core::application_state::AppState).
//! **Invariants:** reads take `AuthUser` and writes `AdminUser`, through each handler's own
//! extractor; every write checks its body before it opens a transaction, locks an existing row
//! before it changes it, and appends its audit line in that transaction; every handler carries
//! the `@route` tag of its registration.

mod create_and_replace;
mod delete;
mod patch;
mod reads;
mod validation;
mod vehicle_rows;

pub use create_and_replace::{create_vehicle, replace_vehicle};
pub use delete::delete_vehicle;
pub use patch::patch_vehicle;
pub use reads::{get_vehicle, list_vehicles};
pub use validation::{VehiclePatchBody, VehicleWriteBody};
