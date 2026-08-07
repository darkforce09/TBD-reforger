//! T-661 — the Mission Settings dialog (environment + flow + render prefs), split from
//! `eden_chrome.rs`.
//!
//! Terrain (readonly) + time/weather author through [`crate::eden_env::author_env`] (T-193 gate);
//! [`render_flow_section`] is the T-224 mission-flow block. Time and weather additionally mirror to
//! the `missions` row through [`crate::eden_top_strip::RowMirror`] (T-192). Renders no DOM while
//! closed; the doc-reading halves are wasm-only.
//!
//! **T-691 (Eden NEW-F2 + 3den E6) — editor preferences, separated from mission settings.** Eden
//! keeps Settings ▸ Preferences (editor-local, per-user) apart from Attributes (the mission
//! document); TBD used to mix both in [`MissionSettingsDialog`]. The per-user half — basemap view
//! and the 12 world-layer toggles, both localStorage-backed via [`crate::world_layer_prefs`] — now
//! lives in its own surface, [`EditorPreferencesDialog`]. [`MissionSettingsDialog`] keeps ONLY the
//! document keys (time / weather / flow / hillshade / grid, all authored through `author_env` into
//! `meta.environment`) and grows a one-line pointer row that opens the preferences dialog. The
//! separation is the ticket: no `author_env` write may live in [`EditorPreferencesDialog`], and no
//! world-layer toggle may remain in [`MissionSettingsDialog`].
//!
//! **T-694 (Eden NEW-F6) — mission shape after creation.** `missions.game_mode` was create-dialog
//! only. `PATCH /missions/:id` has always accepted it (`handlers/missions.rs` `PatchMissionInput`),
//! but no editor surface ever sent it, so a mission could not change shape once it existed.
//! [`render_shape_section`] is that surface. It is drawn as its own block rather than folded in
//! beside Time and Weather because it writes the **row**, not the document: the two halves fail
//! differently (a row PATCH is refused for a non-author; an `author_env` write cannot be) and an
//! author who cannot tell them apart cannot understand either failure.
//!
//! **The "min/max players" half of that ticket was reinterpreted by the operator**, which is why
//! there is no `min_players` anywhere in this file, in `dto.rs`, or in a migration. A TBD mission's
//! player count is not a number somebody types into a menu — it is how many slots have been placed.
//! So this dialog *derives* it from `MissionDocCore::slot_count` and shows the stored `max_players`
//! beside it, **unreconciled**, whenever the two disagree ([`PLAYER_COUNT_DISAGREE_NOTE`]).
//!
//! What it deliberately does **not** do: clamp either figure to a server limit, reuse the slot count
//! as a capacity, or invent a minimum. A slot is a seat, not a player — 160 slots across 20 squads
//! may seat 80 people — and an Arma Reforger server caps connections regardless of either number.
//! What the authoritative cap should be is an open question, and a dialog that guessed it would be
//! stating a rule nobody has decided. [`SLOTS_PLACED_NOTE`] says so on screen.
//!
//! **Opener (in-owns).** The gear/menu that opens [`MissionSettingsDialog`] lives in
//! `eden_top_strip`/`mission_editor` (not this slice's `owns`), so rather than route a second menu
//! item, [`EditorPreferencesDialog`] is mounted as a sibling *inside* [`MissionSettingsDialog`] and
//! opened by [`open_editor_preferences`] from the pointer row. Both share one `RwSignal<bool>`
//! parked in a `thread_local` at [`MissionSettingsDialog`] setup (the `context_menu`/`attrs_open`
//! idiom), so the opener stays entirely within `eden_settings.rs`.
#![allow(dead_code)]
use leptos::prelude::*;

use crate::eden_env::ENV_UNCARRIED_NOTE;
use crate::ui::MaterialIcon;

// T-691 — the Editor Preferences dialog's open flag, parked here from `MissionSettingsDialog`'s
// setup so the pointer row (and any future in-owns caller) can arm it without threading a prop
// through `mission_editor`/`eden_top_strip`. `RwSignal<bool>` is `Copy` and wasm is single-threaded,
// so this mirrors `context_menu::MENU`. `true` ⇒ the preferences dialog is open.
thread_local! {
    static PREFS_OPEN: std::cell::RefCell<Option<RwSignal<bool>>> =
        const { std::cell::RefCell::new(None) };
}

/// Install the Editor Preferences open signal (once, from [`MissionSettingsDialog`] setup).
fn set_prefs_signal(sig: RwSignal<bool>) {
    PREFS_OPEN.with(|p| *p.borrow_mut() = Some(sig));
}

/// Open the Editor Preferences dialog. Called from the pointer row inside
/// [`MissionSettingsDialog`]; a no-op if the signal has not been installed yet (no dialog mounted).
/// Exposed `pub` so a later in-owns opener (or a menu item, once routed) can reach it without a prop.
pub fn open_editor_preferences() {
    PREFS_OPEN.with(|p| {
        if let Some(sig) = *p.borrow() {
            sig.set(true);
        }
    });
}

// T-688 — the All Settings dialog's open flag. Same `thread_local` idiom, and parked from the same
// place, as [`PREFS_OPEN`]: the opener is a pointer row inside [`MissionSettingsDialog`], so no prop
// has to be threaded through `mission_editor`/`eden_top_strip` (neither of which this slice owns).
thread_local! {
    static ALL_SETTINGS_OPEN: std::cell::RefCell<Option<RwSignal<bool>>> =
        const { std::cell::RefCell::new(None) };
}

/// Install the All Settings open signal (once, from [`MissionSettingsDialog`] setup).
fn set_all_settings_signal(sig: RwSignal<bool>) {
    ALL_SETTINGS_OPEN.with(|p| *p.borrow_mut() = Some(sig));
}

/// Open the All Settings dialog ([`AllSettingsDialog`]). No-op before the dialog is mounted.
pub fn open_all_settings() {
    ALL_SETTINGS_OPEN.with(|p| {
        if let Some(sig) = *p.borrow() {
            sig.set(true);
        }
    });
}

/* ───────────────────────────── T-694 — mission shape (the row half) ───────────────────────────── */

/// The game modes `PATCH /missions/:id` accepts, with the labels the create dialog shows.
///
/// The **server's** enum is the authority: `handlers/missions.rs::valid_game_mode` maps exactly these
/// three strings and 400s everything else, so a fourth row here would be a control that can only
/// fail. The labels are `create_mission_dialog.rs`'s on purpose — a mission must not change
/// vocabulary between the screen that made it and the screen that edits it.
const GAME_MODES: [(&str, &str); 3] = [("pve_coop", "Co-op PvE"), ("pvp", "PvP"), ("zeus", "Zeus")];

/// Is `v` one of [`GAME_MODES`]? The `<select>` can only emit its own options, so this can only fail
/// if the table above ever drifts from the server enum — in which case refusing locally is the right
/// answer: the PATCH would 400 and the author would watch their choice revert with no explanation.
/// Same guard, and the same reasoning, as the `JIP_OPTIONS` check in [`render_flow_section`].
fn is_known_game_mode(v: &str) -> bool {
    GAME_MODES.iter().any(|(k, _)| *k == v)
}

/// Is the route `:id` a real `missions` row?
///
/// Deliberately duplicated from `eden_top_strip::is_mission_row_id`, which is private to a file this
/// slice does not own. The check is not optional: the editor also mounts on synthetic ids
/// (`mission_editor` falls back to `draft`; the gate route drives a smoke id) where both the shape
/// GET and the shape PATCH are guaranteed failures. Cheap shape test — the SPA carries no `uuid`
/// dependency.
fn is_row_id(s: &str) -> bool {
    s.len() == 36
        && s.as_bytes().iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

/// The `missions` row fields this dialog reads. Neither lives in the mission document, and the
/// editor's own hydrate keeps only `compiled_meta()` (which has `max_players` but no `game_mode`, and
/// is private to `mission_commands`), so the dialog reads the row itself on open.
#[derive(Clone, PartialEq, Eq, Debug)]
struct RowShape {
    game_mode: String,
    max_players: i64,
}

/// T-694 — the two numbers this dialog puts under **Players**, and the fact that it reconciles
/// neither.
///
/// `placed` is `MissionDocCore::slot_count`: the seats actually in the document. `declared` is
/// `missions.max_players`, the figure chosen once in the create dialog and never checked against the
/// mission since; `None` means the row has not arrived (or the route id is not a row at all).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct PlayerCount {
    placed: usize,
    declared: Option<i64>,
}

impl PlayerCount {
    /// Do the two figures differ? Only then is [`PLAYER_COUNT_DISAGREE_NOTE`] shown — a mission whose
    /// author happens to have placed exactly `max_players` slots needs no essay about it.
    fn disagrees(self) -> bool {
        matches!(self.declared, Some(d) if d != i64::try_from(self.placed).unwrap_or(i64::MAX))
    }
}

/// What the derived figure is, and — just as load-bearing — what it is not.
///
/// It is **not** a capacity check and this copy must not start reading like one. A slot is a seat:
/// 160 slots across 20 squads may seat 80 people, and an Arma Reforger server enforces its own
/// connection limit whatever this dialog says. Saying that here is cheaper than the alternative,
/// which is an author reading the number as a promise.
const SLOTS_PLACED_NOTE: &str = "Counted from the slots placed in this mission — nobody types it. \
                                 A slot is a seat, not a player: this is what the mission contains, \
                                 not how many people your server will hold.";

/// Shown only when the placed and declared figures differ ([`PlayerCount::disagrees`]).
///
/// **Why it shows both instead of choosing.** The slot count is what the mission contains; the stored
/// `max_players` is what the compiled mission carries (`dto::MissionDetail::compiled_meta`) and what
/// the library card advertises. Silently preferring either would delete the author's only evidence
/// that the two have drifted apart, which is the entire reason this row exists.
const PLAYER_COUNT_DISAGREE_NOTE: &str =
    "These two do not agree. Max players was chosen once, in the create dialog, and nothing has \
     compared it to the mission since; the slot figure is what the mission actually contains. \
     Neither is enforced here, so both are shown.";

/// Shown in place of the game-mode control when the row could not be read: an unsaved draft has no
/// row to change, and a failed read must not present a working-looking `<select>` that silently
/// PATCHes nothing.
const SHAPE_UNAVAILABLE_NOTE: &str =
    "The mission row has not loaded, so game mode cannot be changed here. A draft that has never \
     been saved to the library has no row yet.";

/// T-694 — what a refused game-mode PATCH tells the author.
///
/// Same two-texts split, for the same reason, as `eden_top_strip::mirror_failure_message` (private to
/// a file this slice does not own): a **403 is structural** — `PATCH /missions/:id` gates on
/// authorship while the editor route gates on role, so a `mission_maker` legitimately editing someone
/// else's mission is refused every time and retrying cannot help. Anything else names what the server
/// said and is worth another go. Both texts state that the control has been put back, because it has.
fn game_mode_failure_message(err: &crate::client::ApiErr) -> String {
    if err.0 == 403 {
        return "Game mode was not saved — you are not this mission's author. It has been put back \
                to the stored value."
            .to_string();
    }
    format!(
        "Could not save the game mode: {}. It has been put back to the stored value.",
        crate::client::api_error_message(err, "the server did not respond")
    )
}

/// T-694 — the `missions` row PATCH/GET pair behind [`render_shape_section`].
///
/// **Why not `eden_top_strip::RowMirror`.** That handle carries a debounce, a per-column dedupe and a
/// single-flight sequencer, all of which exist for the time scrubber — ~30 distinct values a second,
/// where out-of-order landing is a real hazard. A `<select>` emits one value per settled choice, so
/// none of that machinery would ever be exercised here; and `RowMirror`'s `commit`/`MirroredField`
/// are private to a file this slice does not own. This is the small honest version, not a fork.
///
/// `Copy` and built from the reactive owner (`expect_context` / `use_toasts` / `use_params_map` all
/// resolve there and would panic from a bare DOM handler), so each control's handler can capture it.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
struct ShapeMirror {
    auth: crate::auth::AuthStore,
    mission_id: StoredValue<String>,
    toasts: crate::toast::Toasts,
}

#[cfg(target_arch = "wasm32")]
impl ShapeMirror {
    fn from_route() -> Self {
        use leptos_router::hooks::use_params_map;
        let id = use_params_map()
            .get_untracked()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_default();
        Self {
            auth: expect_context::<crate::auth::AuthStore>(),
            mission_id: StoredValue::new(id),
            toasts: crate::toast::use_toasts(),
        }
    }

    /// Read the row into `shape`. Runs on every *open* rather than once at mount: the row is edited
    /// from the library and the dossier too, and a dialog that cached a stale `game_mode` would offer
    /// to "change" the mission to the value it already has.
    ///
    /// A failure clears `shape` instead of leaving the last-known value on screen — see
    /// [`SHAPE_UNAVAILABLE_NOTE`]. Showing a mode nobody has confirmed is the same lie as showing a
    /// reverted one.
    fn load(self, shape: RwSignal<Option<RowShape>>) {
        let id = self.mission_id.get_value();
        if !is_row_id(&id) {
            shape.set(None);
            return;
        }
        let auth = self.auth;
        leptos::task::spawn_local(async move {
            let got = crate::client::api_get::<crate::dto::MissionDetail>(
                auth,
                &format!("/missions/{id}"),
            )
            .await;
            match got {
                Ok(d) => shape.set(Some(RowShape {
                    game_mode: d.game_mode,
                    max_players: d.max_players,
                })),
                Err(e) => {
                    leptos::logging::warn!(
                        "T-694: could not read the mission row's shape: {}",
                        crate::client::api_error_message(&e, "GET /missions/:id failed")
                    );
                    shape.set(None);
                }
            }
        });
    }

    /// PATCH `missions.game_mode`.
    ///
    /// Optimistic, then reverted on refusal: the `<select>` has already repainted itself by the time
    /// this runs, so `shape` is moved first and put back if the server says no. A refused value must
    /// not stay on screen — the same rule the flow-duration boxes follow, and the reason
    /// [`game_mode_failure_message`] tells the author the control has moved back.
    fn set_game_mode(self, next: String, shape: RwSignal<Option<RowShape>>) {
        let id = self.mission_id.get_value();
        let Some(previous) = shape.get_untracked() else {
            return;
        };
        if !is_row_id(&id) || !is_known_game_mode(&next) || previous.game_mode == next {
            return;
        }
        shape.set(Some(RowShape {
            game_mode: next.clone(),
            max_players: previous.max_players,
        }));
        let auth = self.auth;
        let toasts = self.toasts;
        leptos::task::spawn_local(async move {
            let body = serde_json::json!({ "game_mode": next });
            let res = crate::client::api_patch::<serde_json::Value>(
                auth,
                &format!("/missions/{id}"),
                body,
            )
            .await;
            if let Err(e) = &res {
                leptos::logging::warn!(
                    "T-694: could not save the mission's game mode: {}",
                    crate::client::api_error_message(e, "PATCH /missions/:id failed")
                );
                toasts.error(game_mode_failure_message(e));
                shape.set(Some(previous));
            }
        });
    }
}

#[cfg(target_arch = "wasm32")]
use crate::eden_env::{
    author_env, fmt_duration_secs, parse_flow_seconds, read_flow_jip, read_flow_seconds,
    FLOW_DEFAULT_BRIEFING_S, FLOW_DEFAULT_SAFESTART_S, FLOW_DEFAULT_TIMELIMIT_S, JIP_OPTIONS,
    SETTINGS_UNREAD_NOTE,
};
#[cfg(target_arch = "wasm32")]
use crate::eden_top_strip::RowMirror;

/// Mission Settings dialog (MissionSettingsDialog.tsx — environment half). Terrain (readonly) +
/// time / weather flow through [`author_env`] (one undo step each); the render-pref controls (map
/// style, grid, hillshade, world-layer toggles) are live below them since T-173 P6. Renders no DOM
/// while closed. T-159.26.
///
/// **T-192:** time and weather additionally mirror to the `missions` row through [`RowMirror`] on
/// commit.
///
/// **T-193:** the View Distance field and the Thermals toggle are gone, and [`ENV_UNCARRIED_NOTE`]
/// stands where they were. They were fully working controls authoring keys that no compiled
/// document, no row column and no mod script has ever read — [`CARRIED_ENV_KEYS`] carries the whole
/// argument for deleting them rather than trying to carry them through.
///
/// **T-224:** [`render_flow_section`] adds the mission-flow block — duration, briefing, safe start
/// and join-in-progress — and [`SETTINGS_UNREAD_NOTE`] says why respawn, spectator policy, night
/// vision and tickets are not beside them.
#[component]
pub fn MissionSettingsDialog(open: RwSignal<bool>, doc_tick: RwSignal<u64>) -> impl IntoView {
    // Esc closes (the suite Dialog behavior).
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked() && ev.key() == "Escape" {
                open.set(false);
            }
        });
        on_cleanup(move || esc.remove());
    }
    // T-192 — read the route id + auth store here, in the component body: the reactive owner is
    // live at setup and gone by the time a control's `on:change` fires.
    #[cfg(target_arch = "wasm32")]
    let row_mirror = RowMirror::from_route();
    let ctrl = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
    // T-691 — the sibling Editor Preferences dialog's open flag. Created in this reactive owner and
    // parked in the module `thread_local` so [`open_editor_preferences`] (the pointer row) can arm
    // it; the dialog itself is mounted below as a sibling so it survives this dialog being closed.
    let prefs_open = RwSignal::new(false);
    set_prefs_signal(prefs_open);
    // T-688 — the sibling All Settings dialog's open flag, parked the same way and for the same
    // reason. Mounted as a sibling below so it outlives this dialog being closed.
    let all_settings_open = RwSignal::new(false);
    set_all_settings_signal(all_settings_open);
    // T-694 — the `missions` row's shape (game mode + the stored max players). `None` until the read
    // lands, and again if it fails: the dialog would rather say it does not know (see
    // [`SHAPE_UNAVAILABLE_NOTE`]) than offer a control over a value it has not confirmed.
    let shape = RwSignal::new(None::<RowShape>);
    // Re-read on every open, not once at mount. The row is also edited from the library and the
    // dossier, and `mission_hydrate`'s boot GET keeps only `compiled_meta()` — which has no
    // `game_mode` and is private to `mission_commands` — so there is nothing cached to reuse.
    #[cfg(target_arch = "wasm32")]
    {
        let loader = ShapeMirror::from_route();
        Effect::new(move |_| {
            if open.get() {
                loader.load(shape);
            }
        });
    }
    let body = move || {
        if !open.get() {
            return None;
        }
        let _ = doc_tick.get(); // re-read env on undo/redo while open
        #[cfg(target_arch = "wasm32")]
        let env = crate::editor_ops::read_env();
        #[cfg(not(target_arch = "wasm32"))]
        let env = crate::dto::MissionEnv::default();
        Some(view! {
            <div
                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
                on:click=move |_| open.set(false)
            ></div>
            <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200">
                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                    <div class="min-w-0">
                        <h2 class="text-headline-sm text-on-surface">"Mission Settings"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            "Environment and flow for this mission."
                        </p>
                    </div>
                    <button
                        type="button"
                        aria-label="Close"
                        on:click=move |_| open.set(false)
                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <MaterialIcon name="close" />
                    </button>
                </div>
                <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                    <div class="flex flex-col gap-4">
                        <label class="flex flex-col gap-1">
                            <span class="text-label-sm uppercase tracking-wider text-outline">
                                "Terrain"
                            </span>
                            <div class="rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 px-2.5 py-1.5 font-mono text-code-md text-on-surface-variant">
                                {env.terrain.clone()}
                            </div>
                        </label>
                        <div class="grid grid-cols-2 gap-3">
                            <label class="flex flex-col gap-1">
                                <span class="text-label-sm uppercase tracking-wider text-outline">
                                    "Time"
                                </span>
                                <input
                                    type="time"
                                    value=env.time.clone()
                                    on:input=move |ev| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let t = event_target_value(&ev);
                                            author_env("time", t.as_str().into());
                                            // T-192 — same handler as the doc write on purpose; a
                                            // `change` listener would not survive this dialog's
                                            // rebuild-per-doc-tick. Partial values arrive as "" and
                                            // repeats are absorbed by the mirror's dedupe.
                                            row_mirror.set_time(&t);
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        let _ = &ev;
                                    }
                                    class=ctrl
                                />
                            </label>
                            // T-193 — Weather moved up beside Time. They are the two settings a
                            // compiled mission actually carries, and they were only ever apart
                            // because View Distance held this cell.
                            <label class="flex flex-col gap-1">
                                <span class="text-label-sm uppercase tracking-wider text-outline">
                                    "Weather"
                                </span>
                                <select
                                    prop:value=env.weather.clone()
                                    on:change=move |ev| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let w = event_target_value(&ev);
                                            author_env("weather", w.as_str().into());
                                            row_mirror.set_weather(&w);
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        let _ = &ev;
                                    }
                                    class=ctrl
                                >
                                    <option value="clear">"Clear"</option>
                                    <option value="overcast">"Overcast"</option>
                                    <option value="heavy_rain">"Heavy Rain"</option>
                                    <option value="dense_fog">"Dense Fog"</option>
                                </select>
                            </label>
                        </div>
                        <p class="text-label-sm normal-case text-outline">{ENV_UNCARRIED_NOTE}</p>
                        {render_all_settings_pointer()}
                        {render_shape_section(ctrl, shape)}
                        {render_flow_section(ctrl)}
                        {render_prefs_section(&env)}
                    </div>
                </div>
            </div>
        })
    };
    // T-691 — the dialog body and the sibling Editor Preferences dialog. The preferences dialog is
    // mounted unconditionally (it gates itself on `prefs_open`) so it can open independently of, and
    // outlive, this dialog being closed.
    view! {
        {body}
        <EditorPreferencesDialog open=prefs_open />
        <AllSettingsDialog open=all_settings_open doc_tick=doc_tick />
    }
}

/// T-688 — the pointer row that opens [`AllSettingsDialog`] from Mission Settings.
///
/// It sits directly under Time / Weather because that is where an author first meets the scatter: the
/// dialog they are looking at holds six of the mission's settings and the rest are somewhere else.
/// Same shape as the T-691 preferences pointer — a `<button>`, no document write.
fn render_all_settings_pointer() -> AnyView {
    view! {
        <button
            type="button"
            class="mt-1 flex items-center justify-between gap-3 rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-2 text-left transition-colors hover:border-primary/50"
            data-open-all-settings
            on:click=move |_| open_all_settings()
        >
            <span class="min-w-0">
                <span class="block text-label-md text-on-surface">"All settings in this mission"</span>
                <span class="block text-label-sm normal-case text-outline">
                    "One read-only list of every authored setting — including the ones that live on placed entities — against the defaults the schema declares."
                </span>
            </span>
            <MaterialIcon name="chevron_right" class="shrink-0 text-base text-outline" />
        </button>
    }
    .into_any()
}

/// T-694 (Eden NEW-F6) — the **mission shape** block: game mode, and how many players this mission
/// is actually for.
///
/// **The row half of this dialog.** Every other section authors `meta.environment` through
/// `author_env`; this one reads and writes the `missions` row through [`ShapeMirror`]. They are drawn
/// apart because they fail apart: a row PATCH is refused for a non-author (403 →
/// [`game_mode_failure_message`]) and an `author_env` write cannot be, so folding game mode in beside
/// Weather would put two controls with different failure modes under one heading.
///
/// **Game mode** is the T-694 gap itself — create-dialog only until now, though `PATCH /missions/:id`
/// has always taken it. It is a plain `<select>` over [`GAME_MODES`] with no debounce: see
/// [`ShapeMirror`] for why the top strip's sequencer is not reused.
///
/// **Players is a report, not a setting.** The number that matters is how many slots have been
/// placed, so it is derived from `MissionDocCore::slot_count` at render time — and because the
/// dialog body re-renders on every `doc_tick`, placing or deleting a slot moves it without a reopen.
/// The stored `max_players` is shown beside it, read-only, and when the two disagree
/// [`PLAYER_COUNT_DISAGREE_NOTE`] says so instead of either figure quietly winning. There is no
/// minimum, no clamp and no server-capacity check here on purpose — [`SLOTS_PLACED_NOTE`] and the
/// module header carry that argument.
///
/// Inert on the native view shell (no document, no row), exactly like [`render_flow_section`].
fn render_shape_section(ctrl: &'static str, shape: RwSignal<Option<RowShape>>) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (ctrl, shape);
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let sect = "text-label-sm uppercase tracking-wider text-outline";
        let hint = "text-label-sm normal-case text-outline";
        let readonly = "rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 px-2.5 py-1.5 font-mono text-code-md text-on-surface-variant";
        // Built here rather than threaded from setup: the section renders inside the dialog body's
        // reactive owner, so the three context lookups resolve, and the resulting `Copy` handle is
        // what the `on:change` closure captures — which is the property that actually matters.
        let mirror = ShapeMirror::from_route();

        // Read the row once per render. `shape.get()` subscribes the dialog body, so the block
        // repaints when the open-time read lands (and again if a PATCH is refused and reverted).
        let row = shape.get();
        // The seats the document actually holds. `doc_handle()` is `None` on a dialog opened before
        // the editor's doc host mounted; zero is the honest answer there, not a hidden row.
        let placed = match crate::mission_history::doc_handle() {
            Some(handle) => {
                let doc = handle.borrow();
                doc.as_ref()
                    .map_or(0, map_engine_core::doc::MissionDocCore::slot_count)
            }
            None => 0,
        };
        let counts = PlayerCount {
            placed,
            declared: row.as_ref().map(|r| r.max_players),
        };

        let mode_control = match row.as_ref() {
            Some(r) => {
                let current = r.game_mode.clone();
                let options = GAME_MODES
                    .into_iter()
                    .map(|(value, label)| view! { <option value=value>{label}</option> })
                    .collect::<Vec<_>>();
                view! {
                    <select
                        prop:value=current
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            mirror.set_game_mode(v, shape);
                        }
                        class=ctrl
                    >
                        {options}
                    </select>
                }
                .into_any()
            }
            None => view! { <p class=hint>{SHAPE_UNAVAILABLE_NOTE}</p> }.into_any(),
        };

        let declared_cell = counts.declared.map(|d| {
            view! {
                <div class="flex flex-col gap-1">
                    <span class=sect>"Max players (set at creation)"</span>
                    <div class=readonly>{d.to_string()}</div>
                </div>
            }
        });

        view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <span class=sect>"Mission shape"</span>
                <label class="flex flex-col gap-1">
                    <span class=sect>"Game mode"</span>
                    {mode_control}
                </label>

                <span class=sect>"Players"</span>
                <div class="grid grid-cols-2 gap-3">
                    <div class="flex flex-col gap-1">
                        <span class=sect>"Slots placed"</span>
                        <div class=readonly>{placed.to_string()}</div>
                    </div>
                    {declared_cell}
                </div>
                <span class=hint>{SLOTS_PLACED_NOTE}</span>
                {counts
                    .disagrees()
                    .then(|| view! { <span class=hint>{PLAYER_COUNT_DISAGREE_NOTE}</span> })}
            </div>
        }
        .into_any()
    }
}

/// T-224 — the mission-flow half of Mission Settings: how long the round runs, how long the
/// briefing and safe start are, and who may join once it has started.
///
/// Every control here writes through [`author_env`], the same one gate the Time and Weather controls
/// take, so the "does anything read this back?" question is asked once per key rather than once per
/// control. [`AUTHORED_FLOW_KEYS`] holds the answers, the compiled path each key becomes, and the
/// one hop (`flatten.rs`, not this slice's file) that is still hardcoded.
///
/// The four settings this dialog deliberately does NOT grow — respawn, spectator policy, night
/// vision and per-faction tickets — are covered by [`SETTINGS_UNREAD_NOTE`], which renders below.
///
/// **`change`, not `input`.** A duration box authored per keystroke would file `5`, `54`, `540`,
/// `5400` as four undo steps, and each of those bumps `doc_tick`, which rebuilds this whole subtree
/// out from under the caret. `change` fires on blur/Enter — one undo step per settled value, and the
/// rebuild lands after the author has already left the field. (Weather already uses `change` for the
/// same reason; Time uses `input` because a `<input type="time">` has no half-typed state worth
/// suppressing and T-192's row mirror rides that handler.)
///
/// Inert on the native view shell (no document), exactly like [`render_prefs_section`].
fn render_flow_section(ctrl: &'static str) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = ctrl;
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let sect = "text-label-sm uppercase tracking-wider text-outline";
        let hint = "text-label-sm normal-case text-outline";

        // Mission duration leads, out of stage order (briefing → safe start → round), because it is
        // the one an author comes to this dialog to set; the other two are the stage lengths around
        // it. Each row is (key, label, committed value, the sentence the field needs or "").
        let rows: [(&str, &str, i64, &str); 3] = [
            (
                "timeLimitSeconds",
                "Mission duration",
                read_flow_seconds("timeLimitSeconds", FLOW_DEFAULT_TIMELIMIT_S),
                "0 means no time limit — the round will not end on the clock.",
            ),
            (
                "briefingSeconds",
                "Briefing",
                read_flow_seconds("briefingSeconds", FLOW_DEFAULT_BRIEFING_S),
                "Announced on entering the briefing; the stage is advanced by an admin, not by this \
                 clock.",
            ),
            (
                "safeStartSeconds",
                "Safe start",
                read_flow_seconds("safeStartSeconds", FLOW_DEFAULT_SAFESTART_S),
                "",
            ),
        ];

        let duration_rows = rows
            .into_iter()
            .map(|(key, label, committed, hint_text)| {
                view! {
                    <label class="flex flex-col gap-1">
                        <span class=sect>
                            {format!("{label} — {}", fmt_duration_secs(committed))}
                        </span>
                        <input
                            type="number"
                            min="0"
                            step="1"
                            value=committed.to_string()
                            on:change=move |ev| {
                                let raw = event_target_value(&ev);
                                if let Some(secs) = parse_flow_seconds(&raw) {
                                    author_env(key, secs.into());
                                } else {
                                    // A refused value must not stay on screen. Leaving "-1" in the
                                    // box while the document still holds 5400 is the editor showing
                                    // the author a setting they do not have — the same lie as a
                                    // silently reverted one, told the other way round.
                                    leptos::logging::warn!(
                                        "refusing meta.environment.{key} = {raw:?}: flow durations are whole seconds >= 0"
                                    );
                                    if let Some(input) = ev
                                        .target()
                                        .and_then(|t| {
                                            wasm_bindgen::JsCast::dyn_into::<
                                                web_sys::HtmlInputElement,
                                            >(t)
                                                .ok()
                                        })
                                    {
                                        input.set_value(&committed.to_string());
                                    }
                                }
                            }
                            class=ctrl
                        />
                        {(!hint_text.is_empty())
                            .then(|| view! { <span class=hint>{hint_text}</span> })}
                    </label>
                }
            })
            .collect::<Vec<_>>();

        let jip = read_flow_jip();
        let jip_options = JIP_OPTIONS
            .into_iter()
            .map(|(value, label)| view! { <option value=value>{label}</option> })
            .collect::<Vec<_>>();

        view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <span class=sect>"Mission flow"</span>
                {duration_rows}
                <label class="flex flex-col gap-1">
                    <span class=sect>"Join in progress"</span>
                    <select
                        prop:value=jip
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            // The <select> can only emit its own options, so this can only fail if
                            // JIP_OPTIONS ever drifts from the schema enum — in which case refusing
                            // is right: TBD_MissionFlow.PolicyFromString maps an unknown string to
                            // ALWAYS, so a bad value holds the door open all round instead of erroring.
                            if JIP_OPTIONS.iter().any(|(k, _)| *k == v) {
                                author_env("jip", v.as_str().into());
                            } else {
                                leptos::logging::error!("refusing meta.environment.jip = {v:?}");
                            }
                        }
                        class=ctrl
                    >
                        {jip_options}
                    </select>
                    <span class=hint>
                        "Whether a player who connects after the mission has started may deploy."
                    </span>
                </label>
                <p class=hint>{SETTINGS_UNREAD_NOTE}</p>
            </div>
        }
        .into_any()
    }
}

/// T-173 P6 — the render-pref half of **Mission Settings**: hillshade on/off + strength slider and
/// the grid toggle. These are per-**mission** document keys (`meta.environment`, authored through
/// [`author_env`]), so they stay in this dialog.
///
/// **T-691:** the per-**user** editor-local controls that used to sit here — basemap view and the 12
/// world-layer toggles ([`crate::world_layer_prefs`], localStorage) — moved to
/// [`EditorPreferencesDialog`]; this section now ends with a one-line pointer row linking there. No
/// world-layer toggle remains in this dialog (the document-vs-local separation pin). On the native
/// view-shell these are inert (no engine), which is fine — the dialog is a wasm surface.
fn render_prefs_section(env: &crate::dto::MissionEnv) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = env;
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let hillshade_on = env.show_hillshade;
        let hillshade_pct = (env.hillshade_opacity * 100.0).round() as i64;
        let grid_on = env.show_grid;
        let sect = "text-label-sm uppercase tracking-wider text-outline";

        view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <div class="flex items-center justify-between py-0.5">
                    <span class="text-label-md text-on-surface-variant">"Show hillshade"</span>
                    <input
                        type="checkbox"
                        prop:checked=hillshade_on
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            author_env("showHillshade", on.into());
                            let op = crate::editor_ops::read_env().hillshade_opacity;
                            crate::world_assets::apply_hillshade(on, op);
                        }
                        class="accent-primary"
                    />
                </div>
                <label class="flex flex-col gap-1" class:opacity-40=move || !hillshade_on>
                    <span class=sect>{move || format!("Hillshade strength — {hillshade_pct}%")}</span>
                    <input
                        type="range"
                        min="0"
                        max="100"
                        step="1"
                        prop:disabled=!hillshade_on
                        value=hillshade_pct.to_string()
                        on:input=move |ev| {
                            let pct: f64 = event_target_value(&ev).parse().unwrap_or(40.0);
                            let op = (pct / 100.0).clamp(0.0, 1.0);
                            author_env("hillshadeOpacity", op.into());
                            crate::world_assets::apply_hillshade(true, op);
                        }
                        class="accent-primary"
                    />
                </label>

                <div class="flex items-center justify-between py-0.5">
                    <span class="text-label-md text-on-surface-variant">"Grid"</span>
                    <input
                        type="checkbox"
                        prop:checked=grid_on
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            author_env("showGrid", on.into());
                            crate::world_assets::apply_grid(on);
                        }
                        class="accent-primary"
                    />
                </div>

                // T-691 — pointer to the separated editor-local surface. Eden keeps per-user
                // preferences apart from the mission document; the basemap view + world-layer
                // toggles live there now.
                <button
                    type="button"
                    class="mt-1 flex items-center justify-between gap-3 rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-2 text-left transition-colors hover:border-primary/50"
                    on:click=move |_| open_editor_preferences()
                >
                    <span class="min-w-0">
                        <span class="block text-label-md text-on-surface">
                            "Editor preferences moved"
                        </span>
                        <span class="block text-label-sm normal-case text-outline">
                            "Basemap view and world layers are now per-user editor preferences."
                        </span>
                    </span>
                    <MaterialIcon name="chevron_right" class="shrink-0 text-base text-outline" />
                </button>
            </div>
        }
        .into_any()
    }
}

/// T-691 (Eden NEW-F2 + 3den E6) — the **Editor Preferences** dialog: the editor-local, per-user
/// half that Eden keeps separate from mission Attributes. Basemap view (Satellite / Map) and the 12
/// world-layer visibility toggles, both persisted to localStorage through
/// [`crate::world_layer_prefs`] (the versioned editor-preferences store) and applied live to the map
/// host. Mounted as a sibling of [`MissionSettingsDialog`] and opened via
/// [`open_editor_preferences`] from that dialog's pointer row.
///
/// **Separation pin:** this dialog contains no [`author_env`] write — every control here is a
/// localStorage editor preference, never a mission-document key. Renders no DOM while closed; inert
/// on the native view-shell (no engine).
#[component]
fn EditorPreferencesDialog(open: RwSignal<bool>) -> impl IntoView {
    // Esc closes (same suite Dialog behavior as MissionSettingsDialog).
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked() && ev.key() == "Escape" {
                open.set(false);
            }
        });
        on_cleanup(move || esc.remove());
    }
    move || {
        if !open.get() {
            return None;
        }
        Some(view! {
            <div
                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
                on:click=move |_| open.set(false)
            ></div>
            <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200">
                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                    <div class="min-w-0">
                        <h2 class="text-headline-sm text-on-surface">"Editor Preferences"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            "Per-user editor settings — saved to this browser, not the mission."
                        </p>
                    </div>
                    <button
                        type="button"
                        aria-label="Close"
                        on:click=move |_| open.set(false)
                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <MaterialIcon name="close" />
                    </button>
                </div>
                <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                    {render_editor_prefs_body()}
                </div>
            </div>
        })
    }
}

/// T-691 — the body of [`EditorPreferencesDialog`]: basemap view + the 12 world-layer toggles, moved
/// verbatim from the old `render_prefs_section`. Every control persists through
/// [`crate::world_layer_prefs`] (localStorage) and applies live to the map host — no `author_env`.
fn render_editor_prefs_body() -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        use crate::world_layer_prefs as wlp;
        let sect = "text-label-sm uppercase tracking-wider text-outline";
        // Basemap view kept in a local signal so the active highlight follows a click within the
        // session (the store is still the source of truth; this only drives the button styling).
        let basemap = RwSignal::new(wlp::load_basemap_view());
        let prefs = wlp::load_prefs();

        let layer_rows = prefs
            .rows()
            .into_iter()
            .map(|(key, on, label)| {
                view! {
                    <div class="flex items-center justify-between py-0.5">
                        <span class="text-label-md text-on-surface-variant">{label}</span>
                        <input
                            type="checkbox"
                            prop:checked=on
                            on:change=move |ev| {
                                let checked = event_target_checked(&ev);
                                let mut p = wlp::load_prefs();
                                p.set(key, checked);
                                wlp::save_prefs(&p);
                                crate::world_assets::refresh_world_layers();
                            }
                            class="accent-primary"
                        />
                    </div>
                }
            })
            .collect::<Vec<_>>();

        view! {
            <div class="flex flex-col gap-4">
                <span class=sect>"Basemap"</span>
                <div class="flex gap-2">
                    {["satellite", "map"]
                        .into_iter()
                        .map(|v| {
                            let label = if v == "satellite" { "Satellite" } else { "Map" };
                            view! {
                                <button
                                    type="button"
                                    class=move || if basemap.get() == v {
                                        "flex-1 rounded-md border border-primary/60 bg-primary/20 px-2.5 py-1.5 text-label-md text-primary"
                                    } else {
                                        "flex-1 rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface-variant transition-colors hover:border-primary/40"
                                    }
                                    on:click=move |_| {
                                        wlp::save_basemap_view(v);
                                        crate::world_assets::apply_basemap_view(v);
                                        basemap.set(v.to_string());
                                    }
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>

                <span class=sect>"World layers"</span>
                <div class="flex flex-col gap-1">{layer_rows}</div>
            </div>
        }
        .into_any()
    }
}

/* ═════════════════ T-688 — the aggregated settings view (READ-ONLY) ═════════════════════════════
 *
 * FNF v4 named this defect about itself (fnf_v4.md:906): "One `config.sqf` became 22 modules whose
 * attributes are only visible when the right module is selected. There is no 'show me every setting
 * in this mission' view." That is the bill for typed-attributes-on-placed-objects, and TBD is at the
 * START of the same curve — [`MissionSettingsDialog`] holds terrain / time / weather / the three flow
 * durations / JIP, while `zone.rules` ALREADY lives on placed entities. Every ticket that puts a
 * setting on an entity widens the scatter. This block is the one place that reads them all back.
 *
 * ═══ TWO CONSTRAINTS, BOTH BINDING ═══
 *
 * (1) **READ-ONLY.** A second editing surface would duplicate every attribute control in the
 *     programme, and the duplicate is what rots. Rows display; they do not edit. No `author_env`, no
 *     `editor_ops` mutator, no `<input>`/`<select>` reaches [`render_all_settings_body`], and
 *     `the_aggregated_view_is_not_a_second_editing_surface` fails if one ever does.
 *
 * (2) **ROWS CLICK THROUGH TO THE OWNING ENTITY** (wog.md 14.6 — a findings list that only prints is
 *     worse than one that selects). The click goes through T-655's SHIPPED router,
 *     [`crate::validation_panel::route_select_by_subject_id`], not a second selection path. What that
 *     router resolves today, and what it does not, is stated on [`OWNER_UNRESOLVED_NOTE`] — and the
 *     row says so out loud rather than swallowing the click.
 *
 * ═══ THE DEFAULTS COME OUT OF THE SCHEMA. FULL STOP. ═══
 *
 * The claim that makes this view cheap — "the schema declares `default` on every key that has one, so
 * the comparison needs no second source of truth" — is also the one that can rot. A hand-written table
 * of default VALUES in Rust would be exactly the second source of truth this view exists to remove,
 * and it would drift silently the first time the schema changed: a view reporting a "diff" against a
 * stale copy is a worse defect than no view at all.
 *
 * So [`SettingDefault::Schema`] is constructed in exactly ONE function, [`schema_default`], out of the
 * embedded `mission.schema.json` bytes, and `the_view_and_the_schema_agree_key_for_key` re-reads the
 * schema independently and compares every `$defs/zoneRules` key against what the view reports. It goes
 * red the moment the two disagree, whatever the cause.
 *
 * **Keys with no declared default are reported as such, never invented.** Of the mission-level keys
 * this dialog authors, `mission.schema.json` declares a `default` for NONE of them: `$defs/flow` states
 * no default for `briefingSeconds` / `safeStartSeconds` / `timeLimitSeconds` / `jip`, and
 * `$defs/environment` states none for `dateTime` / `weatherPreset`. `eden_env`'s `FLOW_DEFAULT_*`
 * constants are NOT those defaults — they mirror the four literals `mission::flatten`'s `ModFlow`
 * splices in, which is a compiler fallback, not a schema declaration. Substituting them here would be
 * precisely the second source of truth, so `no_flow_constant_is_passed_off_as_a_schema_default` bans
 * them from this block by name.
 */

/// `mission.schema.json`, embedded — the ONE source of every default this view reports.
///
/// `eden_zones.rs` embeds the same path for the same reason (its `MISSION_SCHEMA`, whose header
/// carries the full argument: `$defs/zoneRules` is `additionalProperties: false` precisely so its
/// consumers would not each invent a copy). That const is private to a file this slice does not own,
/// so this is a second `include_str!` of THE SAME BYTES — not a second vocabulary. Both point at
/// `packages/tbd-schema/schema/mission.schema.json`; change the schema and both move together.
const MISSION_SCHEMA_JSON: &str =
    include_str!("../../../../packages/tbd-schema/schema/mission.schema.json");

/// Where a row's default came from — and, for the two negative cases, why there isn't one.
///
/// Exactly one variant carries a VALUE, and [`schema_default`] is the only place it is built. That is
/// the invariant the whole block rests on: a default cannot enter this view except through a schema
/// read.
#[derive(Clone, Debug, PartialEq)]
pub enum SettingDefault {
    /// The schema declares `default` at `pointer`. The value is the schema's, verbatim.
    Schema {
        pointer: String,
        value: serde_json::Value,
    },
    /// The schema declares the key at `pointer` but states NO `default`. Reported honestly — a key
    /// with no declared default has nothing to diff against, and guessing one is the defect.
    Declared { pointer: String },
    /// The key is not in `mission.schema.json` at all: an editor-local preference
    /// (`showHillshade` / `hillshadeOpacity` / `showGrid` — see `eden_env::CARRIED_ENV_KEYS`), which
    /// is authored into the document but never compiled. It is still an authored setting, so it is
    /// still listed; it just has no wire contract to compare with.
    NotInSchema,
}

impl SettingDefault {
    /// **The ONE constructor of a default that carries a value**, and it takes that value out of a
    /// schema node — never out of a Rust literal.
    ///
    /// `node` is the declared property; `resolved` is the same node with a one-hop `$ref` followed
    /// (`targetAlias` → `$defs/alias`), so a default declared on the referent is still found.
    ///
    /// Funnelling every construction through here is what makes the source pin possible:
    /// `a_default_value_is_built_in_exactly_one_place` asserts `Self::Schema` is written exactly once
    /// in the whole file, so a second site that manufactured a default from a hand-typed table would
    /// have to go through this function — which cannot invent one — or go red.
    #[must_use]
    fn from_schema_node(
        pointer: &str,
        node: &serde_json::Value,
        resolved: &serde_json::Value,
    ) -> Self {
        match node.get("default").or_else(|| resolved.get("default")) {
            Some(value) => Self::Schema {
                pointer: pointer.to_string(),
                value: value.clone(),
            },
            None => Self::Declared {
                pointer: pointer.to_string(),
            },
        }
    }
}

/// Who owns an authored setting — the answer the scatter destroys and this view restores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingOwner {
    /// The mission document itself (the `meta` / `meta.environment` bag behind Mission Settings).
    /// There is no entity to select, so these rows are not click-through targets.
    Mission,
    /// A placed entity. `id` is the document id the click routes on; `label` is what the row shows.
    Entity {
        kind: &'static str,
        id: String,
        label: String,
    },
}

impl SettingOwner {
    /// The id a row click routes to selection, if this owner is an entity at all.
    #[must_use]
    pub fn subject_id(&self) -> Option<&str> {
        match self {
            Self::Mission => None,
            Self::Entity { id, .. } => Some(id.as_str()),
        }
    }

    /// The owner column's text.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Mission => "Mission (this document)".to_string(),
            Self::Entity { kind, label, .. } => format!("{kind} — {label}"),
        }
    }
}

/// One authored setting: the key as the wire spells it, who owns it, what was authored, and what the
/// schema declares. Nothing here is editable — see constraint (1) in the block header.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingRow {
    /// The schema property name. This IS the wire key, never re-spelled.
    pub key: String,
    pub owner: SettingOwner,
    /// The authored value, verbatim from the document.
    pub value: serde_json::Value,
    pub default: SettingDefault,
}

/// Where a row sits relative to its declared default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffState {
    /// A default is declared and the authored value differs from it.
    Differs,
    /// A default is declared and the authored value equals it.
    Matches,
    /// No default is declared, so the question cannot be answered. NOT a synonym for "matches" —
    /// collapsing the two is how a view starts claiming a mission is at its defaults when nobody
    /// knows.
    Unknown,
}

impl SettingRow {
    #[must_use]
    pub fn diff_state(&self) -> DiffState {
        match &self.default {
            SettingDefault::Schema { value, .. } => {
                if values_agree(&self.value, value) {
                    DiffState::Matches
                } else {
                    DiffState::Differs
                }
            }
            SettingDefault::Declared { .. } | SettingDefault::NotInSchema => DiffState::Unknown,
        }
    }

    /// The diff-from-default filter's predicate: keep every row that is NOT *provably* at its
    /// declared default.
    ///
    /// [`DiffState::Unknown`] rows are KEPT. Hiding a row whose default nobody declared would be the
    /// view asserting something it cannot know — the same lie as an invented default, told by
    /// omission. They render with their state named instead.
    #[must_use]
    pub fn survives_diff_filter(&self) -> bool {
        self.diff_state() != DiffState::Matches
    }
}

/// Numeric-aware value comparison. `120` (schema, integer) and `120.0` (authored through a number
/// control) are the same setting; `serde_json::Value`'s derived `PartialEq` would call them different
/// and paint a row as "changed" that nobody changed.
#[must_use]
fn values_agree(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => (x - y).abs() <= f64::EPSILON * x.abs().max(y.abs()).max(1.0),
        _ => a == b,
    }
}

/// Where each key the Mission Settings dialog authors is DECLARED in `mission.schema.json`.
///
/// **This is a table of LOCATIONS, never of values.** The pointer is how the reader finds the
/// declaration; the `default` it finds there is the schema's. A key absent from this table is not
/// dropped — [`aggregate_settings`] still emits it, as [`SettingDefault::NotInSchema`], because the
/// walk is over the DOCUMENT and this table only translates. That is what stops a future
/// `author_env` key from silently vanishing out of the aggregation.
///
/// The editor's key spellings are its own (`time`, `weather`), and the compiled document's are the
/// schema's (`dateTime`, `weatherPreset`) — `eden_env::CARRIED_ENV_KEYS` is the mapping's authority
/// and its third column says which compiled path each becomes. `every_declared_pointer_resolves`
/// fails if any pointer here stops resolving in the schema.
const MISSION_SETTING_POINTERS: &[(&str, &str)] = &[
    ("terrain", "#/$defs/meta/properties/terrain"),
    ("time", "#/$defs/environment/properties/dateTime"),
    ("weather", "#/$defs/environment/properties/weatherPreset"),
    ("briefingSeconds", "#/$defs/flow/properties/briefingSeconds"),
    (
        "safeStartSeconds",
        "#/$defs/flow/properties/safeStartSeconds",
    ),
    (
        "timeLimitSeconds",
        "#/$defs/flow/properties/timeLimitSeconds",
    ),
    ("jip", "#/$defs/flow/properties/jip"),
];

/// The `$defs/zoneRules` property a zone rule key is declared by. The rules vocabulary is closed
/// (`additionalProperties: false`, T-241), so this is a formatted pointer rather than a table.
#[must_use]
fn zone_rule_pointer(key: &str) -> String {
    format!("#/$defs/zoneRules/properties/{key}")
}

/// The schema location declared for a mission-level key, if this view knows one.
#[must_use]
fn mission_setting_pointer(key: &str) -> Option<&'static str> {
    MISSION_SETTING_POINTERS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, p)| *p)
}

/// **The only place a default value enters this view.** Reads `pointer` out of the parsed schema and
/// reports what it finds there — a declared `default`, a declaration with none, or no declaration.
///
/// Resolves a one-hop `$ref` into `#/$defs/*` for the same reason `eden_zones::resolve_ref` does:
/// `targetAlias` is declared as a `$ref` to `$defs/alias`, and a reader that ignored `$ref` would
/// report "not in the schema" for a key the schema very much declares.
#[must_use]
fn schema_default(schema: Option<&serde_json::Value>, pointer: &str) -> SettingDefault {
    let Some(schema) = schema else {
        return SettingDefault::NotInSchema;
    };
    let Some(node) = schema.pointer(pointer.trim_start_matches('#')) else {
        return SettingDefault::NotInSchema;
    };
    let resolved = node
        .get("$ref")
        .and_then(serde_json::Value::as_str)
        .filter(|r| r.starts_with("#/$defs/"))
        .and_then(|r| schema.pointer(r.trim_start_matches('#')))
        .unwrap_or(node);
    SettingDefault::from_schema_node(pointer, node, resolved)
}

/// The embedded schema, parsed. `None` only if the committed bytes stop being JSON, which
/// `the_view_and_the_schema_agree_key_for_key` fails loudly on rather than rendering a defaults-free
/// list that looks like "this mission authors nothing standard".
#[must_use]
fn mission_schema() -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(MISSION_SCHEMA_JSON).ok()
}

/// **Every authored setting in the document, whoever owns it.**
///
/// Takes the document's `small_maps_json()` root and returns one row per authored key. Pure — no
/// signals, no engine, no `cfg`, so `cargo test -p website-frontend` runs it natively on the same
/// code the dialog renders.
///
/// **The walk is over the DOCUMENT, not over a list of keys.** Every key present in
/// `meta.environment` becomes a row whether or not [`MISSION_SETTING_POINTERS`] knows it, and every
/// key present in a zone's `rules` becomes a row whether or not `$defs/zoneRules` still declares it.
/// A key this file has never heard of is reported as [`SettingDefault::NotInSchema`] rather than
/// skipped — an aggregation that can silently omit a setting is the defect the ticket names, not a
/// tidier list.
///
/// Order is deterministic (the pointer table's order first, then unlisted keys sorted; then zones by
/// id, rules by key) so a test can pin it and an author's eye can find the same row twice.
#[must_use]
pub fn aggregate_settings(root: &serde_json::Value) -> Vec<SettingRow> {
    let schema = mission_schema();
    let schema = schema.as_ref();
    let mut rows: Vec<SettingRow> = Vec::new();

    let mission_row = |key: &str, value: &serde_json::Value| SettingRow {
        key: key.to_string(),
        owner: SettingOwner::Mission,
        value: value.clone(),
        default: match mission_setting_pointer(key) {
            Some(p) => schema_default(schema, p),
            None => SettingDefault::NotInSchema,
        },
    };

    let meta = root.get("meta");
    // `meta.terrain` is the one document setting that does not ride the `environment` bag.
    if let Some(v) = meta.and_then(|m| m.get("terrain")) {
        rows.push(mission_row("terrain", v));
    }
    if let Some(env) = meta
        .and_then(|m| m.get("environment"))
        .and_then(serde_json::Value::as_object)
    {
        for (key, _) in MISSION_SETTING_POINTERS {
            if let Some(v) = env.get(*key) {
                rows.push(mission_row(key, v));
            }
        }
        let mut unlisted: Vec<&String> = env
            .keys()
            .filter(|k| mission_setting_pointer(k).is_none())
            .collect();
        unlisted.sort();
        for key in unlisted {
            rows.push(mission_row(key, &env[key]));
        }
    }

    // `zone.rules` — the settings that already live on placed entities, and the reason this view
    // exists before the scatter gets any wider.
    if let Some(zones) = root.get("zonesById").and_then(serde_json::Value::as_object) {
        let mut ids: Vec<&String> = zones.keys().collect();
        ids.sort();
        for id in ids {
            let zone = &zones[id];
            let Some(rules) = zone.get("rules").and_then(serde_json::Value::as_object) else {
                continue;
            };
            // The zone's own name if it has one, else its type — the same fallback
            // `TBD_BriefingService.PrettyZoneTitle` makes, and the id last so a row is never faceless.
            let label = zone
                .get("label")
                .and_then(serde_json::Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .or_else(|| zone.get("type").and_then(serde_json::Value::as_str))
                .unwrap_or(id.as_str())
                .to_string();
            let mut keys: Vec<&String> = rules.keys().collect();
            keys.sort();
            for key in keys {
                rows.push(SettingRow {
                    key: key.clone(),
                    owner: SettingOwner::Entity {
                        kind: "Zone",
                        id: id.clone(),
                        label: label.clone(),
                    },
                    value: rules[key].clone(),
                    default: schema_default(schema, &zone_rule_pointer(key)),
                });
            }
        }
    }

    rows
}

/// A JSON value as the row shows it: a string without its quotes, everything else as written.
/// Presentation only — `value` itself is what the document holds.
#[must_use]
pub fn fmt_setting_value(v: &serde_json::Value) -> String {
    match v.as_str() {
        Some("") => "(empty)".to_string(),
        Some(s) => s.to_string(),
        None => v.to_string(),
    }
}

/// The default column's text for one row.
#[must_use]
pub fn fmt_setting_default(d: &SettingDefault) -> String {
    match d {
        SettingDefault::Schema { value, .. } => fmt_setting_value(value),
        SettingDefault::Declared { .. } => NO_DEFAULT_DECLARED.to_string(),
        SettingDefault::NotInSchema => NOT_A_SCHEMA_KEY.to_string(),
    }
}

/// Shown where a default would be for a key the schema declares WITHOUT one.
///
/// Pinned copy. The temptation this stands against is filling the cell with the editor's own
/// fallback — `eden_env::FLOW_DEFAULT_TIMELIMIT_S` and friends, which mirror what `ModFlow` splices
/// in — and calling it "the default". It is not: it is a compiler fallback, it lives in Rust, and the
/// schema says nothing. A cell that says so is worth more than a number that is nearly true.
pub const NO_DEFAULT_DECLARED: &str = "no default declared";

/// Shown for an authored key `mission.schema.json` does not declare at all — the editor-local render
/// prefs. It is a real authored setting (it is in the document, it takes an undo step), it simply has
/// no wire contract, so there is nothing to diff it against.
pub const NOT_A_SCHEMA_KEY: &str = "not a schema key (editor-local)";

/// What the view says when a row's owner could not be selected.
///
/// **The honest half of constraint (2).** T-655's router
/// ([`crate::validation_panel::register_select_by_id`], registered from `mission_editor.rs`) resolves
/// a subject id against the slot SoA and `vehiclesById`. A zone is in neither — zone selection is a
/// panel-local signal in `eden_dock_right.rs` — so routing a zone id returns `false` today. Both files
/// are outside this slice's `owns`, so the view reuses the SHIPPED router rather than standing up a
/// second selection path, and when the router declines it says which panel holds the zone instead of
/// swallowing the click. A dead click that explains itself is recoverable; a silent one is not.
pub const OWNER_UNRESOLVED_NOTE: &str = "The editor's click-to-select router resolves slots and \
                                         vehicles; a zone is selected in the Zones panel of the \
                                         right-hand dock.";

/// The header copy. Says what this view is (one list, read-only) and — load-bearing — that a default
/// cell is the SCHEMA's, so an author reading a diff knows what it was measured against.
pub const ALL_SETTINGS_NOTE: &str =
    "Every setting authored in this mission, whichever entity owns \
                                     it. Read-only: change a value where it is authored. Defaults \
                                     are read from mission.schema.json, so a key the schema states \
                                     no default for is shown as such rather than guessed.";

/// The document's `small_maps_json()` root, or `None` before the editor's doc host has mounted.
/// The one impure hop between the live document and [`aggregate_settings`].
#[cfg(target_arch = "wasm32")]
#[must_use]
fn document_root() -> Option<serde_json::Value> {
    let handle = crate::mission_history::doc_handle()?;
    let doc = handle.borrow();
    let core = doc.as_ref()?;
    serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok()
}

/// T-688 — the **All Settings** dialog: every authored setting in this mission, in one list.
///
/// Mounted as a sibling of [`MissionSettingsDialog`] and opened by [`open_all_settings`] from that
/// dialog's pointer row — the same idiom, and for the same reason, as [`EditorPreferencesDialog`]:
/// the gear/menu that would otherwise route it lives in files this slice does not own.
///
/// Renders no DOM while closed. READ-ONLY by construction; see the block header.
#[component]
fn AllSettingsDialog(open: RwSignal<bool>, doc_tick: RwSignal<u64>) -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked() && ev.key() == "Escape" {
                open.set(false);
            }
        });
        on_cleanup(move || esc.remove());
    }
    // The diff-from-default filter. Off by default: the ticket's view is "every authored setting",
    // and the filter narrows it — an author who opens the list to "show me everything" and is shown
    // a subset has been answered a question they did not ask.
    let only_diffs = RwSignal::new(false);
    move || {
        if !open.get() {
            return None;
        }
        let _ = doc_tick.get(); // re-aggregate on undo/redo while open
        Some(view! {
            <div
                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
                on:click=move |_| open.set(false)
            ></div>
            <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-3xl -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200">
                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                    <div class="min-w-0">
                        <h2 class="text-headline-sm text-on-surface">"All Settings"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            {ALL_SETTINGS_NOTE}
                        </p>
                    </div>
                    <button
                        type="button"
                        aria-label="Close"
                        on:click=move |_| open.set(false)
                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <MaterialIcon name="close" />
                    </button>
                </div>
                <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                    {render_all_settings_body(only_diffs)}
                </div>
            </div>
        })
    }
}

/// T-688 — the body of [`AllSettingsDialog`]: the filter toggle, then one row per authored setting.
///
/// **Read-only by construction.** The only interactive elements here are the filter toggle and the
/// per-row click-through — both `<button>`s, neither of which touches the document. There is no
/// `<input>`, no `<select>`, no `author_env` and no `editor_ops` mutator anywhere in this function,
/// and `the_aggregated_view_is_not_a_second_editing_surface` fails if one appears.
///
/// Inert on the native view shell (no document), exactly like [`render_flow_section`].
fn render_all_settings_body(only_diffs: RwSignal<bool>) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = only_diffs;
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let hint = "text-label-sm normal-case text-outline";
        let root = document_root().unwrap_or(serde_json::Value::Null);
        let all = aggregate_settings(&root);
        let total = all.len();
        let filtered = only_diffs.get();
        let rows: Vec<SettingRow> = if filtered {
            all.into_iter()
                .filter(SettingRow::survives_diff_filter)
                .collect()
        } else {
            all
        };
        let shown = rows.len();
        let toasts = crate::toast::use_toasts();

        // The filter is a toggle button, not a checkbox: the T-668 state vocabulary gives a
        // toggled-on control a lighter plate + a 1px dark top border ([`TOGGLED_PLATE`]), distinct
        // BY CONSTRUCTION from any hover fill — and a `<button>` keeps this surface free of the
        // `<input>` an editing control would need, which is the read-only pin's needle.
        let toggle_class = if filtered {
            format!(
                "rounded-md px-2.5 py-1.5 text-label-md {} {}",
                crate::eden_layout::TOGGLED_PLATE,
                crate::eden_layout::HOVER_FILL
            )
        } else {
            format!(
                "rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface-variant {}",
                crate::eden_layout::HOVER_FILL
            )
        };

        let body = if rows.is_empty() {
            let empty = if total == 0 {
                "This mission authors no settings yet."
            } else {
                "Every authored setting is at the default its schema declares."
            };
            view! { <p class=hint data-all-settings-empty>{empty}</p> }.into_any()
        } else {
            rows.into_iter()
                .map(|r| setting_row_view(r, toasts))
                .collect::<Vec<_>>()
                .into_any()
        };

        view! {
            <div class="flex flex-col gap-3" data-all-settings>
                <div class="flex items-center justify-between gap-3">
                    <span class=hint>
                        {format!("{shown} of {total} authored settings")}
                    </span>
                    <button
                        type="button"
                        class=toggle_class
                        aria-pressed=filtered.to_string()
                        data-all-settings-filter=filtered.to_string()
                        title="Hide every setting that is provably at the default its schema declares. Rows whose schema declares no default stay, because nothing can prove those are unchanged."
                        on:click=move |_| only_diffs.update(|v| *v = !*v)
                    >
                        "Changed from default"
                    </button>
                </div>
                <div class="flex flex-col gap-1">{body}</div>
            </div>
        }
        .into_any()
    }
}

/// One aggregated row: key, owning entity, authored value, schema default — and a click that routes
/// to the owner through T-655's shipped router.
#[cfg(target_arch = "wasm32")]
fn setting_row_view(row: SettingRow, toasts: crate::toast::Toasts) -> AnyView {
    let state = row.diff_state();
    let subject = row.owner.subject_id().map(ToString::to_string);
    let selectable = subject.is_some();
    let owner_label = row.owner.label();
    let click_id = subject.clone().unwrap_or_default();
    let click_owner = owner_label.clone();
    let key_label = crate::eden_zones::humanize_key(&row.key);
    let value_text = fmt_setting_value(&row.value);
    let default_text = fmt_setting_default(&row.default);
    // The pointer the default was READ FROM, on the row itself. An author who doubts a diff can go
    // look at the same line of the schema the view did.
    let source = match &row.default {
        SettingDefault::Schema { pointer, .. } | SettingDefault::Declared { pointer } => {
            pointer.clone()
        }
        SettingDefault::NotInSchema => String::new(),
    };
    let (badge, badge_class) = match state {
        DiffState::Differs => ("changed", "bg-tactical-yellow/20 text-tactical-yellow"),
        DiffState::Matches => ("default", "bg-primary/15 text-on-surface-variant"),
        DiffState::Unknown => ("no default", "bg-surface-variant/40 text-outline"),
    };
    let cursor = if selectable {
        "cursor-pointer hover:bg-primary/10"
    } else {
        "cursor-default"
    };
    view! {
        <button
            type="button"
            class=format!(
                "grid w-full grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)_minmax(0,0.9fr)_minmax(0,0.9fr)_auto] items-baseline gap-3 rounded px-2 py-1.5 text-left outline-none transition-colors {cursor}",
            )
            data-all-settings-row=row.key.clone()
            data-owner-id=subject.clone().unwrap_or_default()
            data-diff=format!("{state:?}").to_lowercase()
            data-default-source=source.clone()
            data-selectable=selectable.to_string()
            title=source
            on:click=move |_| {
                // Constraint (2): route through T-655's SHIPPED router, never a second selection
                // path. A row with no entity to select does not pretend to have one.
                if selectable && !crate::validation_panel::route_select_by_subject_id(&click_id) {
                    toasts.message(format!("{click_owner} — {OWNER_UNRESOLVED_NOTE}"));
                }
            }
        >
            <span class="min-w-0 truncate text-label-md text-on-surface" title=row.key.clone()>
                {key_label}
            </span>
            <span class="min-w-0 truncate text-label-sm text-on-surface-variant">
                {owner_label}
            </span>
            <span class="min-w-0 truncate font-mono text-code-md text-on-surface">
                {value_text}
            </span>
            <span class="min-w-0 truncate font-mono text-code-md text-outline">
                {default_text}
            </span>
            <span class=format!("shrink-0 rounded px-1.5 py-0.5 text-label-sm {badge_class}")>
                {badge}
            </span>
        </button>
    }
    .into_any()
}

// T-691 — source-scan pins for the editor-vs-document split. These search the file they live in, so
// every needle is assembled from fragments at run time (the `class_r_scrub` house rule): a needle
// spelled out contiguously here would put itself in the haystack, and an absence check could then
// never legitimately fail. `live_code` blanks string literals AND cuts this test module, so a needle
// that means "a real call" cannot false-green off a doc-comment, a label string, or this test.
#[cfg(test)]
mod t691_editor_prefs_split {
    use crate::arsenal::class_r_scrub::{live_code, only_body};

    /// Fragment-assembled `world_layer_prefs` store-call needles. If any of these appear inside the
    /// Mission Settings (document) dialog, the editor-local half did not actually move.
    fn store_call_needles() -> Vec<String> {
        vec![
            format!("save{}", "_prefs"),
            format!("save{}", "_basemap_view"),
            format!("apply{}", "_basemap_view"),
            format!("refresh{}", "_world_layers"),
            format!("world{}", "_layer_prefs"),
        ]
    }

    /// Separation pin (document half): after the move, no world-layer/basemap store call survives in
    /// `render_prefs_section` — the render-pref block that stays in Mission Settings holds only the
    /// hillshade/grid document keys and the pointer row. Perturbation that this catches: leaving (or
    /// pasting back) the basemap buttons or the 12 layer toggles into Mission Settings.
    #[test]
    fn mission_settings_render_prefs_holds_no_world_layer_toggles() {
        let src = live_code(include_str!("eden_settings.rs"));
        let body = only_body(&src, &format!("fn render{}", "_prefs_section"));
        for needle in store_call_needles() {
            assert!(
                !body.contains(&needle),
                "T-691: `{needle}` must not remain in render_prefs_section — the editor-local \
                 basemap/world-layer controls moved to EditorPreferencesDialog"
            );
        }
        // The document keys it DOES keep: hillshade + grid still author through the env gate.
        let author = format!("author{}", "_env");
        assert!(
            body.contains(&author),
            "T-691: render_prefs_section must still author the hillshade/grid document keys"
        );
    }

    /// Separation pin (document half, dialog scope): the whole Mission Settings dialog body carries
    /// no world-layer store call either (guards against the toggles being reintroduced directly in
    /// the component rather than via the helper).
    #[test]
    fn mission_settings_dialog_body_holds_no_store_calls() {
        let src = live_code(include_str!("eden_settings.rs"));
        let body = only_body(&src, &format!("fn Mission{}", "SettingsDialog"));
        for needle in store_call_needles() {
            assert!(
                !body.contains(&needle),
                "T-691: `{needle}` must not appear in MissionSettingsDialog — editor-local prefs \
                 live only in EditorPreferencesDialog"
            );
        }
    }

    /// Separation pin (editor-local half): EditorPreferencesDialog's content contains NO
    /// `author_env` write — every control there is a localStorage editor preference, never a
    /// mission-document key. Perturbation this catches: wiring a hillshade/grid (or any
    /// document-key) control into the preferences body. The content lives in
    /// `render_editor_prefs_body`, sliced out here.
    #[test]
    fn editor_preferences_dialog_writes_no_author_env() {
        let src = live_code(include_str!("eden_settings.rs"));
        let body = only_body(&src, &format!("fn render{}", "_editor_prefs_body"));
        let author = format!("author{}", "_env");
        assert!(
            !body.contains(&author),
            "T-691: EditorPreferencesDialog must contain no `{author}` — it is the editor-local, \
             per-user surface, not a mission-document editor"
        );
        // And it MUST carry the moved editor-local controls (the move actually happened).
        assert!(
            body.contains(&format!("save{}", "_basemap_view"))
                && body.contains(&format!("save{}", "_prefs")),
            "T-691: the basemap + world-layer store writes must live in the preferences body"
        );
    }

    /// Dialog-opens pin: the opener is wired end to end without leaving `owns`. The pointer row in
    /// `render_prefs_section` calls `open_editor_preferences`; that fn arms the parked signal; and
    /// `MissionSettingsDialog` both registers the signal (`set_prefs_signal`) and mounts the
    /// preferences dialog on it. Perturbation this catches: dropping the mount, the registration, or
    /// the pointer-row call.
    #[test]
    fn editor_preferences_opener_is_wired() {
        let src = live_code(include_str!("eden_settings.rs"));
        let opener = format!("open{}", "_editor_preferences");
        let mount = format!("Editor{}", "PreferencesDialog");
        let register = format!("set{}", "_prefs_signal");

        // (a) the opener fn arms a boolean signal to true.
        let opener_body = only_body(&src, &format!("fn {opener}"));
        assert!(
            opener_body.contains(".set(true)"),
            "T-691: {opener} must set the parked open signal to true"
        );

        // (b) the pointer row in the document dialog calls the opener.
        let prefs_body = only_body(&src, &format!("fn render{}", "_prefs_section"));
        assert!(
            prefs_body.contains(&format!("{opener}()")),
            "T-691: the Mission Settings pointer row must call {opener}()"
        );

        // (c) MissionSettingsDialog registers the signal and mounts the preferences dialog on it.
        let dlg_body = only_body(&src, &format!("fn Mission{}", "SettingsDialog"));
        assert!(
            dlg_body.contains(&register),
            "T-691: MissionSettingsDialog must register the prefs signal via {register}"
        );
        assert!(
            dlg_body.contains(&mount) && dlg_body.contains("prefs_open"),
            "T-691: MissionSettingsDialog must mount {mount} bound to prefs_open"
        );

        // (d) the preferences dialog is a real component that gates on its `open` signal.
        let comp_body = only_body(&src, &format!("fn {mount}"));
        assert!(
            comp_body.contains("open.get()"),
            "T-691: {mount} must render no DOM while closed (gate on open.get())"
        );
    }
}

// T-694 — mission shape. Two kinds of pin: pure unit tests over the helpers, and source scans over
// [`render_shape_section`] / [`ShapeMirror`]. The scans exist because the interesting claims are
// *absences* — no minimum, no clamp, no capacity rule — and an absence cannot be observed by calling
// a function. Same house rules as the T-691 module above: needles are assembled from fragments, and
// `live_code` blanks string literals and cuts every test module, so a needle meaning "a real call"
// cannot false-green off a doc comment or a label.
#[cfg(test)]
mod t694_mission_shape {
    use super::{
        game_mode_failure_message, is_known_game_mode, is_row_id, PlayerCount, GAME_MODES,
        PLAYER_COUNT_DISAGREE_NOTE, SLOTS_PLACED_NOTE,
    };
    use crate::arsenal::class_r_scrub::{live_code, only_body};

    /// The select's table is the server's enum. `handlers/missions.rs::valid_game_mode` maps exactly
    /// `pve_coop` / `pvp` / `zeus` and 400s the rest, so drift here ships a control that can only
    /// fail. Every entry also needs a label — an option with a blank face is not a choice.
    #[test]
    fn game_mode_table_is_the_patch_enum() {
        let values: Vec<&str> = GAME_MODES.iter().map(|(v, _)| *v).collect();
        assert_eq!(values, vec!["pve_coop", "pvp", "zeus"]);
        for (value, label) in GAME_MODES {
            assert!(
                !label.trim().is_empty(),
                "T-694: game mode {value} has no label"
            );
            assert!(is_known_game_mode(value));
        }
        for bogus in ["", "PVP", "coop", "training", "pve"] {
            assert!(
                !is_known_game_mode(bogus),
                "T-694: {bogus:?} is not a game mode the PATCH accepts"
            );
        }
    }

    /// The row guard. The editor mounts on synthetic ids too (`draft`, the gate's smoke id), where a
    /// shape GET/PATCH is a guaranteed failure, so the id must be checked before either goes out.
    #[test]
    fn row_id_guard_rejects_the_synthetic_editor_ids() {
        assert!(is_row_id("3f2504e0-4f89-11d3-9a0c-0305e82c3301"));
        for not_a_row in [
            "",
            "draft",
            "smoke",
            "3f2504e0-4f89-11d3-9a0c-0305e82c330", // too short
            "3f2504e0-4f89-11d3-9a0c-0305e82c33011", // too long
            "3f2504e04f8911d39a0c0305e82c3301aaaa", // right length, no dashes
            "zzzzzzzz-4f89-11d3-9a0c-0305e82c3301", // not hex
        ] {
            assert!(
                !is_row_id(not_a_row),
                "T-694: {not_a_row:?} must not be treated as a mission row id"
            );
        }
    }

    /// The disagreement rule: both numbers are reported, and the explanatory note appears **only**
    /// when they differ. An author who placed exactly `max_players` slots needs no essay; an author
    /// whose two numbers have drifted needs to be told nothing here reconciles them.
    #[test]
    fn player_count_flags_disagreement_and_nothing_else() {
        assert!(PlayerCount {
            placed: 84,
            declared: Some(64)
        }
        .disagrees());
        assert!(PlayerCount {
            placed: 0,
            declared: Some(64)
        }
        .disagrees());
        assert!(!PlayerCount {
            placed: 64,
            declared: Some(64)
        }
        .disagrees());
        // No row means nothing to disagree WITH — the block shows the derived count alone.
        assert!(!PlayerCount {
            placed: 84,
            declared: None
        }
        .disagrees());
    }

    /// Copy pin. [`SLOTS_PLACED_NOTE`] must keep saying that a slot is not a player and that this is
    /// not a server limit — that sentence is the whole of the operator's open question, and a later
    /// tidy-up that trims it turns an honest report back into an implied guarantee.
    #[test]
    fn slots_note_refuses_to_claim_a_server_capacity() {
        let note = SLOTS_PLACED_NOTE.to_lowercase();
        assert!(
            note.contains("seat, not a player"),
            "T-694: the slots note must say a slot is a seat, not a player"
        );
        assert!(
            note.contains("server"),
            "T-694: the slots note must say this is not what the server will hold"
        );
        let disagree = PLAYER_COUNT_DISAGREE_NOTE.to_lowercase();
        assert!(
            disagree.contains("neither is enforced"),
            "T-694: the disagreement note must say neither figure is enforced here"
        );
    }

    /// A refused PATCH must name the 403 case separately (retrying cannot help a non-author) and must
    /// tell the author the control has been put back — because it has.
    #[test]
    fn refused_game_mode_patch_explains_itself() {
        let forbidden = game_mode_failure_message(&(403, None));
        assert!(forbidden.to_lowercase().contains("author"));
        assert!(forbidden.to_lowercase().contains("put back"));
        // `api_error_message` sentence-cases what the server said, so compare case-insensitively.
        let other = game_mode_failure_message(&(500, Some("boom".into())));
        assert!(
            other.to_lowercase().contains("boom"),
            "T-694: a non-403 must name what the server said, got {other:?}"
        );
        assert!(other.to_lowercase().contains("put back"));
    }

    /// **The derived count is derived.** The shape section must read the live document's slot count
    /// rather than any stored figure. Perturbation this catches: swapping the call for the row's
    /// `max_players`, or for a hand-rolled counter.
    #[test]
    fn player_count_comes_from_the_document_slot_count() {
        let src = live_code(include_str!("eden_settings.rs"));
        let body = only_body(&src, &format!("fn render{}", "_shape_section"));
        assert!(
            body.contains(&format!("slot{}", "_count")),
            "T-694: the players figure must come from MissionDocCore's slot count"
        );
        assert!(
            body.contains(&format!("doc{}", "_handle")),
            "T-694: the slot count must be read from the live document handle"
        );
    }

    /// **The absences.** This slice was told not to answer the "what is the real cap?" question, so
    /// the section must invent no minimum, clamp nothing to a server limit, and reduce the two
    /// figures to neither. Perturbation this catches: a well-meaning `min(128)`, a `min_players`
    /// control, or a `max(placed, declared)` that quietly picks a winner.
    #[test]
    fn shape_section_invents_no_player_limit() {
        let src = live_code(include_str!("eden_settings.rs"));
        let body = only_body(&src, &format!("fn render{}", "_shape_section"));
        for banned in [
            format!("min{}", "_players"),
            "clamp".to_string(),
            ".min(".to_string(),
            ".max(".to_string(),
            "128".to_string(),
        ] {
            assert!(
                !body.contains(&banned),
                "T-694: `{banned}` must not appear in the shape section — the authoritative player \
                 cap is an open question and this dialog reports, it does not decide"
            );
        }
        // Whole-file: no `min_players` anywhere. There is no such column, model field or DTO key,
        // and adding one is the migration this ticket was reinterpreted to avoid.
        assert!(
            !src.contains(&format!("min{}", "_players")),
            "T-694: min_players exists nowhere in the platform — do not introduce it here"
        );
    }

    /// **Game mode is editable after creation** — the gap the ticket names — and it reaches the row
    /// by PATCH, not by an `author_env` document write. Perturbation this catches: wiring the select
    /// into the document (where nothing would read it) or dropping the mirror call entirely.
    #[test]
    fn game_mode_select_patches_the_missions_row() {
        let src = live_code(include_str!("eden_settings.rs"));
        let setter = format!("set{}", "_game_mode");

        // (a) the section's control calls the setter and offers the table's options.
        let body = only_body(&src, &format!("fn render{}", "_shape_section"));
        assert!(
            body.contains(&format!("{setter}(")),
            "T-694: the game mode select must call {setter}"
        );
        assert!(
            body.contains(&format!("GAME{}", "_MODES")),
            "T-694: the options must come from the shared game-mode table"
        );
        // (b) it is a ROW write, not a document write: the row half of this dialog must never reach
        // for the env gate, or the change would land somewhere no compile reads.
        assert!(
            !body.contains(&format!("author{}", "_env")),
            "T-694: game mode is a `missions` row column, not a meta.environment key"
        );

        // (c) the setter itself PATCHes /missions/:id with the game_mode column.
        let setter_body = only_body(&src, &format!("fn {setter}"));
        assert!(
            setter_body.contains(&format!("api{}", "_patch")),
            "T-694: {setter} must PATCH the mission row"
        );
        assert!(
            setter_body.contains(&format!("is{}", "_row_id")),
            "T-694: {setter} must refuse synthetic editor ids before hitting the wire"
        );
    }
}

// T-688 — the aggregated settings view. Two families of pin:
//
//   * BEHAVIOURAL — `the_view_and_the_schema_agree_key_for_key` re-reads `mission.schema.json`
//     independently of the production reader and compares, key for key, what the view reports about
//     every `$defs/zoneRules` property. It goes red on ANY disagreement, whatever the cause: a
//     hand-written table, a typo, a schema change the view did not follow. That is the ticket's
//     explicit requirement, and it is the one that survives a refactor of everything below it.
//   * SOURCE-SCAN — the read-only constraint, the click-through reuse, and the single construction
//     site for a default value. Same `class_r_scrub` house rules as the T-691/T-694 modules above:
//     needles are assembled from fragments so this module cannot become its own haystack.
#[cfg(test)]
mod t688_aggregated_settings {
    use super::{
        aggregate_settings, fmt_setting_default, fmt_setting_value, mission_setting_pointer,
        values_agree, DiffState, SettingDefault, SettingOwner, SettingRow, ALL_SETTINGS_NOTE,
        MISSION_SCHEMA_JSON, MISSION_SETTING_POINTERS, NOT_A_SCHEMA_KEY, NO_DEFAULT_DECLARED,
        OWNER_UNRESOLVED_NOTE,
    };
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};
    use serde_json::json;

    fn schema() -> serde_json::Value {
        serde_json::from_str(MISSION_SCHEMA_JSON).expect(
            "T-688: the embedded mission.schema.json must parse — it is the only source of \
                     every default this view reports",
        )
    }

    fn zone_rule_props() -> serde_json::Map<String, serde_json::Value> {
        schema()["$defs"]["zoneRules"]["properties"]
            .as_object()
            .expect("T-688: $defs/zoneRules/properties")
            .clone()
    }

    /// **THE pin.** The view's default for every `$defs/zoneRules` key must be exactly what the
    /// schema declares there — read here a second time, independently, straight out of the committed
    /// bytes.
    ///
    /// This is the test the ticket asks for: it fails when the schema and the view disagree, which is
    /// the only observable symptom a second source of truth ever has. A hand-written table of
    /// defaults in Rust passes on the day it is written and goes red the first time the schema moves;
    /// a typo goes red immediately. Perturbation this catches: replacing the schema read with a
    /// literal for any single key.
    #[test]
    fn the_view_and_the_schema_agree_key_for_key() {
        let schema = schema();
        let props = zone_rule_props();
        assert!(
            props.len() >= 16,
            "T-688: $defs/zoneRules is the closed 16+ key vocabulary (T-241/T-685); found {}",
            props.len()
        );

        // A document authoring EVERY rule key on one zone. The authored values are deliberately
        // nonsense — what is under test is where the DEFAULT came from, not the value beside it.
        let mut rules = serde_json::Map::new();
        for key in props.keys() {
            rules.insert(key.clone(), json!("__authored_probe__"));
        }
        let doc = json!({
            "zonesById": {
                "zone-a": { "type": "objective_capture", "label": "Hilltop", "rules": rules }
            }
        });

        let rows = aggregate_settings(&doc);
        assert_eq!(
            rows.len(),
            props.len(),
            "T-688: every authored rule key must produce exactly one row — an aggregation that can \
             drop a setting is the defect this view exists to remove"
        );

        let mut with_default = 0usize;
        let mut without_default = 0usize;
        for (key, declared) in &props {
            let row = rows
                .iter()
                .find(|r| r.key == *key)
                .unwrap_or_else(|| panic!("T-688: no row for authored rule key `{key}`"));
            // One-hop `$ref`, exactly as the schema declares it (`targetAlias` → `$defs/alias`).
            let resolved = declared
                .get("$ref")
                .and_then(serde_json::Value::as_str)
                .and_then(|r| schema.pointer(r.trim_start_matches('#')))
                .unwrap_or(declared);
            let want = declared.get("default").or_else(|| resolved.get("default"));
            match (&row.default, want) {
                (SettingDefault::Schema { value, pointer }, Some(expected)) => {
                    assert_eq!(
                        value, expected,
                        "T-688: the view and the schema DISAGREE about `{key}`'s default — the view \
                         says {value}, mission.schema.json says {expected}"
                    );
                    assert!(
                        pointer.ends_with(key.as_str()),
                        "T-688: `{key}`'s default must name the schema location it was read from, \
                         got {pointer:?}"
                    );
                    with_default += 1;
                }
                (SettingDefault::Declared { .. }, None) => without_default += 1,
                (got, want) => panic!(
                    "T-688: `{key}` — the view reports {got:?} but mission.schema.json declares \
                     default={want:?}"
                ),
            }
        }
        assert!(
            with_default >= 11,
            "T-688: $defs/zoneRules declares defaults on at least 11 keys; the view found \
             {with_default}"
        );
        assert!(
            without_default > 0,
            "T-688: some rule keys declare no default (holdSeconds, points, …) and the view must \
             report them as such rather than inventing one"
        );
    }

    /// The honest half of the same rule for the MISSION-level keys: `mission.schema.json` declares a
    /// `default` for NONE of them, so the view must say so.
    ///
    /// **This is where a second source of truth would have been most tempting.** `eden_env` holds
    /// `FLOW_DEFAULT_TIMELIMIT_S = 5400` and friends — but those mirror the literals
    /// `mission::flatten`'s `ModFlow` splices in, a COMPILER FALLBACK, not a schema declaration.
    /// Printing 5400 in a "schema default" column would be the view reporting a diff against a number
    /// the schema never stated. Perturbation this catches: exactly that substitution.
    #[test]
    fn no_flow_constant_is_passed_off_as_a_schema_default() {
        let doc = json!({
            "meta": {
                "terrain": "everon",
                "environment": {
                    "time": "06:00", "weather": "overcast",
                    "briefingSeconds": 600, "safeStartSeconds": 300,
                    "timeLimitSeconds": 900, "jip": "always"
                }
            }
        });
        let rows = aggregate_settings(&doc);
        for key in [
            "terrain",
            "time",
            "weather",
            "briefingSeconds",
            "safeStartSeconds",
            "timeLimitSeconds",
            "jip",
        ] {
            let row = rows
                .iter()
                .find(|r| r.key == key)
                .unwrap_or_else(|| panic!("T-688: no row for authored mission key `{key}`"));
            assert!(
                matches!(row.default, SettingDefault::Declared { .. }),
                "T-688: mission.schema.json declares NO default for `{key}` — the view must report \
                 that, not substitute one. Got {:?}",
                row.default
            );
            assert_eq!(fmt_setting_default(&row.default), NO_DEFAULT_DECLARED);
            assert_eq!(row.diff_state(), DiffState::Unknown);
            assert_eq!(row.owner, SettingOwner::Mission);
        }

        // …and the constants themselves are named nowhere in the aggregation or its rendering.
        let src = live_code(include_str!("eden_settings.rs"));
        let banned = format!("FLOW{}", "_DEFAULT_");
        for f in [
            format!("fn aggregate{}", "_settings"),
            format!("fn schema{}", "_default"),
            format!("fn render{}", "_all_settings_body"),
            format!("fn setting{}", "_row_view"),
            format!("fn fmt{}", "_setting_default"),
        ] {
            assert!(
                !only_body(&src, &f).contains(&banned),
                "T-688: `{banned}*` is a compiler fallback, not a schema default — it must not \
                 reach `{f}`"
            );
        }
    }

    /// **A default value is built in exactly ONE place**, and that place reads it out of a schema
    /// node. Any second construction site is a second source of truth by definition, so the count is
    /// pinned rather than trusted.
    ///
    /// Perturbation this catches: a `SettingDefault::Schema { … }` assembled from a hand-typed table
    /// anywhere else in the file.
    #[test]
    fn a_default_value_is_built_in_exactly_one_place() {
        let src = live_code(include_str!("eden_settings.rs"));
        let ctor = format!("Self::{} {{", "Schema");
        assert_eq!(
            src.matches(&ctor).count(),
            1,
            "T-688: the value-carrying default variant must be constructed exactly once (in \
             from_schema_node, out of a schema node). A second site is a second source of truth."
        );
        // …and that one site reads the schema's own `default` key rather than deciding anything.
        let lit = live_source(include_str!("eden_settings.rs"));
        let body = only_body(&lit, &format!("fn from{}", "_schema_node"));
        assert!(
            body.contains("\"default\""),
            "T-688: the one constructor must read the schema's `default` key"
        );
        // The reader that feeds it addresses the schema by JSON pointer — no key list of its own.
        let reader = only_body(&lit, &format!("fn schema{}", "_default"));
        assert!(
            reader.contains(".pointer("),
            "T-688: schema_default must locate a declaration by pointer in the embedded schema"
        );
    }

    /// Every schema location this view declares must actually resolve. A pointer that stopped
    /// resolving would silently downgrade a real wire key to "editor-local", which reads as "nothing
    /// to compare" — a lie by omission rather than by value.
    #[test]
    fn every_declared_pointer_resolves() {
        let schema = schema();
        for (key, pointer) in MISSION_SETTING_POINTERS {
            assert!(
                schema.pointer(pointer.trim_start_matches('#')).is_some(),
                "T-688: `{key}`'s declared location {pointer} no longer resolves in \
                 mission.schema.json"
            );
            assert_eq!(mission_setting_pointer(key), Some(*pointer));
        }
        // The zone-rule pointers are formatted, so one representative proves the shape.
        assert!(schema
            .pointer("/$defs/zoneRules/properties/graceSeconds")
            .is_some());
    }

    /// **The walk is over the DOCUMENT.** A key this file has never heard of still gets a row — as
    /// `NotInSchema`, never skipped. That is what stops a future `author_env` key, or a rule the
    /// schema later drops, from vanishing out of "every setting in this mission".
    ///
    /// Perturbation this catches: iterating a key table instead of the document.
    #[test]
    fn the_aggregation_walks_the_document_and_omits_nothing() {
        let doc = json!({
            "meta": { "environment": {
                "showGrid": true,
                "hillshadeOpacity": 0.4,
                "aKeyNobodyHasWrittenYet": 7
            } },
            "zonesById": { "z1": { "type": "boundary", "rules": { "notARuleAnyMore": 3 } } }
        });
        let rows = aggregate_settings(&doc);
        let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
        for key in [
            "showGrid",
            "hillshadeOpacity",
            "aKeyNobodyHasWrittenYet",
            "notARuleAnyMore",
        ] {
            assert!(
                keys.contains(&key),
                "T-688: `{key}` is authored in the document and must appear; got {keys:?}"
            );
            let row = rows.iter().find(|r| r.key == key).expect("row");
            assert_eq!(
                row.default,
                SettingDefault::NotInSchema,
                "T-688: `{key}` is not declared in mission.schema.json — say so, do not guess"
            );
            assert_eq!(fmt_setting_default(&row.default), NOT_A_SCHEMA_KEY);
        }
    }

    /// The diff-from-default filter keeps everything it cannot PROVE is unchanged. Hiding a row whose
    /// schema declares no default would be the view asserting a fact nobody has — the same defect as
    /// an invented default, told by omission.
    #[test]
    fn the_diff_filter_keeps_what_it_cannot_prove() {
        let doc = json!({
            "meta": { "environment": { "timeLimitSeconds": 900 } },
            "zonesById": { "z1": { "type": "objective_capture", "rules": {
                // `penalty`'s schema default is "warn"; `contestable`'s is true.
                "penalty": "warn",
                "contestable": false
            } } }
        });
        let rows = aggregate_settings(&doc);
        let by = |k: &str| rows.iter().find(|r| r.key == k).expect("row").clone();

        assert_eq!(by("penalty").diff_state(), DiffState::Matches);
        assert_eq!(by("contestable").diff_state(), DiffState::Differs);
        assert_eq!(by("timeLimitSeconds").diff_state(), DiffState::Unknown);

        let kept: Vec<String> = rows
            .iter()
            .filter(|r| r.survives_diff_filter())
            .map(|r| r.key.clone())
            .collect();
        assert!(kept.contains(&"contestable".to_string()));
        assert!(
            kept.contains(&"timeLimitSeconds".to_string()),
            "T-688: a key with no declared default cannot be shown to be at its default, so the \
             filter must keep it"
        );
        assert!(
            !kept.contains(&"penalty".to_string()),
            "T-688: a row provably at its schema default is what the filter hides"
        );
    }

    /// A zone rule's owner is the ZONE, named and addressable. The `subject_id` is the document id a
    /// click routes on; the label is what the row shows (name, else type, else id — never faceless).
    #[test]
    fn zone_rules_are_owned_by_their_zone() {
        let doc = json!({ "zonesById": {
            "z-named": { "type": "objective_capture", "label": "Hilltop",
                         "rules": { "captureSeconds": 90 } },
            "z-plain": { "type": "boundary", "rules": { "graceSeconds": 45 } }
        } });
        let rows = aggregate_settings(&doc);
        let named = rows
            .iter()
            .find(|r| r.key == "captureSeconds")
            .expect("row");
        assert_eq!(named.owner.subject_id(), Some("z-named"));
        assert!(named.owner.label().contains("Hilltop"));
        assert!(named.owner.label().contains("Zone"));

        let plain = rows.iter().find(|r| r.key == "graceSeconds").expect("row");
        assert_eq!(plain.owner.subject_id(), Some("z-plain"));
        assert!(
            plain.owner.label().contains("boundary"),
            "T-688: an unlabelled zone falls back to its type, got {:?}",
            plain.owner.label()
        );
        // A mission-level row names no entity, so it is not a click-through target.
        let doc = json!({ "meta": { "terrain": "everon" } });
        assert_eq!(aggregate_settings(&doc)[0].owner.subject_id(), None);
    }

    /// `120` (schema, integer) and `120.0` (authored through a number control) are the same setting.
    /// Derived `Value` equality would call them different and paint an untouched row "changed" —
    /// which is exactly the false diff this whole view must not produce.
    #[test]
    fn numeric_defaults_compare_across_int_and_float() {
        assert!(values_agree(&json!(120), &json!(120.0)));
        assert!(values_agree(&json!(0), &json!(-0.0)));
        assert!(!values_agree(&json!(120), &json!(121)));
        assert!(values_agree(&json!("warn"), &json!("warn")));
        assert!(!values_agree(&json!("warn"), &json!("kill")));
        assert!(!values_agree(&json!(true), &json!(1)));

        let doc = json!({ "zonesById": { "z1": { "type": "objective_capture",
            "rules": { "captureSeconds": 120.0 } } } });
        assert_eq!(
            aggregate_settings(&doc)[0].diff_state(),
            DiffState::Matches,
            "T-688: an authored 120.0 against a schema default of 120 is not a change"
        );
    }

    /// **Constraint (1) — READ-ONLY.** The aggregated view must not become a second editing surface:
    /// duplicating every attribute control in the programme is the failure mode the ticket names.
    ///
    /// Perturbation this catches: wiring any cell to `author_env`, an `editor_ops` mutator, or
    /// dropping an `<input>`/`<select>`/`<textarea>` into a row "just for the numbers".
    #[test]
    fn the_aggregated_view_is_not_a_second_editing_surface() {
        let src = live_source(include_str!("eden_settings.rs"));
        let editing_needles = [
            format!("author{}", "_env"),
            format!("update{}", "_environment"),
            format!("set{}", "_zone_rule"),
            format!("attrs{}", "_update"),
            "<input".to_string(),
            "<select".to_string(),
            "<textarea".to_string(),
            "contenteditable".to_string(),
        ];
        for f in [
            format!("fn render{}", "_all_settings_body"),
            format!("fn setting{}", "_row_view"),
            format!("fn render{}", "_all_settings_pointer"),
        ] {
            let body = only_body(&src, &f);
            for needle in &editing_needles {
                assert!(
                    !body.contains(needle.as_str()),
                    "T-688: `{needle}` must not appear in `{f}` — the aggregated view displays, it \
                     does not edit"
                );
            }
        }
    }

    /// **Constraint (2) — rows click through to the owning entity, through the SHIPPED router.**
    ///
    /// wog.md 14.6: a findings list that only prints is worse than one that selects. T-655 already
    /// ships that path (`validation_panel::route_select_by_subject_id`, registered from
    /// `mission_editor.rs`), so this view reuses it rather than standing up a second selection
    /// mechanism. Perturbation this catches: a bespoke selection path, or a row that names an owner
    /// and does nothing with it.
    #[test]
    fn rows_click_through_the_shipped_t655_router() {
        let src = live_code(include_str!("eden_settings.rs"));
        let body = only_body(&src, &format!("fn setting{}", "_row_view"));
        assert!(
            body.contains(&format!("route{}", "_select_by_subject_id")),
            "T-688: a row click must route through T-655's registered click-to-select router"
        );
        assert!(
            body.contains(&format!("validation{}", "_panel")),
            "T-688: the router is the validation panel's — reuse it, do not fork it"
        );
        assert!(
            !body.contains(&format!("register{}", "_select_by_id")),
            "T-688: this view must not REGISTER a router — that would replace T-655's"
        );
        // The row asks the owner for its id rather than re-deriving one, so a mission-level row
        // (which has no entity) cannot be given a click that clears the selection.
        assert!(
            body.contains(&format!("subject{}", "_id")),
            "T-688: the click id must come from the row's owner"
        );
    }

    /// Copy pins. The header must say the defaults are the SCHEMA's (an author reading a diff has to
    /// know what it was measured against), and the unresolved-owner note must name where a zone IS
    /// selected — a dead click that explains itself is recoverable, a silent one is not.
    #[test]
    fn the_copy_says_what_the_numbers_mean() {
        let note = ALL_SETTINGS_NOTE.to_lowercase();
        assert!(
            note.contains("mission.schema.json"),
            "T-688: the header must name the schema as the source of the default column"
        );
        assert!(
            note.contains("read-only"),
            "T-688: the header must say the view does not edit"
        );
        let unresolved = OWNER_UNRESOLVED_NOTE.to_lowercase();
        assert!(
            unresolved.contains("zones panel"),
            "T-688: when the router declines, the note must say where the zone IS selected"
        );
    }

    /// Presentation only: a string loses its quotes, an empty string is visible as empty rather than
    /// as a blank cell, everything else is shown as the document writes it.
    #[test]
    fn values_render_without_reshaping_them() {
        assert_eq!(fmt_setting_value(&json!("overcast")), "overcast");
        assert_eq!(fmt_setting_value(&json!("")), "(empty)");
        assert_eq!(fmt_setting_value(&json!(5400)), "5400");
        assert_eq!(fmt_setting_value(&json!(true)), "true");
        assert_eq!(fmt_setting_value(&json!(0.4)), "0.4");
    }

    /// Row order is deterministic — the pointer table's order, then unlisted mission keys sorted,
    /// then zones by id and rules by key. An author who scrolls to a row must find it in the same
    /// place next time, and a test can only pin what does not shuffle.
    #[test]
    fn row_order_is_stable() {
        let doc = json!({
            "meta": { "terrain": "everon", "environment": {
                "jip": "always", "time": "06:00", "showGrid": true, "showHillshade": false
            } },
            "zonesById": {
                "z-b": { "type": "boundary", "rules": { "penalty": "kill", "graceSeconds": 10 } },
                "z-a": { "type": "spawn", "rules": { "warnEverySeconds": 2 } }
            }
        });
        let rows: Vec<(String, Option<String>)> = aggregate_settings(&doc)
            .into_iter()
            .map(|r| (r.key, r.owner.subject_id().map(ToString::to_string)))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("terrain".into(), None),
                ("time".into(), None),
                ("jip".into(), None),
                ("showGrid".into(), None),
                ("showHillshade".into(), None),
                ("warnEverySeconds".into(), Some("z-a".into())),
                ("graceSeconds".into(), Some("z-b".into())),
                ("penalty".into(), Some("z-b".into())),
            ]
        );
    }

    /// The dialog is reachable: Mission Settings grows the pointer row that opens it, and the dialog
    /// is mounted as a sibling so it outlives that dialog being closed (the T-691 idiom).
    #[test]
    fn the_view_is_reachable_from_mission_settings() {
        let src = live_code(include_str!("eden_settings.rs"));
        let dialog = only_body(&src, "fn MissionSettingsDialog");
        assert!(
            dialog.contains(&format!("render{}", "_all_settings_pointer")),
            "T-688: Mission Settings must carry the pointer row that opens the aggregated view"
        );
        assert!(
            dialog.contains(&format!("All{}", "SettingsDialog")),
            "T-688: the aggregated view must be mounted as a sibling of Mission Settings"
        );
        let pointer = only_body(&src, &format!("fn render{}", "_all_settings_pointer"));
        assert!(
            pointer.contains(&format!("open{}", "_all_settings")),
            "T-688: the pointer row must open the aggregated view"
        );
    }

    /// A `SettingRow` carries all four columns the ticket names — key, owning entity, authored value,
    /// schema default — so no column can be quietly dropped from the type the view renders.
    #[test]
    fn a_row_carries_all_four_columns() {
        let doc = json!({ "zonesById": { "z1": { "type": "objective_capture", "label": "Hill",
            "rules": { "captureSeconds": 240 } } } });
        let row: SettingRow = aggregate_settings(&doc).remove(0);
        assert_eq!(row.key, "captureSeconds");
        assert_eq!(row.owner.label(), "Zone — Hill");
        assert_eq!(fmt_setting_value(&row.value), "240");
        assert_eq!(fmt_setting_default(&row.default), "120");
        assert_eq!(row.diff_state(), DiffState::Differs);
    }
}
