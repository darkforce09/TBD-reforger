//! The vehicle form: the one dialog an administrator adds a vehicle and edits a stored one with.
//!
//! **Role:** renders a labelled input per vehicle field, checks the draft on submit, sends the
//! create or replace request its [`FormTarget`] picks, and shows why a draft or a request was
//! refused.
//! **Position:** a [`Dialog`] over the vehicle index, opened from the "Add vehicle" action or the
//! dossier's Edit action; the page re-creates it on every opening.
//! **Signals & state:** owns one text signal per field, seeded from the target when created, plus
//! the field problems, the refusal sentence and the busy flag. Closes the page's `open` flag and
//! hands the stored row to `on_saved` when the backend accepts the write; reads the `AuthStore`
//! from context.
//! **Invariants:** a draft that breaks a field rule sends nothing and marks each broken field; a
//! request is in flight at most once; the dialog stays open, with its text, until the backend
//! accepts, so a refusal never loses what was typed. The send runs on `wasm32` only, as does the
//! form.

#[cfg(target_arch = "wasm32")]
use super::vehicle_draft::{FieldProblem, FormTarget, VehicleDraft, VehicleField};
#[cfg(target_arch = "wasm32")]
use crate::foundation::transport::dto::vehicles::Vehicle;
#[cfg(target_arch = "wasm32")]
use crate::foundation::ui::Dialog;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The sentence shown when a refused write carries no sentence of its own.
#[cfg(target_arch = "wasm32")]
const SAVE_FALLBACK: &str = "The vehicle could not be saved.";
/// A field's input while it breaks no rule.
#[cfg(target_arch = "wasm32")]
const INPUT_CLASS: &str = "w-full rounded-xl border border-white/10 bg-black/30 px-3 py-2 text-sm text-on-surface placeholder:text-on-surface-variant/50 focus:border-primary/50 focus:outline-none";
/// A field's input while it breaks a rule.
#[cfg(target_arch = "wasm32")]
const INPUT_INVALID_CLASS: &str = "w-full rounded-xl border border-error-alert/60 bg-black/30 px-3 py-2 text-sm text-on-surface placeholder:text-on-surface-variant/50 focus:border-error-alert focus:outline-none";

/// The vehicle form for `target`, open while `open` holds; `on_saved` receives the row the
/// backend stored.
#[cfg(target_arch = "wasm32")]
pub(super) fn vehicle_form_dialog(
    open: RwSignal<bool>,
    target: FormTarget,
    on_saved: Callback<Vehicle>,
) -> impl IntoView {
    let store = expect_context::<crate::foundation::auth::AuthStore>();
    let title = target.title();
    let submit_label = target.submit_label();
    let seed = target.draft();
    let inputs = VehicleField::ALL.map(|field| (field, RwSignal::new(seed.text(field).to_owned())));
    let problems = RwSignal::new(Vec::<FieldProblem>::new());
    let refusal = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let target = StoredValue::new(target);

    let submit = move || {
        if busy.get_untracked() {
            return;
        }
        let mut draft = VehicleDraft::default();
        for (field, text) in inputs {
            draft.set_text(field, text.get_untracked());
        }
        refusal.set(None);
        let write = match draft.validate() {
            Ok(write) => write,
            Err(found) => {
                problems.set(found);
                return;
            }
        };
        problems.set(Vec::new());
        let request = target.with_value(|target| target.request(write));
        busy.set(true);
        leptos::task::spawn_local(async move {
            let answer = super::vehicle_writes::send_vehicle_request(store, request).await;
            busy.set(false);
            match answer {
                Ok(saved) => {
                    open.set(false);
                    on_saved.run(saved);
                }
                Err(refused) => refusal.set(Some(super::write_refusal::refusal_sentence(
                    &refused,
                    SAVE_FALLBACK,
                ))),
            }
        });
    };

    view! {
        <Dialog open=open title=title class="max-w-xl">
            <form
                class="flex flex-col gap-4"
                on:submit=move |ev| {
                    ev.prevent_default();
                    submit();
                }
            >
                {inputs
                    .into_iter()
                    .map(|(field, text)| form_field(field, text, problems))
                    .collect_view()}
                {move || {
                    refusal
                        .get()
                        .map(|sentence| {
                            view! {
                                <p role="alert" class="font-mono text-sm text-error-alert">
                                    {sentence}
                                </p>
                            }
                        })
                }}
                <div class="flex justify-end gap-2">
                    <button
                        type="button"
                        on:click=move |_| open.set(false)
                        class="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
                    >
                        "Cancel"
                    </button>
                    <button
                        type="submit"
                        prop:disabled=move || busy.get()
                        class="rounded-md bg-action px-3 py-1.5 text-label-md font-bold text-on-action transition-colors hover:bg-action/90 disabled:opacity-60"
                    >
                        {move || if busy.get() { "Saving…" } else { submit_label }}
                    </button>
                </div>
            </form>
        </Dialog>
    }
}

/// One labelled input bound to `text`, with the problem `problems` holds for `field` under it.
#[cfg(target_arch = "wasm32")]
fn form_field(
    field: VehicleField,
    text: RwSignal<String>,
    problems: RwSignal<Vec<FieldProblem>>,
) -> impl IntoView {
    let problem = move || {
        problems.with(|all| {
            all.iter()
                .find(|problem| problem.field == field)
                .map(|problem| problem.message.clone())
        })
    };
    let limit = field
        .max_chars()
        .map(|limit| format!(" · at most {limit} characters"));
    view! {
        <label class="flex flex-col gap-1">
            <span class="font-mono text-xs tracking-wider text-on-surface-variant uppercase">
                {field.label()}
                {(!field.is_required()).then_some(" (optional)")}
                {limit}
            </span>
            <input
                type="text"
                placeholder={field.placeholder()}
                aria-invalid=move || if problem().is_some() { "true" } else { "false" }
                prop:value=move || text.get()
                on:input=move |ev| text.set(event_target_value(&ev))
                class=move || if problem().is_some() { INPUT_INVALID_CLASS } else { INPUT_CLASS }
            />
            {move || {
                problem()
                    .map(|message| {
                        view! { <span class="text-label-md text-error-alert">{message}</span> }
                    })
            }}
        </label>
    }
}
