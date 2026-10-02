//! The equipment data viewer's goldens, reproduced from a committed export by the production
//! importer and router.
//!
//! **Role:** proves every committed answer of the twelve `GET /api/v1/debug/equipment-data/*`
//! routes (the six `positive/` samples in `contracts/fixtures/equipment-data-viewer/`, the
//! route answers in `tests/fixtures/equipment_data_viewer/route_responses/` and the downloaded
//! document) is exactly what the API serves after importing the committed export, and that every
//! positive sample and every route has such a golden.
//!
//! **Position:** reads the committed publications under `tests/fixtures/equipment_data_viewer/`:
//! `diagnostic_export/` goes through `generation_import::poll` into the diagnostic catalog's
//! directory and `export_source/` is the configured export source whose `gameplay/` publication
//! the gameplay catalog imports, both into one temporary equipment data directory; then boots
//! `core::http_router::router` with a development configuration over this binary's own test
//! database and drives it with `oneshot` requests.
//!
//! **Signals & state:** the temporary equipment data directory, removed when the case ends; each
//! request carries a fresh synthetic peer so the rate limiter never answers in a route's place.
//!
//! **Invariants:** a live answer equals its golden as a `serde_json::Value` (object key order is
//! irrelevant, nothing is normalised), validates against its route's schema and decodes into the
//! generated type; the download equals the committed document byte for byte. The committed
//! publications carry their own `bytes`/`sha256` manifest and pointer digest, so an edited
//! document must update both, and the importer refuses anything else. A missing
//! `TEST_DATABASE_URL` panics; no case passes without its database.

mod common;
mod contract_support;
#[path = "contract_parity_support/json_difference.rs"]
mod json_difference;

use std::collections::BTreeSet;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use api::community_content::models::generated::equipment_data_viewer::{
    dataset::EquipmentDatasetStatus, field_inventory::EquipmentFieldPage,
    relationships::EquipmentRelationshipPage, resource_cards::EquipmentResourceCardPage,
    resources::EquipmentResourcePage, source_inspection::EquipmentSourcePage,
};
use api::community_content::services::equipment_data_viewer::EquipmentDataService;
use api::community_content::services::equipment_data_viewer::importing::generation_import;
use api::core::application_state::AppState;
use api::core::configuration::Config;
use api::core::database;
use api::core::http_router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

/// The prefix every equipment data viewer route shares.
const ROUTE_PREFIX: &str = "/api/v1/debug/equipment-data/";

/// The tag that marks an equipment data viewer handler in `src/`.
const ROUTE_TAG: &str = "/// @route GET /api/v1/debug/equipment-data/";

/// The handler folder whose `@route` tags list every equipment data viewer route.
const HANDLER_FOLDER: &str = "src/community_content/handlers/equipment_data_viewer";

/// Where a JSON golden lives.
enum GoldenFile {
    /// A `positive/` sample of `contracts/fixtures/equipment-data-viewer/`.
    ContractSample(&'static str),
    /// An answer under `tests/fixtures/equipment_data_viewer/route_responses/`.
    RouteResponse(&'static str),
}

/// What one request's answer must equal.
enum Golden {
    /// A JSON page: the golden file, the route's schema and its generated-type decoder.
    Json {
        file: GoldenFile,
        schema: &'static str,
        decode: fn(&str, &Value),
    },
    /// A downloaded document: its `document` kind and its path under
    /// `tests/fixtures/equipment_data_viewer/`.
    Document {
        kind: &'static str,
        file: &'static str,
    },
}

/// One request per route and the golden its answer reproduces.
struct GoldenRequest {
    route: &'static str,
    query: &'static str,
    golden: Golden,
}

fn json(file: GoldenFile, schema: &'static str, decode: fn(&str, &Value)) -> Golden {
    Golden::Json {
        file,
        schema,
        decode,
    }
}

/// Every golden this binary reproduces, one row per route.
fn golden_requests() -> Vec<GoldenRequest> {
    use GoldenFile::{ContractSample, RouteResponse};
    use contract_support::assert_decodes;
    const DATASET: &str = "equipment-data-viewer/dataset.schema.json";
    const RESOURCES: &str = "equipment-data-viewer/resources.schema.json";
    const RELATIONSHIPS: &str = "equipment-data-viewer/relationships.schema.json";
    const FIELDS: &str = "equipment-data-viewer/field-inventory.schema.json";
    const CARDS: &str = "equipment-data-viewer/resource-cards.schema.json";
    const SOURCE: &str = "equipment-data-viewer/source-inspection.schema.json";
    let status = assert_decodes::<EquipmentDatasetStatus>;
    let source = assert_decodes::<EquipmentSourcePage>;
    let row = |route, query, golden| GoldenRequest {
        route,
        query,
        golden,
    };
    vec![
        row(
            "status",
            "?dataset=diagnostic",
            json(ContractSample("dataset.json"), DATASET, status),
        ),
        row(
            "overview",
            "",
            json(RouteResponse("overview.json"), DATASET, status),
        ),
        row(
            "resources",
            "?dataset=diagnostic",
            json(
                ContractSample("resources.json"),
                RESOURCES,
                assert_decodes::<EquipmentResourcePage>,
            ),
        ),
        row(
            "relationships",
            "?dataset=diagnostic&resource_id=guid:1111111111111111&view=all",
            json(
                ContractSample("relationships.json"),
                RELATIONSHIPS,
                assert_decodes::<EquipmentRelationshipPage>,
            ),
        ),
        row(
            "fields",
            "?dataset=diagnostic",
            json(
                ContractSample("field-inventory.json"),
                FIELDS,
                assert_decodes::<EquipmentFieldPage>,
            ),
        ),
        row(
            "resource-cards",
            "?dataset=diagnostic&resource_id=guid:1111111111111111",
            json(
                ContractSample("resource-cards.json"),
                CARDS,
                assert_decodes::<EquipmentResourceCardPage>,
            ),
        ),
        row(
            "selection",
            "",
            json(RouteResponse("selection.json"), SOURCE, source),
        ),
        row(
            "containers",
            "?dataset=diagnostic&resource_id=guid:1111111111111111&node_id=root&view=all",
            json(RouteResponse("containers.json"), SOURCE, source),
        ),
        row(
            "properties",
            "?dataset=diagnostic&resource_id=guid:1111111111111111\
             &node_id=root/properties/components/0",
            json(ContractSample("source-inspection.json"), SOURCE, source),
        ),
        row(
            "values",
            "?dataset=diagnostic&resource_id=guid:1111111111111111&node_id=root&property=components",
            json(RouteResponse("values.json"), SOURCE, source),
        ),
        row(
            "documents",
            "?dataset=diagnostic&document=record&resource_id=guid:1111111111111111",
            json(RouteResponse("documents.json"), SOURCE, source),
        ),
        row(
            "download",
            "?dataset=diagnostic&document=source&resource_id=guid:1111111111111111",
            Golden::Document {
                kind: "source",
                file: "diagnostic_export/published/D1A6000000000001/sources/backpack_fixture.json",
            },
        ),
    ]
}

/// The committed importer input and route answers of this binary.
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/equipment_data_viewer")
}

/// The contract samples the frontend's DTO parity tests also read.
fn contract_samples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../contracts/fixtures/equipment-data-viewer/positive")
}

impl GoldenFile {
    fn path(&self) -> PathBuf {
        match self {
            GoldenFile::ContractSample(name) => contract_samples().join(name),
            GoldenFile::RouteResponse(name) => fixture_root().join("route_responses").join(name),
        }
    }
}

/// A per-case equipment data directory, removed when the case ends.
struct EquipmentDataDirectory(PathBuf);

impl EquipmentDataDirectory {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("equipment-viewer-goldens-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).expect("create the equipment data directory");
        Self(path)
    }
}

impl Drop for EquipmentDataDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

static PEER: AtomicU32 = AtomicU32::new(1);

/// A synthetic client address no other request in this binary has used.
fn next_peer() -> SocketAddr {
    let [_, b, c, d] = PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 42000))
}

/// Imports both committed publications with the production importer and returns the
/// development router that serves them.
async fn imported_router(data_dir: &Path) -> Router {
    let url = common::require_test_database_url()
        .expect("the per-binary test database is provisioned before any case runs");
    let pool = database::connect(&url).await.expect("connect");
    let diagnostic_importer = Arc::new(EquipmentDataService::new(
        data_dir,
        Some(fixture_root().join("diagnostic_export")),
    ));
    generation_import::poll(diagnostic_importer)
        .await
        .expect("the importer publishes the committed diagnostic export");
    let mut config = Config::for_tests(url, "equipment-viewer-goldens-secret");
    config.equipment_data_dir = data_dir.display().to_string();
    config.equipment_export_source_dir =
        Some(fixture_root().join("export_source").display().to_string());
    let state = AppState::new(pool, config);
    generation_import::initialize(&state.equipment_data.diagnostic)
        .await
        .expect("the diagnostic catalog activates the imported generation");
    generation_import::initialize(&state.equipment_data.gameplay)
        .await
        .expect("the gameplay catalog starts empty");
    generation_import::poll(state.equipment_data.gameplay.clone())
        .await
        .expect("the importer publishes the committed gameplay export");
    http_router::router(state)
}

/// Status, headers and body of one anonymous `GET uri`.
async fn get(app: &Router, uri: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .expect("request");
    request.extensions_mut().insert(ConnectInfo(next_peer()));
    let response = app.clone().oneshot(request).await.expect("infallible");
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read the body");
    (status, headers, body.to_vec())
}

/// Every way the answer to `uri` departs from a JSON golden; an answer its schema or generated
/// type rejects panics with every violation named.
fn json_failures(
    uri: &str,
    body: &[u8],
    file: &GoldenFile,
    schema: &str,
    decode: fn(&str, &Value),
) -> Vec<String> {
    let path = file.path();
    let live: Value = match serde_json::from_slice(body) {
        Ok(value) => value,
        Err(error) => return vec![format!("GET {uri}: the answer is not JSON: {error}")],
    };
    contract_support::assert_valid(schema, None, &live);
    decode(uri, &live);
    let golden: Value = match fs::read(&path).map(|bytes| serde_json::from_slice(&bytes)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => return vec![format!("{}: not JSON: {error}", path.display())],
        Err(error) => return vec![format!("{}: {error}", path.display())],
    };
    json_difference::differences(&golden, &live)
        .into_iter()
        .map(|line| format!("GET {uri} differs from {}: {line}", path.display()))
        .collect()
}

/// Every way a download of document `kind` departs from its committed `file`.
fn document_failures(
    uri: &str,
    headers: &HeaderMap,
    body: &[u8],
    kind: &str,
    file: &str,
) -> Vec<String> {
    let path = fixture_root().join(file);
    let expected = fs::read(&path).expect("read the committed document");
    let mut failures = Vec::new();
    for (name, value) in [
        (header::CONTENT_TYPE, "application/json".to_owned()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{kind}.json\""),
        ),
        (header::CONTENT_LENGTH, expected.len().to_string()),
    ] {
        if headers.get(&name).and_then(|v| v.to_str().ok()) != Some(value.as_str()) {
            failures.push(format!(
                "GET {uri}: {name} is {:?}, expected {value:?}",
                headers.get(&name)
            ));
        }
    }
    if body != expected.as_slice() {
        failures.push(format!(
            "GET {uri}: the download differs from {}",
            path.display()
        ));
    }
    failures
}

#[tokio::test]
async fn contract_parity_equipment_viewer_goldens_are_reproduced_from_the_committed_fixture() {
    let data_dir = EquipmentDataDirectory::new();
    let app = imported_router(&data_dir.0).await;
    let mut failures = Vec::new();
    for request in golden_requests() {
        let uri = format!("{ROUTE_PREFIX}{}{}", request.route, request.query);
        let (status, headers, body) = get(&app, &uri).await;
        if status != StatusCode::OK {
            let answer = serde_json::from_slice(&body)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body).into_owned()));
            failures.push(format!(
                "GET {uri} answered {status}: {}",
                json_difference::excerpt(&answer)
            ));
            continue;
        }
        match &request.golden {
            Golden::Json {
                file,
                schema,
                decode,
            } => {
                failures.extend(json_failures(&uri, &body, file, schema, *decode));
            }
            Golden::Document { kind, file } => {
                failures.extend(document_failures(&uri, &headers, &body, kind, file));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} equipment viewer golden(s) are not reproduced:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The file names directly inside `folder`.
fn file_names(folder: &Path) -> BTreeSet<String> {
    fs::read_dir(folder)
        .unwrap_or_else(|error| panic!("{}: {error}", folder.display()))
        .map(|entry| entry.expect("folder entry"))
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// The JSON sample names (`*.json`) directly inside `folder`.
fn json_sample_names(folder: &Path) -> BTreeSet<String> {
    file_names(folder)
        .into_iter()
        .filter(|name| Path::new(name).extension().is_some_and(|ext| ext == "json"))
        .collect()
}

/// Every JSON sample in the contract's positive folder is the golden of exactly one request,
/// every committed route answer is the golden of exactly one request, and every tagged handler
/// route has exactly one golden request. Only `*.json` files count as samples: the folder's
/// `README.md` documents the samples and is not one of them.
#[test]
fn contract_parity_equipment_viewer_every_positive_fixture_and_route_is_covered() {
    let requests = golden_requests();
    let mut samples = BTreeSet::new();
    let mut responses = BTreeSet::new();
    for request in &requests {
        if let Golden::Json { file, .. } = &request.golden {
            match file {
                GoldenFile::ContractSample(name) => assert!(samples.insert(name.to_string())),
                GoldenFile::RouteResponse(name) => assert!(responses.insert(name.to_string())),
            }
        }
    }
    assert_eq!(
        samples,
        json_sample_names(&contract_samples()),
        "every positive sample is the golden of exactly one request, and every golden exists"
    );
    assert_eq!(
        responses,
        file_names(&fixture_root().join("route_responses")),
        "every committed route answer is the golden of exactly one request"
    );
    let handlers = Path::new(env!("CARGO_MANIFEST_DIR")).join(HANDLER_FOLDER);
    let mut tagged = BTreeSet::new();
    for name in file_names(&handlers) {
        let text = fs::read_to_string(handlers.join(&name)).expect("read handler source");
        for line in text.lines() {
            if let Some(route) = line.strip_prefix(ROUTE_TAG) {
                assert!(
                    tagged.insert(route.trim().to_owned()),
                    "{route} is tagged twice"
                );
            }
        }
    }
    let requested: BTreeSet<String> = requests.iter().map(|r| r.route.to_owned()).collect();
    assert_eq!(
        requested.len(),
        requests.len(),
        "one golden request per route"
    );
    assert_eq!(
        requested, tagged,
        "every tagged equipment data viewer route has exactly one golden request"
    );
}
