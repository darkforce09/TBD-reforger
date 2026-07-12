//! Registry envelope ingest (T-068.9) — idempotent, modpack-scoped upsert of the
//! T-150 Workbench exports (items + compat edges) into Postgres.
//!
//! The whole pipeline is a pure function of the envelope: kinds and edge types are
//! carried as plain text end-to-end, so a new edge family or item kind ships via a
//! schema-enum bump + `make schema-codegen` alone — no importer, DDL, or API change.
//! Used by the `import-registry` binary and the integration tests.
//!
//! @contract registry-items.schema.json#/
//! @contract registry-compat.schema.json#/

use std::collections::BTreeMap;

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::contract::generated::{registry_compat, registry_items};
use crate::contract::{
    ContractError, validate_registry_compat_envelope, validate_registry_items_envelope,
};

/// Rows per UNNEST statement. Arrays are single bind parameters (the 65k-param
/// limit does not apply); chunking only bounds per-statement memory so envelope
/// size is unbounded.
const CHUNK: usize = 10_000;

/// Outcome of one envelope ingest. `total` = envelope rows, `unique` = after
/// last-wins key dedupe (duplicate keys in one envelope would abort the upsert
/// statement), `inserted + updated + unchanged = unique`.
#[derive(Debug, Default)]
pub struct ImportCounts {
    pub total: u64,
    pub unique: u64,
    pub inserted: u64,
    pub updated: u64,
    pub pruned: u64,
    /// Per-kind (items) or per-edge_type (compat) envelope histogram.
    pub histogram: BTreeMap<String, u64>,
}

/// Ingest failure. `Invalid` carries the schema-violation details (the envelope
/// never reaches SQL); the rest are environmental.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("envelope failed schema validation: {}", .0.join("; "))]
    Invalid(Vec<String>),
    #[error("envelope is not parseable: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("modpack id is not a UUID: {0}")]
    BadModpack(String),
    #[error(transparent)]
    Contract(#[from] ContractError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

/// Serialize a schema enum to its wire string (serde rename is the vocabulary).
fn wire_str<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|x| x.as_str().map(String::from))
        .unwrap_or_default()
}

/// Resolve the target modpack: explicit override wins, else the envelope's
/// `modpackId` (which must parse as a UUID).
fn resolve_modpack_id(envelope_id: &str, over: Option<Uuid>) -> Result<Uuid, ImportError> {
    match over {
        Some(id) => Ok(id),
        None => {
            Uuid::parse_str(envelope_id).map_err(|_| ImportError::BadModpack(envelope_id.into()))
        }
    }
}

/// Upsert the modpack FK-by-convention row so `?modpack=` resolution works for a
/// fresh import target. Never touches an existing row and never sets
/// `is_current` — importing must not steal the platform's current modpack.
pub async fn ensure_modpack(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO modpacks (id, name, version, total_size_bytes, is_current, created_at) \
         VALUES ($1, 'Imported registry (T-150 export)', '0', 0, false, now()) \
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

const COUNT_ITEMS: &str = "SELECT count(*) FROM registry_items WHERE modpack_id = $1";
const COUNT_COMPAT: &str = "SELECT count(*) FROM registry_compat WHERE modpack_id = $1";

async fn count_rows(
    tx: &mut sqlx::PgConnection,
    sql: &'static str,
    modpack: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(sql)
        .bind(modpack)
        .fetch_one(tx)
        .await
}

/// Ingest a T-150 items envelope. Idempotent: upsert by `(modpack_id,
/// resource_name)`; the `IS DISTINCT FROM` guard makes a no-op re-run touch zero
/// rows (stable `updated_at` ⇒ stable ETag). `icon_url` is written on insert but
/// never updated (curated icons survive re-imports). `sort_order` = envelope
/// index. `prune` deletes modpack rows absent from the envelope.
pub async fn import_items(
    pool: &PgPool,
    raw: &[u8],
    modpack_override: Option<Uuid>,
    prune: bool,
) -> Result<ImportCounts, ImportError> {
    let details = validate_registry_items_envelope(raw)?;
    if !details.is_empty() {
        return Err(ImportError::Invalid(details));
    }
    let env: registry_items::RegistryItems = serde_json::from_slice(raw)?;
    let modpack = resolve_modpack_id(&env.modpack_id, modpack_override)?;
    ensure_modpack(pool, modpack).await?;

    let mut counts = ImportCounts {
        total: env.items.len() as u64,
        ..Default::default()
    };
    // Last-wins dedupe by resource_name, keeping each survivor's envelope index
    // as its sort_order (deterministic browse order = export order).
    let mut by_key: BTreeMap<&str, (usize, &registry_items::ItemElement)> = BTreeMap::new();
    for (i, it) in env.items.iter().enumerate() {
        *counts.histogram.entry(wire_str(&it.kind)).or_insert(0) += 1;
        by_key.insert(it.resource_name.as_str(), (i, it));
    }
    let unique: Vec<(usize, &registry_items::ItemElement)> = by_key.into_values().collect();
    counts.unique = unique.len() as u64;

    let mut tx = pool.begin().await?;
    let before = count_rows(&mut tx, COUNT_ITEMS, modpack).await?;

    let mut affected = 0u64;
    for chunk in unique.chunks(CHUNK) {
        let rns: Vec<String> = chunk
            .iter()
            .map(|(_, it)| it.resource_name.clone())
            .collect();
        let dns: Vec<String> = chunk
            .iter()
            .map(|(_, it)| it.display_name.clone())
            .collect();
        let cats: Vec<String> = chunk.iter().map(|(_, it)| it.category.clone()).collect();
        let kinds: Vec<String> = chunk.iter().map(|(_, it)| wire_str(&it.kind)).collect();
        let icons: Vec<Option<String>> = chunk
            .iter()
            .map(|(_, it)| it.icon_url.clone().filter(|s| !s.is_empty()))
            .collect();
        let orders: Vec<i64> = chunk.iter().map(|(i, _)| *i as i64).collect();
        // v3 (T-068.10.2) metadata — all optional; v2 envelopes bind NULL columns.
        let abstracts: Vec<Option<bool>> = chunk
            .iter()
            .map(|(_, it)| it.registry_items_schema_abstract)
            .collect();
        let arsenal_types: Vec<Option<String>> =
            chunk.iter().map(|(_, it)| it.arsenal_type.clone()).collect();
        let weights: Vec<Option<f64>> = chunk.iter().map(|(_, it)| it.weight_kg).collect();
        let volumes: Vec<Option<f64>> = chunk.iter().map(|(_, it)| it.volume_cm3).collect();
        let max_weights: Vec<Option<f64>> = chunk.iter().map(|(_, it)| it.max_weight_kg).collect();
        let max_volumes: Vec<Option<f64>> = chunk.iter().map(|(_, it)| it.max_volume_cm3).collect();
        let addons: Vec<Option<String>> = chunk.iter().map(|(_, it)| it.addon.clone()).collect();
        let variant_ofs: Vec<Option<String>> =
            chunk.iter().map(|(_, it)| it.variant_of.clone()).collect();
        affected += sqlx::query(
            "INSERT INTO registry_items \
               (modpack_id, resource_name, display_name, category, kind, icon_url, sort_order, \
                \"abstract\", arsenal_type, weight_kg, volume_cm3, max_weight_kg, max_volume_cm3, addon, \
                variant_of, created_at, updated_at) \
             SELECT $1, u.rn, u.dn, u.cat, u.kind, u.icon, u.ord, \
                    u.abs, u.aty, u.wkg, u.vcm, u.mwkg, u.mvcm, u.addon, u.varof, now(), now() \
             FROM UNNEST($2::text[], $3::text[], $4::text[], $5::text[], $6::text[], $7::bigint[], \
                         $8::boolean[], $9::text[], $10::float8[], $11::float8[], $12::float8[], $13::float8[], $14::text[], \
                         $15::text[]) \
               AS u(rn, dn, cat, kind, icon, ord, abs, aty, wkg, vcm, mwkg, mvcm, addon, varof) \
             ON CONFLICT (modpack_id, resource_name) DO UPDATE SET \
               display_name = EXCLUDED.display_name, category = EXCLUDED.category, \
               kind = EXCLUDED.kind, sort_order = EXCLUDED.sort_order, \
               \"abstract\" = EXCLUDED.\"abstract\", arsenal_type = EXCLUDED.arsenal_type, \
               weight_kg = EXCLUDED.weight_kg, volume_cm3 = EXCLUDED.volume_cm3, \
               max_weight_kg = EXCLUDED.max_weight_kg, max_volume_cm3 = EXCLUDED.max_volume_cm3, \
               addon = EXCLUDED.addon, variant_of = EXCLUDED.variant_of, updated_at = now() \
             WHERE (registry_items.display_name, registry_items.category, registry_items.kind, \
                    registry_items.sort_order, registry_items.\"abstract\", registry_items.arsenal_type, \
                    registry_items.weight_kg, registry_items.volume_cm3, registry_items.max_weight_kg, \
                    registry_items.max_volume_cm3, registry_items.addon, registry_items.variant_of) \
               IS DISTINCT FROM \
                   (EXCLUDED.display_name, EXCLUDED.category, EXCLUDED.kind, EXCLUDED.sort_order, \
                    EXCLUDED.\"abstract\", EXCLUDED.arsenal_type, EXCLUDED.weight_kg, EXCLUDED.volume_cm3, \
                    EXCLUDED.max_weight_kg, EXCLUDED.max_volume_cm3, EXCLUDED.addon, EXCLUDED.variant_of)",
        )
        .bind(modpack)
        .bind(&rns)
        .bind(&dns)
        .bind(&cats)
        .bind(&kinds)
        .bind(&icons)
        .bind(&orders)
        .bind(&abstracts)
        .bind(&arsenal_types)
        .bind(&weights)
        .bind(&volumes)
        .bind(&max_weights)
        .bind(&max_volumes)
        .bind(&addons)
        .bind(&variant_ofs)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    }

    if prune {
        let keep: Vec<String> = unique
            .iter()
            .map(|(_, it)| it.resource_name.clone())
            .collect();
        counts.pruned = sqlx::query(
            "DELETE FROM registry_items \
             WHERE modpack_id = $1 AND NOT (resource_name = ANY($2::text[]))",
        )
        .bind(modpack)
        .bind(&keep)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    }

    let after = count_rows(&mut tx, COUNT_ITEMS, modpack).await?;
    tx.commit().await?;

    counts.inserted = (after + i64::try_from(counts.pruned).unwrap_or(0) - before).max(0) as u64;
    counts.updated = affected.saturating_sub(counts.inserted);
    Ok(counts)
}

/// Ingest a T-150 compat envelope. Idempotent: upsert by `(modpack_id, from_node,
/// to_node, edge_type)`; only `evidence` is updatable, guarded by `IS DISTINCT
/// FROM` (no-op re-run touches zero rows). Empty-string evidence is stored as
/// NULL (canonical form: NULL ≡ '' ≡ absent). `prune` deletes modpack edges
/// absent from the envelope.
pub async fn import_compat(
    pool: &PgPool,
    raw: &[u8],
    modpack_override: Option<Uuid>,
    prune: bool,
) -> Result<ImportCounts, ImportError> {
    let details = validate_registry_compat_envelope(raw)?;
    if !details.is_empty() {
        return Err(ImportError::Invalid(details));
    }
    let env: registry_compat::RegistryCompat = serde_json::from_slice(raw)?;
    let modpack = resolve_modpack_id(&env.modpack_id, modpack_override)?;
    ensure_modpack(pool, modpack).await?;

    let mut counts = ImportCounts {
        total: env.edges.len() as u64,
        ..Default::default()
    };
    // Last-wins dedupe by the edge key (duplicate keys in one statement abort
    // ON CONFLICT DO UPDATE).
    let mut by_key: BTreeMap<(String, String, String), &registry_compat::EdgeElement> =
        BTreeMap::new();
    for e in &env.edges {
        let ty = wire_str(&e.edge_type);
        *counts.histogram.entry(ty.clone()).or_insert(0) += 1;
        by_key.insert((e.from_node.clone(), e.to_node.clone(), ty), e);
    }
    counts.unique = by_key.len() as u64;

    let mut tx = pool.begin().await?;
    let before = count_rows(&mut tx, COUNT_COMPAT, modpack).await?;

    let entries: Vec<((String, String, String), &registry_compat::EdgeElement)> =
        by_key.into_iter().collect();
    let mut affected = 0u64;
    for chunk in entries.chunks(CHUNK) {
        let froms: Vec<String> = chunk.iter().map(|((f, _, _), _)| f.clone()).collect();
        let tos: Vec<String> = chunk.iter().map(|((_, t, _), _)| t.clone()).collect();
        let types: Vec<String> = chunk.iter().map(|((_, _, ty), _)| ty.clone()).collect();
        let evs: Vec<Option<String>> = chunk
            .iter()
            .map(|(_, e)| e.evidence.clone().filter(|s| !s.is_empty()))
            .collect();
        affected += sqlx::query(
            "INSERT INTO registry_compat \
               (modpack_id, from_node, to_node, edge_type, evidence, created_at, updated_at) \
             SELECT $1, u.f, u.t, u.ty, u.ev, now(), now() \
             FROM UNNEST($2::text[], $3::text[], $4::text[], $5::text[]) AS u(f, t, ty, ev) \
             ON CONFLICT (modpack_id, from_node, to_node, edge_type) DO UPDATE SET \
               evidence = EXCLUDED.evidence, updated_at = now() \
             WHERE registry_compat.evidence IS DISTINCT FROM EXCLUDED.evidence",
        )
        .bind(modpack)
        .bind(&froms)
        .bind(&tos)
        .bind(&types)
        .bind(&evs)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    }

    if prune {
        let froms: Vec<String> = entries.iter().map(|((f, _, _), _)| f.clone()).collect();
        let tos: Vec<String> = entries.iter().map(|((_, t, _), _)| t.clone()).collect();
        let types: Vec<String> = entries.iter().map(|((_, _, ty), _)| ty.clone()).collect();
        counts.pruned = sqlx::query(
            "DELETE FROM registry_compat rc WHERE rc.modpack_id = $1 AND NOT EXISTS ( \
               SELECT 1 FROM UNNEST($2::text[], $3::text[], $4::text[]) AS u(f, t, ty) \
               WHERE u.f = rc.from_node AND u.t = rc.to_node AND u.ty = rc.edge_type)",
        )
        .bind(modpack)
        .bind(&froms)
        .bind(&tos)
        .bind(&types)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    }

    let after = count_rows(&mut tx, COUNT_COMPAT, modpack).await?;
    tx.commit().await?;

    counts.inserted = (after + i64::try_from(counts.pruned).unwrap_or(0) - before).max(0) as u64;
    counts.updated = affected.saturating_sub(counts.inserted);
    Ok(counts)
}
