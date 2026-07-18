//! T-068.9 registry ingest + compat API slice — proof-ledger gates G1–G5, G9, G10
//! (see `.ai/artifacts/t068_9_verify_log.md`): ingest bijection, idempotency,
//! API fidelity, `edge_type` filter, DB referential integrity, any-mod synthetic
//! round-trip + modpack isolation + prune, histograms.
//!
//! Uses the committed T-150 envelopes as ground truth, imported under a fixed
//! test-scoped modpack (never `is_current` — `content_read.rs` asserts the
//! no-current-modpack 404 on this shared DB). Skips unless `TEST_DATABASE_URL`
//! points at a migrated DB.

use std::collections::{BTreeMap, BTreeSet};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::config::Config;
use website_api::services::registry_import::{import_compat, import_items};
use website_api::state::AppState;
use website_api::{app, db};

/// Fixed test-scoped modpacks: vanilla T-150 envelopes + the synthetic "any mod".
const TEST_MP: &str = "00000000-0000-4000-a000-00000000c0de";
const TEST_MP2: &str = "00000000-0000-4000-a000-00000000c0d2";

const ITEMS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../packages/tbd-schema/registry/registry-items.workbench.json"
);
const COMPAT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../packages/tbd-schema/registry/registry-compat.workbench.json"
);

async fn setup() -> Option<(Router, PgPool, String, String)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    // Own rows only — other suites share this DB.
    for mp in [TEST_MP, TEST_MP2] {
        let id = Uuid::parse_str(mp).unwrap();
        for q in [
            "DELETE FROM registry_compat WHERE modpack_id = $1",
            "DELETE FROM registry_items WHERE modpack_id = $1",
            "DELETE FROM modpacks WHERE id = $1",
        ] {
            sqlx::query(q).bind(id).execute(&pool).await.expect("clean");
        }
    }
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "registry-secret"),
    ));
    let maker = dev_login(&app, "mission_maker").await;
    let enlisted = dev_login(&app, "enlisted").await;
    Some((app, pool, maker, enlisted))
}

async fn dev_login(app: &Router, role: &str) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/auth/dev-login?role={role}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    loc.split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
        .to_string()
}

async fn get(app: &Router, uri: &str, bearer: &str, etag: Option<&str>) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"));
    if let Some(t) = etag {
        b = b.header(header::IF_NONE_MATCH, t);
    }
    let resp = app
        .clone()
        .oneshot(b.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Normalized edge tuple: evidence NULL ≡ '' ≡ absent.
type EdgeKey = (String, String, String, String);

fn edge_key(from: &str, to: &str, ty: &str, ev: Option<&str>) -> EdgeKey {
    (
        from.to_string(),
        to.to_string(),
        ty.to_string(),
        ev.unwrap_or("").to_string(),
    )
}

fn envelope_edges(env: &Value) -> BTreeSet<EdgeKey> {
    env["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            edge_key(
                e["from_node"].as_str().unwrap(),
                e["to_node"].as_str().unwrap(),
                e["edge_type"].as_str().unwrap(),
                e["evidence"].as_str(),
            )
        })
        .collect()
}

fn envelope_items(env: &Value) -> BTreeSet<(String, String, String, String)> {
    env["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|it| {
            (
                it["resource_name"].as_str().unwrap().to_string(),
                it["display_name"].as_str().unwrap().to_string(),
                it["category"].as_str().unwrap().to_string(),
                it["kind"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn histogram(env: &Value, list: &str, field: &str) -> BTreeMap<String, u64> {
    let mut h = BTreeMap::new();
    for row in env[list].as_array().unwrap() {
        *h.entry(row[field].as_str().unwrap().to_string())
            .or_insert(0) += 1;
    }
    h
}

async fn db_edges(pool: &PgPool, mp: Uuid) -> BTreeSet<EdgeKey> {
    let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT from_node, to_node, edge_type, evidence FROM registry_compat WHERE modpack_id = $1",
    )
    .bind(mp)
    .fetch_all(pool)
    .await
    .unwrap();
    rows.iter()
        .map(|(f, t, ty, ev)| edge_key(f, t, ty, ev.as_deref()))
        .collect()
}

async fn db_items(pool: &PgPool, mp: Uuid) -> BTreeSet<(String, String, String, String)> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT resource_name, display_name, category, kind FROM registry_items WHERE modpack_id = $1",
    )
    .bind(mp)
    .fetch_all(pool)
    .await
    .unwrap();
    rows.into_iter().collect()
}

/// Full-row snapshot (ids + timestamps) — byte-level idempotency evidence (G2).
async fn db_edge_snapshot(
    pool: &PgPool,
    mp: Uuid,
) -> Vec<(Uuid, String, String, String, Option<String>, String, String)> {
    sqlx::query_as(
        "SELECT id, from_node, to_node, edge_type, evidence, created_at::text, updated_at::text \
         FROM registry_compat WHERE modpack_id = $1 ORDER BY from_node, to_node, edge_type",
    )
    .bind(mp)
    .fetch_all(pool)
    .await
    .unwrap()
}

fn api_edge_set(body: &Value) -> BTreeSet<EdgeKey> {
    body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            edge_key(
                e["from_node"].as_str().unwrap(),
                e["to_node"].as_str().unwrap(),
                e["edge_type"].as_str().unwrap(),
                e["evidence"].as_str(), // absent when '' (skip_serializing_if)
            )
        })
        .collect()
}

#[tokio::test]
async fn registry_compat_ingest_api_worker_gates() {
    let Some((app, pool, maker, enlisted)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let mp = Uuid::parse_str(TEST_MP).unwrap();
    let mp2 = Uuid::parse_str(TEST_MP2).unwrap();
    let items_raw = std::fs::read(ITEMS_PATH).expect("read items envelope");
    let compat_raw = std::fs::read(COMPAT_PATH).expect("read compat envelope");
    let items_env: Value = serde_json::from_slice(&items_raw).unwrap();
    let compat_env: Value = serde_json::from_slice(&compat_raw).unwrap();

    // ── Import the committed T-150 envelopes under the test modpack ──────────
    let ci = import_items(&pool, &items_raw, Some(mp), false)
        .await
        .expect("items");
    let cc = import_compat(&pool, &compat_raw, Some(mp), false)
        .await
        .expect("compat");
    // T-068.10.2 census-gated envelope (see .ai/artifacts/t068_10_2_census.md):
    // 1,857 items (23 predicted drops from the 1,880 T-150 set) / 4,685 edges
    // (+character_default_weapon family; 16 mag edges moved to the vehicle family
    // with the statics reclassification).
    assert_eq!(
        (ci.total, ci.unique, ci.inserted, ci.updated),
        (1857, 1857, 1857, 0)
    );
    assert_eq!(
        (cc.total, cc.unique, cc.inserted, cc.updated),
        (4685, 4685, 4685, 0)
    );

    // G10 — importer histograms equal envelope histograms.
    assert_eq!(ci.histogram, histogram(&items_env, "items", "kind"));
    assert_eq!(cc.histogram, histogram(&compat_env, "edges", "edge_type"));
    assert_eq!(cc.histogram["mag_in_weapon"], 529);
    assert_eq!(cc.histogram["mag_in_vehicle_weapon"], 134);
    assert_eq!(cc.histogram["character_default_weapon"], 673);

    // G1 — ingest bijection: DB row-set ≡ envelope set (items + edges).
    assert_eq!(db_items(&pool, mp).await, envelope_items(&items_env));
    assert_eq!(db_edges(&pool, mp).await, envelope_edges(&compat_env));

    // G5 — DB referential integrity: every edge endpoint is a catalog item.
    let dangling: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM registry_compat c WHERE c.modpack_id = $1 AND ( \
           NOT EXISTS (SELECT 1 FROM registry_items i \
                       WHERE i.modpack_id = c.modpack_id AND i.resource_name = c.from_node) OR \
           NOT EXISTS (SELECT 1 FROM registry_items i \
                       WHERE i.modpack_id = c.modpack_id AND i.resource_name = c.to_node))",
    )
    .bind(mp)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(dangling, 0, "G5: dangling edge endpoints");

    // ── API (G3): full graph, named edges, ETag machinery ────────────────────
    let compat_uri = format!("/api/v1/registry/compat?modpack={TEST_MP}");
    let (st, body) = get(&app, &compat_uri, &maker, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["modpack_id"], TEST_MP);
    assert_eq!(body["data"].as_array().unwrap().len(), 4685);
    assert_eq!(
        api_edge_set(&body),
        envelope_edges(&compat_env),
        "G3: API ≡ envelope"
    );
    let etag = body["etag"].as_str().unwrap().to_string();

    // Named infantry + vehicle-weapon edges from the committed sample.
    let has = |from: &str, to: &str, ty: &str| {
        body["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["from_node"] == from && e["to_node"] == to && e["edge_type"] == ty)
    };
    assert!(
        has(
            "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et",
            "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et",
            "mag_in_weapon"
        ),
        "STANAG M855 -> M16A2"
    );
    assert!(
        has(
            "{AAF51CFA75A9CF8B}Prefabs/Weapons/Magazines/Box_762x51_M60_100rnd_4AP_1Tracer.et",
            "{6AF5FA1A839A4980}Prefabs/Weapons/MachineGuns/M60/MG_M60_Mounted.et",
            "mag_in_vehicle_weapon"
        ),
        "M60 box -> MG_M60_Mounted"
    );

    // 304 replay.
    let (st, _) = get(&app, &compat_uri, &maker, Some(&etag)).await;
    assert_eq!(st, StatusCode::NOT_MODIFIED);

    // G4 — edge_type filter: set-equal to the oracle filter; distinct ETag.
    let filt_uri = format!("{compat_uri}&edge_type=mag_in_weapon");
    let (st, filt) = get(&app, &filt_uri, &maker, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(filt["data"].as_array().unwrap().len(), 529);
    let oracle: BTreeSet<EdgeKey> = envelope_edges(&compat_env)
        .into_iter()
        .filter(|(_, _, ty, _)| ty == "mag_in_weapon")
        .collect();
    assert_eq!(
        api_edge_set(&filt),
        oracle,
        "G4: filtered API ≡ oracle filter"
    );
    assert_ne!(filt["etag"], etag, "G4: filter-discriminated ETag");
    // Filtered ETag must not satisfy the unfiltered resource.
    let (st, _) = get(
        &app,
        &compat_uri,
        &maker,
        Some(filt["etag"].as_str().unwrap()),
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // Items exposed via the existing route.
    let (st, items_body) = get(
        &app,
        &format!("/api/v1/registry?modpack={TEST_MP}"),
        &maker,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(items_body["data"].as_array().unwrap().len(), 1857);

    // ── G2 — idempotency: re-run touches nothing ─────────────────────────────
    let snap = db_edge_snapshot(&pool, mp).await;
    let ci2 = import_items(&pool, &items_raw, Some(mp), false)
        .await
        .unwrap();
    let cc2 = import_compat(&pool, &compat_raw, Some(mp), false)
        .await
        .unwrap();
    assert_eq!(
        (ci2.inserted, ci2.updated, ci2.pruned),
        (0, 0, 0),
        "G2 items"
    );
    assert_eq!(
        (cc2.inserted, cc2.updated, cc2.pruned),
        (0, 0, 0),
        "G2 compat"
    );
    assert_eq!(
        db_edge_snapshot(&pool, mp).await,
        snap,
        "G2: row snapshot identical"
    );
    let (_, body2) = get(&app, &compat_uri, &maker, None).await;
    assert_eq!(body2["etag"].as_str().unwrap(), etag, "G2: ETag identical");

    // ── Tier + resolution failures ────────────────────────────────────────────
    let (st, _) = get(&app, &compat_uri, &enlisted, None).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = get(
        &app,
        &format!("/api/v1/registry/compat?modpack={}", Uuid::new_v4()),
        &maker,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = get(
        &app,
        "/api/v1/registry/compat?modpack=garbage",
        &maker,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // ── G9 — any-mod synthetic round-trip (all 16 kinds, all 7 edge types, ────
    // regex-edge-case names, one evidence-less edge), isolated second modpack.
    let kinds = [
        "character",
        "gear_primary",
        "gear_handgun",
        "gear_launcher",
        "gear_uniform",
        "gear_vest",
        "gear_helmet",
        "gear_backpack",
        "magazine",
        "ammo",
        "optic",
        "attachment",
        "vehicle",
        "vehicle_weapon",
        "crate",
        "other",
    ];
    let rn = |i: usize| {
        format!("{{AB12CD34EF56AB{i:02}}}Prefabs/Any Mod's Pack (v2)/Sub-dir_1.0/Item {i:02}.et")
    };
    let syn_items: Vec<Value> = kinds
        .iter()
        .enumerate()
        .map(|(i, k)| {
            json!({
                "resource_name": rn(i),
                "display_name": format!("Synthetic {k}"),
                "category": format!("AnyMod/{k}"),
                "kind": k,
            })
        })
        .collect();
    let syn_items_env = json!({
        "registryItemsVersion": "2",
        "modpackId": TEST_MP2,
        "items": syn_items,
    });
    // Edge per type; index into `kinds` picks plausible endpoints; the ammo
    // families prove the pipeline accepts them the moment an export ships them
    // (test fixture only — the committed T-150 data keeps them empty).
    let syn_edges = vec![
        json!({"from_node": rn(8), "to_node": rn(1), "edge_type": "mag_in_weapon", "evidence": "SynWell"}),
        json!({"from_node": rn(9), "to_node": rn(8), "edge_type": "ammo_in_mag", "evidence": "SynAmmo"}),
        json!({"from_node": rn(10), "to_node": rn(1), "edge_type": "optic_on_weapon", "evidence": "SynOptic"}),
        // Evidence-less edge: NULL ≡ '' ≡ absent normalization path.
        json!({"from_node": rn(11), "to_node": rn(1), "edge_type": "attachment_on_weapon"}),
        json!({"from_node": rn(8), "to_node": rn(13), "edge_type": "mag_in_vehicle_weapon", "evidence": "SynVWell"}),
        json!({"from_node": rn(9), "to_node": rn(13), "edge_type": "ammo_in_vehicle_weapon", "evidence": "SynShell"}),
        json!({"from_node": rn(6), "to_node": rn(0), "edge_type": "character_default_loadout", "evidence": "SynSlot"}),
    ];
    let syn_compat_env = json!({
        "registryCompatVersion": "1",
        "modpackId": TEST_MP2,
        "edges": syn_edges,
    });

    let si = import_items(
        &pool,
        &serde_json::to_vec(&syn_items_env).unwrap(),
        None,
        false,
    )
    .await
    .expect("synthetic items (envelope modpackId path)");
    let sc = import_compat(
        &pool,
        &serde_json::to_vec(&syn_compat_env).unwrap(),
        None,
        false,
    )
    .await
    .expect("synthetic compat");
    assert_eq!((si.inserted, sc.inserted), (16, 7));
    assert_eq!(
        db_items(&pool, mp2).await,
        envelope_items(&syn_items_env),
        "G9: items bijection"
    );
    assert_eq!(
        db_edges(&pool, mp2).await,
        envelope_edges(&syn_compat_env),
        "G9: edges bijection"
    );

    // All 7 edge families present via API for the synthetic modpack.
    let (st, syn_body) = get(
        &app,
        &format!("/api/v1/registry/compat?modpack={TEST_MP2}"),
        &maker,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let types: BTreeSet<String> = syn_body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["edge_type"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(types.len(), 7, "G9: all 7 edge families round-trip");

    // Prune: subset envelope with prune=true ⇒ DB set-equals subset exactly.
    let subset_env = json!({
        "registryCompatVersion": "1",
        "modpackId": TEST_MP2,
        "edges": [syn_edges[0].clone(), syn_edges[3].clone()],
    });
    let sp = import_compat(&pool, &serde_json::to_vec(&subset_env).unwrap(), None, true)
        .await
        .unwrap();
    assert_eq!(
        (sp.inserted, sp.updated, sp.pruned),
        (0, 0, 5),
        "G9 prune counts"
    );
    assert_eq!(
        db_edges(&pool, mp2).await,
        envelope_edges(&subset_env),
        "G9: pruned ≡ subset"
    );

    // Isolation: the vanilla test modpack is untouched by all MP2 traffic.
    let (_, body3) = get(&app, &compat_uri, &maker, None).await;
    assert_eq!(
        body3["etag"].as_str().unwrap(),
        etag,
        "G9: MP1 ETag unchanged"
    );
    assert_eq!(body3["data"].as_array().unwrap().len(), 4685);

    // Invalid envelope is rejected before SQL (schema gate).
    let bad = json!({"registryCompatVersion": "1", "modpackId": TEST_MP2, "edges": [
        {"from_node": "not-a-resource-name", "to_node": rn(1), "edge_type": "mag_in_weapon"}
    ]});
    let err = import_compat(&pool, &serde_json::to_vec(&bad).unwrap(), None, false).await;
    assert!(err.is_err(), "schema-invalid envelope must be rejected");
}
