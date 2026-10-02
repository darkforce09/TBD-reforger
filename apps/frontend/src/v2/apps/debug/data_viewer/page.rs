//! Full-window page in the normal website router and API lifecycle.
use super::{data::use_polled_read, navigation_state::Navigation};
use crate::v2::core::api::dto::equipment_data_viewer::EquipmentDatasetStatus;
use leptos::prelude::*;

/// The state every viewer panel reads from the Leptos context: the parsed location, the export
/// generation reads target (the pinned one, or the latest published one while following), and
/// the most recent dataset status poll.
#[derive(Clone, Copy)]
pub struct ViewerContext {
    pub nav: Memo<Navigation>,
    pub generation: Memo<String>,
    pub status: RwSignal<Option<EquipmentDatasetStatus>>,
}

/// The equipment and vehicle data viewer at `/debug/data-viewer`. It polls the export status
/// every five seconds while the browser tab is visible, resolves the generation to read, moves to
/// a newly published generation when the location follows `latest`, shows import progress, and
/// mounts the tab the location names.
#[component]
pub fn DataViewerPage() -> impl IntoView {
    let query = leptos_router::hooks::use_query_map();
    let nav = Memo::new(move |_| Navigation::parse(&query.get().to_query_string()));
    let tick = RwSignal::new(0u64);
    let status = use_polled_read::<EquipmentDatasetStatus>(Memo::new(move |_| {
        format!(
            "/debug/equipment-data/status?dataset={}&poll={}",
            nav.get().dataset(),
            tick.get()
        )
    }));
    let generation = Memo::new(move |_| {
        let nav = nav.get();
        if nav.generation() != "latest" {
            nav.generation().into()
        } else {
            status
                .value
                .get()
                .filter(|s| s.dataset_kind == nav.dataset())
                .and_then(|s| s.generation_id)
                .unwrap_or_default()
        }
    });
    provide_context(ViewerContext {
        nav,
        generation,
        status: status.value,
    });
    let timer = set_interval_with_handle(
        move || {
            if !web_sys::window()
                .and_then(|w| w.document())
                .is_some_and(|d| d.hidden())
            {
                tick.update(|v| *v += 1);
            }
        },
        std::time::Duration::from_secs(5),
    )
    .ok();
    on_cleanup(move || {
        if let Some(timer) = timer {
            timer.clear();
        }
    });
    let previous = RwSignal::new(String::new());
    let navigate = leptos_router::hooks::use_navigate();
    Effect::new(move |_| {
        let current = generation.get();
        let old = previous.get_untracked();
        if let Some(href) = nav.get_untracked().after_publication(&old, &current) {
            navigate(
                &href,
                leptos_router::NavigateOptions {
                    replace: true,
                    ..Default::default()
                },
            );
        }
        if !current.is_empty() {
            previous.set(current);
        }
    });
    let active_tab = Memo::new(move |_| nav.get().tab().to_owned());
    let ready = Memo::new(move |_| !generation.get().is_empty());
    view! {<div class="dv-app" data-testid="equipment-data-viewer"><style>{include_str!("viewer.css")}</style>
        <header class="dv-header"><div><div class="dv-eyebrow">{move||if nav.get().dataset()=="gameplay"{"GAMEPLAY CATALOG"}else{"FULL SOURCE DIAGNOSTICS"}}</div><h1>"Equipment & vehicle data"</h1></div>
            <div class="dv-generation"><span class="dv-live-dot" class:dv-connection-warning=move||status.error.get().is_some()/><span class="dv-update-state" role="status" title=move||status.error.get().unwrap_or_default()>{move||if status.error.get().is_some(){"Update unavailable · retrying"}else if status.loading.get(){"Loading export status…"}else if nav.get().generation()=="latest"{"Following latest export"}else{"Pinned generation"}}</span><small>{move||generation.get()}</small></div>
        </header>
        <nav class="dv-tabs">{[ ("overview","Overview"),("resources","Resources"),("fields","Fields")].into_iter().map(|(tab,label)|view!{<a class:dv-active=move||nav.get().tab()==tab href=move||nav.get().href(&[("tab",tab),("cursor","")])>{label}</a>}).collect_view()}
            <span class="dv-spacer"/><a href=move||nav.get().href(&[("dataset",if nav.get().dataset()=="gameplay"{"diagnostic"}else{"gameplay"})])>{move||if nav.get().dataset()=="gameplay"{"Inspect diagnostics"}else{"Gameplay catalog"}}</a><a href=move||nav.get().href(&[("tab","generations")])>"Export history"</a>
            <a href=move||{let g=generation.get();nav.get().href(&[("generation",if nav.get().generation()=="latest"{&g}else{"latest"})])}>{move||if nav.get().generation()=="latest"{"Pin this export"}else{"Follow latest"}}</a>
        </nav>
        {move||status.value.get().filter(|s|s.stage!="ready").map(|s|view!{<div class="dv-import" role="status"><strong>{super::layout::title(&s.stage)}</strong><span>{s.message.unwrap_or_default()}</span>{(s.total>0).then(||view!{<progress value=s.completed max=s.total/><span>{format!("{} / {}",super::layout::number(s.completed),super::layout::number(s.total))}</span>})}</div>})}
        <main class="dv-main">{move||{
            if !ready.get(){return view!{<div class="dv-empty"><h2>"Preparing the source library"</h2><p>"The website is available while the published files are verified and indexed. Progress appears above."</p></div>}.into_any();}
            match active_tab.get().as_str(){"resources"=>view!{<super::resources::Resources/>}.into_any(),"fields"=>view!{<super::field_inventory::Fields/>}.into_any(),"selection"=>view!{<super::overview::SelectionSummary/>}.into_any(),"generations"=>view!{<super::overview::GenerationHistory/>}.into_any(),"document"=>view!{<super::source_inspector::DocumentInspector/>}.into_any(),_=>view!{<super::overview::Overview/>}.into_any()}
        }}</main>
    </div>}
}
