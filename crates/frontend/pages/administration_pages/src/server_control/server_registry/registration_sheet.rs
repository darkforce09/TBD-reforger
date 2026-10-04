//! The registration sheet: the form that registers a game server or changes a server's
//! registration, and the control that takes a server out of service or puts it back.
//!
//! **Role:** renders the sheet the picker's "Add server" and the card's "Edit" open — the name,
//! address, game port and required-modpack fields with the save control, the refusal the last
//! write came back with, and, for a registered server, its service state with the deactivate
//! control that asks to be confirmed, or the reactivate control.
//! **Position:** a side sheet over the server control screen.
//! **Signals & state:** reads and writes the [`ServerRegistry`]; owns the form's four fields,
//! seeded each time the sheet opens — empty for a new server, from the row for a registered one —
//! the problem the last check found, and whether the deactivation is being confirmed.
//! **Invariants:** a registration or a change is checked as the backend checks it before it is
//! sent, and a change names only the fields that differ from the row the form was seeded from, so
//! saving an untouched form sends nothing. The form keeps its fields until the backend accepts the
//! write. Every request is browser-only.

#[cfg(target_arch = "wasm32")]
use super::registration_wording::{modpack_choice_label, server_change, server_registration};
#[cfg(target_arch = "wasm32")]
use super::{ModpackChoices, ServerRegistry};
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::ServerRowDto;
#[cfg(target_arch = "wasm32")]
use frontend_ui::{MaterialIcon, Sheet};
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// Shared styling for the sheet's fields.
#[cfg(target_arch = "wasm32")]
const FIELD: &str = "w-full rounded-md border border-outline-variant/40 bg-surface px-3 py-1.5 text-sm text-on-surface outline-none focus:border-primary/60";

/// Shared styling for the fields' captions.
#[cfg(target_arch = "wasm32")]
const CAPTION: &str = "mb-1 block text-xs text-on-surface-variant";

/// The form's four fields, as typed.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
struct RegistrationForm {
    name: RwSignal<String>,
    address: RwSignal<String>,
    port: RwSignal<String>,
    /// The chosen modpack's id; empty for none.
    modpack: RwSignal<String>,
}

#[cfg(target_arch = "wasm32")]
impl RegistrationForm {
    /// Empty for a new server, or holding a registered server's row.
    fn seeded(stored: Option<&ServerRowDto>) -> Self {
        Self {
            name: RwSignal::new(stored.map(|s| s.name.clone()).unwrap_or_default()),
            address: RwSignal::new(stored.map(|s| s.ip.clone()).unwrap_or_default()),
            port: RwSignal::new(stored.map(|s| s.port.to_string()).unwrap_or_default()),
            modpack: RwSignal::new(
                stored
                    .and_then(|s| s.required_modpack_id.as_ref().map(ToString::to_string))
                    .unwrap_or_default(),
            ),
        }
    }
}

/// The registration sheet.
#[cfg(target_arch = "wasm32")]
pub(in super::super) fn registration_sheet(registry: ServerRegistry) -> impl IntoView {
    view! {
        <Sheet open=registry.sheet_open bleed=true class="w-full max-w-none md:w-[36rem]">
            {sheet_body(registry)}
        </Sheet>
    }
}

/// The sheet's heading, form, refusal and service section, built each time the sheet opens.
#[cfg(target_arch = "wasm32")]
fn sheet_body(registry: ServerRegistry) -> impl IntoView {
    let stored = registry
        .editing
        .get_untracked()
        .and_then(|id| registry.row_untracked(&id));
    let form = RegistrationForm::seeded(stored.as_ref());
    let (heading, subtitle) = match &stored {
        Some(row) => (
            "Server settings",
            format!("{} · {}:{}", row.name, row.ip, row.port),
        ),
        None => (
            "Add a server",
            "Register a game server to issue its machine credentials, send it fleet commands and \
             deploy missions to it."
                .to_string(),
        ),
    };
    view! {
        <div class="flex h-full flex-col" data-testid="server-registration">
            <header class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                <div class="min-w-0">
                    <h2 class="text-headline-sm text-on-surface">{heading}</h2>
                    <p class="mt-1 text-label-md text-on-surface-variant">{subtitle}</p>
                </div>
                <button type="button" aria-label="Close" on:click=move |_| registry.sheet_open.set(false)
                    class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface">
                    <MaterialIcon name="close" />
                </button>
            </header>
            <div class="custom-scrollbar flex-1 space-y-6 overflow-y-auto px-6 py-5">
                {registration_form(registry, form, stored.clone())}
                {move || registry.refusal.get().map(|why| view! { <p role="alert" class="text-sm text-error-alert">{why}</p> })}
                {stored.map(|row| service_section(registry, row.id.into_inner()))}
            </div>
        </div>
    }
}

/// The name, address, game port and required-modpack fields, and the control that saves them.
#[cfg(target_arch = "wasm32")]
fn registration_form(
    registry: ServerRegistry,
    form: RegistrationForm,
    stored: Option<ServerRowDto>,
) -> impl IntoView {
    let problem = RwSignal::new(None::<String>);
    let editing = stored.is_some();
    let stored = StoredValue::new(stored);
    let save = move |_| {
        let (name, address, port, modpack) = (
            form.name.get_untracked(),
            form.address.get_untracked(),
            form.port.get_untracked(),
            form.modpack.get_untracked(),
        );
        let checked = match stored.get_value() {
            Some(row) => match server_change(&row, &name, &address, &port, &modpack) {
                Ok(Some(change)) => {
                    registry.save_change(row.id.into_inner(), change);
                    None
                }
                Ok(None) => Some("Nothing to save: every field is as registered".to_string()),
                Err(why) => Some(why),
            },
            None => match server_registration(&name, &address, &port, &modpack) {
                Ok(registration) => {
                    registry.register(registration);
                    None
                }
                Err(why) => Some(why),
            },
        };
        problem.set(checked);
    };
    view! {
        <section class="space-y-3 rounded-xl border border-white/10 p-4">
            <h3 class="text-sm font-semibold text-on-surface">"Registration"</h3>
            <label class="block">
                <span class=CAPTION>"Name"</span>
                <input aria-label="Server name" placeholder="TBD Staging — Everon"
                    prop:value=move || form.name.get()
                    on:input=move |ev| form.name.set(event_target_value(&ev))
                    class=FIELD />
            </label>
            <div class="grid gap-3 md:grid-cols-[1fr_8rem]">
                <label class="block">
                    <span class=CAPTION>"Address — a literal IPv4 or IPv6 address, not a hostname"</span>
                    <input aria-label="Server address" placeholder="203.0.113.24"
                        prop:value=move || form.address.get()
                        on:input=move |ev| form.address.set(event_target_value(&ev))
                        class=format!("{FIELD} font-mono") />
                </label>
                <label class="block">
                    <span class=CAPTION>"Game port"</span>
                    <input type="number" min="1" max="65535" aria-label="Game port" placeholder="2001"
                        prop:value=move || form.port.get()
                        on:input=move |ev| form.port.set(event_target_value(&ev))
                        class=format!("{FIELD} font-mono [color-scheme:dark]") />
                </label>
            </div>
            {modpack_field(registry, form)}
            <div class="flex items-center justify-between gap-3">
                <p class="text-xs text-error-alert">{move || problem.get()}</p>
                <button type="button" on:click=save prop:disabled=move || registry.busy.get()
                    data-testid="server-registration-save"
                    class="shrink-0 rounded-full bg-action px-4 py-1.5 text-sm font-medium text-on-action disabled:opacity-50">
                    {if editing { "Save changes" } else { "Register server" }}
                </button>
            </div>
        </section>
    }
}

/// The required-modpack choice: none, or one of the modpacks as read.
///
/// While the modpacks are unread the choice cannot change, so a registered server's requirement
/// is kept as it is rather than shown as none. The option chosen when the list arrives is marked
/// `selected`, because the select's value is written before its options exist.
#[cfg(target_arch = "wasm32")]
fn modpack_field(registry: ServerRegistry, form: RegistrationForm) -> impl IntoView {
    view! {
        <label class="block">
            <span class=CAPTION>"Required modpack — a deployment's artifact must be compiled against it"</span>
            {move || match registry.modpacks.get() {
                ModpackChoices::Loaded(packs) => {
                    let chosen = form.modpack.get_untracked();
                    view! {
                        <select aria-label="Required modpack"
                            prop:value=move || form.modpack.get()
                            on:change=move |ev| form.modpack.set(event_target_value(&ev))
                            class=FIELD>
                            <option value="" selected=chosen.is_empty()>"No required modpack"</option>
                            {packs
                                .iter()
                                .map(|pack| {
                                    let id = pack.modpack.id.clone();
                                    let selected = id == chosen.as_str();
                                    view! { <option value=id.to_string() selected=selected>{modpack_choice_label(pack)}</option> }
                                })
                                .collect_view()}
                        </select>
                    }
                    .into_any()
                }
                ModpackChoices::Failed(why) => view! {
                    <p class="text-xs text-error-alert">{format!("{why}; the required modpack stays as it is")}</p>
                }
                .into_any(),
                ModpackChoices::Loading | ModpackChoices::Idle => view! {
                    <p class="text-xs text-on-surface-variant">"Reading the modpacks…"</p>
                }
                .into_any(),
            }}
        </label>
    }
}

/// A registered server's service state, with the control that changes it.
#[cfg(target_arch = "wasm32")]
fn service_section(registry: ServerRegistry, server_id: String) -> impl IntoView {
    let confirming = RwSignal::new(false);
    let id = StoredValue::new(server_id);
    let active = Memo::new(move |_| {
        id.with_value(|id| registry.row(id))
            .is_some_and(|row| row.is_active)
    });
    view! {
        <section class="space-y-3 rounded-xl border border-white/10 p-4" data-testid="server-registration-service">
            <h3 class="text-sm font-semibold text-on-surface">"Service"</h3>
            {move || {
                if active.get() {
                    view! {
                        <div class="flex flex-wrap items-center justify-between gap-3">
                            <p class="min-w-0 flex-1 text-xs text-on-surface-variant">
                                "Active: its host agent and game runtime authenticate, and it takes fleet commands, deployments and new credentials."
                            </p>
                            <button type="button" on:click=move |_| confirming.update(|c| *c = !*c)
                                class="shrink-0 rounded-full border border-error-alert/30 px-3 py-1 text-xs text-error-alert hover:bg-error-alert/10">
                                {move || if confirming.get() { "Keep it active" } else { "Deactivate" }}
                            </button>
                        </div>
                        {move || confirming.get().then(|| view! {
                            <div class="flex flex-wrap items-center gap-3 rounded-lg border border-error-alert/30 bg-error-alert/10 p-3 text-xs">
                                <span class="text-on-surface">
                                    "Deactivating takes the server out of service at once: its host agent and game runtime are refused, it takes no fleet command, deployment or new credential, and members stop seeing it. Nothing is deleted — reactivating it restores all of this with the same credentials."
                                </span>
                                <button type="button" prop:disabled=move || registry.busy.get()
                                    on:click=move |_| registry.deactivate(id.get_value())
                                    data-testid="server-registration-deactivate"
                                    class="rounded-full bg-error-alert/20 px-3 py-1 text-error-alert disabled:opacity-50">
                                    "Deactivate the server"
                                </button>
                            </div>
                        })}
                    }
                    .into_any()
                } else {
                    view! {
                        <div class="flex flex-wrap items-center justify-between gap-3">
                            <p class="min-w-0 flex-1 text-xs text-on-surface-variant">
                                "Deactivated: its host agent and game runtime are refused, it takes no fleet command, deployment or new credential, and members do not see it."
                            </p>
                            <button type="button" prop:disabled=move || registry.busy.get()
                                on:click=move |_| registry.reactivate(id.get_value())
                                data-testid="server-registration-reactivate"
                                class="shrink-0 rounded-full bg-action px-4 py-1.5 text-sm font-medium text-on-action disabled:opacity-50">
                                "Reactivate"
                            </button>
                        </div>
                    }
                    .into_any()
                }
            }}
        </section>
    }
}
