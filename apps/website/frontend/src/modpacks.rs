//! Server Modpacks (/modpacks) — load from `GET /modpacks`, admin Save → `PUT /modpacks/:id`
//! (T-271). Create / set-current / delete hit the matching write routes. No MOCK_MODPACKS.
#![allow(dead_code)]
use crate::dto::{DataEnvelope, ModpackDto};
use crate::nav::Role;
use crate::split_pane::{GlassSplit, ListDetailItem, SidebarSearch};
use crate::ui::MaterialIcon;
use leptos::prelude::*;
use serde_json::{json, Value};

const BADGE_SUCCESS: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-success/30 bg-success/15 text-success";

/// Session-local draft while editing (committed via PUT on Save).
#[derive(Clone, PartialEq)]
struct ModEdit {
    name: String,
    required: bool,
    workshop_id: String,
    mod_guid: String,
    version: String,
}

#[derive(Clone, PartialEq)]
struct PackEdit {
    name: String,
    version: String,
    total_size_bytes: i64,
    workshop_url: String,
    is_current: bool,
    mods: Vec<ModEdit>,
}

impl PackEdit {
    fn from_dto(p: &ModpackDto) -> Self {
        Self {
            name: p.modpack.name.clone(),
            version: p.modpack.version.clone(),
            total_size_bytes: p.modpack.total_size_bytes,
            workshop_url: p.modpack.workshop_url.clone(),
            is_current: p.modpack.is_current,
            mods: p
                .mods
                .iter()
                .map(|m| ModEdit {
                    name: vstr(m, "name"),
                    required: m
                        .get("is_key_dependency")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    workshop_id: vstr(m, "workshop_id"),
                    mod_guid: vstr(m, "mod_guid"),
                    version: vstr(m, "version"),
                })
                .collect(),
        }
    }

    fn to_put_body(&self) -> Value {
        json!({
            "name": self.name,
            "version": self.version,
            "total_size_bytes": self.total_size_bytes,
            "workshop_url": self.workshop_url,
            "is_current": self.is_current,
            "mods": self.mods.iter().enumerate().map(|(i, m)| json!({
                "name": m.name,
                "is_key_dependency": m.required,
                "sort_order": i as i64,
                "workshop_id": m.workshop_id,
                "mod_guid": m.mod_guid,
                "version": m.version,
            })).collect::<Vec<_>>(),
        })
    }
}

fn vstr(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum MpMode {
    Read,
    Edit,
}

#[component]
pub fn ModpacksPage() -> impl IntoView {
    view! {
        <crate::ui::AuthGate>
            <ModpacksInner />
        </crate::ui::AuthGate>
    }
}

#[component]
fn ModpacksInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let packs = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<DataEnvelope<ModpackDto>>(store, "/modpacks")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<DataEnvelope<ModpackDto>>
        }
    });
    view! {
        <Suspense fallback=move || {
            view! { <p class="px-8 py-10 text-on-surface-variant">"Loading modpacks…"</p> }
        }>
            {move || {
                packs.get().map(|opt| match opt {
                    Some(env) => modpacks_board(env.data, packs).into_any(),
                    None => {
                        view! { <p class="px-8 py-10 text-error">"Failed to load modpacks."</p> }
                            .into_any()
                    }
                })
            }}
        </Suspense>
    }
}

fn modpacks_board(
    list: Vec<ModpackDto>,
    packs_res: LocalResource<Option<DataEnvelope<ModpackDto>>>,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let is_admin = store.has_min_role(Role::Admin);
    let selected_id = RwSignal::new(
        list.first()
            .map(|p| p.modpack.id.clone())
            .unwrap_or_default(),
    );
    let search = RwSignal::new(String::new());
    let mode = RwSignal::new(MpMode::Read);
    let toasts = crate::toast::use_toasts();
    let create_busy = RwSignal::new(false);

    Effect::new(move |prev: Option<String>| {
        let id = selected_id.get();
        if prev.as_ref().is_some_and(|p| p != &id) {
            mode.set(MpMode::Read);
        }
        id
    });

    let list_master = list.clone();
    let list_detail = list;

    view! {
        <GlassSplit
            master_width="18rem"
            master_header=master_header(search, is_admin, create_busy, packs_res, selected_id, toasts)
                .into_any()
            master=view! {
                {move || {
                    pack_list(
                        &list_master,
                        selected_id,
                        &search.get(),
                    )
                }}
            }
                .into_any()
            detail=view! {
                {move || {
                    let id = selected_id.get();
                    let Some(p) = list_detail.iter().find(|p| p.modpack.id == id) else {
                        return view! {
                            <p class="px-8 py-10 text-on-surface-variant">
                                "No modpack selected."
                            </p>
                        }
                            .into_any();
                    };
                    if mode.get() == MpMode::Edit && is_admin {
                        editor(p, mode, packs_res, toasts).into_any()
                    } else {
                        dossier(p, mode, is_admin, packs_res, toasts).into_any()
                    }
                }}
            }
                .into_any()
        />
    }
}

fn master_header(
    search: RwSignal<String>,
    is_admin: bool,
    create_busy: RwSignal<bool>,
    packs_res: LocalResource<Option<DataEnvelope<ModpackDto>>>,
    selected_id: RwSignal<String>,
    toasts: crate::toast::Toasts,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    view! {
        <div class="w-full space-y-3">
            <div class="flex items-center justify-between gap-2">
                <h1 class="text-headline-sm tracking-wide text-on-surface uppercase">"Modpacks"</h1>
                {is_admin.then(|| {
                    view! {
                        <button
                            type="button"
                            disabled=move || create_busy.get()
                            class="rounded-full border border-white/10 px-3 py-1 font-mono text-[11px] tracking-wider text-on-surface-variant uppercase transition hover:bg-white/5 disabled:opacity-50"
                            on:click=move |_| {
                                if create_busy.get_untracked() {
                                    return;
                                }
                                create_busy.set(true);
                                let body = json!({
                                    "name": "New Modpack",
                                    "version": "0.1.0",
                                    "total_size_bytes": 0,
                                    "workshop_url": "",
                                    "is_current": false,
                                    "mods": [],
                                });
                                #[cfg(target_arch = "wasm32")]
                                {
                                    leptos::task::spawn_local(async move {
                                        match crate::client::api_post::<ModpackDto>(
                                            store, "/modpacks", body,
                                        )
                                        .await
                                        {
                                            Ok(created) => {
                                                selected_id.set(created.modpack.id.clone());
                                                toasts.success(format!(
                                                    "Created \"{}\"",
                                                    created.modpack.name
                                                ));
                                                packs_res.refetch();
                                            }
                                            Err(e) => {
                                                toasts.error(crate::client::api_error_message(
                                                    &e,
                                                    "Failed to create modpack",
                                                ));
                                            }
                                        }
                                        create_busy.set(false);
                                    });
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    let _ = (store, body, packs_res, selected_id, toasts);
                                    create_busy.set(false);
                                }
                            }
                        >
                            {move || if create_busy.get() { "…" } else { "+ New" }}
                        </button>
                    }
                })}
            </div>
            <SidebarSearch placeholder="Search packs & mods…" bind=search />
        </div>
    }
}

fn pack_list(packs: &[ModpackDto], selected_id: RwSignal<String>, query: &str) -> impl IntoView {
    let query = query.to_string();
    packs
        .iter()
        .filter(|p| {
            let mods: String = p
                .mods
                .iter()
                .map(|m| vstr(m, "name"))
                .collect::<Vec<_>>()
                .join(" ");
            crate::split_pane::search_matches(&query, &format!("{} {mods}", p.modpack.name))
        })
        .cloned()
        .map(|p| {
            let trailing = if p.modpack.is_current {
                view! { <span class=BADGE_SUCCESS>"Active"</span> }.into_any()
            } else {
                ().into_any()
            };
            let preview = view! {
                <span class="font-mono text-on-surface-variant">
                    "v"
                    {p.modpack.version.clone()}
                    " · "
                    {p.mods.len() as i64}
                    " mods · "
                    {format_bytes(p.modpack.total_size_bytes)}
                </span>
            }
            .into_any();
            let id = p.modpack.id.clone();
            let id_click = id.clone();
            let title = p.modpack.name.clone();
            view! {
                <ListDetailItem
                    active=id == selected_id.get()
                    title=view! { {title} }.into_any()
                    trailing=trailing
                    preview=preview
                    on_click=Callback::new(move |()| selected_id.set(id_click.clone()))
                />
            }
        })
        .collect_view()
}

fn dossier(
    p: &ModpackDto,
    mode: RwSignal<MpMode>,
    is_admin: bool,
    packs_res: LocalResource<Option<DataEnvelope<ModpackDto>>>,
    toasts: crate::toast::Toasts,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let data = PackEdit::from_dto(p);
    let mod_count = data.mods.len() as i64;
    let pack_id = p.modpack.id.clone();
    let workshop_url = p.modpack.workshop_url.clone();
    let version = p.modpack.version.clone();
    let size = p.modpack.total_size_bytes;
    let is_current = p.modpack.is_current;
    let set_busy = RwSignal::new(false);
    let del_busy = RwSignal::new(false);

    view! {
        <div class="mx-auto flex min-h-full w-full max-w-3xl flex-col px-8 py-10">
            <header class="flex items-start justify-between gap-4">
                <div>
                    <h2 class="text-4xl font-bold tracking-tight text-on-surface">{data.name.clone()}</h2>
                    <div class="mt-3 flex flex-wrap items-center gap-x-6 gap-y-1 font-mono text-sm text-on-surface-variant">
                        <span>"v"{version.clone()}</span>
                        <span>
                            <span class="text-on-surface">{format_bytes(size)}</span>
                            " total"
                        </span>
                        <span>
                            <span class="text-on-surface">{mod_count}</span>
                            " mods included"
                        </span>
                    </div>
                </div>
                {is_admin.then(|| read_edit_toggle(mode))}
            </header>
            <ul class="mt-8">
                {data
                    .mods
                    .into_iter()
                    .map(|m| {
                        let wid = m.workshop_id.clone();
                        view! {
                            <li class="flex items-center gap-4 rounded-xl border-b border-white/5 px-4 py-5 transition hover:bg-white/[0.02]">
                                <div class="flex size-10 shrink-0 items-center justify-center rounded-lg bg-white/5 text-on-surface-variant">
                                    <MaterialIcon name="extension" />
                                </div>
                                <div class="flex min-w-0 flex-1 flex-col">
                                    <span class="font-medium text-on-surface">{m.name.clone()}</span>
                                    {(!wid.is_empty()).then(|| {
                                        view! {
                                            <span class="font-mono text-[11px] text-on-surface-variant/70">
                                                {wid}
                                            </span>
                                        }
                                    })}
                                </div>
                                {m.required.then(|| {
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
            <div class="mt-10 space-y-3 pt-2">
                <button
                    type="button"
                    class="w-full rounded-full bg-action py-5 text-lg font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
                    on:click=move |_| toasts.message("Launch requires the Reforger client")
                >
                    "[ Launch Game & Auto-Download ]"
                </button>
                {(!workshop_url.is_empty()).then(|| {
                    let url = workshop_url.clone();
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
                {is_admin.then(|| {
                    let pack_id_set = pack_id.clone();
                    let pack_id_del = pack_id.clone();
                    view! {
                        <div class="flex flex-wrap gap-2 pt-4">
                            {(!is_current).then(|| {
                                let pack_id_set = pack_id_set.clone();
                                view! {
                                    <button
                                        type="button"
                                        disabled=move || set_busy.get()
                                        class="rounded-full border border-success/30 bg-success/10 px-4 py-2 font-mono text-xs tracking-wider text-success uppercase transition hover:bg-success/20 disabled:opacity-50"
                                        on:click=move |_| {
                                            if set_busy.get_untracked() {
                                                return;
                                            }
                                            set_busy.set(true);
                                            let path = format!("/modpacks/{pack_id_set}/set-current");
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                leptos::task::spawn_local(async move {
                                                    match crate::client::api_post_ok(
                                                        store,
                                                        &path,
                                                        json!({}),
                                                    )
                                                    .await
                                                    {
                                                        Ok(()) => {
                                                            toasts.success("Set as current modpack");
                                                            packs_res.refetch();
                                                        }
                                                        Err(e) => {
                                                            toasts.error(
                                                                crate::client::api_error_message(
                                                                    &e,
                                                                    "Failed to set current",
                                                                ),
                                                            );
                                                        }
                                                    }
                                                    set_busy.set(false);
                                                });
                                            }
                                            #[cfg(not(target_arch = "wasm32"))]
                                            {
                                                let _ = (store, path, packs_res, toasts);
                                                set_busy.set(false);
                                            }
                                        }
                                    >
                                        {move || {
                                            if set_busy.get() {
                                                "Setting…"
                                            } else {
                                                "Set current"
                                            }
                                        }}
                                    </button>
                                }
                            })}
                            <button
                                type="button"
                                disabled=move || del_busy.get()
                                class="rounded-full border border-error-alert/30 px-4 py-2 font-mono text-xs tracking-wider text-error-alert uppercase transition hover:bg-error-alert/10 disabled:opacity-50"
                                on:click=move |_| {
                                    if del_busy.get_untracked() {
                                        return;
                                    }
                                    del_busy.set(true);
                                    let path = format!("/modpacks/{pack_id_del}");
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        leptos::task::spawn_local(async move {
                                            match crate::client::api_delete(store, &path).await {
                                                Ok(()) => {
                                                    toasts.success("Modpack deleted");
                                                    packs_res.refetch();
                                                }
                                                Err(e) => {
                                                    toasts.error(crate::client::api_error_message(
                                                        &e,
                                                        "Failed to delete modpack",
                                                    ));
                                                }
                                            }
                                            del_busy.set(false);
                                        });
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        let _ = (store, path, packs_res, toasts);
                                        del_busy.set(false);
                                    }
                                }
                            >
                                {move || if del_busy.get() { "Deleting…" } else { "Delete" }}
                            </button>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}

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

fn editor(
    p: &ModpackDto,
    mode: RwSignal<MpMode>,
    packs_res: LocalResource<Option<DataEnvelope<ModpackDto>>>,
    toasts: crate::toast::Toasts,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let initial = PackEdit::from_dto(p);
    let name = RwSignal::new(initial.name.clone());
    let version = RwSignal::new(initial.version.clone());
    let workshop_url = RwSignal::new(initial.workshop_url.clone());
    let total_size = RwSignal::new(initial.total_size_bytes);
    let is_current = RwSignal::new(initial.is_current);
    let mods = RwSignal::new(initial.mods);
    let new_mod = RwSignal::new(String::new());
    let new_workshop = RwSignal::new(String::new());
    let pack_id = p.modpack.id.clone();
    let fallback_name = p.modpack.name.clone();
    let save_busy = RwSignal::new(false);
    let save_err = RwSignal::new(None::<String>);

    let add_mod = move || {
        let trimmed = new_mod.get_untracked().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let wid = new_workshop.get_untracked().trim().to_string();
        mods.update(|m| {
            m.push(ModEdit {
                name: trimmed,
                required: false,
                workshop_id: wid,
                mod_guid: String::new(),
                version: String::new(),
            })
        });
        new_mod.set(String::new());
        new_workshop.set(String::new());
    };

    view! {
        <div class="mx-auto flex min-h-full w-full max-w-3xl flex-col px-8 py-10">
            <header class="flex items-start justify-between gap-4">
                <div class="flex-1 space-y-3">
                    <div>
                        <label class="mb-1 block font-mono text-xs tracking-wider text-on-surface-variant uppercase">
                            "Modpack name"
                        </label>
                        <input
                            prop:value=initial.name.clone()
                            on:input=move |ev| name.set(event_target_value(&ev))
                            class="w-full rounded-xl border border-white/10 bg-black/30 px-4 py-3 text-2xl font-bold tracking-tight text-on-surface focus:border-primary/50 focus:outline-none"
                        />
                    </div>
                    <div class="flex flex-wrap gap-3">
                        <div class="min-w-[8rem] flex-1">
                            <label class="mb-1 block font-mono text-xs tracking-wider text-on-surface-variant uppercase">
                                "Version"
                            </label>
                            <input
                                prop:value=initial.version.clone()
                                on:input=move |ev| version.set(event_target_value(&ev))
                                class="w-full rounded-xl border border-white/10 bg-black/30 px-3 py-2 font-mono text-sm text-on-surface focus:border-primary/50 focus:outline-none"
                            />
                        </div>
                        <div class="min-w-[12rem] flex-[2]">
                            <label class="mb-1 block font-mono text-xs tracking-wider text-on-surface-variant uppercase">
                                "Workshop URL"
                            </label>
                            <input
                                prop:value=initial.workshop_url.clone()
                                on:input=move |ev| workshop_url.set(event_target_value(&ev))
                                class="w-full rounded-xl border border-white/10 bg-black/30 px-3 py-2 text-sm text-on-surface focus:border-primary/50 focus:outline-none"
                            />
                        </div>
                    </div>
                    <label class="flex items-center gap-2 font-mono text-xs text-on-surface-variant">
                        <input
                            type="checkbox"
                            prop:checked=initial.is_current
                            on:change=move |ev| {
                                is_current.set(event_target_checked(&ev));
                            }
                        />
                        "Mark as current modpack"
                    </label>
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
                            let wid = m.workshop_id.clone();
                            view! {
                                <li class="flex flex-col gap-2 rounded-xl border-b border-white/5 px-4 py-4">
                                    <div class="flex items-center gap-3">
                                        <MaterialIcon
                                            name="drag_indicator"
                                            class="text-on-surface-variant/50"
                                        />
                                        <span class="flex-1 font-medium text-on-surface">{m.name.clone()}</span>
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
                                    </div>
                                    <input
                                        prop:value=wid
                                        placeholder="Workshop id (game.mods[].modId)"
                                        on:input=move |ev| {
                                            let v = event_target_value(&ev);
                                            mods.update(|list| {
                                                if let Some(entry) = list.get_mut(i) {
                                                    entry.workshop_id = v;
                                                }
                                            })
                                        }
                                        class="ml-8 rounded-lg border border-white/10 bg-black/20 px-3 py-1.5 font-mono text-xs text-on-surface placeholder:text-on-surface-variant/50 focus:border-primary/50 focus:outline-none"
                                    />
                                </li>
                            }
                                .into_any()
                        })
                        .collect_view()
                        .into_any()
                }}
            </ul>
            <div class="mt-4 flex flex-col gap-2 sm:flex-row">
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
                <input
                    prop:value=move || new_workshop.get()
                    on:input=move |ev| new_workshop.set(event_target_value(&ev))
                    placeholder="Workshop id"
                    class="w-full rounded-xl border border-white/10 bg-black/30 px-4 py-3 font-mono text-sm text-on-surface placeholder:text-on-surface-variant/60 focus:border-primary/50 focus:outline-none sm:w-44"
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
            <div class="mt-10 flex flex-col gap-3 pt-2">
                {move || {
                    save_err.get().map(|m| {
                        view! {
                            <p class="font-mono text-sm text-error-alert">{m}</p>
                        }
                    })
                }}
                <div class="flex gap-3">
                    <button
                        type="button"
                        disabled=move || save_busy.get()
                        on:click=move |_| {
                            if save_busy.get_untracked() {
                                return;
                            }
                            let n = name.get_untracked().trim().to_string();
                            let final_name = if n.is_empty() {
                                fallback_name.clone()
                            } else {
                                n
                            };
                            let mut ver = version.get_untracked().trim().to_string();
                            if ver.is_empty() {
                                ver = "0.0.0".into();
                            }
                            let edit = PackEdit {
                                name: final_name.clone(),
                                version: ver,
                                total_size_bytes: total_size.get_untracked(),
                                workshop_url: workshop_url.get_untracked(),
                                is_current: is_current.get_untracked(),
                                mods: mods.get_untracked(),
                            };
                            let body = edit.to_put_body();
                            let path = format!("/modpacks/{pack_id}");
                            save_busy.set(true);
                            save_err.set(None);
                            #[cfg(target_arch = "wasm32")]
                            {
                                leptos::task::spawn_local(async move {
                                    match crate::client::api_put::<ModpackDto>(store, &path, body)
                                        .await
                                    {
                                        Ok(saved) => {
                                            toasts.success(format!(
                                                "Saved \"{}\"",
                                                saved.modpack.name
                                            ));
                                            mode.set(MpMode::Read);
                                            packs_res.refetch();
                                        }
                                        Err(e) => {
                                            save_err.set(Some(crate::client::api_error_message(
                                                &e,
                                                "Failed to save modpack",
                                            )));
                                        }
                                    }
                                    save_busy.set(false);
                                });
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let _ = (store, path, body, packs_res, toasts, mode);
                                save_busy.set(false);
                            }
                        }
                        class="flex-1 rounded-full bg-action py-4 text-lg font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90 disabled:opacity-50"
                    >
                        {move || if save_busy.get() { "Saving…" } else { "Save Changes" }}
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
        </div>
    }
}
