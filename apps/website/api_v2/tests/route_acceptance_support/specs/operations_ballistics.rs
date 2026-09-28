//! Route specs of the `operations_ballistics` part: the public ballistics catalog reads, the
//! administrator catalog upload, and the fire-mission save and an event's saved list.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the ownership probes, the domain's malformed and boundary probes, and a reason for every
//! dimension that does not apply. Fixture keys other than a spec key are minted by
//! `world/operations_ballistics.rs`.

use serde_json::json;

use super::super::spec::{Actor, Change, Contract, Dimension, Expect, Probe, Role, RouteSpec};

const CATALOG: &str = "ballistics-catalog.schema.json";
const FIRE: &str = "fire-mission.schema.json";
const ENLISTED: Actor = Actor::User(Role::Enlisted);
const ADMIN: Actor = Actor::User(Role::Admin);

const PUBLIC_RECORD: &str = "a stored catalog version is a public, immutable record: no caller \
     owns it";
const COMMUNITY_RECORD: &str = "an uploaded catalog version belongs to the community, not to \
     the administrator who uploaded it";
const SHARED_READ: &str = "an event's fire missions are shared with every member who fully sees \
     the event: no row is owned by the reader";
const OWN_NEW_ROW: &str = "the saved fire mission is the caller's own new row; the event it \
     names is shared by every member";
const NO_INPUT: &str = "the list takes no path parameter, query or body";
const MEMBERS_ONLY: &str = "the world's events keep the default members-only access policy, \
     which hides them from a guest, and a hidden event answers the same 404 as a missing one";

/// The boundary of every multipart body the specs write by hand.
const MULTIPART_TYPE: &str = "multipart/form-data; boundary=route-acceptance-catalog";

/// A `multipart/form-data` body of JSON `parts` (name and bytes) with [`MULTIPART_TYPE`]'s
/// boundary.
fn multipart(parts: &[(&str, &[u8])]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, bytes) in parts {
        body.extend_from_slice(
            format!(
                "--route-acceptance-catalog\r\nContent-Disposition: form-data; name=\"{name}\"; \
                 filename=\"{name}.json\"\r\nContent-Type: application/json\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(b"--route-acceptance-catalog--\r\n");
    body
}

fn catalog_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::public("GET /api/v1/ballistics-catalogs")
            .ok(200, Contract::schema(CATALOG, "BallisticsCatalogList"))
            .no_ownership(PUBLIC_RECORD)
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .boundary(Probe::new(
                "a signed-in member reads the same list",
                ENLISTED,
            )),
        RouteSpec::public("GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}")
            .ok(200, Contract::schema(CATALOG, "BallisticsCatalog"))
            .text_param("catalogId")
            .text_param("version")
            .no_ownership(PUBLIC_RECORD)
            .malformed(
                Probe::new("a version that is not an integer", Actor::Anonymous)
                    .param("version", "one")
                    .expect(400),
            )
            .boundary(
                Probe::new("an unknown catalog", Actor::Anonymous)
                    .param("catalogId", "no_such_catalog")
                    .expect_with(
                        Expect::status(404).error_contains("ballistics catalog version not found"),
                    ),
            )
            .boundary(
                Probe::new("an unknown version of a stored catalog", Actor::Anonymous)
                    .param("version", "999")
                    .expect_with(
                        Expect::status(404).error_contains("ballistics catalog version not found"),
                    ),
            )
            .boundary(
                Probe::new(
                    "a request naming any stored copy is not modified",
                    Actor::Anonymous,
                )
                .change(Change::Header("if-none-match", "*".to_string()))
                .expect(304),
            ),
        RouteSpec::role("POST /api/v1/ballistics-catalogs", Role::Admin)
            .ok(201, Contract::schema(CATALOG, "CatalogUploadReport"))
            .multipart_body()
            .no_ownership(COMMUNITY_RECORD)
            .malformed(
                Probe::new("a JSON body instead of a form", ADMIN)
                    .change(Change::RawBody(b"{}".to_vec(), Some("application/json")))
                    .expect(415),
            )
            .malformed(
                Probe::new("a form without its calibration part", ADMIN)
                    .change(Change::RawBody(
                        multipart(&[("catalog", b"{}")]),
                        Some(MULTIPART_TYPE),
                    ))
                    .expect_with(Expect::status(400).details_code("missing_part")),
            )
            .malformed(
                Probe::new("a form carrying the catalog part twice", ADMIN)
                    .change(Change::RawBody(
                        multipart(&[("catalog", b"{}"), ("catalog", b"{}")]),
                        Some(MULTIPART_TYPE),
                    ))
                    .expect_with(Expect::status(400).details_code("repeated_part")),
            )
            .malformed(
                Probe::new("parts that are not a catalog and a bundle", ADMIN)
                    .change(Change::RawBody(
                        multipart(&[("catalog", b"{}"), ("calibration", b"{}")]),
                        Some(MULTIPART_TYPE),
                    ))
                    .expect(400),
            )
            .boundary(
                Probe::new("the world's stored version again", ADMIN)
                    .fixture("catalog-upload-duplicate")
                    .expect_with(Expect::status(409).details_code("catalog_version_exists")),
            )
            .boundary(
                Probe::new("a bundle pinning other catalog bytes", ADMIN)
                    .fixture("catalog-upload-stale-sha")
                    .expect_with(
                        Expect::status(422)
                            .error_contains("the calibration bundle refuses the catalog")
                            .json_at("/details/accepted", json!(false))
                            .json_at(
                                "/details/failures/0/case_id",
                                json!("provenance/catalog_sha256"),
                            ),
                    ),
            ),
    ]
}

fn fire_mission_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/events/{id}/fire-missions")
            .ok(200, Contract::schema(FIRE, "FireMissionList"))
            .authorized_as(ENLISTED)
            .override_derived(
                "guest",
                Expect::status(404).error_contains("event not found"),
                MEMBERS_ONLY,
            )
            .no_ownership(SHARED_READ)
            .boundary(
                Probe::new("an event without fire missions lists none", ENLISTED)
                    .fixture("fire-missions-none")
                    .expect_with(Expect::success().json_at("/data", json!([]))),
            ),
        RouteSpec::authenticated("POST /api/v1/fire-missions")
            .ok(201, Contract::schema(FIRE, "SavedFireMission"))
            .request_contract(Contract::schema(FIRE, "FireMissionSave"))
            .authorized_as(ENLISTED)
            .override_derived(
                "guest",
                Expect::status(404).error_contains("event not found"),
                MEMBERS_ONLY,
            )
            .no_ownership(OWN_NEW_ROW)
            .malformed(
                Probe::new("blank event id", ENLISTED)
                    .merge_body(json!({"event_id": " "}))
                    .expect(400),
            )
            .malformed(
                Probe::new("non-uuid event id", ENLISTED)
                    .merge_body(json!({"event_id": "not-a-uuid"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("blank grid", ENLISTED)
                    .merge_body(json!({"target_grid": "  "}))
                    .expect(400),
            )
            .malformed(
                Probe::new("catalog version zero", ENLISTED)
                    .merge_body(json!({"catalog_version": 0}))
                    .expect(400),
            )
            .malformed(
                Probe::new("the retired single-tube body", ENLISTED)
                    .body(
                        json!({"weapon_system": "M252 81mm", "fp_x": 1000.0, "fp_y": 2000.0,
                        "tgt_x": 1500.0, "tgt_y": 2600.0, "fp_grid": "010020",
                        "target_grid": "015026"}),
                    )
                    .expect(400),
            )
            .boundary(
                Probe::new("a fire mission without an event", ENLISTED)
                    .merge_body(json!({"event_id": null})),
            )
            .boundary(
                Probe::new("nonexistent event", ENLISTED)
                    .merge_body(json!({"event_id": "00000000-0000-4000-8000-000000000000"}))
                    .expect_with(Expect::status(404).error_contains("event not found")),
            )
            .boundary(
                Probe::new("an unknown catalog", ENLISTED)
                    .merge_body(json!({"catalog_id": "no_such_catalog"}))
                    .expect(404),
            )
            .boundary(
                Probe::new("a client solution the server does not reproduce", ENLISTED)
                    .fixture("fire-mission-tampered")
                    .expect_with(Expect::status(422).details_code("solution_mismatch")),
            ),
    ]
}

/// Every spec of the part.
pub fn specs() -> Vec<RouteSpec> {
    [catalog_specs(), fire_mission_specs()]
        .into_iter()
        .flatten()
        .collect()
}
