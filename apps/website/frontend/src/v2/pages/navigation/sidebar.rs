//! The sidebar, in both the shapes it takes: a permanent column and a slide-over drawer.
//!
//! **Role:** owns the brand block, the link list built from the navigation registry, and the
//! toggle button that opens the drawer on a narrow viewport.
//! **Position:** the first child of the chromed frame. Wide viewports render the permanent
//! column; narrow ones render the toggle, and the drawer reuses the same brand and link list.
//! **Signals & state:** reads the session store from context and the live pathname. The drawer's
//! open signal is owned by the frame and reaches the toggle as a prop.
//! **Invariants:** which links appear is decided per render from the viewer's tier under the
//! browse-mode rule, so a signed-out visitor sees the full set rather than an empty column, and
//! the privileged section is the only one gated above the baseline tier. At most one link is
//! active, by the frame's active-link rule.

use crate::v2::core::auth::{has_min_role, AuthStore, Role};
use crate::v2::core::ui::{cn, MaterialIcon};
use leptos::prelude::*;
use leptos_router::hooks::use_location;

use super::layout::is_active;
use super::nav_config::{NavItem, NAVIGATION};

/// The hamburger button that opens the narrow-viewport drawer.
///
/// Fixed to the top-left corner and hidden once the viewport is wide enough for the permanent
/// sidebar. Takes the drawer's open signal and toggles it.
#[component]
pub(crate) fn SidebarMobileToggle(open: RwSignal<bool>) -> impl IntoView {
    view! {
        <button
            type="button"
            class="fixed top-3 left-3 z-50 rounded-md bg-surface-container p-2 lg:hidden"
            aria-label="Open menu"
            on:click=move |_| open.update(|v| *v = !*v)
        >
            <MaterialIcon name="menu" />
        </button>
    }
}

/// The permanent sidebar, shown only once the viewport is wide enough for it.
///
/// A fixed-width column holding the brand block above the scrolling link list. On a narrow
/// viewport it renders nothing and the same two pieces appear in the drawer instead.
#[component]
pub(crate) fn Sidebar() -> impl IntoView {
    view! {
        <aside class="hidden h-screen w-80 shrink-0 flex-col bg-surface-container-low lg:flex">
            <SidebarBrand />
            <SidebarNav />
        </aside>
    }
}

/// The product name at the top of the sidebar and of the mobile drawer.
#[component]
pub(crate) fn SidebarBrand() -> impl IntoView {
    view! {
        <header class="relative flex h-16 shrink-0 items-center px-6">
            <div class="flex items-center gap-2">
                <span class="text-2xl font-bold tracking-wide text-primary">"TBD"</span>
                <span class="text-2xl font-bold tracking-wide text-on-surface">"Reforger"</span>
            </div>
        </header>
    }
}

/// The link list: every section of the registry the viewer may see, in registry order.
///
/// Drops a section whose tier the viewer does not clear, drops the individual links the viewer
/// does not clear, and drops a section left with no links. The privileged section carries its own
/// framing. Exactly one link may be active, marked both by class and by `aria-current`.
///
/// `on_nav` fires on any link click; the mobile drawer passes a callback that closes itself.
#[component]
pub(crate) fn SidebarNav(
    /// Invoked on any nav-link click; the mobile drawer closes itself through this.
    #[prop(optional)]
    on_nav: Option<Callback<()>>,
) -> impl IntoView {
    // The role and the live pathname are read inside one reactive closure, so the active
    // highlight follows navigation and the privileged section appears or disappears with the
    // session without a reload. The role is memoized from the profile, so a profile poll that
    // leaves it alone does not rebuild the list.
    let auth = expect_context::<AuthStore>();
    let role = Memo::new(move |_| auth.user.with(|user| user.as_ref().map(|u| u.role)));
    let pathname = use_location().pathname;
    view! {
        <nav class="custom-scrollbar flex-1 overflow-y-auto px-3 py-4">
            {move || {
                let user_role = role.get();
                let current = pathname.get();
                NAVIGATION
                .iter()
                .filter_map(|section| {
                    if section.admin && !has_min_role(user_role, Role::Admin) {
                        return None;
                    }
                    let items: Vec<&NavItem> =
                        section.items.iter().filter(|i| has_min_role(user_role, i.min_role)).collect();
                    if items.is_empty() {
                        return None;
                    }
                    let section_class = cn(&[
                        "mb-6",
                        if section.admin { "rounded-lg border border-red-500/20 bg-red-900/10 p-3" } else { "" },
                    ]);
                    let h3_class = cn(&[
                        "mb-2 px-2 text-xs font-bold tracking-widest uppercase",
                        if section.admin { "text-red-400" } else { "text-gray-500" },
                    ]);
                    Some(view! {
                        <div class=section_class>
                            <h3 class=h3_class>{section.title}</h3>
                            <ul class="space-y-1">
                                {items
                                    .into_iter()
                                    .map(|item| {
                                        let active = is_active(item.path, &current);
                                        // `text-label-md` is deliberately absent: the trailing
                                        // text-colour utility overrides it, so the link takes the
                                        // inherited body size either way.
                                        let a_class = cn(&[
                                            "flex items-center gap-3 rounded-md px-3 py-2.5 font-medium transition-colors",
                                            if active {
                                                "nav-item-active text-primary"
                                            } else {
                                                "text-on-surface-variant hover:bg-surface-variant/40 hover:text-on-surface"
                                            },
                                        ]);
                                        view! {
                                            <li>
                                                <a
                                                    href=item.path
                                                    class=a_class
                                                    aria-current=active.then_some("page")
                                                    on:click=move |_| {
                                                        if let Some(cb) = on_nav {
                                                            cb.run(());
                                                        }
                                                    }
                                                >
                                                    <MaterialIcon name=item.icon class="text-[22px]" />
                                                    {item.label}
                                                </a>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        </div>
                    })
                })
                .collect_view()
            }}
        </nav>
    }
}
