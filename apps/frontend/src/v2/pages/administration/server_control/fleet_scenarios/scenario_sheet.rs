//! The fleet scenario sheet: every terrain's registered scenario, the form that registers or
//! replaces one, and the control that removes one.
//!
//! **Role:** renders the sheet the picker's heading opens — the list of terrain → scenario mappings
//! with who last changed each and when, an edit control that loads a mapping into the form, a
//! remove control that asks to be confirmed, and the form itself.
//! **Position:** a side sheet over the server control screen.
//! **Signals & state:** reads and writes the [`ScenarioRegistry`]; owns the form's three fields,
//! the problem the last check found, and per row whether its removal is being confirmed.
//! **Invariants:** a registration is checked as the backend checks it before it is sent; the form
//! keeps its fields until the backend stores the mapping. Every request is browser-only.

use super::scenario_wording::{scenario_registration, updated_line};
use super::{RegistryRead, ScenarioRegistry};
use crate::v2::core::api::dto::FleetScenario;
use crate::v2::core::ui::{MaterialIcon, Sheet};
use leptos::prelude::*;

/// Shared styling for the sheet's fields.
const FIELD: &str = "w-full rounded-md border border-outline-variant/40 bg-surface px-3 py-1.5 text-sm text-on-surface outline-none focus:border-primary/60";

/// The form's three fields.
#[derive(Clone, Copy)]
struct ScenarioForm {
    terrain_key: RwSignal<String>,
    scenario_id: RwSignal<String>,
    display_name: RwSignal<String>,
}

impl ScenarioForm {
    /// Load an existing mapping into the form, so saving it replaces that terrain's scenario.
    fn load(self, scenario: &FleetScenario) {
        self.terrain_key.set(scenario.terrain_key.clone());
        self.scenario_id.set(scenario.scenario_id.clone());
        self.display_name.set(scenario.display_name.clone());
    }

    /// Empty the form.
    fn clear(self) {
        self.terrain_key.set(String::new());
        self.scenario_id.set(String::new());
        self.display_name.set(String::new());
    }
}

/// The fleet scenario sheet.
pub(in super::super) fn scenario_sheet(registry: ScenarioRegistry) -> impl IntoView {
    let me = StoredValue::new(registry.store.user.get_untracked().map(|u| u.discord_id));
    let form = ScenarioForm {
        terrain_key: RwSignal::new(String::new()),
        scenario_id: RwSignal::new(String::new()),
        display_name: RwSignal::new(String::new()),
    };
    view! {
        <Sheet open=registry.open bleed=true class="w-full max-w-none md:w-[40rem]">
            <div class="flex h-full flex-col">
                <header class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                    <div class="min-w-0">
                        <h2 class="text-headline-sm text-on-surface">"Fleet scenarios"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            "A deployment runs its artifact on the scenario registered for the artifact's terrain; a terrain with none cannot be deployed."
                        </p>
                    </div>
                    <button type="button" aria-label="Close" on:click=move |_| registry.open.set(false)
                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface">
                        <MaterialIcon name="close" />
                    </button>
                </header>
                <div class="custom-scrollbar flex-1 space-y-6 overflow-y-auto px-6 py-5">
                    {registration_form(registry, form)}
                    {move || registry.refusal.get().map(|why| view! { <p role="alert" class="text-sm text-error-alert">{why}</p> })}
                    {move || match registry.registry.get() {
                        RegistryRead::Loaded(list) if list.is_empty() => view! {
                            <p class="text-sm text-on-surface-variant">"No terrain has a registered scenario yet."</p>
                        }
                        .into_any(),
                        RegistryRead::Loaded(list) => view! {
                            <ul class="space-y-2" data-testid="fleet-scenarios">
                                {list
                                    .into_iter()
                                    .map(|scenario| scenario_row(registry, form, scenario, me.get_value()))
                                    .collect_view()}
                            </ul>
                        }
                        .into_any(),
                        RegistryRead::Failed(why) => view! { <p class="text-sm text-error-alert">{why}</p> }.into_any(),
                        RegistryRead::Loading | RegistryRead::Idle => view! {
                            <p class="text-sm text-on-surface-variant">"Loading the fleet scenarios…"</p>
                        }
                        .into_any(),
                    }}
                </div>
            </div>
        </Sheet>
    }
}

/// The form that registers a terrain's scenario, or replaces the one registered.
fn registration_form(registry: ScenarioRegistry, form: ScenarioForm) -> impl IntoView {
    let problem = RwSignal::new(None::<String>);
    let save = move |_| match scenario_registration(
        &form.terrain_key.get_untracked(),
        &form.scenario_id.get_untracked(),
        &form.display_name.get_untracked(),
    ) {
        Ok((terrain, update)) => {
            problem.set(None);
            registry.put(terrain, update, move || form.clear());
        }
        Err(why) => problem.set(Some(why)),
    };
    view! {
        <section class="rounded-xl border border-white/10 p-4">
            <h3 class="mb-2 text-sm font-semibold text-on-surface">"Register or replace a terrain's scenario"</h3>
            <div class="grid gap-2 md:grid-cols-2">
                <input aria-label="Terrain key" placeholder="Terrain key, for example arland"
                    prop:value=move || form.terrain_key.get()
                    on:input=move |ev| form.terrain_key.set(event_target_value(&ev))
                    class=FIELD />
                <input aria-label="Display name" placeholder="Display name"
                    prop:value=move || form.display_name.get()
                    on:input=move |ev| form.display_name.set(event_target_value(&ev))
                    class=FIELD />
            </div>
            <input aria-label="Scenario id" placeholder="{1111222233334444}Missions/TBD_Arland.conf"
                prop:value=move || form.scenario_id.get()
                on:input=move |ev| form.scenario_id.set(event_target_value(&ev))
                class=format!("{FIELD} mt-2 font-mono") />
            <div class="mt-2 flex items-center justify-between gap-3">
                <p class="text-xs text-error-alert">{move || problem.get()}</p>
                <button type="button" on:click=save prop:disabled=move || registry.busy.get()
                    data-testid="fleet-scenario-save"
                    class="shrink-0 rounded-full bg-action px-4 py-1.5 text-sm font-medium text-on-action disabled:opacity-50">
                    "Save scenario"
                </button>
            </div>
        </section>
    }
}

/// One mapping: its terrain, display name and scenario, who last changed it, and its controls.
fn scenario_row(
    registry: ScenarioRegistry,
    form: ScenarioForm,
    scenario: FleetScenario,
    me: Option<String>,
) -> impl IntoView {
    let confirming = RwSignal::new(false);
    let updated = updated_line(&scenario, me.as_deref());
    let terrain = StoredValue::new(scenario.terrain_key.clone());
    let stored = StoredValue::new(scenario.clone());
    view! {
        <li class="rounded-xl border border-white/10 p-3 text-sm">
            <div class="flex flex-wrap items-start justify-between gap-2">
                <div class="min-w-0">
                    <p class="text-on-surface">
                        <span class="font-mono text-code-md text-primary">{scenario.terrain_key.clone()}</span>
                        " — " {scenario.display_name.clone()}
                    </p>
                    <p class="mt-1 break-all font-mono text-code-md text-outline">{scenario.scenario_id.clone()}</p>
                    <p class="text-xs text-on-surface-variant">{updated}</p>
                </div>
                <div class="flex gap-2">
                    <button type="button" on:click=move |_| stored.with_value(|s| form.load(s))
                        class="rounded-full border border-white/10 px-3 py-1 text-xs text-on-surface hover:bg-white/5">
                        "Edit"
                    </button>
                    <button type="button" on:click=move |_| confirming.update(|c| *c = !*c)
                        class="rounded-full border border-error-alert/30 px-3 py-1 text-xs text-error-alert hover:bg-error-alert/10">
                        {move || if confirming.get() { "Keep" } else { "Remove" }}
                    </button>
                </div>
            </div>
            {move || {
                confirming
                    .get()
                    .then(|| {
                        view! {
                            <div class="mt-3 flex flex-wrap items-center gap-3 rounded-lg border border-error-alert/30 bg-error-alert/10 p-3 text-xs">
                                <span class="text-on-surface">
                                    "New deployments of this terrain are refused until a scenario is registered again; deployments already recorded are untouched."
                                </span>
                                <button type="button" prop:disabled=move || registry.busy.get()
                                    on:click=move |_| registry.remove(terrain.get_value())
                                    class="rounded-full bg-error-alert/20 px-3 py-1 text-error-alert disabled:opacity-50">
                                    "Remove the scenario"
                                </button>
                            </div>
                        }
                    })
            }}
        </li>
    }
}
