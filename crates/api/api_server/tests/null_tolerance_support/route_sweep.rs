//! The table of GET routes the NULL blast is swept across, addressed through the ids of the
//! seeded graph.
//!
//! **Role:** [`route_sweep`] names every swept GET route with its registered template, a concrete
//! URI over the [`Seed`] rows and the caller that authenticates it.
//! **Position:** read by `tests/null_tolerance_reads.rs`, which drives each entry through the
//! router after [`super::database_fixtures::blast_nulls`] and compares the templates with the API
//! route tables; the fire-mission and the per-row entries come from the sibling modules.
//! **Signals & state:** none; pure functions.
//! **Invariants:** every template is spelled as the API route tables register it, so the coverage
//! check matches each swept route to exactly one registration.

use super::database_fixtures::Seed;
use super::fire_mission_fixtures::ballistics_catalog_sweep;
use super::{NULL_UID, SweepCaller};

/// `(route template as registered by the API route tables, concrete URI, how it authenticates)`.
///
/// The template is carried alongside the URI so
/// `every_get_route_is_swept_or_skipped_with_a_reason` can prove this table covers the whole
/// router instead of trusting that someone remembered to extend it.
pub(crate) fn route_sweep(s: &Seed) -> Vec<(&'static str, String, SweepCaller)> {
    let (m, pm, v) = (s.mission, s.pending_mission, s.version);
    let (e, em, a) = (s.event, s.event_mission, s.announcement);
    let (srv, fac, slug) = (s.server, s.faction, &s.wiki_slug);
    vec![
        ("/healthz", "/healthz".into(), SweepCaller::Member),
        // Swept rather than listed in ROUTE_SWEEP_SKIP: `/metrics` reads no model,
        // but it does run a live `SELECT 1` and read the pool, so the NULL blast is a free
        // check that the scrape path cannot 5xx. Gated by the observability bearer.
        ("/metrics", "/metrics".into(), SweepCaller::Observability),
        (
            "/dashboard",
            "/api/v1/dashboard".into(),
            SweepCaller::Member,
        ),
        ("/me", "/api/v1/me".into(), SweepCaller::Member),
        (
            "/matches/{matchId}/events",
            format!("/api/v1/matches/{}/events", s.match_id),
            SweepCaller::Member,
        ),
        (
            "/me/deployments",
            "/api/v1/me/deployments".into(),
            SweepCaller::Member,
        ),
        (
            "/me/leave-requests",
            "/api/v1/me/leave-requests".into(),
            SweepCaller::Member,
        ),
        (
            "/me/link/status",
            "/api/v1/me/link/status".into(),
            SweepCaller::Member,
        ),
        (
            "/members",
            "/api/v1/members?q=Null".into(),
            SweepCaller::Member,
        ),
        ("/missions", "/api/v1/missions".into(), SweepCaller::Member),
        (
            "/missions/{id}",
            format!("/api/v1/missions/{m}"),
            SweepCaller::Member,
        ),
        (
            "/missions/{id}/armory",
            format!("/api/v1/missions/{m}/armory"),
            SweepCaller::Member,
        ),
        (
            "/missions/{id}/export",
            format!("/api/v1/missions/{m}/export"),
            SweepCaller::Member,
        ),
        (
            "/missions/{id}/versions/{vid}",
            format!("/api/v1/missions/{m}/versions/{v}"),
            SweepCaller::Member,
        ),
        (
            "/missions/{id}/reviews",
            format!("/api/v1/missions/{m}/reviews"),
            SweepCaller::Member,
        ),
        (
            "/missions/{id}/artifacts/{artifact_id}",
            format!("/api/v1/missions/{m}/artifacts/{}", s.artifact),
            SweepCaller::Member,
        ),
        (
            "/missions/{id}/artifacts/{artifact_id}/document",
            format!("/api/v1/missions/{m}/artifacts/{}/document", s.artifact),
            SweepCaller::Member,
        ),
        (
            "/missions/{id}/artifacts/{artifact_id}/workspace",
            format!("/api/v1/missions/{m}/artifacts/{}/workspace", s.artifact),
            SweepCaller::Member,
        ),
        ("/events", "/api/v1/events".into(), SweepCaller::Member),
        (
            "/events/{id}",
            format!("/api/v1/events/{e}"),
            SweepCaller::Member,
        ),
        // Administrator access views read groups, rosters, slot policies and quota pools.
        (
            "/events/{id}/access",
            format!("/api/v1/events/{e}/access"),
            SweepCaller::Member,
        ),
        (
            "/events/{id}/access/participants",
            format!("/api/v1/events/{e}/access/participants"),
            SweepCaller::Member,
        ),
        (
            "/events/{id}/fire-missions",
            format!("/api/v1/events/{e}/fire-missions"),
            SweepCaller::Member,
        ),
        (
            "/event-missions/{emid}/orbat",
            format!("/api/v1/event-missions/{em}/orbat"),
            SweepCaller::Member,
        ),
        (
            "/announcements",
            "/api/v1/announcements".into(),
            SweepCaller::Member,
        ),
        (
            "/announcements/{id}",
            format!("/api/v1/announcements/{a}"),
            SweepCaller::Member,
        ),
        // Admin CMS master list (drafts + published); the public feed is `/announcements` above.
        (
            "/cms/announcements",
            "/api/v1/cms/announcements".into(),
            SweepCaller::Member,
        ),
        (
            "/approvals",
            "/api/v1/approvals".into(),
            SweepCaller::Member,
        ),
        (
            "/admin/users",
            "/api/v1/admin/users".into(),
            SweepCaller::Member,
        ),
        (
            "/admin/audit-logs",
            "/api/v1/admin/audit-logs".into(),
            SweepCaller::Member,
        ),
        (
            "/admin/audit-logs/export.csv",
            "/api/v1/admin/audit-logs/export.csv".into(),
            SweepCaller::Member,
        ),
        (
            "/admin/leave-requests",
            "/api/v1/admin/leave-requests".into(),
            SweepCaller::Member,
        ),
        // Swept, not skipped: the endpoint aggregates jsonb over mission_versions'
        // latest payloads, which is precisely the NULL-blast class this sweep protects.
        (
            "/admin/mission-default-overrides",
            "/api/v1/admin/mission-default-overrides".into(),
            SweepCaller::Member,
        ),
        (
            "/leaderboards",
            "/api/v1/leaderboards".into(),
            SweepCaller::Member,
        ),
        (
            "/users/{discordId}/stats",
            format!("/api/v1/users/{NULL_UID}/stats"),
            SweepCaller::Member,
        ),
        ("/servers", "/api/v1/servers".into(), SweepCaller::Member),
        (
            "/servers/{id}/status",
            format!("/api/v1/servers/{srv}/status"),
            SweepCaller::Member,
        ),
        (
            "/servers/{id}/credentials",
            format!("/api/v1/servers/{srv}/credentials"),
            SweepCaller::Member,
        ),
        (
            "/servers/{id}/commands",
            format!("/api/v1/servers/{srv}/commands"),
            SweepCaller::Member,
        ),
        (
            "/servers/{id}/commands/{commandId}",
            format!("/api/v1/servers/{srv}/commands/{}", s.command),
            SweepCaller::Member,
        ),
        ("/modpacks", "/api/v1/modpacks".into(), SweepCaller::Member),
        (
            "/modpacks/current",
            "/api/v1/modpacks/current".into(),
            SweepCaller::Member,
        ),
        ("/wiki", "/api/v1/wiki".into(), SweepCaller::Member),
        super::wiki_page_sweep(slug),
        super::wiki_revisions_sweep(slug),
        super::wiki_revision_sweep(slug),
        (
            "/vehicle-database",
            "/api/v1/vehicle-database".into(),
            SweepCaller::Member,
        ),
        super::vehicle_row_sweep(),
        ("/factions", "/api/v1/factions".into(), SweepCaller::Member),
        (
            "/factions/{id}",
            format!("/api/v1/factions/{fac}"),
            SweepCaller::Member,
        ),
        ("/registry", "/api/v1/registry".into(), SweepCaller::Member),
        (
            "/registry/compat",
            "/api/v1/registry/compat".into(),
            SweepCaller::Member,
        ),
        (
            "/game-runtime/events/{id}/roster",
            format!("/api/v1/game-runtime/events/{e}/roster"),
            SweepCaller::Machine,
        ),
        (
            "/game-runtime/deployment",
            "/api/v1/game-runtime/deployment".into(),
            SweepCaller::Machine,
        ),
        (
            "/game-runtime/artifacts/{artifactId}",
            format!("/api/v1/game-runtime/artifacts/{}", s.artifact),
            SweepCaller::Machine,
        ),
        (
            "/game-runtime/missions",
            "/api/v1/game-runtime/missions".into(),
            SweepCaller::Machine,
        ),
        (
            "/servers/{id}/deployments",
            format!("/api/v1/servers/{srv}/deployments"),
            SweepCaller::Member,
        ),
        (
            "/servers/{id}/deployments/{deploymentId}",
            format!("/api/v1/servers/{srv}/deployments/{}", s.deployment),
            SweepCaller::Member,
        ),
        (
            "/fleet/scenarios",
            "/api/v1/fleet/scenarios".into(),
            SweepCaller::Member,
        ),
        (
            "/missions/{id}",
            format!("/api/v1/missions/{pm}"),
            SweepCaller::Member,
        ),
    ]
    .into_iter()
    .chain(ballistics_catalog_sweep())
    .collect()
}
