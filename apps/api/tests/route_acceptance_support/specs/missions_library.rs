//! Route specs of the `missions_library` part: the mission library and its bookmarks, mission
//! creation, metadata, deletion and export, versions, the armory, the review workspace, the
//! administrators' default-override report, the faction library and the registry.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the ownership probes (the author, a peer mission maker and an administrator), the domain
//! refusals and boundaries, and a reason for every dimension that does not apply. Fixture keys
//! other than route keys are resolved by `world/missions_library.rs`.

use contract_schema_types::missions::mission_review::{
    MissionRow, MissionVersion, ReviewWorkspace,
};
use serde_json::{Value, json};

use super::super::contracts::{round_trip, violations};
use super::super::spec::{Actor, Change, Contract, Dimension, Expect, Probe, Role, RouteSpec};

const GUEST: Actor = Actor::User(Role::Guest);
const MAKER: Actor = Actor::User(Role::MissionMaker);
const PEER: Actor = Actor::Peer(Role::MissionMaker);
const ADMIN: Actor = Actor::User(Role::Admin);
const NO_INPUT: &str = "the route reads no path parameter, query or body";
const OWN_FACTIONS: &str = "the faction library lists the caller's own rows only";
const SHARED_CATALOGUE: &str = "the registry is shared modpack catalogue data owned by no account";

fn library(definition: &'static str) -> Contract {
    Contract::schema("mission-library.schema.json", definition)
}

fn review(definition: &'static str) -> Contract {
    Contract::schema("mission-review.schema.json", definition)
}

fn arsenal(definition: &'static str) -> Contract {
    Contract::schema("arsenal-envelopes.schema.json", definition)
}

fn faction_document() -> Contract {
    Contract::schema_root("faction-library.schema.json")
}

/// Every `doc` of a faction response validates against the faction-library document schema.
fn faction_documents(documents: Vec<&Value>) -> Result<(), String> {
    let problems: Vec<String> = documents
        .into_iter()
        .enumerate()
        .flat_map(|(index, document)| {
            violations("faction-library.schema.json", None, document)
                .into_iter()
                .map(move |violation| format!("doc {index}: {violation}"))
        })
        .collect();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// The faction list's parity check: every row's `doc` is a faction-library document.
fn faction_list_documents(value: &Value) -> Result<Value, String> {
    let rows = value["data"]
        .as_array()
        .ok_or("the faction list has no `data` array")?;
    faction_documents(rows.iter().map(|row| &row["doc"]).collect())?;
    Ok(value.clone())
}

/// One faction's parity check: its `doc` is a faction-library document.
fn faction_row_document(value: &Value) -> Result<Value, String> {
    faction_documents(vec![&value["doc"]])?;
    Ok(value.clone())
}

/// The owner reads the draft, an administrator stands in for the author, a peer gets 404.
fn hidden_draft_read(spec: RouteSpec, fixture: &'static str) -> RouteSpec {
    spec.ownership(Probe::new("author reads the draft", MAKER).fixture(fixture))
        .ownership(Probe::new("administrator reads the draft", ADMIN).fixture(fixture))
        .ownership(
            Probe::new("peer mission maker cannot see the draft", PEER)
                .fixture(fixture)
                .expect_with(Expect::status(404).error_contains("mission not found")),
        )
}

/// A peer mission maker gets 403 "not your mission"; an administrator stands in for the author.
fn author_or_admin_write(spec: RouteSpec) -> RouteSpec {
    spec.ownership(
        Probe::new("peer mission maker is refused", PEER)
            .expect_with(Expect::status(403).error_contains("not your mission")),
    )
    .ownership(Probe::new("administrator overrides the author", ADMIN))
}

fn mission_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/missions")
            .ok(200, library("MissionLibraryPage"))
            .ownership(
                Probe::new("author finds the own draft", MAKER)
                    .fixture("library-draft-search")
                    .expect_with(Expect::success().json_at("/total", json!(1))),
            )
            .ownership(
                Probe::new("peer mission maker does not see the draft", PEER)
                    .fixture("library-draft-search")
                    .expect_with(Expect::success().json_at("/total", json!(0))),
            )
            .malformed(
                Probe::new("non-numeric limit", GUEST)
                    .query("limit=x")
                    .expect(400),
            )
            .malformed(
                Probe::new("non-numeric offset", GUEST)
                    .query("offset=first")
                    .expect(400),
            )
            .boundary(
                Probe::new("limit of 100 is honoured", GUEST)
                    .query("limit=100")
                    .expect_with(Expect::success().json_at("/limit", json!(100))),
            )
            .boundary(
                Probe::new("limit over 100 falls back to 20", GUEST)
                    .query("limit=101")
                    .expect_with(
                        Expect::success()
                            .json_at("/limit", json!(20))
                            .max_items("/data", 20),
                    ),
            )
            .boundary(
                Probe::new("negative offset starts at 0", GUEST)
                    .query("offset=-5")
                    .expect_with(Expect::success().json_at("/offset", json!(0))),
            ),
        RouteSpec::role("POST /api/v1/missions", Role::MissionMaker)
            .ok(201, review("MissionRow"))
            .decodes(round_trip::<MissionRow>)
            .request_contract(library("MissionCreation"))
            .no_ownership("creation makes the caller the author; no existing mission is addressed")
            .malformed(
                Probe::new("blank title", MAKER)
                    .merge_body(json!({"title": "   "}))
                    .expect(400),
            )
            .malformed(
                Probe::new("unknown terrain", MAKER)
                    .merge_body(json!({"terrain": "altis"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("clock out of range", MAKER)
                    .merge_body(json!({"time_of_day": "25:00"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("missing game mode", MAKER)
                    .change(Change::RemoveField("game_mode"))
                    .expect(400),
            )
            .boundary(
                Probe::new("no players", MAKER)
                    .merge_body(json!({"max_players": 0}))
                    .expect(400),
            )
            .boundary(
                Probe::new("257 players", MAKER)
                    .merge_body(json!({"max_players": 257}))
                    .expect(400),
            )
            .boundary(Probe::new("256 players", MAKER).merge_body(json!({"max_players": 256}))),
        hidden_draft_read(
            RouteSpec::authenticated("GET /api/v1/missions/{id}").ok(200, library("MissionDetail")),
            "draft-mission",
        )
        .boundary(
            Probe::new("deleted mission", MAKER)
                .fixture("deleted-mission")
                .expect(404),
        ),
        author_or_admin_write(
            RouteSpec::role("PATCH /api/v1/missions/{id}", Role::MissionMaker)
                .ok(200, review("MissionRow"))
                .decodes(round_trip::<MissionRow>)
                .request_contract(library("MissionChange")),
        )
        .malformed(
            Probe::new("unknown weather", MAKER)
                .merge_body(json!({"weather": "sandstorm"}))
                .expect(400),
        )
        .malformed(
            Probe::new("blank title", MAKER)
                .merge_body(json!({"title": " "}))
                .expect(400),
        )
        .malformed(
            Probe::new("status cannot jump to live", MAKER)
                .merge_body(json!({"status": "live"}))
                .expect(400),
        )
        .boundary(
            Probe::new("257 players", MAKER)
                .merge_body(json!({"max_players": 257}))
                .expect(400),
        )
        .boundary(Probe::new("one player", MAKER).merge_body(json!({"max_players": 1})))
        .boundary(
            Probe::new("archive a draft", MAKER)
                .body(json!({"status": "archived"}))
                .expect_with(Expect::success().json_at("/status", json!("archived"))),
        )
        .boundary(
            Probe::new("only an archived mission returns to draft", MAKER)
                .fixture("live-mission-change")
                .body(json!({"status": "draft"}))
                .expect(409),
        ),
        author_or_admin_write(
            RouteSpec::role("DELETE /api/v1/missions/{id}", Role::MissionMaker)
                .ok(204, Contract::NoBody),
        )
        .boundary(
            Probe::new("already deleted", MAKER)
                .fixture("deleted-mission")
                .expect(404),
        ),
        RouteSpec::authenticated("POST /api/v1/missions/{id}/bookmark")
            .ok(200, library("BookmarkState"))
            .ownership(
                Probe::new("peer cannot bookmark a hidden draft", PEER)
                    .fixture("draft-mission")
                    .expect_with(Expect::status(404).error_contains("mission not found")),
            )
            .ownership(Probe::new("author bookmarks the own draft", MAKER).fixture("draft-mission"))
            .boundary(
                Probe::new("bookmarking twice is idempotent", GUEST)
                    .expect_with(Expect::success().json_at("/bookmarked", json!(true))),
            ),
        RouteSpec::authenticated("DELETE /api/v1/missions/{id}/bookmark")
            .ok(200, library("BookmarkState"))
            .no_ownership("the removal deletes the caller's own bookmark row only")
            .boundary(
                Probe::new("removing twice is idempotent", GUEST)
                    .expect_with(Expect::success().json_at("/bookmarked", json!(false))),
            ),
        hidden_draft_read(
            RouteSpec::authenticated("GET /api/v1/missions/{id}/armory")
                .ok(200, library("MissionArmoryList")),
            "draft-mission",
        ),
        author_or_admin_write(
            RouteSpec::role("PUT /api/v1/missions/{id}/armory", Role::MissionMaker)
                .ok(200, library("MissionArmoryList"))
                .request_contract(library("MissionArmoryChange")),
        )
        .malformed(
            Probe::new("missing items", MAKER)
                .body(json!({}))
                .expect(400),
        )
        .malformed(
            Probe::new("blank item name", MAKER)
                .body(json!({"items": [{"faction": "BLUFOR", "item_name": " "}]}))
                .expect(400),
        )
        .malformed(
            Probe::new("padded faction", MAKER)
                .body(json!({"items": [{"faction": " BLUFOR", "item_name": "M16A2"}]}))
                .expect(400),
        )
        .boundary(
            Probe::new("empty items clear the armory", MAKER)
                .body(json!({"items": []}))
                .expect_with(Expect::success().max_items("/data", 0)),
        )
        .boundary(
            Probe::new("negative quantity", MAKER)
                .body(json!({"items": [{"faction": "BLUFOR", "item_name": "M16A2",
                    "quantity": -1}]}))
                .expect(400),
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
                .ok(201, review("MissionVersion"))
                .decodes(round_trip::<MissionVersion>)
                .json_body(),
        )
        .malformed(
            Probe::new("semver without patch", MAKER)
                .merge_body(json!({"semver": "1.0"}))
                .expect(400),
        )
        .malformed(
            Probe::new("missing payload", MAKER)
                .change(Change::RemoveField("payload"))
                .expect(400),
        )
        .boundary(
            Probe::new("vacuous payload", MAKER)
                .merge_body(json!({"payload": {}}))
                .expect(400),
        )
        .boundary(
            Probe::new("duplicate semver", MAKER)
                .fixture("version-duplicate")
                .expect_with(Expect::status(409).error_contains("version already exists")),
        ),
        hidden_draft_read(
            RouteSpec::authenticated("GET /api/v1/missions/{id}/versions/{vid}")
                .ok(200, review("MissionVersion"))
                .decodes(round_trip::<MissionVersion>),
            "draft-version",
        )
        .boundary(
            Probe::new("version of another mission", MAKER)
                .fixture("foreign-version")
                .expect_with(Expect::status(404).error_contains("version not found")),
        ),
        author_or_admin_write(
            RouteSpec::role(
                "POST /api/v1/missions/{id}/versions/{vid}/set-current",
                Role::MissionMaker,
            )
            .ok(200, review("MissionRow"))
            .decodes(round_trip::<MissionRow>),
        )
        .boundary(
            Probe::new("version of another mission", MAKER)
                .fixture("foreign-version")
                .expect_with(Expect::status(404).error_contains("version not found")),
        ),
        RouteSpec::authenticated("GET /api/v1/missions/{id}/artifacts/{artifact_id}/workspace")
            .ok(200, review("ReviewWorkspace"))
            .decodes(round_trip::<ReviewWorkspace>)
            .authorized_as(MAKER)
            .ownership(
                Probe::new("peer cannot see a pending mission", PEER)
                    .expect_with(Expect::status(404).error_contains("mission not found")),
            )
            .ownership(
                Probe::new("peer is refused a live mission's workspace", PEER)
                    .fixture("live-workspace")
                    .expect_with(Expect::status(403).error_contains("not your mission")),
            )
            .ownership(Probe::new("administrator reviews the workspace", ADMIN))
            .override_derived(
                "guest",
                Expect::status(404).error_contains("mission not found"),
                "the workspace serves the mission's author or an administrator only; a guest \
                 cannot author a mission, and a pending mission is hidden from non-authors",
            )
            .boundary(
                Probe::new("artifact of another mission", MAKER)
                    .fixture("foreign-artifact")
                    .expect(404),
            ),
        RouteSpec::role("GET /api/v1/admin/mission-default-overrides", Role::Admin)
            .ok(
                200,
                Contract::schema(
                    "mission-default-overrides.schema.json",
                    "MissionDefaultOverrideReport",
                ),
            )
            .no_ownership("an administrative aggregate over every mission; no owned resource")
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(
                Dimension::Boundary,
                "the report takes no parameter, body or paging",
            ),
    ]
}

fn faction_specs() -> Vec<RouteSpec> {
    let peer_misses = |name: &'static str| {
        Probe::new(name, PEER).expect_with(Expect::status(404).error_contains("faction not found"))
    };
    vec![
        RouteSpec::role("GET /api/v1/factions", Role::MissionMaker)
            .ok(200, arsenal("FactionList"))
            .decodes(faction_list_documents)
            .ownership(
                Probe::new(
                    "peer mission maker sees none of the author's factions",
                    PEER,
                )
                .expect_with(Expect::success().json_at("/total", json!(0))),
            )
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(Dimension::Boundary, OWN_FACTIONS),
        RouteSpec::role("POST /api/v1/factions", Role::MissionMaker)
            .ok(201, arsenal("UserFaction"))
            .decodes(faction_row_document)
            .request_contract(faction_document())
            .no_ownership("creation makes the caller the owner; no existing faction is addressed")
            .malformed(
                Probe::new("unknown field", MAKER)
                    .merge_body(json!({"colour": "red"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("padded name", MAKER)
                    .merge_body(json!({"name": " Route acceptance "}))
                    .expect(400),
            )
            .malformed(
                Probe::new("missing roles", MAKER)
                    .change(Change::RemoveField("roles"))
                    .expect(400),
            )
            .boundary(
                Probe::new("name over 80 characters", MAKER)
                    .merge_body(json!({"name": "n".repeat(81)}))
                    .expect(400),
            )
            .boundary(
                Probe::new("name of 80 characters", MAKER)
                    .merge_body(json!({"name": "n".repeat(80)})),
            )
            .boundary(
                Probe::new("duplicate name", MAKER)
                    .fixture("faction-name-taken")
                    .expect(409),
            ),
        RouteSpec::role("GET /api/v1/factions/{id}", Role::MissionMaker)
            .ok(200, arsenal("UserFaction"))
            .decodes(faction_row_document)
            .ownership(peer_misses("peer mission maker gets 404"))
            .ownership(
                Probe::new("administrators have no override", ADMIN)
                    .expect_with(Expect::status(404).error_contains("faction not found")),
            ),
        RouteSpec::role("PUT /api/v1/factions/{id}", Role::MissionMaker)
            .ok(200, arsenal("UserFaction"))
            .decodes(faction_row_document)
            .request_contract(faction_document())
            .ownership(peer_misses("peer mission maker gets 404"))
            .malformed(
                Probe::new("unknown side", MAKER)
                    .merge_body(json!({"side": "GREEN"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("unknown field", MAKER)
                    .merge_body(json!({"colour": "red"}))
                    .expect(400),
            )
            .boundary(
                Probe::new("name over 80 characters", MAKER)
                    .merge_body(json!({"name": "n".repeat(81)}))
                    .expect(400),
            )
            .boundary(
                Probe::new("rename onto a sibling's name", MAKER)
                    .fixture("faction-rename-clash")
                    .expect(409),
            ),
        RouteSpec::role("DELETE /api/v1/factions/{id}", Role::MissionMaker)
            .ok(204, Contract::NoBody)
            .ownership(peer_misses("peer mission maker gets 404"))
            .boundary(
                Probe::new("already deleted", MAKER)
                    .fixture("faction-deleted")
                    .expect(404),
            ),
    ]
}

fn registry_specs() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("GET /api/v1/registry", Role::MissionMaker)
            .ok(200, arsenal("RegistryItemPage"))
            .no_ownership(SHARED_CATALOGUE)
            .malformed(
                Probe::new("non-numeric limit", MAKER)
                    .fixture("registry-limit-text")
                    .expect(400),
            )
            .boundary(
                Probe::new("page of one", MAKER)
                    .fixture("registry-page-of-one")
                    .expect_with(
                        Expect::success()
                            .max_items("/data", 1)
                            .json_at("/total", json!(3))
                            .json_at("/limit", json!(1)),
                    ),
            )
            .boundary(
                Probe::new("limit clamps to 500", MAKER)
                    .fixture("registry-limit-over")
                    .expect_with(Expect::success().json_at("/limit", json!(500))),
            )
            .boundary(
                Probe::new("unknown modpack", MAKER)
                    .fixture("registry-unknown-modpack")
                    .expect_with(Expect::status(404).error_contains("modpack not found")),
            )
            .boundary(
                Probe::new("unchanged catalogue answers 304", MAKER)
                    .fixture("registry-not-modified")
                    .expect(304),
            ),
        RouteSpec::role("GET /api/v1/registry/compat", Role::MissionMaker)
            .ok(200, arsenal("RegistryCompatPage"))
            .no_ownership(SHARED_CATALOGUE)
            .malformed(
                Probe::new("non-numeric offset", MAKER)
                    .fixture("compat-offset-text")
                    .expect(400),
            )
            .boundary(
                Probe::new("edge type filter", MAKER)
                    .fixture("compat-mag-edges")
                    .expect_with(
                        Expect::success()
                            .max_items("/data", 1)
                            .json_at("/data/0/edge_type", json!("mag_in_weapon")),
                    ),
            )
            .boundary(
                Probe::new("limit clamps to 500", MAKER)
                    .fixture("compat-limit-over")
                    .expect_with(Expect::success().json_at("/limit", json!(500))),
            )
            .boundary(
                Probe::new("cargo defaults view", MAKER)
                    .fixture("compat-cargo-defaults")
                    .expect_with(
                        Expect::status(200)
                            .contract(arsenal("RegistryCargoDefaults"))
                            .json_at("/source_edge_count", json!(1)),
                    ),
            )
            .boundary(
                Probe::new("unchanged graph answers 304", MAKER)
                    .fixture("compat-not-modified")
                    .expect(304),
            ),
    ]
}

/// Every spec of the `missions_library` part.
pub fn specs() -> Vec<RouteSpec> {
    let mut specs = mission_specs();
    specs.extend(version_specs());
    specs.extend(faction_specs());
    specs.extend(registry_specs());
    specs
}
