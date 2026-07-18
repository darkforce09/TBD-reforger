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

/// Session-local admin edits of a pack (React ModpackEditor parity — in-memory until the
/// modpacks backend ships). Keyed by pack id in `overrides`.
#[derive(Clone, PartialEq)]
struct ModEdit {
    name: String,
    required: bool,
}

#[derive(Clone, PartialEq)]
struct PackEdit {
    name: String,
    mods: Vec<ModEdit>,
}

impl PackEdit {
    fn from_pack(p: &Pack) -> Self {
        Self {
            name: p.name.to_string(),
            mods: p
                .mods
                .iter()
                .map(|m| ModEdit {
                    name: m.name.to_string(),
                    required: m.required,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MpMode {
    Read,
    Edit,
}

type PackOverrides = RwSignal<std::collections::HashMap<&'static str, PackEdit>>;

#[component]
pub fn ModpacksPage() -> impl IntoView {
    // Live selection + search (T-172 A6) + the READ/EDIT pack editor (T-172 H8 sweep).
    let selected_id = RwSignal::new(MOCK_MODPACKS[0].id);
    let search = RwSignal::new(String::new());
    let mode = RwSignal::new(MpMode::Read);
    let overrides: PackOverrides = RwSignal::new(std::collections::HashMap::new());
    Effect::new(move |prev: Option<&'static str>| {
        let id = selected_id.get();
        if prev.is_some_and(|p| p != id) {
            mode.set(MpMode::Read);
        }
        id
    });
    view! {
        <crate::ui::AuthGate>
            <GlassSplit
                master_width="18rem"
                master_header=master_header(search).into_any()
                master=view! { {move || pack_list(selected_id, &search.get(), overrides)} }
                    .into_any()
                detail=view! {
                    {move || {
                        let p = MOCK_MODPACKS
                            .iter()
                            .find(|p| p.id == selected_id.get())
                            .unwrap_or(&MOCK_MODPACKS[0]);
                        if mode.get() == MpMode::Edit {
                            editor(p, mode, overrides).into_any()
                        } else {
                            dossier(p, mode, overrides).into_any()
                        }
                    }}
                }
                    .into_any()
            />
        </crate::ui::AuthGate>
    }
}

fn master_header(search: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="w-full space-y-3">
            <h1 class="text-headline-sm tracking-wide text-on-surface uppercase">"Modpacks"</h1>
            <SidebarSearch placeholder="Search packs & mods…" bind=search />
        </div>
    }
}

fn pack_list(
    selected_id: RwSignal<&'static str>,
    query: &str,
    overrides: PackOverrides,
) -> impl IntoView {
    let query = query.to_string();
    MOCK_MODPACKS
        .iter()
        .map(move |p| {
            // Session-local saves win over the static mock (name + mod list).
            let data = overrides
                .with(|o| o.get(p.id).cloned())
                .unwrap_or_else(|| PackEdit::from_pack(p));
            (p, data)
        })
        .filter({
            let query = query.clone();
            move |(_, data)| {
                // React filters pack name + contained mod names.
                let mods: String = data
                    .mods
                    .iter()
                    .map(|m| m.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                crate::split_pane::search_matches(&query, &format!("{} {mods}", data.name))
            }
        })
        .map(move |(p, data)| {
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
                    {data.mods.len() as i64}
                    " mods · "
                    {format_bytes(p.total_size_bytes)}
                </span>
            }
            .into_any();
            let id = p.id;
            view! {
                <ListDetailItem
                    active=p.id == selected_id.get()
                    title=view! { {data.name.clone()} }.into_any()
                    trailing=trailing
                    preview=preview
                    on_click=Callback::new(move |()| selected_id.set(id))
                />
            }
        })
        .collect_view()
}

fn dossier(p: &'static Pack, mode: RwSignal<MpMode>, overrides: PackOverrides) -> impl IntoView {
    // Session-local override (saved from the editor) wins over the static mock.
    let data = overrides
        .with(|o| o.get(p.id).cloned())
        .unwrap_or_else(|| PackEdit::from_pack(p));
    let mod_count = data.mods.len() as i64;
    let toasts = crate::toast::use_toasts();
    view! {
        <div class="mx-auto flex min-h-full w-full max-w-3xl flex-col px-8 py-10">
            <header class="flex items-start justify-between gap-4">
                <div>
                    <h2 class="text-4xl font-bold tracking-tight text-on-surface">{data.name.clone()}</h2>
                    <div class="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 font-mono text-sm text-on-surface-variant">
                        <span>"v"{p.version}</span>
                        <span>
                            <span class="text-on-surface">{format_bytes(p.total_size_bytes)}</span>
                            " total"
                        </span>
                        <span>
                            <span class="text-on-surface">{mod_count}</span>
                            " mods included"
                        </span>
                    </div>
                </div>
                {read_edit_toggle(mode)}
            </header>
            <ul class="mt-8">
                {data
                    .mods
                    .into_iter()
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
                    on:click=move |_| toasts.message("Launch requires the Reforger client")
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

/// The `[ read ]` / `[ edit ]` pill toggle — live (T-172 H8 sweep). React maps over
/// ['read','edit'], so the label is a bound value → a separate text node between the `[ ` / ` ]`;
/// class strings per state match the gate-era static render byte-for-byte.
fn read_edit_toggle(mode: RwSignal<MpMode>) -> impl IntoView {
    view! {
        <div class="flex shrink-0 items-center rounded-full border border-white/10 bg-black/30 p-1 font-mono text-xs">
            {[("read", MpMode::Read), ("edit", MpMode::Edit)]
                .into_iter()
                .map(|(m, target)| {
                    let class = move || {
                        if mode.get() == target {
                            "rounded-full px-4 py-1.5 tracking-wider uppercase transition bg-primary/20 text-primary shadow-[0_0_12px_rgba(173,198,255,0.25)]"
                        } else {
                            "rounded-full px-4 py-1.5 tracking-wider uppercase transition text-on-surface-variant hover:text-on-surface"
                        }
                    };
                    view! {
                        <button type="button" class=class on:click=move |_| mode.set(target)>
                            "[ "{m}" ]"
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// Admin edit view (React ModpackEditor parity): rename the pack, add/remove/flag mods —
/// session-local (`overrides`) until the modpacks backend ships.
fn editor(p: &'static Pack, mode: RwSignal<MpMode>, overrides: PackOverrides) -> impl IntoView {
    let initial = overrides
        .with_untracked(|o| o.get(p.id).cloned())
        .unwrap_or_else(|| PackEdit::from_pack(p));
    let name = RwSignal::new(initial.name.clone());
    let mods = RwSignal::new(initial.mods);
    let new_mod = RwSignal::new(String::new());
    let pack_id = p.id;
    let fallback_name = p.name;

    let add_mod = move || {
        let trimmed = new_mod.get_untracked().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        mods.update(|m| {
            m.push(ModEdit {
                name: trimmed,
                required: false,
            })
        });
        new_mod.set(String::new());
    };
    let toasts = crate::toast::use_toasts();
    let save = move |_| {
        let n = name.get_untracked().trim().to_string();
        let final_name = if n.is_empty() {
            fallback_name.to_string()
        } else {
            n
        };
        toasts.success(format!("Saved \"{final_name}\""));
        overrides.update(|o| {
            o.insert(
                pack_id,
                PackEdit {
                    name: final_name.clone(),
                    mods: mods.get_untracked(),
                },
            );
        });
        mode.set(MpMode::Read);
    };

    view! {
        <div class="mx-auto flex min-h-full w-full max-w-3xl flex-col px-8 py-10">
            <header class="flex items-start justify-between gap-4">
                <div class="flex-1">
                    <label class="mb-1 block font-mono text-xs tracking-wider text-on-surface-variant uppercase">
                        "Modpack name"
                    </label>
                    <input
                        prop:value=initial.name
                        on:input=move |ev| name.set(event_target_value(&ev))
                        class="w-full rounded-xl border border-white/10 bg-black/30 px-4 py-3 text-2xl font-bold tracking-tight text-on-surface focus:border-primary/50 focus:outline-none"
                    />
                </div>
                {read_edit_toggle(mode)}
            </header>
            <ul class="mt-8">
                {move || {
                    let list = mods.get();
                    if list.is_empty() {
                        return view! {
                            <li class="px-4 py-6 text-center text-sm text-on-surface-variant">
                                "No mods yet — add one below."
                            </li>
                        }
                            .into_any();
                    }
                    list.into_iter()
                        .enumerate()
                        .map(|(i, m)| {
                            let req_class = if m.required {
                                "rounded-md border px-2.5 py-1 font-mono text-xs tracking-wider transition border-tactical-yellow/20 bg-tactical-yellow/10 text-tactical-yellow"
                            } else {
                                "rounded-md border px-2.5 py-1 font-mono text-xs tracking-wider transition border-white/10 text-on-surface-variant hover:bg-white/5"
                            };
                            let remove_label = format!("Remove {}", m.name);
                            view! {
                                <li class="flex items-center gap-3 rounded-xl border-b border-white/5 px-4 py-4">
                                    <MaterialIcon
                                        name="drag_indicator"
                                        class="text-on-surface-variant/50"
                                    />
                                    <span class="flex-1 font-medium text-on-surface">{m.name}</span>
                                    <button
                                        type="button"
                                        class=req_class
                                        on:click=move |_| {
                                            mods.update(|list| {
                                                if let Some(entry) = list.get_mut(i) {
                                                    entry.required = !entry.required;
                                                }
                                            })
                                        }
                                    >
                                        "[ REQUIRED ]"
                                    </button>
                                    <button
                                        type="button"
                                        aria-label=remove_label
                                        class="flex size-8 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-error-alert/10 hover:text-error-alert"
                                        on:click=move |_| {
                                            mods.update(|list| {
                                                list.remove(i);
                                            })
                                        }
                                    >
                                        <MaterialIcon name="close" />
                                    </button>
                                </li>
                            }
                                .into_any()
                        })
                        .collect_view()
                        .into_any()
                }}
            </ul>
            <div class="mt-4 flex gap-2">
                <input
                    prop:value=move || new_mod.get()
                    on:input=move |ev| new_mod.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            add_mod();
                        }
                    }
                    placeholder="Add a mod (e.g. ACE Reforged)…"
                    class="flex-1 rounded-xl border border-white/10 bg-black/30 px-4 py-3 text-sm text-on-surface placeholder:text-on-surface-variant/60 focus:border-primary/50 focus:outline-none"
                />
                <button
                    type="button"
                    on:click=move |_| add_mod()
                    class="flex items-center gap-1.5 rounded-xl border border-white/10 px-4 text-sm font-medium text-on-surface transition hover:bg-white/5"
                >
                    <MaterialIcon name="add" class="text-base" />
                    "Add"
                </button>
            </div>
            <div class="mt-10 flex gap-3 pt-2">
                <button
                    type="button"
                    on:click=save
                    class="flex-1 rounded-full bg-action py-4 text-lg font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
                >
                    "Save Changes"
                </button>
                <button
                    type="button"
                    on:click=move |_| mode.set(MpMode::Read)
                    class="rounded-full border border-white/10 px-8 text-base font-medium text-on-surface-variant transition hover:bg-white/5 hover:text-on-surface"
                >
                    "Cancel"
                </button>
            </div>
        </div>
    }
}
