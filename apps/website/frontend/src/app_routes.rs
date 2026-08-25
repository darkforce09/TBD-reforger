//! leptos_router `<Routes>` — the render side of the route contract in router.rs. Every route
//! mounts its real page component (T-159.8+). The "*" catch-all (NotFoundPage) is the
//! `<Routes fallback>`. The chrome (Sidebar/TopNav) lives in AppLayout OUTSIDE `<Routes>`, so it
//! persists across navigation — `<Routes>` swaps only `<main>`. The path list mirrors router.rs
//! `ROUTES` (the S-routes gate's source of truth).
use crate::editor::library::mission_library::MissionLibraryPage;
use crate::pages::admin::approvals::MissionApprovalsPage;
use crate::pages::admin::audit::AuditLogsPage;
use crate::pages::admin::content::ContentManagerPage;
use crate::pages::admin::personnel::PersonnelRosterPage;
use crate::pages::admin::server_control::ServerControlPage;
use crate::pages::operations::event_schedule::EventSchedulePage;
use crate::pages::public::announcements::AnnouncementsPage;
use crate::pages::public::dashboard::DashboardPage;
use crate::pages::public::deployments::DeploymentsPage;
use crate::pages::public::leaderboards::LeaderboardsPage;
use crate::pages::public::modpacks::ModpacksPage;
use crate::pages::public::mortar::MortarCalculatorPage;
use crate::pages::public::server_intel::ServerIntelPage;
use crate::pages::public::settings::SettingsPage;
use crate::pages::public::vehicles::VehicleDatabasePage;
use leptos::prelude::*;
use leptos_router::components::{Route, Routes};
use leptos_router::path;

/// Login page (auth.tsx) — rendered bare (no chrome). A guest sees the sign-in card; the button
/// starts the real Discord OAuth flow (full-page redirect — the API 302s to Discord and lands
/// back on /auth/callback). T-172 H9.
#[component]
fn LoginPage() -> impl IntoView {
    view! {
        <div class="flex min-h-screen flex-col items-center justify-center bg-background p-6">
            <div class="w-full max-w-md rounded-xl border border-border-subtle bg-surface-container p-8 text-center">
                <h1 class="text-2xl font-bold">
                    <span class="text-primary">"TBD"</span>
                    " Reforger"
                </h1>
                <p class="mt-2 text-on-surface-variant">
                    "Sign in to register, deploy, and manage operations."
                </p>
                <button
                    type="button"
                    class="mt-6 w-full rounded-lg bg-primary py-3 font-medium text-on-primary"
                    on:click=move |_| {
                        if let Some(win) = web_sys::window() {
                            let _ = win.location().set_href("/api/v1/auth/discord/login");
                        }
                    }
                >
                    "Sign in with Discord"
                </button>
                <a href="/" class="mt-4 block text-sm text-on-surface-variant hover:text-primary">
                    "Continue browsing without signing in"
                </a>
            </div>
        </div>
    }
}

/// 404 (utility.tsx) — renders inside the chrome (the <Routes fallback>).
#[component]
fn NotFoundPage() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center py-24 text-center">
            <span class="text-6xl font-bold text-primary">"404"</span>
            <h1 class="mt-4 text-2xl font-bold">"Sector Not Found"</h1>
            <p class="mt-2 text-on-surface-variant">
                "The requested route does not exist in this AO."
            </p>
            <a href="/" class="mt-6 text-primary hover:underline">"Return to Dashboard"</a>
        </div>
    }
}

#[component]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <Routes fallback=|| view! { <NotFoundPage /> }>
            <Route path=path!("/login") view=LoginPage />
            <Route path=path!("/auth/callback") view=crate::core::auth::AuthCallbackPage />
            <Route path=path!("/") view=DashboardPage />
            <Route path=path!("/server-intel") view=ServerIntelPage />
            <Route path=path!("/announcements") view=AnnouncementsPage />
            <Route path=path!("/announcements/:id") view=AnnouncementsPage />
            <Route path=path!("/deployments") view=DeploymentsPage />
            <Route path=path!("/leaderboards") view=LeaderboardsPage />
            <Route path=path!("/missions") view=MissionLibraryPage />
            <Route path=path!("/missions/:id") view=crate::editor::library::mission_overview::MissionOverviewPage />
            <Route
                path=path!("/missions/:id/edit")
                view=crate::editor::mission_editor::MissionEditorPage
            />
            <Route path=path!("/events") view=EventSchedulePage />
            <Route path=path!("/events/:id") view=crate::pages::operations::event_hub::EventHubPage />
            <Route
                path=path!("/events/:id/missions/:emid/orbat")
                view=crate::pages::operations::orbat_selection::OrbatSelectionPage
            />
            <Route path=path!("/wiki") view=crate::pages::public::wiki::WikiPage />
            <Route path=path!("/wiki/:slug") view=crate::pages::public::wiki::WikiPage />
            <Route path=path!("/vehicles") view=VehicleDatabasePage />
            <Route path=path!("/modpacks") view=ModpacksPage />
            <Route path=path!("/tools/mortar") view=MortarCalculatorPage />
            <Route path=path!("/settings") view=SettingsPage />
            <Route path=path!("/admin/events") view=crate::pages::admin::event_manager::EventManagerPage />
            <Route path=path!("/admin/approvals") view=MissionApprovalsPage />
            <Route path=path!("/admin/server") view=ServerControlPage />
            <Route path=path!("/admin/personnel") view=PersonnelRosterPage />
            <Route path=path!("/admin/content") view=ContentManagerPage />
            <Route path=path!("/admin/audit") view=AuditLogsPage />
        </Routes>
    }
}
