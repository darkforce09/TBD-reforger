//! Route specs of the `administration_center_content` part: the administration routes
//! (personnel roster, discipline, membership grace, role resync, audit log), the command center
//! reads (dashboard, leaderboards, player statistics), the community content routes
//! (announcements and their content manager, uploads, modpacks, vehicle database, wiki) and the
//! development-only equipment data viewer reads.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the domain probes (unknown fields, bad queries, paging clamps, length limits, unknown text
//! keys) and a reason for every dimension that does not apply. The equipment data viewer's
//! request queries live here ([`equipment_query`]) so the spec and the world read one table.

use contract_schema_types::administration::audit_log::AuditLogPage;
use contract_schema_types::administration::personnel_roster::PersonnelPage;
use contract_schema_types::community_content::content_upload::UploadResponse;
use contract_schema_types::community_content::equipment_data_viewer::{
    dataset::EquipmentDatasetStatus, field_inventory::EquipmentFieldPage,
    relationships::EquipmentRelationshipPage, resource_cards::EquipmentResourceCardPage,
    resources::EquipmentResourcePage, source_inspection::EquipmentSourcePage,
};
use contract_schema_types::community_content::vehicle_database::{Vehicle, VehicleList};
use contract_schema_types::community_content::wiki_page::{
    WikiArticle, WikiPageList, WikiRevision, WikiRevisionPage,
};
use serde_json::json;

use super::super::contracts::round_trip;
use super::super::spec::{
    Actor, Change, Contract, Dimension, Expect, Probe, Role, RoundTrip, RouteSpec,
};

const ADMIN: Actor = Actor::User(Role::Admin);
const ENLISTED: Actor = Actor::User(Role::Enlisted);
const ADMINISTRATIVE: &str = "administrative route: any administrator acts on any account or \
     row, so no caller owns the resource";
const SHARED_CONTENT: &str = "shared community content: members read every row and any \
     administrator curates every row; no row has an owner";
const COMMUNITY_READ: &str = "community read: every member reads the same rows";
const DEBUG_READ: &str = "development-only debug read of the exported equipment datasets; \
     no row has an owner";
const NO_INPUT: &str = "the route reads no path parameter, query or body";
const NO_PAGING: &str = "no parameter, body or paging: the route answers the whole list";
const TEXT_KEY: &str = "the Discord id is a text key: every string is an id or a miss \
     (the unknown account is a boundary probe)";
const WIKI_SLUG_READ: &str = "the slug is a text key: every string is a page or a miss \
     (the unknown page is a boundary probe)";
const EQUIPMENT: &str = "GET /api/v1/debug/equipment-data/";
const DIAGNOSTIC_RESOURCE: &str = "dataset=diagnostic&resource_id=guid:1111111111111111";
const DISCORD_ROLES: &str = "website roles derive from Discord: the route validates the body and \
     answers 409 in the error envelope, documented in the handler and the route inventory";
const ROLE_CHANGE_REFUSAL: &str = "Website roles are derived from Discord; change the Discord \
     role mapping or use a membership grace extension";
const ROLE_CHANGE_REFUSAL_REASON: &str = "Discord owns every website role: `update_user` in \
     crates/api/api_administration/src/handlers/role_management.rs validates the body and refuses every valid role \
     change with 409";

/// The query string each equipment data viewer read is authorized with: the committed
/// diagnostic export's backpack resource, or the gameplay export where the golden reads it.
pub(crate) fn equipment_query(key: &str) -> Option<&'static str> {
    Some(match key.strip_prefix(EQUIPMENT)? {
        "status" | "resources" | "fields" => "dataset=diagnostic",
        "overview" | "selection" => "",
        "relationships" => "dataset=diagnostic&resource_id=guid:1111111111111111&view=all",
        "resource-cards" => DIAGNOSTIC_RESOURCE,
        "containers" => {
            "dataset=diagnostic&resource_id=guid:1111111111111111&node_id=root&view=all"
        }
        "properties" => {
            "dataset=diagnostic&resource_id=guid:1111111111111111\
             &node_id=root/properties/components/0"
        }
        "values" => {
            "dataset=diagnostic&resource_id=guid:1111111111111111&node_id=root\
             &property=components"
        }
        "documents" => "dataset=diagnostic&document=record&resource_id=guid:1111111111111111",
        "download" => "dataset=diagnostic&document=source&resource_id=guid:1111111111111111",
        _ => return None,
    })
}

/// `query` with `extra` appended.
fn with(query: &str, extra: &str) -> String {
    if query.is_empty() {
        extra.to_string()
    } else {
        format!("{query}&{extra}")
    }
}

fn equipment_read(key: &'static str, contract: Contract, decodes: Option<RoundTrip>) -> RouteSpec {
    let query = equipment_query(key).unwrap_or_default();
    let spec = RouteSpec::development_only(key)
        .ok(200, contract)
        .no_ownership(DEBUG_READ)
        .malformed(
            Probe::new("unsupported dataset", Actor::Anonymous)
                .query("dataset=route-acceptance")
                .expect_with(Expect::status(400).error_contains("unsupported dataset kind")),
        )
        .malformed(
            Probe::new("non-numeric field id", Actor::Anonymous)
                .query(with(query, "field_id=first"))
                .expect(400),
        );
    let spec = if key.ends_with("/status") {
        spec.boundary(
            Probe::new("cursor over the limit", Actor::Anonymous)
                .query(with(query, "cursor=100000001"))
                .expect_with(Expect::status(400).error_contains("cursor is too large")),
        )
    } else {
        spec.boundary(
            Probe::new("unknown generation", Actor::Anonymous)
                .query(with(query, "generation=route-acceptance-missing"))
                .expect(400),
        )
        .boundary(
            Probe::new("generation outside the data directory", Actor::Anonymous)
                .query(with(query, "generation=../../outside"))
                .expect(400),
        )
    };
    match decodes {
        Some(decodes) => spec.decodes(decodes),
        None => spec,
    }
}

fn viewer_schema(file: &'static str) -> Contract {
    Contract::schema_root(file)
}

fn equipment_reads() -> Vec<RouteSpec> {
    let status = round_trip::<EquipmentDatasetStatus> as RoundTrip;
    let source = round_trip::<EquipmentSourcePage> as RoundTrip;
    let dataset = || viewer_schema("equipment-data-viewer/dataset.schema.json");
    let inspection = || viewer_schema("equipment-data-viewer/source-inspection.schema.json");
    vec![
        equipment_read(
            "GET /api/v1/debug/equipment-data/status",
            dataset(),
            Some(status),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/overview",
            dataset(),
            Some(status),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/resources",
            viewer_schema("equipment-data-viewer/resources.schema.json"),
            Some(round_trip::<EquipmentResourcePage>),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/relationships",
            viewer_schema("equipment-data-viewer/relationships.schema.json"),
            Some(round_trip::<EquipmentRelationshipPage>),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/fields",
            viewer_schema("equipment-data-viewer/field-inventory.schema.json"),
            Some(round_trip::<EquipmentFieldPage>),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/resource-cards",
            viewer_schema("equipment-data-viewer/resource-cards.schema.json"),
            Some(round_trip::<EquipmentResourceCardPage>),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/selection",
            inspection(),
            Some(source),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/containers",
            inspection(),
            Some(source),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/properties",
            inspection(),
            Some(source),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/values",
            inspection(),
            Some(source),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/documents",
            inspection(),
            Some(source),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/download",
            Contract::Binary("application/json"),
            None,
        ),
    ]
}

fn personnel_actions(definition: &'static str) -> Contract {
    Contract::schema("personnel-actions.schema.json", definition)
}

fn announcement(definition: &'static str) -> Contract {
    Contract::schema("announcement.schema.json", definition)
}

fn modpack(definition: &'static str) -> Contract {
    Contract::schema("modpack.schema.json", definition)
}

fn vehicle(definition: &'static str) -> Contract {
    Contract::schema("vehicle-database.schema.json", definition)
}

fn wiki(definition: &'static str) -> Contract {
    Contract::schema("wiki-page.schema.json", definition)
}

fn unknown_account(actor: Actor, status: u16) -> Probe {
    Probe::new("unknown account", actor)
        .param("discordId", "route-acceptance-unknown-account")
        .expect(status)
}

fn blank_reason() -> Probe {
    Probe::new("blank reason", ADMIN)
        .merge_body(json!({"reason": "   "}))
        .expect(400)
}

fn unknown_field(actor: Actor) -> Probe {
    Probe::new("unknown field", actor)
        .merge_body(json!({"route_acceptance": true}))
        .expect(400)
}

fn non_numeric(name: &str, actor: Actor, query: &'static str) -> Probe {
    Probe::new(format!("non-numeric {name}"), actor)
        .query(query)
        .expect(400)
}

fn administration() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("GET /api/v1/admin/audit-logs", Role::Admin)
            .ok(
                200,
                Contract::schema("audit-log.schema.json", "AuditLogPage"),
            )
            .decodes(round_trip::<AuditLogPage>)
            .no_ownership(ADMINISTRATIVE)
            .malformed(non_numeric("limit", ADMIN, "limit=many"))
            .malformed(non_numeric("cursor", ADMIN, "before=latest"))
            .boundary(
                Probe::new("limit over the maximum is served the default", ADMIN)
                    .query("limit=100000")
                    .expect_with(Expect::success().max_items("/data", 100)),
            )
            .boundary(
                Probe::new("cursor before the first row", ADMIN)
                    .query("before=1")
                    .expect_with(
                        Expect::success()
                            .json_at("/data", json!([]))
                            .json_at("/next_cursor", json!(null)),
                    ),
            ),
        RouteSpec::role("GET /api/v1/admin/audit-logs/export.csv", Role::Admin)
            .ok(200, Contract::Binary("text/csv"))
            .no_ownership(ADMINISTRATIVE)
            .malformed(non_numeric("cursor", ADMIN, "before=latest"))
            .boundary(Probe::new("severity filter", ADMIN).query("severity=crit"))
            .boundary(
                Probe::new("search matching nothing", ADMIN).query("q=route-acceptance-none"),
            ),
        RouteSpec::role("GET /api/v1/admin/audit-logs/stream", Role::Admin)
            .ok(
                200,
                Contract::event_stream("audit-log.schema.json", "AuditStreamReady"),
            )
            .no_ownership(ADMINISTRATIVE)
            .malformed(
                Probe::new("non-numeric Last-Event-ID", ADMIN)
                    .change(Change::Header("last-event-id", "latest".into()))
                    .expect(400),
            )
            .malformed(
                Probe::new("negative Last-Event-ID", ADMIN)
                    .change(Change::Header("last-event-id", "-1".into()))
                    .expect(400),
            )
            .boundary(
                Probe::new("resume from cursor zero", ADMIN)
                    .change(Change::Header("last-event-id", "0".into())),
            ),
        RouteSpec::role("GET /api/v1/admin/users", Role::Admin)
            .ok(
                200,
                Contract::schema("personnel-roster.schema.json", "PersonnelPage"),
            )
            .decodes(round_trip::<PersonnelPage>)
            .no_ownership(ADMINISTRATIVE)
            .malformed(Probe::new("page zero", ADMIN).query("page=0").expect(400))
            .malformed(non_numeric("page size", ADMIN, "per_page=all"))
            .boundary(
                Probe::new("page size clamps to 100", ADMIN)
                    .query("per_page=100000")
                    .expect_with(
                        Expect::success()
                            .json_at("/per_page", json!(100))
                            .max_items("/items", 100),
                    ),
            )
            .boundary(
                Probe::new("page past the end", ADMIN)
                    .query("page=9223372036854775807")
                    .expect_with(Expect::success().json_at("/items", json!([]))),
            ),
        RouteSpec::role("PATCH /api/v1/admin/users/{discordId}", Role::Admin)
            .refusal_only(409, ROLE_CHANGE_REFUSAL, ROLE_CHANGE_REFUSAL_REASON)
            .json_body()
            .text_param("discordId")
            .no_ownership(DISCORD_ROLES)
            .malformed(
                Probe::new("unknown role", ADMIN)
                    .merge_body(json!({"role": "quartermaster"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("missing role", ADMIN)
                    .change(Change::RemoveField("role"))
                    .expect(400),
            )
            .boundary(
                Probe::new("padded role is refused", ADMIN)
                    .merge_body(json!({"role": " admin "}))
                    .expect(400),
            ),
        RouteSpec::role("POST /api/v1/admin/users/{discordId}/ban", Role::Admin)
            .ok(200, personnel_actions("BanState"))
            .request_contract(personnel_actions("BanRequest"))
            .text_param("discordId")
            .no_ownership(ADMINISTRATIVE)
            .malformed(blank_reason())
            .boundary(unknown_account(ADMIN, 404)),
        RouteSpec::role("DELETE /api/v1/admin/users/{discordId}/ban", Role::Admin)
            .ok(200, personnel_actions("BanState"))
            .text_param("discordId")
            .no_ownership(ADMINISTRATIVE)
            .not_applicable(
                Dimension::Malformed,
                "the Discord id is a text key and the route reads no query or body",
            )
            .boundary(unknown_account(ADMIN, 404)),
        RouteSpec::role("POST /api/v1/admin/users/{discordId}/warnings", Role::Admin)
            .ok(201, personnel_actions("Warning"))
            .request_contract(personnel_actions("WarningRequest"))
            .text_param("discordId")
            .no_ownership(ADMINISTRATIVE)
            .malformed(blank_reason())
            .boundary(unknown_account(ADMIN, 404)),
        RouteSpec::role(
            "POST /api/v1/admin/users/{discordId}/membership-grace",
            Role::Admin,
        )
        .ok(200, personnel_actions("MembershipGraceExtension"))
        .request_contract(personnel_actions("MembershipGraceRequest"))
        .text_param("discordId")
        .override_derived(
            "rank below",
            Expect::status(403).error_contains("verified administrator required"),
            "the grace extension checks the caller's verified administrator authority inside \
             its transaction (api_identity_and_access::services::membership_grace_overrides), so \
             the refusal names that authority",
        )
        .override_derived(
            "guest",
            Expect::status(403).error_contains("verified administrator required"),
            "the grace extension checks the caller's verified administrator authority inside \
             its transaction, so the refusal names that authority",
        )
        .no_ownership(ADMINISTRATIVE)
        .malformed(unknown_field(ADMIN))
        .malformed(blank_reason())
        .boundary(
            Probe::new("zero hours", ADMIN)
                .merge_body(json!({"duration_hours": 0}))
                .expect(400),
        )
        .boundary(
            Probe::new("49 hours", ADMIN)
                .merge_body(json!({"duration_hours": 49}))
                .expect(400),
        )
        .boundary(Probe::new("48 hours", ADMIN).merge_body(json!({"duration_hours": 48})))
        .boundary(unknown_account(ADMIN, 404)),
        RouteSpec::role("POST /api/v1/admin/roles/sync", Role::Admin)
            .ok(200, personnel_actions("RoleResyncOutcome"))
            .no_ownership(ADMINISTRATIVE)
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .boundary(Probe::new("a second resync is idempotent", ADMIN)),
    ]
}

fn command_center() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/dashboard")
            .ok(
                200,
                Contract::schema("command-center.schema.json", "Dashboard"),
            )
            .no_ownership("the dashboard answers the caller's own assignments and shared rows")
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(Dimension::Boundary, NO_PAGING),
        RouteSpec::authenticated("GET /api/v1/leaderboards")
            .ok(
                200,
                Contract::schema("command-center.schema.json", "LeaderboardPage"),
            )
            .no_ownership(COMMUNITY_READ)
            .malformed(
                Probe::new("unknown category", ENLISTED)
                    .query("category=bravery")
                    .expect(400),
            )
            .malformed(non_numeric("limit", ENLISTED, "limit=many"))
            .boundary(
                Probe::new("limit clamps to 50", ENLISTED)
                    .query("limit=100")
                    .expect_with(Expect::success().max_items("/data", 50)),
            )
            .boundary(
                Probe::new("offset past the end", ENLISTED)
                    .query("category=missions&offset=1000000")
                    .expect_with(Expect::success().json_at("/data", json!([]))),
            ),
        RouteSpec::authenticated("GET /api/v1/users/{discordId}/stats")
            .ok(
                200,
                Contract::schema("command-center.schema.json", "PlayerStatsCard"),
            )
            .text_param("discordId")
            .no_ownership(COMMUNITY_READ)
            .not_applicable(Dimension::Malformed, TEXT_KEY)
            .boundary(unknown_account(ENLISTED, 404)),
    ]
}

fn announcement_list(spec: RouteSpec, actor: Actor) -> RouteSpec {
    spec.ok(200, announcement("AnnouncementPage"))
        .no_ownership(SHARED_CONTENT)
        .malformed(non_numeric("limit", actor, "limit=many"))
        .malformed(non_numeric("offset", actor, "offset=start"))
        .boundary(
            Probe::new("limit over the maximum is served the default", actor)
                .query("limit=100000")
                .expect_with(Expect::success().max_items("/data", 100)),
        )
}

fn announcements() -> Vec<RouteSpec> {
    vec![
        announcement_list(
            RouteSpec::authenticated("GET /api/v1/announcements"),
            ENLISTED,
        ),
        RouteSpec::authenticated("GET /api/v1/announcements/{id}")
            .ok(200, announcement("Announcement"))
            .no_ownership(SHARED_CONTENT)
            .boundary(
                Probe::new("a draft is not public", ENLISTED)
                    .fixture("draft-announcement")
                    .expect(404),
            ),
        announcement_list(
            RouteSpec::role("GET /api/v1/cms/announcements", Role::Admin),
            ADMIN,
        ),
        RouteSpec::role("POST /api/v1/cms/announcements", Role::Admin)
            .ok(201, announcement("Announcement"))
            .request_contract(announcement("AnnouncementCreation"))
            .no_ownership(SHARED_CONTENT)
            .malformed(
                Probe::new("blank title", ADMIN)
                    .merge_body(json!({"title": "  "}))
                    .expect(400),
            )
            .malformed(
                Probe::new("unknown status", ADMIN)
                    .merge_body(json!({"status": "PUBLISHED"}))
                    .expect(400),
            )
            .malformed(
                Probe::new("unknown tag", ADMIN)
                    .merge_body(json!({"tag": "gossip"}))
                    .expect(400),
            )
            .boundary(
                Probe::new("script thumbnail url", ADMIN)
                    .merge_body(json!({"thumbnail_url": "javascript:alert(1)"}))
                    .expect(400),
            )
            .boundary(
                Probe::new("snippet over 200 characters is capped", ADMIN)
                    .merge_body(json!({"snippet": "s".repeat(300)})),
            ),
        RouteSpec::role("PATCH /api/v1/cms/announcements/{id}", Role::Admin)
            .ok(200, announcement("Announcement"))
            .request_contract(announcement("AnnouncementChange"))
            .no_ownership(SHARED_CONTENT)
            .malformed(
                Probe::new("blank title", ADMIN)
                    .merge_body(json!({"title": ""}))
                    .expect(400),
            )
            .malformed(
                Probe::new("empty status", ADMIN)
                    .merge_body(json!({"status": ""}))
                    .expect(400),
            ),
        RouteSpec::role("DELETE /api/v1/cms/announcements/{id}", Role::Admin)
            .ok(204, Contract::NoBody)
            .no_ownership(SHARED_CONTENT),
        RouteSpec::role(
            "POST /api/v1/cms/announcements/{id}/push-discord",
            Role::Admin,
        )
        .ok(200, announcement("DiscordPushOutcome"))
        .no_ownership(SHARED_CONTENT)
        .boundary(
            Probe::new("a draft cannot be pushed", ADMIN)
                .fixture("draft-announcement")
                .expect(400),
        ),
        RouteSpec::role("POST /api/v1/cms/uploads", Role::Admin)
            .ok(
                201,
                Contract::schema("content-upload.schema.json", "UploadResponse"),
            )
            .decodes(round_trip::<UploadResponse>)
            .multipart_body()
            .no_ownership("an upload creates a new public file; it addresses no existing row")
            .malformed(
                Probe::new("no file field", ADMIN)
                    .fixture("upload-without-file")
                    .expect(400),
            )
            .malformed(
                Probe::new("text file", ADMIN)
                    .fixture("upload-text-file")
                    .expect(415),
            ),
    ]
}

fn modpack_write(key: &'static str) -> RouteSpec {
    RouteSpec::role(key, Role::Admin)
        .request_contract(modpack("ModpackWrite"))
        .no_ownership(SHARED_CONTENT)
        .malformed(
            Probe::new("blank name", ADMIN)
                .merge_body(json!({"name": " "}))
                .expect(400),
        )
        .malformed(
            Probe::new("blank mod name", ADMIN)
                .merge_body(json!({"mods": [{"name": ""}]}))
                .expect(400),
        )
        .boundary(
            Probe::new("negative size", ADMIN)
                .merge_body(json!({"total_size_bytes": -1}))
                .expect(400),
        )
}

fn modpacks() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/modpacks")
            .ok(200, modpack("ModpackList"))
            .no_ownership(COMMUNITY_READ)
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(Dimension::Boundary, NO_PAGING),
        RouteSpec::authenticated("GET /api/v1/modpacks/current")
            .ok(200, modpack("Modpack"))
            .no_ownership(COMMUNITY_READ)
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(
                Dimension::Boundary,
                "the route answers the single current pack; it takes no parameter, body or \
                 paging",
            ),
        modpack_write("POST /api/v1/modpacks").ok(201, modpack("Modpack")),
        modpack_write("PUT /api/v1/modpacks/{id}").ok(200, modpack("Modpack")),
        RouteSpec::role("POST /api/v1/modpacks/{id}/set-current", Role::Admin)
            .ok(200, modpack("Modpack"))
            .no_ownership(SHARED_CONTENT),
        RouteSpec::role("DELETE /api/v1/modpacks/{id}", Role::Admin)
            .ok(204, Contract::NoBody)
            .no_ownership(SHARED_CONTENT),
    ]
}

fn long_vehicle_name(length: usize, status: u16) -> Probe {
    Probe::new(format!("name of {length} characters"), ADMIN)
        .merge_body(json!({"name": "n".repeat(length)}))
        .expect(status)
}

fn vehicle_write(key: &'static str, success: u16) -> RouteSpec {
    RouteSpec::role(key, Role::Admin)
        .ok(success, vehicle("Vehicle"))
        .decodes(round_trip::<Vehicle>)
        .request_contract(vehicle("VehicleWrite"))
        .no_ownership(SHARED_CONTENT)
        .malformed(unknown_field(ADMIN))
        .malformed(
            Probe::new("missing name", ADMIN)
                .change(Change::RemoveField("name"))
                .expect(400),
        )
        .boundary(long_vehicle_name(121, 400))
        .boundary(long_vehicle_name(120, success))
}

fn vehicles() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/vehicle-database")
            .ok(200, vehicle("VehicleList"))
            .decodes(round_trip::<VehicleList>)
            .no_ownership(COMMUNITY_READ)
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(Dimension::Boundary, NO_PAGING),
        RouteSpec::authenticated("GET /api/v1/vehicle-database/{id}")
            .ok(200, vehicle("Vehicle"))
            .decodes(round_trip::<Vehicle>)
            .no_ownership(COMMUNITY_READ),
        vehicle_write("POST /api/v1/vehicle-database", 201),
        vehicle_write("PUT /api/v1/vehicle-database/{id}", 200),
        RouteSpec::role("PATCH /api/v1/vehicle-database/{id}", Role::Admin)
            .ok(200, vehicle("Vehicle"))
            .decodes(round_trip::<Vehicle>)
            .request_contract(vehicle("VehiclePatch"))
            .no_ownership(SHARED_CONTENT)
            .malformed(unknown_field(ADMIN))
            .boundary(long_vehicle_name(121, 400))
            .boundary(Probe::new("empty patch keeps the row", ADMIN).body(json!({}))),
        RouteSpec::role("DELETE /api/v1/vehicle-database/{id}", Role::Admin)
            .ok(200, vehicle("Vehicle"))
            .decodes(round_trip::<Vehicle>)
            .no_ownership(SHARED_CONTENT),
    ]
}

fn wiki_pages() -> Vec<RouteSpec> {
    let unknown_page = |actor| {
        Probe::new("unknown page", actor)
            .param("slug", "route-acceptance-missing-page")
            .expect(404)
    };
    vec![
        RouteSpec::authenticated("GET /api/v1/wiki")
            .ok(200, wiki("WikiPageList"))
            .decodes(round_trip::<WikiPageList>)
            .no_ownership(COMMUNITY_READ)
            .not_applicable(Dimension::Malformed, NO_INPUT)
            .not_applicable(Dimension::Boundary, NO_PAGING),
        RouteSpec::authenticated("GET /api/v1/wiki/{slug}")
            .ok(200, wiki("WikiArticle"))
            .decodes(round_trip::<WikiArticle>)
            .text_param("slug")
            .no_ownership(COMMUNITY_READ)
            .not_applicable(Dimension::Malformed, WIKI_SLUG_READ)
            .boundary(unknown_page(ENLISTED)),
        RouteSpec::role("PUT /api/v1/wiki/{slug}", Role::Admin)
            .ok(201, wiki("WikiArticle"))
            .decodes(round_trip::<WikiArticle>)
            .request_contract(wiki("WikiSaveRequest"))
            .text_param("slug")
            .no_ownership(SHARED_CONTENT)
            .malformed(unknown_field(ADMIN))
            .malformed(
                Probe::new("missing base revision", ADMIN)
                    .change(Change::RemoveField("base_revision"))
                    .expect(400),
            )
            .malformed(
                Probe::new("uppercase slug", ADMIN)
                    .param("slug", "Route-Acceptance")
                    .expect(400),
            )
            .boundary(
                Probe::new("save the next revision", ADMIN)
                    .fixture("wiki-revise")
                    .expect_with(Expect::status(200).contract(wiki("WikiArticle"))),
            )
            .boundary(
                Probe::new("create over an existing page", ADMIN)
                    .fixture("wiki-existing")
                    .expect_with(Expect::status(409).details_code("wiki_revision_conflict")),
            )
            .boundary(
                Probe::new("revise a missing page", ADMIN)
                    .merge_body(json!({"base_revision": 1}))
                    .expect(404),
            )
            .boundary(
                Probe::new("body over 256 KiB", ADMIN)
                    .merge_body(json!({"body_md": "a".repeat(262_145)}))
                    .expect_with(Expect::status(400).details_code("wiki_body_too_large")),
            )
            .boundary(
                Probe::new("slug over 64 bytes", ADMIN)
                    .param("slug", "a".repeat(65))
                    .expect(400),
            ),
        RouteSpec::authenticated("GET /api/v1/wiki/{slug}/revisions")
            .ok(200, wiki("WikiRevisionPage"))
            .decodes(round_trip::<WikiRevisionPage>)
            .text_param("slug")
            .no_ownership(COMMUNITY_READ)
            .malformed(
                Probe::new("page zero", ENLISTED)
                    .query("page=0")
                    .expect(400),
            )
            .malformed(non_numeric("page size", ENLISTED, "per_page=all"))
            .boundary(
                Probe::new("page size clamps to 100", ENLISTED)
                    .query("per_page=100000")
                    .expect_with(Expect::success().json_at("/per_page", json!(100))),
            )
            .boundary(unknown_page(ENLISTED)),
        RouteSpec::authenticated("GET /api/v1/wiki/{slug}/revisions/{revision}")
            .ok(200, wiki("WikiRevision"))
            .decodes(round_trip::<WikiRevision>)
            .text_param("slug")
            .text_param("revision")
            .no_ownership(COMMUNITY_READ)
            .malformed(
                Probe::new("revision is not a number", ENLISTED)
                    .param("revision", "first")
                    .expect(400),
            )
            .boundary(
                Probe::new("revision past the history", ENLISTED)
                    .param("revision", "99999")
                    .expect(404),
            )
            .boundary(
                Probe::new("revision beyond 32 bits", ENLISTED)
                    .param("revision", "99999999999")
                    .expect(404),
            )
            .boundary(unknown_page(ENLISTED)),
    ]
}

/// Every spec of the `administration_center_content` part.
pub(crate) fn specs() -> Vec<RouteSpec> {
    [
        administration(),
        command_center(),
        announcements(),
        modpacks(),
        vehicles(),
        wiki_pages(),
        equipment_reads(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
