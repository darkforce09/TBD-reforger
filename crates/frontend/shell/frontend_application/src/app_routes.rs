//! The router's route table, in render form.
//!
//! **Role:** binds every path to the component that renders it, and names the fallback used when
//! none match.
//! **Position:** rendered by the frame — inside `<main>` for a chromed route, and directly for
//! the bare and chromeless ones. The chrome lives outside this component, so navigation swaps
//! only what is declared here.
//! **Signals & state:** none. Each route component owns its own.
//! **Invariants:** this list mirrors the route table in `router.rs`, which is the contract the
//! layout flags and the required tiers are read from; a path added here without a row there
//! renders with default layout and no tier requirement.

use crate::shell::not_found::NotFoundPage;
use account_pages::login::LoginPage;
use account_pages::settings::SettingsPage;
use administration_pages::approvals::MissionApprovalsPage;
use administration_pages::audit_logs::AuditLogsPage;
use administration_pages::ballistics_catalogs::BallisticsCatalogsPage;
use administration_pages::content_manager::ContentManagerPage;
use administration_pages::personnel::PersonnelRosterPage;
use administration_pages::server_control::ServerControlPage;
use command_center_pages::announcements::AnnouncementsPage;
use command_center_pages::dashboard::DashboardPage;
use command_center_pages::server_intel::ServerIntelPage;
use doctrine_pages::modpacks::ModpacksPage;
use doctrine_pages::vehicles::VehicleDatabasePage;
use doctrine_pages::wiki::WikiPage;
use field_tools_pages::mortar::MortarCalculatorPage;
use leptos::prelude::*;
use leptos_router::components::{Route, Routes};
use leptos_router::path;
use mission_hub_pages::library::MissionLibraryPage;
use operations_pages::deployments::DeploymentsPage;
use operations_pages::leaderboards::LeaderboardsPage;
use operations_pages::schedule::EventSchedulePage;

/// Every route the application answers, plus the fallback.
#[component]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <Routes fallback=|| view! { <NotFoundPage /> }>
            <Route path=path!("/debug/data-viewer") view=debug_benches::data_viewer::DataViewerPage />
            <Route path=path!("/login") view=LoginPage />
            <Route path=path!("/auth/callback") view=account_pages::auth_callback::AuthCallbackPage />
            <Route path=path!("/") view=DashboardPage />
            <Route path=path!("/server-intel") view=ServerIntelPage />
            <Route path=path!("/announcements") view=AnnouncementsPage />
            <Route path=path!("/announcements/:id") view=AnnouncementsPage />
            <Route path=path!("/deployments") view=DeploymentsPage />
            <Route path=path!("/leaderboards") view=LeaderboardsPage />
            <Route path=path!("/missions") view=MissionLibraryPage />
            <Route path=path!("/missions/:id") view=mission_hub_pages::overview::MissionOverviewPage />
            <Route
                path=path!("/missions/:id/edit")
                view=mission_creator_workspace::MissionEditorPage
            />
            <Route
                path=path!("/missions/:id/artifacts/:artifact_id/workspace")
                view=mission_creator_workspace::ReviewWorkspacePage
            />
            <Route path=path!("/events") view=EventSchedulePage />
            <Route path=path!("/events/:id") view=operations_pages::event_detail::EventHubPage />
            <Route
                path=path!("/events/:id/missions/:emid/orbat")
                view=operations_pages::orbat_selection::OrbatSelectionPage
            />
            <Route path=path!("/wiki") view=WikiPage />
            <Route path=path!("/wiki/:slug") view=WikiPage />
            <Route path=path!("/vehicles") view=VehicleDatabasePage />
            <Route path=path!("/modpacks") view=ModpacksPage />
            <Route path=path!("/tools/mortar") view=MortarCalculatorPage />
            <Route
                path=path!("/debug/building-viewer")
                view=debug_benches::building_viewer::BuildingViewerPage
            />
            <Route
                path=path!("/debug/world-los")
                view=debug_benches::world_los::WorldLosPage
            />
            <Route
                path=path!("/debug/ballistics-agreement")
                view=debug_benches::ballistics_agreement::BallisticsAgreementPage
            />
            <Route path=path!("/settings") view=SettingsPage />
            <Route path=path!("/admin/events") view=administration_pages::event_manager::EventManagerPage />
            <Route path=path!("/admin/approvals") view=MissionApprovalsPage />
            <Route path=path!("/admin/server") view=ServerControlPage />
            <Route path=path!("/admin/personnel") view=PersonnelRosterPage />
            <Route path=path!("/admin/content") view=ContentManagerPage />
            <Route path=path!("/admin/audit") view=AuditLogsPage />
            <Route path=path!("/admin/ballistics-catalogs") view=BallisticsCatalogsPage />
        </Routes>
    }
}
