//! Route specs of the `operations_ballistics` part: the public ballistics catalog reads, the
//! administrator catalog upload, and the fire-mission save and an event's saved list.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the authorized caller and fixture where the defaults do not fit, and the unauthorized probes
//! and overrides the access class does not imply.

use super::super::spec::{Actor, Contract, Role, RouteSpec};

const CATALOG: &str = "ballistics-catalog.schema.json";
const FIRE: &str = "fire-mission.schema.json";
const ENLISTED: Actor = Actor::User(Role::Enlisted);

fn catalog_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::public("GET /api/v1/ballistics-catalogs")
            .ok(200, Contract::schema(CATALOG, "BallisticsCatalogList")),
        RouteSpec::public("GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}")
            .ok(200, Contract::schema(CATALOG, "BallisticsCatalog")),
        RouteSpec::role("POST /api/v1/ballistics-catalogs", Role::Admin)
            .ok(201, Contract::schema(CATALOG, "CatalogUploadReport")),
    ]
}

fn fire_mission_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/events/{id}/fire-missions")
            .ok(200, Contract::schema(FIRE, "FireMissionList"))
            .authorized_as(ENLISTED),
        RouteSpec::authenticated("POST /api/v1/fire-missions")
            .ok(201, Contract::schema(FIRE, "SavedFireMission"))
            .authorized_as(ENLISTED),
    ]
}

/// Every spec of the part.
pub(crate) fn specs() -> Vec<RouteSpec> {
    [catalog_specs(), fire_mission_specs()]
        .into_iter()
        .flatten()
        .collect()
}
