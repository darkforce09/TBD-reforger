//! Pointer and keyboard resizing stays local to the viewer.
use leptos::prelude::*;
/// A side panel, `initial` pixels wide, that the reader resizes between 220 and 640 pixels by
/// dragging its separator or with the Left and Right arrow keys in 20-pixel steps; `label` names
/// the separator for assistive technology.
#[component]
pub fn Panel(
    #[prop(default = 300)] initial: i32,
    #[prop(default = "Panel width")] label: &'static str,
    children: Children,
) -> impl IntoView {
    let width = RwSignal::new(initial);
    let dragging = RwSignal::new(None::<i32>);
    let node = NodeRef::<leptos::html::Div>::new();
    view! {<aside class="dv-panel" style:width=move||format!("{}px",width.get())>{children()}
        <div node_ref=node class="dv-resizer" role="separator" tabindex="0" aria-label=label aria-orientation="vertical" aria-valuemin="220" aria-valuemax="640" aria-valuenow=move||width.get()
            on:pointerdown=move|e|{e.prevent_default();dragging.set(Some(e.client_x()));if let Some(node)=node.get(){let _=node.set_pointer_capture(e.pointer_id());}}
            on:pointermove=move|e|{if let Some(previous)=dragging.get_untracked(){width.update(|v|*v=(*v+e.client_x()-previous).clamp(220,640));dragging.set(Some(e.client_x()));}}
            on:pointerup=move|_|dragging.set(None) on:pointercancel=move|_|dragging.set(None)
            on:keydown=move|e|{let delta=match e.key().as_str(){"ArrowLeft"=>-20,"ArrowRight"=>20,_=>0};if delta!=0{e.prevent_default();width.update(|v|*v=(*v+delta).clamp(220,640));}}/>
    </aside>}
}
