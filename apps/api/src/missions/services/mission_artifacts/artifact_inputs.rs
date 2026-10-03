//! Everything that determines a compiled mission document, read on one connection so the inputs
//! and the bytes they produce belong to one snapshot: the mission row the compiler reads, the
//! version payload, the current modpack's cargo catalog, and the compiler's own identity.

use mission_wire_safety::{CargoPhys, CargoPhysCatalog};
use serde_json::{Value, json};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;
use crate::core::wire_format::content_digest::sha256_hex;
use crate::missions::models::mission::Mission;

/// The cargo catalog of the current modpack, with the digest of its canonical form.
pub struct CatalogSnapshot {
    pub catalog: CargoPhysCatalog,
    pub sha256: String,
    pub modpack: Option<(Uuid, String)>,
}

#[derive(sqlx::FromRow)]
struct CatalogRow {
    resource_name: String,
    display_name: String,
    weight_kg: Option<f64>,
    volume_cm3: Option<f64>,
    max_weight_kg: Option<f64>,
    max_volume_cm3: Option<f64>,
}

/// Load the current modpack's catalog. Rows are ordered by resource name (C collation), so the
/// canonical form and its digest do not depend on storage order.
pub async fn load_catalog_snapshot(
    connection: &mut PgConnection,
) -> Result<CatalogSnapshot, ApiError> {
    let modpack: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, version FROM modpacks WHERE is_current ORDER BY id LIMIT 1")
            .fetch_optional(&mut *connection)
            .await?;
    let rows: Vec<CatalogRow> = sqlx::query_as(
        "SELECT ri.resource_name, ri.display_name, ri.weight_kg, ri.volume_cm3,
             ri.max_weight_kg, ri.max_volume_cm3
         FROM registry_items ri JOIN modpacks m ON m.id = ri.modpack_id
         WHERE m.is_current = true ORDER BY ri.resource_name COLLATE \"C\"",
    )
    .fetch_all(connection)
    .await?;
    let canonical: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!([
                row.resource_name,
                row.display_name,
                row.weight_kg,
                row.volume_cm3,
                row.max_weight_kg,
                row.max_volume_cm3
            ])
        })
        .collect();
    let sha256 = sha256_hex(
        serde_json::to_string(&canonical)
            .unwrap_or_default()
            .as_bytes(),
    );
    let mut catalog = CargoPhysCatalog::with_capacity(rows.len());
    for row in rows {
        catalog.insert(
            row.resource_name,
            CargoPhys {
                display_name: row.display_name,
                weight_kg: row.weight_kg,
                volume_cm3: row.volume_cm3,
                max_weight_kg: row.max_weight_kg,
                max_volume_cm3: row.max_volume_cm3,
            },
        );
    }
    Ok(CatalogSnapshot {
        catalog,
        sha256,
        modpack,
    })
}

/// The mission fields the compiler reads, in a canonical object.
pub fn compiled_metadata(mission: &Mission) -> Value {
    json!({
        "id": mission.id,
        "title": mission.title,
        "author": mission.author_id,
        "terrain": mission.terrain.as_str(),
        "custom_terrain_name": mission.custom_terrain_name,
        "game_mode": mission.game_mode.as_str(),
        "max_players": mission.max_players,
        "time_of_day": mission.time_of_day,
        "weather": mission.weather.as_str(),
    })
}
