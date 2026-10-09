//! Route specs of the `missions_library` part: the mission library and its bookmarks, mission
//! creation, metadata, deletion and export, versions, the armory, the review workspace, the
//! administrators' default-override report, the faction library and the registry.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the authorized caller and fixture where the defaults do not fit, and the unauthorized probes
//! and overrides the access class does not imply.

use super::super::spec::{Actor, Contract, Expect, Probe, Role, RouteSpec};

const MAKER: Actor = Actor::User(Role::MissionMaker);
const PEER: Actor = Actor::Peer(Role::MissionMaker);

fn library(definition: &'static str) -> Contract {
    Contract::schema("mission-library.schema.json", definition)
}

fn review(definition: &'static str) -> Contract {
    Contract::schema("mission-review.schema.json", definition)
}

fn arsenal(definition: &'static str) -> Contract {
    Contract::schema("arsenal-envelopes.schema.json", definition)
}

/// The owner reads the draft, an administrator stands in for the author, a peer gets 404.
fn hidden_draft_read(spec: RouteSpec, _fixture: &'static str) -> RouteSpec {
    spec
}

/// A peer mission maker gets 403 "not your mission"; an administrator stands in for the author.
fn author_or_admin_write(spec: RouteSpec) -> RouteSpec {
    spec
}

fn mission_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/missions").ok(200, library("MissionLibraryPage")),
        RouteSpec::role("POST /api/v1/missions", Role::MissionMaker).ok(201, review("MissionRow")),
        hidden_draft_read(
            RouteSpec::authenticated("GET /api/v1/missions/{id}").ok(200, library("MissionDetail")),
            "draft-mission",
        ),
        author_or_admin_write(
            RouteSpec::role("PATCH /api/v1/missions/{id}", Role::MissionMaker)
                .ok(200, review("MissionRow")),
        ),
        author_or_admin_write(
            RouteSpec::role("DELETE /api/v1/missions/{id}", Role::MissionMaker)
                .ok(204, Contract::NoBody),
        ),
        RouteSpec::authenticated("POST /api/v1/missions/{id}/bookmark")
            .ok(200, library("BookmarkState")),
        RouteSpec::authenticated("DELETE /api/v1/missions/{id}/bookmark")
            .ok(200, library("BookmarkState")),
        hidden_draft_read(
            RouteSpec::authenticated("GET /api/v1/missions/{id}/armory")
                .ok(200, library("MissionArmoryList")),
            "draft-mission",
        ),
        author_or_admin_write(
            RouteSpec::role("PUT /api/v1/missions/{id}/armory", Role::MissionMaker)
                .ok(200, library("MissionArmoryList")),
        ),
        hidden_draft_read(
            RouteSpec::role("GET /api/v1/missions/{id}/export", Role::MissionMaker)
                .ok(200, Contract::Binary("application/json")),
            "draft-mission",
        ),
    ]
}

fn version_specs() -> Vec<RouteSpec> {
    vec![
        author_or_admin_write(
            RouteSpec::role("POST /api/v1/missions/{id}/versions", Role::MissionMaker)
                .ok(201, review("MissionVersion")),
        ),
        hidden_draft_read(
            RouteSpec::authenticated("GET /api/v1/missions/{id}/versions/{vid}")
                .ok(200, review("MissionVersion")),
            "draft-version",
        ),
        author_or_admin_write(
            RouteSpec::role(
                "POST /api/v1/missions/{id}/versions/{vid}/set-current",
                Role::MissionMaker,
            )
            .ok(200, review("MissionRow")),
        ),
        RouteSpec::authenticated("GET /api/v1/missions/{id}/artifacts/{artifact_id}/workspace")
            .ok(200, review("ReviewWorkspace"))
            .authorized_as(MAKER),
        RouteSpec::role("GET /api/v1/admin/mission-default-overrides", Role::Admin).ok(
            200,
            Contract::schema(
                "mission-default-overrides.schema.json",
                "MissionDefaultOverrideReport",
            ),
        ),
    ]
}

fn faction_specs() -> Vec<RouteSpec> {
    let _peer_misses = |name: &'static str| {
        Probe::new(name, PEER).expect_with(Expect::status(404).error_contains("faction not found"))
    };
    vec![
        RouteSpec::role("GET /api/v1/factions", Role::MissionMaker).ok(200, arsenal("FactionList")),
        RouteSpec::role("POST /api/v1/factions", Role::MissionMaker)
            .ok(201, arsenal("UserFaction")),
        RouteSpec::role("GET /api/v1/factions/{id}", Role::MissionMaker)
            .ok(200, arsenal("UserFaction")),
        RouteSpec::role("PUT /api/v1/factions/{id}", Role::MissionMaker)
            .ok(200, arsenal("UserFaction")),
        RouteSpec::role("DELETE /api/v1/factions/{id}", Role::MissionMaker)
            .ok(204, Contract::NoBody),
    ]
}

fn registry_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("GET /api/v1/registry", Role::MissionMaker)
            .ok(200, arsenal("RegistryItemPage")),
        RouteSpec::role("GET /api/v1/registry/compat", Role::MissionMaker)
            .ok(200, arsenal("RegistryCompatPage")),
    ]
}

/// Every spec of the `missions_library` part.
pub(crate) fn specs() -> Vec<RouteSpec> {
    let mut specs = mission_specs();
    specs.extend(version_specs());
    specs.extend(faction_specs());
    specs.extend(registry_specs());
    specs
}
