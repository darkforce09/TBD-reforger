//! The operations ballistics part's world: a catalog version of its own uploaded through the
//! real route, two members-only events (one with a saved fire mission, one without), and a fresh
//! catalog pair or save body per probe.
//!
//! **Role:** implements [`PartWorld`] for `specs/operations_ballistics.rs`: the public catalog
//! list and version reads, the administrator catalog upload, and the fire-mission save and an
//! event's saved list.
//!
//! **Position:** mounted by `tests/route_acceptance_operations_ballistics.rs` with `#[path]`.
//! Every catalog pair is minted from the committed vanilla pair
//! (`contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json` and
//! `contracts/fixtures/ballistics/vanilla_mortars.v1/calibration.json`) by renaming its
//! `catalog_id` in both documents and re-pinning the bundle's `catalog_sha256` to the renamed
//! catalog bytes, so the calibration still flies every case of the committed bundle. Each client
//! solution is solved with the same map-engine assembler the mortar calculator runs.
//!
//! **Signals & state:** the committed pair's text, read once per world, and the world's own
//! catalog version.
//!
//! **Invariants:** a catalog version can be stored once per database and the worlds of a binary
//! share one database, so every world stores its own catalog id and every upload probe mints
//! another; the worlds' events keep the default members-only access policy, which the world's
//! enlisted account fully sees; the world never stores the committed `vanilla_mortars` id.

use axum::Router;
use serde_json::{Value, json};
use uuid::Uuid;
use website_api::core::application_state::AppState;
use website_api::core::http_router;
use website_api::core::wire_format::content_digest::sha256_hex;
use website_map_engine::data::scenario::ballistics::catalog::BallisticsCatalog;
use website_map_engine::data::scenario::ballistics::fire_mission::{
    FireMissionGunPosition, FireMissionInputs, FireMissionPoint, HeightSource, solve_fire_mission,
};

use crate::route_acceptance_support::actors::Actors;
use crate::route_acceptance_support::requests::{Outgoing, send};
use crate::route_acceptance_support::spec::{Actor, Role};
use crate::route_acceptance_support::world::{Fixture, PartWorld, WorldCore};

/// The committed catalog, relative to the repository root.
const COMMITTED_CATALOG: &str = "contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json";
/// The committed calibration bundle, relative to the repository root.
const COMMITTED_CALIBRATION: &str =
    "contracts/fixtures/ballistics/vanilla_mortars.v1/calibration.json";
/// The committed pair's catalog id, as both documents spell it.
const COMMITTED_CATALOG_ID: &str = "\"catalog_id\": \"vanilla_mortars\"";
/// The upload route.
const UPLOAD_URI: &str = "/api/v1/ballistics-catalogs";
/// The save route.
const SAVE_URI: &str = "/api/v1/fire-missions";
/// The boundary of every catalog upload form the world builds.
const BOUNDARY: &str = "route-acceptance-world-catalog";

/// One catalog pair ready to upload.
struct CatalogPair {
    catalog: Vec<u8>,
    calibration: Vec<u8>,
}

/// The operations ballistics world.
pub struct OperationsBallisticsWorld {
    /// The committed catalog text.
    committed_catalog: String,
    /// The committed calibration bundle text.
    committed_calibration: String,
    /// The catalog id the world stored at build.
    catalog_id: String,
    /// The world's stored catalog, decoded.
    catalog: BallisticsCatalog,
    /// A members-only event with one saved fire mission.
    event: Uuid,
    /// A members-only event with no fire mission.
    idle_event: Uuid,
}

fn repository_text(relative: &str) -> String {
    let path = format!("{}/../../../{relative}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read committed fixture {path}: {error}"))
}

/// A catalog id no earlier world or probe used.
fn fresh_catalog_id() -> String {
    format!("route_acceptance_{}", Uuid::new_v4().simple())
}

/// A `multipart/form-data` body of the pair's `catalog` and `calibration` parts; answers the
/// bytes and their content type.
fn upload_form(pair: &CatalogPair) -> (Vec<u8>, String) {
    let mut body = Vec::new();
    for (name, bytes) in [
        ("catalog", &pair.catalog),
        ("calibration", &pair.calibration),
    ] {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"; \
                 filename=\"{name}.json\"\r\nContent-Type: application/json\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    (body, format!("multipart/form-data; boundary={BOUNDARY}"))
}

/// A members-only event authored by `author`.
async fn plant_event(state: &AppState, author: &str, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, created_by, created_at) \
         VALUES ($1, now() + interval '2 days', $2, now()) RETURNING id",
    )
    .bind(name)
    .bind(author)
    .fetch_one(&state.pool)
    .await
    .expect("plant a members-only event")
}

/// Send one setup request and require `expected`; answers the JSON body.
async fn require(app: &Router, outgoing: &Outgoing, expected: u16) -> Value {
    let received = send(app, outgoing).await;
    assert_eq!(
        received.status.as_u16(),
        expected,
        "setup {} {}: {}",
        outgoing.method,
        outgoing.uri,
        received.excerpt()
    );
    received.json().unwrap_or(Value::Null)
}

impl OperationsBallisticsWorld {
    /// The committed pair renamed to `catalog_id`; the bundle pins the renamed catalog bytes
    /// unless `stale_sha` keeps the committed catalog's SHA-256.
    fn pair(
        committed_catalog: &str,
        committed_calibration: &str,
        catalog_id: &str,
        stale_sha: bool,
    ) -> CatalogPair {
        let renamed = format!("\"catalog_id\": \"{catalog_id}\"");
        assert_eq!(
            committed_catalog.matches(COMMITTED_CATALOG_ID).count(),
            1,
            "the committed catalog names its id once"
        );
        let catalog = committed_catalog.replacen(COMMITTED_CATALOG_ID, &renamed, 1);
        let mut calibration = committed_calibration.replacen(COMMITTED_CATALOG_ID, &renamed, 1);
        if !stale_sha {
            let committed_sha = format!(
                "\"catalog_sha256\": \"{}\"",
                sha256_hex(committed_catalog.as_bytes())
            );
            assert_eq!(
                calibration.matches(&committed_sha).count(),
                1,
                "the committed bundle pins the committed catalog bytes once"
            );
            let renamed_sha = format!("\"catalog_sha256\": \"{}\"", sha256_hex(catalog.as_bytes()));
            calibration = calibration.replacen(&committed_sha, &renamed_sha, 1);
        }
        CatalogPair {
            catalog: catalog.into_bytes(),
            calibration: calibration.into_bytes(),
        }
    }

    fn minted_pair(&self, catalog_id: &str, stale_sha: bool) -> CatalogPair {
        Self::pair(
            &self.committed_catalog,
            &self.committed_calibration,
            catalog_id,
            stale_sha,
        )
    }

    /// M252 firing M821 HE from one gun onto a target 1.2 km away, in calm air.
    fn inputs(catalog_id: &str) -> FireMissionInputs {
        FireMissionInputs {
            catalog_id: catalog_id.to_owned(),
            catalog_version: 1,
            weapon_id: "m252".to_owned(),
            shell_id: "m821".to_owned(),
            charge_rings: None,
            target: FireMissionPoint {
                x: 2200.0,
                y: 1800.0,
                height_m: 40.0,
                height_source: HeightSource::Dem,
            },
            guns: vec![FireMissionGunPosition {
                label: "Gun 1".to_owned(),
                x: 1000.0,
                y: 2000.0,
                height_m: 25.0,
                height_source: HeightSource::Manual,
            }],
            wind: None,
            burst_height_m: None,
            crest_profile: None,
        }
    }

    /// A `FireMissionSave` body on `event` against `catalog`, with the engine's own solution.
    fn save_body(catalog: &BallisticsCatalog, event: Uuid) -> Value {
        let inputs = Self::inputs(&catalog.catalog_id);
        let solution = solve_fire_mission(catalog, &inputs).expect("the world's inputs solve");
        let mut body = serde_json::to_value(&inputs).expect("serialise the inputs");
        body["event_id"] = json!(event.to_string());
        body["target_grid"] = json!("0220 0180");
        body["client_solution"] = serde_json::to_value(&solution).expect("serialise the solution");
        body
    }

    /// The save body with every solved elevation of the client solution moved by 5 mils.
    fn tampered_save_body(&self) -> Value {
        let mut body = Self::save_body(&self.catalog, self.event);
        let guns = body["client_solution"]["guns"]
            .as_array_mut()
            .expect("a solution lists its guns");
        for charge in guns.iter_mut().flat_map(|gun| {
            gun["charges"]
                .as_array_mut()
                .expect("a gun lists its charges")
        }) {
            if let Some(elevation) = charge["elevation_mils"].as_f64() {
                charge["elevation_mils"] = json!(elevation + 5.0);
            }
        }
        body
    }
}

impl PartWorld for OperationsBallisticsWorld {
    async fn build(state: &mut AppState, actors: &Actors) -> Self {
        let app = http_router::router(state.clone());
        let committed_catalog = repository_text(COMMITTED_CATALOG);
        let committed_calibration = repository_text(COMMITTED_CALIBRATION);
        let catalog_id = fresh_catalog_id();
        let pair = Self::pair(
            &committed_catalog,
            &committed_calibration,
            &catalog_id,
            false,
        );
        let (form, content_type) = upload_form(&pair);
        let admin = actors.user(Role::Admin);
        let mut upload = Outgoing::new("POST", UPLOAD_URI).bearer(&admin.token);
        upload.body = Some((form, Some(content_type)));
        require(&app, &upload, 201).await;
        let catalog =
            BallisticsCatalog::from_json_slice(&pair.catalog).expect("decode the world's catalog");

        let event = plant_event(state, &admin.discord_id, "Route acceptance fire missions").await;
        let idle_event =
            plant_event(state, &admin.discord_id, "Route acceptance idle battery").await;
        let enlisted = actors.user(Role::Enlisted);
        let save = Outgoing::new("POST", SAVE_URI)
            .bearer(&enlisted.token)
            .json(&Self::save_body(&catalog, event));
        require(&app, &save, 201).await;

        OperationsBallisticsWorld {
            committed_catalog,
            committed_calibration,
            catalog_id,
            catalog,
            event,
            idle_event,
        }
    }

    async fn fixture(&self, _core: &WorldCore, key: &str, _actor: Actor) -> Option<Fixture> {
        let upload = |pair: CatalogPair| {
            let (form, content_type) = upload_form(&pair);
            Fixture::new().raw_body(form, content_type)
        };
        let fixture = match key {
            "GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}" => Fixture::new()
                .param("catalogId", self.catalog_id.clone())
                .param("version", "1"),
            "POST /api/v1/ballistics-catalogs" => {
                upload(self.minted_pair(&fresh_catalog_id(), false))
            }
            "catalog-upload-duplicate" => upload(self.minted_pair(&self.catalog_id, false)),
            "catalog-upload-stale-sha" => upload(self.minted_pair(&fresh_catalog_id(), true)),
            "GET /api/v1/events/{id}/fire-missions" => {
                Fixture::new().param("id", self.event.to_string())
            }
            "fire-missions-none" => Fixture::new().param("id", self.idle_event.to_string()),
            "POST /api/v1/fire-missions" => {
                Fixture::new().body(Self::save_body(&self.catalog, self.event))
            }
            "fire-mission-tampered" => Fixture::new().body(self.tampered_save_body()),
            _ => return None,
        };
        Some(fixture)
    }
}
