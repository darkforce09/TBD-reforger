//! Server Modpacks (/modpacks) — ported from pages/doctrine.tsx `ModpacksPage`. `<AuthGate>` → a
//! `GlassSplit`: a searchable modpack list (master) + an App-Store-style dossier (detail) with an
//! admin Read/Edit toggle. Fully client-MOCK-driven (`MOCK_MODPACKS`).
//!
//! **Gate scope (this slice):** the default render (search empty, first pack selected, read mode) —
//! the pack list (Active badge, mod-count/size preview) + the `ModpackDossier` (header meta + mod
//! list + launch/workshop actions) is byte-exact-verified. Search + the edit mode (`ModpackEditor`)
//! + the mutations are behavior — a follow-up.
#![allow(dead_code)]
use crate::split_pane::{GlassSplit, ListDetailItem, SidebarSearch};
use crate::ui::MaterialIcon;
use leptos::prelude::*;

struct Mod {
    name: &'static str,
    required: bool,
}
struct Pack {
    id: &'static str,
    name: &'static str,
    version: &'static str,
    total_size_bytes: i64,
    workshop_url: Option<&'static str>,
    is_current: bool,
    mods: &'static [Mod],
}

const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";
const WORKSHOP: &str = "https://reforger.armaplatform.com/workshop";

const MOCK_MODPACKS: &[Pack] = &[
    Pack {
        id: "core-modern",
        name: "Core Modern Expansion",
        version: "2.4.1",
        total_size_bytes: 18_897_856_102,
        workshop_url: Some(WORKSHOP),
        is_current: true,
        mods: &[
            Mod {
                name: "RHS: Status Quo",
                required: true,
            },
            Mod {
                name: "TFAR — Task Force Radio",
                required: true,
            },
            Mod {
                name: "ACE Reforged — Medical",
                required: true,
            },
            Mod {
                name: "Enhanced Movement Plus",
                required: false,
            },
            Mod {
                name: "WCS — Weapon Customization Suite",
                required: false,
            },
            Mod {
                name: "Everon Topographic Maps",
                required: false,
            },
        ],
    },
    Pack {
        id: "desert-storm",
        name: "Operation Desert Storm",
        version: "1.1.0",
        total_size_bytes: 12_348_030_976,
        workshop_url: Some(WORKSHOP),
        is_current: false,
        mods: &[
            Mod {
                name: "RHS: Gulf War Arsenal",
                required: true,
            },
            Mod {
                name: "TFAR — Task Force Radio",
                required: true,
            },
            Mod {
                name: "Sand & Heat Environment Pack",
                required: false,
            },
            Mod {
                name: "M1A1 Abrams Pack",
                required: false,
            },
            Mod {
                name: "Coalition Uniforms 1991",
                required: false,
            },
        ],
    },
    Pack {
        id: "cold-war-80s",
        name: "Cold War 1980s",
        version: "0.9.3",
        total_size_bytes: 9_663_676_416,
        workshop_url: None,
        is_current: false,
        mods: &[
            Mod {
                name: "RHS: GREF",
                required: true,
            },
            Mod {
                name: "TFAR — Task Force Radio",
                required: true,
            },
            Mod {
                name: "Spectrum Devices — Cold War Optics",
                required: false,
            },
            Mod {
                name: "Arland Winter Retexture",
                required: false,
            },
        ],
    },
];

/// `formatBytes` (lib/format.ts).
fn format_bytes(bytes: i64) -> String {
    if bytes < 1 {
        return "0 B".into();
    }
    let gb = bytes as f64 / 1024f64.powi(3);
    if gb >= 1.0 {
        return format!("{gb:.1} GB");
    }
    format!("{:.0} MB", bytes as f64 / 1024f64.powi(2))
}

#[component]
pub fn ModpacksPage() -> impl IntoView {
    let selected = &MOCK_MODPACKS[0];
    view! {
        <crate::ui::AuthGate>
            <GlassSplit
                master_width="18rem"
                master_header=master_header().into_any()
                master=pack_list(selected.id).into_any()
                detail=dossier(selected).into_any()
            />
        </crate::ui::AuthGate>
    }
}

fn master_header() -> impl IntoView {
    view! {
        <div class="w-full space-y-3">
            <h1 class="text-headline-sm tracking-wide text-on-surface uppercase">"Modpacks"</h1>
            <SidebarSearch placeholder="Search packs & mods…" />
        </div>
    }
}

fn pack_list(selected_id: &'static str) -> impl IntoView {
    MOCK_MODPACKS
        .iter()
        .map(move |p| {
            // trailing is an optional-prop AnyView (Leptos strips the Option) — use an empty () view
            // for the non-current case (React passes `undefined`, i.e. nothing).
            let trailing = if p.is_current {
                view! { <span class=BADGE_SUCCESS>"Active"</span> }.into_any()
            } else {
                ().into_any()
            };
            let preview = view! {
                <span class="font-mono text-on-surface-variant">
                    "v"
                    {p.version}
                    " · "
                    {p.mods.len() as i64}
                    " mods · "
                    {format_bytes(p.total_size_bytes)}
                </span>
            }
            .into_any();
            view! {
                <ListDetailItem
                    active=p.id == selected_id
                    title=view! { {p.name} }.into_any()
                    trailing=trailing
                    preview=preview
                />
            }
        })
        .collect_view()
}

fn dossier(p: &'static Pack) -> impl IntoView {
    view! {
        <div class="mx-auto flex min-h-full w-full max-w-3xl flex-col px-8 py-10">
            <header class="flex items-start justify-between gap-4">
                <div>
                    <h2 class="text-4xl font-bold tracking-tight text-on-surface">{p.name}</h2>
                    <div class="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 font-mono text-sm text-on-surface-variant">
                        <span>"v"{p.version}</span>
                        <span>
                            <span class="text-on-surface">{format_bytes(p.total_size_bytes)}</span>
                            " total"
                        </span>
                        <span>
                            <span class="text-on-surface">{p.mods.len() as i64}</span>
                            " mods included"
                        </span>
                    </div>
                </div>
                {read_edit_toggle()}
            </header>
            <ul class="mt-8">
                {p.mods
                    .iter()
                    .map(|m| {
                        view! {
                            <li class="flex items-center gap-4 rounded-xl border-b border-white/5 px-4 py-5 transition hover:bg-white/[0.02]">
                                <div class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-white/5 text-on-surface-variant">
                                    <MaterialIcon name="extension" />
                                </div>
                                <span class="flex-1 font-medium text-on-surface">{m.name}</span>
                                {m
                                    .required
                                    .then(|| {
                                        view! {
                                            <span class="rounded-md border border-tactical-yellow/20 bg-tactical-yellow/10 px-2.5 py-1 font-mono text-xs tracking-wider text-tactical-yellow">
                                                "[ REQUIRED ]"
                                            </span>
                                        }
                                    })}
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
            <div class="mt-10 pt-2">
                <button
                    type="button"
                    class="w-full rounded-full bg-action py-5 text-lg font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
                >
                    "[ Launch Game & Auto-Download ]"
                </button>
                {p
                    .workshop_url
                    .map(|url| {
                        view! {
                            <a
                                href=url
                                target="_blank"
                                rel="noreferrer"
                                class="mt-4 block text-center text-sm text-on-surface-variant transition hover:text-on-surface"
                            >
                                "View collection in Reforger Workshop ↗"
                            </a>
                        }
                    })}
            </div>
        </div>
    }
}

/// ReadEditToggle at mode="read" — the two `[ read ]` / `[ edit ]` buttons (React maps over
/// ['read','edit'], so the label is a bound value → a separate text node between the `[ ` / ` ]`).
fn read_edit_toggle() -> impl IntoView {
    view! {
        <div class="flex shrink-0 items-center rounded-full border border-white/10 bg-black/30 p-1 font-mono text-xs">
            {["read", "edit"]
                .into_iter()
                .map(|m| {
                    let class = if m == "read" {
                        "rounded-full px-4 py-1.5 tracking-wider uppercase transition bg-primary/20 text-primary shadow-[0_0_12px_rgba(173,198,255,0.25)]"
                    } else {
                        "rounded-full px-4 py-1.5 tracking-wider uppercase transition text-on-surface-variant hover:text-on-surface"
                    };
                    view! {
                        <button type="button" class=class>"[ "{m}" ]"</button>
                    }
                })
                .collect_view()}
        </div>
    }
}
