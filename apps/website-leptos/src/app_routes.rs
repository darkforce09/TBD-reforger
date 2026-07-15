//! leptos_router `<Routes>` — the render side of the route contract in router.rs. Every route
//! currently renders [`PageStub`]; the real page components replace it at T-159.8+. The "*"
//! catch-all (NotFoundPage) is the `<Routes fallback>`. The chrome (Sidebar/TopNav) lives in
//! AppLayout OUTSIDE `<Routes>`, so it persists across navigation — `<Routes>` swaps only `<main>`.
//! The path list mirrors router.rs `ROUTES` (the S-routes gate's source of truth).
use crate::announcements::AnnouncementsPage;
use crate::approvals::MissionApprovalsPage;
use crate::audit::AuditLogsPage;
use crate::dashboard::DashboardPage;
use crate::deployments::DeploymentsPage;
use crate::leaderboards::LeaderboardsPage;
use crate::missions::MissionLibraryPage;
use crate::modpacks::ModpacksPage;
use crate::mortar::MortarCalculatorPage;
use crate::personnel::PersonnelRosterPage;
use crate::server_control::ServerControlPage;
use crate::server_intel::ServerIntelPage;
use crate::settings::SettingsPage;
use crate::ui::AuthGate;
use crate::vehicles::VehicleDatabasePage;
use leptos::prelude::*;
use leptos_router::components::{Route, Routes};
use leptos_router::path;

/// Placeholder for a not-yet-ported page. Sits inside `<main>`, which the chrome V-gate excludes,
/// so its content doesn't affect shell parity.
#[component]
fn PageStub() -> impl IntoView {
    view! { <div class="p-6 text-on-surface-variant">"(page)"</div> }
}

// Dashboard route → crate::dashboard::DashboardPage (AuthGate → /dashboard Resource → hero-bento).

/// Generic AuthGate-wrapped API page: a guest sees the sign-in CTA (the state the V gate checks);
/// the real page content + data replace PageStub as each page is ported (T-159.9+).
#[component]
fn ApiPage() -> impl IntoView {
    view! {
        <AuthGate>
            <PageStub />
        </AuthGate>
    }
}

/// Login page (auth.tsx) — rendered bare (no chrome). A guest sees the sign-in card; the
/// authed-user redirect to "/" + the Discord OAuth start are follow-ups (need the auth flow).
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

/// OAuth callback (auth.tsx) — rendered bare. A fresh load with no fragment shows the "no_session"
/// error card (the guest state the V gate checks); the token/handshake path is a follow-up.
#[component]
fn CallbackPage() -> impl IntoView {
    view! {
        <div class="flex min-h-screen items-center justify-center bg-background p-6">
            <div class="max-w-md rounded-xl border border-border-subtle bg-surface-container p-8 text-center">
                <h1 class="text-xl font-semibold text-error">"Sign-in failed"</h1>
                <p class="mt-2 text-sm text-on-surface-variant">
                    "No sign-in details were found. Please start from the login page."
                </p>
                <a href="/login" class="mt-4 inline-block text-primary hover:underline">
                    "Back to login"
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
            <Route path=path!("/auth/callback") view=CallbackPage />
            <Route path=path!("/") view=DashboardPage />
            <Route path=path!("/server-intel") view=ServerIntelPage />
            <Route path=path!("/announcements") view=AnnouncementsPage />
            <Route path=path!("/deployments") view=DeploymentsPage />
            <Route path=path!("/leaderboards") view=LeaderboardsPage />
            <Route path=path!("/missions") view=MissionLibraryPage />
            <Route path=path!("/missions/:id") view=ApiPage />
            <Route path=path!("/missions/:id/edit") view=PageStub />
            <Route path=path!("/events") view=ApiPage />
            <Route path=path!("/events/:id") view=ApiPage />
            <Route path=path!("/events/:id/missions/:emid/orbat") view=ApiPage />
            <Route path=path!("/wiki") view=ApiPage />
            <Route path=path!("/wiki/:slug") view=ApiPage />
            <Route path=path!("/vehicles") view=VehicleDatabasePage />
            <Route path=path!("/modpacks") view=ModpacksPage />
            <Route path=path!("/tools/mortar") view=MortarCalculatorPage />
            <Route path=path!("/settings") view=SettingsPage />
            <Route path=path!("/admin/events") view=ApiPage />
            <Route path=path!("/admin/approvals") view=MissionApprovalsPage />
            <Route path=path!("/admin/server") view=ServerControlPage />
            <Route path=path!("/admin/personnel") view=PersonnelRosterPage />
            <Route path=path!("/admin/content") view=ApiPage />
            <Route path=path!("/admin/audit") view=AuditLogsPage />
        </Routes>
    }
}
