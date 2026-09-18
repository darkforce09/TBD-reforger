//! The registry phys table the cargo-capacity walk is measured against.
//!
//! The map engine's `scan_cargo_capacity` knows the shape of a loadout but carries no registry of
//! its own: item weights/volumes and container maxima live in `registry_items`, keyed by
//! `resource_name`, and only the **current** modpack's rows count. Whoever wants a capacity verdict
//! has to supply that table, so this loader is the single place it is read.
//!
//! Two callers need the identical table, and they must agree or a mission refused at one boundary
//! is seated at the other:
//!
//! * save-time validation — `missions::handlers::mission_versions::validate_payload`, which
//!   refuses an over-capacity payload before it reaches `mission_versions.json_payload`, and
//!   `missions::handlers::mission_export::get_compiled_mission`, which compiles the stored tip for
//!   a game server.
//! * roster ingest — `operations::handlers::roster_ingest::ingest_event_roster`, which recompiles
//!   every mission of an event to recover slot uids and must omit a mission Save would have
//!   refused, rather than seat it from an empty-catalog no-op.
//!
//! It lives under `missions::services` because the catalog is mission-compile vocabulary; the
//! operations domain reaches it through services, never through the mission handlers.

use sqlx::PgPool;
use website_map_engine::data::scenario::wire_safety::{CargoPhys, CargoPhysCatalog};

use crate::core::error_handling::api_error::ApiError;

/// Phys attrs for the cargo-capacity walk — only the columns `scan_cargo_capacity` needs.
#[derive(sqlx::FromRow)]
struct CargoPhysRow {
    resource_name: String,
    display_name: String,
    weight_kg: Option<f64>,
    volume_cm3: Option<f64>,
    max_weight_kg: Option<f64>,
    max_volume_cm3: Option<f64>,
}

/// Load `CargoPhysCatalog` from the **current** modpack's `registry_items`.
///
/// Missing weights / maxima stay `None` (never invent — the same silence as an empty catalog). No
/// current modpack or an empty table → empty catalog → the cargo walk is a no-op.
pub(crate) async fn load_cargo_phys_catalog(pool: &PgPool) -> Result<CargoPhysCatalog, ApiError> {
    let rows: Vec<CargoPhysRow> = sqlx::query_as(
        "SELECT ri.resource_name, ri.display_name, \
                ri.weight_kg, ri.volume_cm3, ri.max_weight_kg, ri.max_volume_cm3 \
         FROM registry_items ri \
         INNER JOIN modpacks m ON m.id = ri.modpack_id \
         WHERE m.is_current = true",
    )
    .fetch_all(pool)
    .await?;
    let mut catalog = CargoPhysCatalog::with_capacity(rows.len());
    for r in rows {
        catalog.insert(
            r.resource_name,
            CargoPhys {
                display_name: r.display_name,
                weight_kg: r.weight_kg,
                volume_cm3: r.volume_cm3,
                max_weight_kg: r.max_weight_kg,
                max_volume_cm3: r.max_volume_cm3,
            },
        );
    }
    Ok(catalog)
}
