//! Top command strip state and view composition.

use super::*;

mod menu_row;
mod overlays;
mod tool_row;
use menu_row::menu_row;
use overlays::overlays;
use tool_row::tool_row;

/// Render the mission identity, command menus, tools, and transient dialogs.
#[component]
pub fn TopCommandStrip(
    title: String,
    can_undo: RwSignal<bool>,
    can_redo: RwSignal<bool>,
    save_semver: RwSignal<String>,
    save_status: RwSignal<String>,
    #[prop(optional)] dirty: Option<RwSignal<bool>>,
    #[prop(optional)] settings_open: Option<RwSignal<bool>>,
    #[prop(optional)] doc_tick: Option<RwSignal<u64>>,
    #[prop(optional)] obj_count: Option<RwSignal<usize>>,
    #[prop(optional)] orbat_open: Option<RwSignal<bool>>,
) -> impl IntoView {
    use leptos::portal::Portal;
    let open_menu = RwSignal::new(None::<usize>);
    let export_open = RwSignal::new(false);
    let save_open = RwSignal::new(false);
    let save_notes = RwSignal::new(String::new());
    let validation_open = RwSignal::new(false);
    let hint_open = RwSignal::new(crate::v2::apps::editor::ui::modals::help_modal::hint_shown());
    let set_hint = move |v: bool| {
        hint_open.set(v);
        crate::v2::apps::editor::ui::modals::help_modal::set_hint_shown(v);
    };
    let close_transients = move || {
        open_menu.set(None);
        export_open.set(false);
        validation_open.set(false);
        set_hint(false);
    };
    #[cfg(target_arch = "wasm32")]
    let transient_closer_id =
        crate::v2::core::ui::modal_stack::register_transient_closer(close_transients);
    #[cfg(target_arch = "wasm32")]
    let row_mirror = RowMirror::from_route();
    #[cfg(target_arch = "wasm32")]
    let toasts = crate::v2::core::ui::toast::use_toasts();
    let save_findings = RwSignal::new(Vec::<String>::new());
    let last_flush = RwSignal::new(None::<f64>);
    let recency_tick = RwSignal::new(0u32);
    #[cfg(target_arch = "wasm32")]
    crate::v2::apps::editor::shell::persist::set_last_flush_signal(last_flush);
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(win) = web_sys::window() {
            let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
                recency_tick.update(|n| *n = n.wrapping_add(1));
            });
            let handle = win
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    1_000,
                )
                .unwrap_or(0);
            cb.forget();
            on_cleanup(move || {
                if let Some(w) = web_sys::window() {
                    w.clear_interval_with_handle(handle);
                }
            });
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if ev.key() == "Escape" {
                if crate::v2::core::ui::modal_stack::escape_consumed() {
                    return;
                }
                if open_menu.get_untracked().is_some() {
                    open_menu.set(None);
                    return;
                }
                if export_open.get_untracked() {
                    export_open.set(false);
                    return;
                }
                if validation_open.get_untracked() {
                    validation_open.set(false);
                    return;
                }
                if save_open.get_untracked() {
                    save_open.set(false);
                    return;
                }
                if hint_open.get_untracked() {
                    set_hint(false);
                }
            }
        });
        on_cleanup(move || {
            esc.remove();
            crate::v2::core::ui::modal_stack::unregister_transient_closer(transient_closer_id);
        });
    }
    let save_was_open = std::rc::Rc::new(std::cell::Cell::new(false));
    Effect::new({
        let save_was_open = save_was_open.clone();
        move |_| {
            let now = save_open.get();
            let rising = now && !save_was_open.get();
            save_was_open.set(now);
            if rising {
                save_status.set(String::new());
                save_findings.set(Vec::new());
            }
        }
    });
    let env = Memo::new(move |_| {
        if let Some(t) = doc_tick {
            t.track();
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::apps::editor::bridge::host_state::editor_context::read_env()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::v2::core::api::dto::MissionEnv::default()
        }
    });
    let census = Memo::new(move |_| {
        if let Some(t) = doc_tick {
            t.track();
        }
        #[cfg(target_arch = "wasm32")]
        {
            let (factions, squads, slot_squad_ids) =
                website_map_engine::editing::hosted_commands::census_input();
            census_from_rows(&factions, &squads, &slot_squad_ids)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            SlotCensus::default()
        }
    });
    let summary = Memo::new(move |_| {
        let c = census.get();
        let terrain = env.get().terrain;
        #[cfg(target_arch = "wasm32")]
        let mode =
            crate::v2::apps::editor::bridge::host_state::editor_context::read_env_value("mode")
                .and_then(|v| v.as_str().map(str::to_string))
                .filter(|s| !s.trim().is_empty());
        #[cfg(not(target_arch = "wasm32"))]
        let mode: Option<String> = None;
        summary_line(&c, &terrain, mode.as_deref())
    });
    let validation_findings = Memo::new(move |_| {
        if let Some(t) = doc_tick {
            t.track();
        }
        match crate::v2::apps::editor::ui::inspector::validation_panel::chip_findings() {
            Some(sig) => sig.get(),
            None => Vec::new(),
        }
    });
    let export_gesture_ok = move |_ev: &leptos::ev::MouseEvent| -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            let stamp = AsRef::<web_sys::Event>::as_ref(_ev).time_stamp();
            crate::v2::apps::editor::shell::document_commands::begin_export_gesture(stamp)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            true
        }
    };
    let run_action = move |a: MenuAction| {
        open_menu.set(None);
        export_open.set(false);
        validation_open.set(false);
        match a {
            MenuAction::Save => {
                set_hint(false);
                save_open.set(true);
            }
            MenuAction::Export => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::shell::document_commands::export_now(
                    &save_semver.get_untracked(),
                );
            }
            MenuAction::ExportCompiled => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::shell::document_commands::export_compiled_now(toasts);
            }
            MenuAction::Undo => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::bridge::document_host::history::undo();
            }
            MenuAction::Redo => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::bridge::document_host::history::redo();
            }
            MenuAction::Settings => {
                if let Some(s) = settings_open {
                    set_hint(false);
                    s.set(true);
                }
            }
            MenuAction::Pattern(_)
            | MenuAction::Align(_)
            | MenuAction::Space(_)
            | MenuAction::Orient(_) => run_arrange_action(a),
            MenuAction::ControlsHint => set_hint(!hint_open.get_untracked()),
            MenuAction::SelectAll => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                    (d.select_all)()
                });
            }
            MenuAction::SetWidget(digit) => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                    (d.set_widget)(digit)
                });
                #[cfg(not(target_arch = "wasm32"))]
                let _ = digit;
            }
            MenuAction::ToggleSnap => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                    (d.toggle_snap)()
                });
            }
            MenuAction::SnapStep(delta) => {
                #[cfg(target_arch = "wasm32")]
                crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                    (d.snap_step)(delta)
                });
                #[cfg(not(target_arch = "wasm32"))]
                let _ = delta;
            }
        }
    };
    let widget_is = move |digit: u8| -> bool {
        let _gen = crate::v2::apps::editor::mission_editor::toolbar_dispatch_generation().get();
        #[cfg(target_arch = "wasm32")]
        {
            let mut active = 0u8;
            crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                active = (d.widget_digit)()
            });
            active == digit
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = digit;
            false
        }
    };
    let snap_on = move || -> bool {
        let _gen = crate::v2::apps::editor::mission_editor::toolbar_dispatch_generation().get();
        #[cfg(target_arch = "wasm32")]
        {
            let mut on = false;
            crate::v2::apps::editor::mission_editor::with_editor_toolbar_dispatch(|d| {
                on = (d.snap_enabled)()
            });
            on
        }
        #[cfg(not(target_arch = "wasm32"))]
        false
    };
    let title_fallback = StoredValue::new(title);
    view! {
        <div class=STRIP_ROWS>
            {menu_row!(title, can_undo, can_redo, save_semver, save_status, dirty, settings_open, doc_tick, obj_count, orbat_open, open_menu, export_open, save_open, save_notes, validation_open, hint_open, set_hint, close_transients, transient_closer_id, row_mirror, toasts, save_findings, last_flush, recency_tick, save_was_open, env, census, summary, validation_findings, export_gesture_ok, run_action, widget_is, snap_on, title_fallback)}
            {tool_row!(title, can_undo, can_redo, save_semver, save_status, dirty, settings_open, doc_tick, obj_count, orbat_open, open_menu, export_open, save_open, save_notes, validation_open, hint_open, set_hint, close_transients, transient_closer_id, row_mirror, toasts, save_findings, last_flush, recency_tick, save_was_open, env, census, summary, validation_findings, export_gesture_ok, run_action, widget_is, snap_on, title_fallback)}
            {overlays!(title, can_undo, can_redo, save_semver, save_status, dirty, settings_open, doc_tick, obj_count, orbat_open, open_menu, export_open, save_open, save_notes, validation_open, hint_open, set_hint, close_transients, transient_closer_id, row_mirror, toasts, save_findings, last_flush, recency_tick, save_was_open, env, census, summary, validation_findings, export_gesture_ok, run_action, widget_is, snap_on, title_fallback)}
        </div>
    }
}
