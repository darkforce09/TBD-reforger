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

use crate::pages::account::login::LoginPage;
use crate::pages::account::settings::SettingsPage;
use crate::pages::administration::approvals::MissionApprovalsPage;
use crate::pages::administration::audit_logs::AuditLogsPage;
use crate::pages::administration::ballistics_catalogs::BallisticsCatalogsPage;
use crate::pages::administration::content_manager::ContentManagerPage;
use crate::pages::administration::personnel::PersonnelRosterPage;
use crate::pages::administration::server_control::ServerControlPage;
use crate::pages::command_center::announcements::AnnouncementsPage;
use crate::pages::command_center::dashboard::DashboardPage;
use crate::pages::command_center::server_intel::ServerIntelPage;
use crate::pages::doctrine_and_info::modpacks::ModpacksPage;
use crate::pages::doctrine_and_info::vehicles::VehicleDatabasePage;
use crate::pages::doctrine_and_info::wiki::WikiPage;
use crate::pages::field_tools::mortar::MortarCalculatorPage;
use crate::pages::mission_hub::library::MissionLibraryPage;
use crate::pages::operations::deployments::DeploymentsPage;
use crate::pages::operations::leaderboards::LeaderboardsPage;
use crate::pages::operations::schedule::EventSchedulePage;
use crate::shell::not_found::NotFoundPage;
use leptos::prelude::*;
use leptos_router::components::{Route, Routes};
use leptos_router::path;

/// Every route the application answers, plus the fallback.
#[component]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <Routes fallback=|| view! { <NotFoundPage /> }>
            <Route path=path!("/debug/data-viewer") view=crate::workspaces::debug::data_viewer::DataViewerPage />
            <Route path=path!("/login") view=LoginPage />
            <Route path=path!("/auth/callback") view=crate::pages::account::auth_callback::AuthCallbackPage />
            <Route path=path!("/") view=DashboardPage />
            <Route path=path!("/server-intel") view=ServerIntelPage />
            <Route path=path!("/announcements") view=AnnouncementsPage />
            <Route path=path!("/announcements/:id") view=AnnouncementsPage />
            <Route path=path!("/deployments") view=DeploymentsPage />
            <Route path=path!("/leaderboards") view=LeaderboardsPage />
            <Route path=path!("/missions") view=MissionLibraryPage />
            <Route path=path!("/missions/:id") view=crate::pages::mission_hub::overview::MissionOverviewPage />
            <Route
                path=path!("/missions/:id/edit")
                view=crate::workspaces::editor::mission_editor::MissionEditorPage
            />
            <Route
                path=path!("/missions/:id/artifacts/:artifact_id/workspace")
                view=crate::workspaces::editor::review_workspace::ReviewWorkspacePage
            />
            <Route path=path!("/events") view=EventSchedulePage />
            <Route path=path!("/events/:id") view=crate::pages::operations::event_detail::EventHubPage />
            <Route
                path=path!("/events/:id/missions/:emid/orbat")
                view=crate::pages::operations::orbat_selection::OrbatSelectionPage
            />
            <Route path=path!("/wiki") view=WikiPage />
            <Route path=path!("/wiki/:slug") view=WikiPage />
            <Route path=path!("/vehicles") view=VehicleDatabasePage />
            <Route path=path!("/modpacks") view=ModpacksPage />
            <Route path=path!("/tools/mortar") view=MortarCalculatorPage />
            <Route
                path=path!("/debug/building-viewer")
                view=crate::workspaces::debug::building_viewer::BuildingViewerPage
            />
            <Route
                path=path!("/debug/world-los")
                view=crate::workspaces::debug::world_los::WorldLosPage
            />
            <Route
                path=path!("/debug/ballistics-agreement")
                view=crate::workspaces::debug::ballistics_agreement::BallisticsAgreementPage
            />
            <Route path=path!("/settings") view=SettingsPage />
            <Route path=path!("/admin/events") view=crate::pages::administration::event_manager::EventManagerPage />
            <Route path=path!("/admin/approvals") view=MissionApprovalsPage />
            <Route path=path!("/admin/server") view=ServerControlPage />
            <Route path=path!("/admin/personnel") view=PersonnelRosterPage />
            <Route path=path!("/admin/content") view=ContentManagerPage />
            <Route path=path!("/admin/audit") view=AuditLogsPage />
            <Route path=path!("/admin/ballistics-catalogs") view=BallisticsCatalogsPage />
        </Routes>
    }
}
