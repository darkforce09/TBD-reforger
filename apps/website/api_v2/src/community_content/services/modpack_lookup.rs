//! Modpack manifest loading: the pack-plus-nested-mods DTO and the three read paths
//! (by id, the active pack, and mod hydration) shared by the modpack endpoints, the
//! dashboard, and the server detail view.

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::community_content::models::modpack::{Modpack, ModpackMod};

/// Columns every modpack SELECT projects — keeps COALESCE null-tolerance identical
/// across list / current / get-by-id / write RETURNING paths.
///
/// A `macro_rules!` (not `const &str`): sqlx 0.9 `SqlSafeStr` only accepts `&'static str`
/// literals; `concat!` keeps one projection without `AssertSqlSafe` (same as `servers.rs`).
macro_rules! modpack_cols {
    () => {
        "id, name, version, total_size_bytes, \
         COALESCE(workshop_url, '') AS workshop_url, is_current, \
         COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at"
    };
}
pub(crate) use modpack_cols;

/// Columns every modpack_mods SELECT projects (the Reforger workshop fields included).
macro_rules! mod_cols {
    () => {
        "id, modpack_id, name, is_key_dependency, sort_order, \
         COALESCE(workshop_id, '') AS workshop_id, COALESCE(mod_guid, '') AS mod_guid, \
         COALESCE(version, '') AS version"
    };
}

/// A modpack with its mod list embedded — flattened so the wire shape is one object.
#[derive(Debug, Serialize)]
pub struct ModpackDto {
    #[serde(flatten)]
    pub modpack: Modpack,
    pub mods: Vec<ModpackMod>,
}

/// Load a modpack's mods (ordered) and wrap it as a DTO.
pub async fn with_mods(pool: &PgPool, modpack: Modpack) -> sqlx::Result<ModpackDto> {
    let mods: Vec<ModpackMod> = sqlx::query_as(concat!(
        "SELECT ",
        mod_cols!(),
        " FROM modpack_mods WHERE modpack_id = $1 \
         ORDER BY is_key_dependency DESC, sort_order ASC"
    ))
    .bind(modpack.id)
    .fetch_all(pool)
    .await?;
    Ok(ModpackDto { modpack, mods })
}

/// The active (`is_current`) modpack as a DTO, or `None` if none configured.
/// Shared by the dashboard + modpack endpoints.
pub async fn load_current_modpack(pool: &PgPool) -> sqlx::Result<Option<ModpackDto>> {
    let mp: Option<Modpack> = sqlx::query_as(concat!(
        "SELECT ",
        modpack_cols!(),
        " FROM modpacks WHERE is_current = true"
    ))
    .fetch_optional(pool)
    .await?;
    match mp {
        Some(mp) => Ok(Some(with_mods(pool, mp).await?)),
        None => Ok(None),
    }
}

/// Load one modpack DTO by id (or `None`).
pub async fn load_modpack(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<ModpackDto>> {
    let mp: Option<Modpack> = sqlx::query_as(concat!(
        "SELECT ",
        modpack_cols!(),
        " FROM modpacks WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?;
    match mp {
        Some(mp) => Ok(Some(with_mods(pool, mp).await?)),
        None => Ok(None),
    }
}
