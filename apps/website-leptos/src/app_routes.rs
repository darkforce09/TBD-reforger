//! leptos_router `<Routes>` — the render side of the route contract in router.rs. Every route
//! currently renders [`PageStub`]; the real page components replace it at T-159.8+. The "*"
//! catch-all (NotFoundPage) is the `<Routes fallback>`. The chrome (Sidebar/TopNav) lives in
//! AppLayout OUTSIDE `<Routes>`, so it persists across navigation — `<Routes>` swaps only `<main>`.
//! The path list mirrors router.rs `ROUTES` (the S-routes gate's source of truth).
use crate::ui::AuthGate;
use leptos::prelude::*;
use leptos_router::components::{Route, Routes};
use leptos_router::path;

/// Placeholder for a not-yet-ported page. Sits inside `<main>`, which the chrome V-gate excludes,
/// so its content doesn't affect shell parity.
#[component]
fn PageStub() -> impl IntoView {
    view! { <div class="p-6 text-on-surface-variant">"(page)"</div> }
}

/// Dashboard route. For a guest, AuthGate renders the sign-in CTA (the state the V gate checks now);
/// the authed hero-bento content + the dashboard DTO/data land at T-159.8.
#[component]
fn DashboardPage() -> impl IntoView {
    view! {
        <AuthGate>
            <div>"(dashboard)"</div>
        </AuthGate>
    }
}

#[component]
pub fn AppRoutes() -> impl IntoView {
    view! {
        <Routes fallback=|| view! { <PageStub /> }>
            <Route path=path!("/login") view=PageStub />
            <Route path=path!("/auth/callback") view=PageStub />
            <Route path=path!("/") view=DashboardPage />
            <Route path=path!("/server-intel") view=PageStub />
            <Route path=path!("/announcements") view=PageStub />
            <Route path=path!("/deployments") view=PageStub />
            <Route path=path!("/leaderboards") view=PageStub />
            <Route path=path!("/missions") view=PageStub />
            <Route path=path!("/missions/:id") view=PageStub />
            <Route path=path!("/missions/:id/edit") view=PageStub />
            <Route path=path!("/events") view=PageStub />
            <Route path=path!("/events/:id") view=PageStub />
            <Route path=path!("/events/:id/missions/:emid/orbat") view=PageStub />
            <Route path=path!("/wiki") view=PageStub />
            <Route path=path!("/wiki/:slug") view=PageStub />
            <Route path=path!("/vehicles") view=PageStub />
            <Route path=path!("/modpacks") view=PageStub />
            <Route path=path!("/tools/mortar") view=PageStub />
            <Route path=path!("/settings") view=PageStub />
            <Route path=path!("/admin/events") view=PageStub />
            <Route path=path!("/admin/approvals") view=PageStub />
            <Route path=path!("/admin/server") view=PageStub />
            <Route path=path!("/admin/personnel") view=PageStub />
            <Route path=path!("/admin/content") view=PageStub />
            <Route path=path!("/admin/audit") view=PageStub />
        </Routes>
    }
}
