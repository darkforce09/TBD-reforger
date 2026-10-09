//! Route specs of the `administration_center_content` part: the administration routes
//! (personnel roster, discipline, membership grace, role resync, audit log), the command center
//! reads (dashboard, leaderboards, player statistics), the community content routes
//! (announcements and their content manager, uploads, modpacks, vehicle database, wiki) and the
//! development-only equipment data viewer reads.
//!
//! Each spec supplies only what the framework cannot derive: the success status and contract,
//! the authorized caller and fixture where the defaults do not fit, and the unauthorized probes
//! and overrides the access class does not imply.

use super::super::spec::{Actor, Contract, Expect, Probe, Role, RouteSpec};

const ADMIN: Actor = Actor::User(Role::Admin);
const ENLISTED: Actor = Actor::User(Role::Enlisted);
const EQUIPMENT: &str = "GET /api/v1/debug/equipment-data/";
const DIAGNOSTIC_RESOURCE: &str = "dataset=diagnostic&resource_id=guid:1111111111111111";
const ROLE_CHANGE_REFUSAL: &str = "Website roles are derived from Discord; change the Discord \
     role mapping or use a membership grace extension";

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

fn equipment_read(key: &'static str, contract: Contract) -> RouteSpec {
    RouteSpec::development_only(key).ok(200, contract)
}

fn viewer_schema(file: &'static str) -> Contract {
    Contract::schema_root(file)
}

fn equipment_reads() -> Vec<RouteSpec> {
    let dataset = || viewer_schema("equipment-data-viewer/dataset.schema.json");
    let inspection = || viewer_schema("equipment-data-viewer/source-inspection.schema.json");
    vec![
        equipment_read("GET /api/v1/debug/equipment-data/status", dataset()),
        equipment_read("GET /api/v1/debug/equipment-data/overview", dataset()),
        equipment_read(
            "GET /api/v1/debug/equipment-data/resources",
            viewer_schema("equipment-data-viewer/resources.schema.json"),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/relationships",
            viewer_schema("equipment-data-viewer/relationships.schema.json"),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/fields",
            viewer_schema("equipment-data-viewer/field-inventory.schema.json"),
        ),
        equipment_read(
            "GET /api/v1/debug/equipment-data/resource-cards",
            viewer_schema("equipment-data-viewer/resource-cards.schema.json"),
        ),
        equipment_read("GET /api/v1/debug/equipment-data/selection", inspection()),
        equipment_read("GET /api/v1/debug/equipment-data/containers", inspection()),
        equipment_read("GET /api/v1/debug/equipment-data/properties", inspection()),
        equipment_read("GET /api/v1/debug/equipment-data/values", inspection()),
        equipment_read("GET /api/v1/debug/equipment-data/documents", inspection()),
        equipment_read(
            "GET /api/v1/debug/equipment-data/download",
            Contract::Binary("application/json"),
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

fn administration() -> Vec<RouteSpec> {
    vec![
        RouteSpec::role("GET /api/v1/admin/audit-logs", Role::Admin).ok(
            200,
            Contract::schema("audit-log.schema.json", "AuditLogPage"),
        ),
        RouteSpec::role("GET /api/v1/admin/audit-logs/export.csv", Role::Admin)
            .ok(200, Contract::Binary("text/csv")),
        RouteSpec::role("GET /api/v1/admin/audit-logs/stream", Role::Admin).ok(
            200,
            Contract::event_stream("audit-log.schema.json", "AuditStreamReady"),
        ),
        RouteSpec::role("GET /api/v1/admin/users", Role::Admin).ok(
            200,
            Contract::schema("personnel-roster.schema.json", "PersonnelPage"),
        ),
        // Discord owns every website role: `update_user` validates the body and refuses every
        // valid role change with 409.
        RouteSpec::role("PATCH /api/v1/admin/users/{discordId}", Role::Admin)
            .refusal_only(409, ROLE_CHANGE_REFUSAL),
        RouteSpec::role("POST /api/v1/admin/users/{discordId}/ban", Role::Admin)
            .ok(200, personnel_actions("BanState")),
        RouteSpec::role("DELETE /api/v1/admin/users/{discordId}/ban", Role::Admin)
            .ok(200, personnel_actions("BanState")),
        RouteSpec::role("POST /api/v1/admin/users/{discordId}/warnings", Role::Admin)
            .ok(201, personnel_actions("Warning")),
        RouteSpec::role(
            "POST /api/v1/admin/users/{discordId}/membership-grace",
            Role::Admin,
        )
        .ok(200, personnel_actions("MembershipGraceExtension"))
        // The grace extension checks the caller's verified administrator authority inside its
        // transaction, so the refusal names that authority.
        .override_derived(
            "rank below",
            Expect::status(403).error_contains("verified administrator required"),
        ),
        RouteSpec::role("POST /api/v1/admin/roles/sync", Role::Admin)
            .ok(200, personnel_actions("RoleResyncOutcome")),
    ]
}

fn command_center() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/dashboard").ok(
            200,
            Contract::schema("command-center.schema.json", "Dashboard"),
        ),
        RouteSpec::authenticated("GET /api/v1/leaderboards").ok(
            200,
            Contract::schema("command-center.schema.json", "LeaderboardPage"),
        ),
        RouteSpec::authenticated("GET /api/v1/users/{discordId}/stats").ok(
            200,
            Contract::schema("command-center.schema.json", "PlayerStatsCard"),
        ),
    ]
}

fn announcement_list(spec: RouteSpec, _actor: Actor) -> RouteSpec {
    spec.ok(200, announcement("AnnouncementPage"))
}

fn announcements() -> Vec<RouteSpec> {
    vec![
        announcement_list(
            RouteSpec::authenticated("GET /api/v1/announcements"),
            ENLISTED,
        ),
        RouteSpec::authenticated("GET /api/v1/announcements/{id}")
            .ok(200, announcement("Announcement")),
        announcement_list(
            RouteSpec::role("GET /api/v1/cms/announcements", Role::Admin),
            ADMIN,
        ),
        RouteSpec::role("POST /api/v1/cms/announcements", Role::Admin)
            .ok(201, announcement("Announcement")),
        RouteSpec::role("PATCH /api/v1/cms/announcements/{id}", Role::Admin)
            .ok(200, announcement("Announcement")),
        RouteSpec::role("DELETE /api/v1/cms/announcements/{id}", Role::Admin)
            .ok(204, Contract::NoBody),
        RouteSpec::role(
            "POST /api/v1/cms/announcements/{id}/push-discord",
            Role::Admin,
        )
        .ok(200, announcement("DiscordPushOutcome")),
        RouteSpec::role("POST /api/v1/cms/uploads", Role::Admin).ok(
            201,
            Contract::schema("content-upload.schema.json", "UploadResponse"),
        ),
    ]
}

fn modpack_write(key: &'static str) -> RouteSpec {
    RouteSpec::role(key, Role::Admin)
}

fn modpacks() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/modpacks").ok(200, modpack("ModpackList")),
        RouteSpec::authenticated("GET /api/v1/modpacks/current").ok(200, modpack("Modpack")),
        modpack_write("POST /api/v1/modpacks").ok(201, modpack("Modpack")),
        modpack_write("PUT /api/v1/modpacks/{id}").ok(200, modpack("Modpack")),
        RouteSpec::role("POST /api/v1/modpacks/{id}/set-current", Role::Admin)
            .ok(200, modpack("Modpack")),
        RouteSpec::role("DELETE /api/v1/modpacks/{id}", Role::Admin).ok(204, Contract::NoBody),
    ]
}

fn vehicle_write(key: &'static str, success: u16) -> RouteSpec {
    RouteSpec::role(key, Role::Admin).ok(success, vehicle("Vehicle"))
}

fn vehicles() -> Vec<RouteSpec> {
    vec![
        RouteSpec::authenticated("GET /api/v1/vehicle-database").ok(200, vehicle("VehicleList")),
        RouteSpec::authenticated("GET /api/v1/vehicle-database/{id}").ok(200, vehicle("Vehicle")),
        vehicle_write("POST /api/v1/vehicle-database", 201),
        vehicle_write("PUT /api/v1/vehicle-database/{id}", 200),
        RouteSpec::role("PATCH /api/v1/vehicle-database/{id}", Role::Admin)
            .ok(200, vehicle("Vehicle")),
        RouteSpec::role("DELETE /api/v1/vehicle-database/{id}", Role::Admin)
            .ok(200, vehicle("Vehicle")),
    ]
}

fn wiki_pages() -> Vec<RouteSpec> {
    let _unknown_page = |actor| {
        Probe::new("unknown page", actor)
            .param("slug", "route-acceptance-missing-page")
            .expect(404)
    };
    vec![
        RouteSpec::authenticated("GET /api/v1/wiki").ok(200, wiki("WikiPageList")),
        RouteSpec::authenticated("GET /api/v1/wiki/{slug}").ok(200, wiki("WikiArticle")),
        RouteSpec::role("PUT /api/v1/wiki/{slug}", Role::Admin).ok(201, wiki("WikiArticle")),
        RouteSpec::authenticated("GET /api/v1/wiki/{slug}/revisions")
            .ok(200, wiki("WikiRevisionPage")),
        RouteSpec::authenticated("GET /api/v1/wiki/{slug}/revisions/{revision}")
            .ok(200, wiki("WikiRevision")),
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
