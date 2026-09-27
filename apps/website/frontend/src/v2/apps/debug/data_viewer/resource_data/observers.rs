//! Observers measure content without adding nested scrolling surfaces.
use leptos::prelude::*;
use wasm_bindgen::{JsCast, closure::Closure};

pub fn size(node: NodeRef<leptos::html::Div>, callback: Callback<(f64, f64)>) {
    let observer = StoredValue::new_local(None);
    Effect::new(move |_| {
        if let Some(element) = node.get() {
            let callback = Closure::<dyn FnMut(js_sys::Array, web_sys::ResizeObserver)>::new(
                move |entries: js_sys::Array, _| {
                    if let Ok(entry) = entries.get(0).dyn_into::<web_sys::ResizeObserverEntry>() {
                        let r = entry.content_rect();
                        callback.run((r.width(), r.height()));
                    }
                },
            );
            if let Ok(watch) = web_sys::ResizeObserver::new(callback.as_ref().unchecked_ref()) {
                watch.observe(&element);
                observer.set_value(Some((watch, callback)));
            }
        }
    });
    on_cleanup(move || {
        observer.with_value(|v| {
            if let Some((watch, _)) = v {
                watch.disconnect();
            }
        });
    });
}

pub fn near(
    node: NodeRef<leptos::html::Div>,
    root: NodeRef<leptos::html::Div>,
    visible: RwSignal<bool>,
) {
    let observer = StoredValue::new_local(None);
    Effect::new(move |_| {
        if let (Some(element), Some(root)) = (node.get(), root.get()) {
            let callback = Closure::<dyn FnMut(js_sys::Array, web_sys::IntersectionObserver)>::new(
                move |entries: js_sys::Array, _| {
                    if let Ok(entry) = entries
                        .get(0)
                        .dyn_into::<web_sys::IntersectionObserverEntry>()
                    {
                        visible.set(entry.is_intersecting());
                    }
                },
            );
            let options = web_sys::IntersectionObserverInit::new();
            options.set_root(Some(&root));
            options.set_root_margin("240px");
            if let Ok(watch) = web_sys::IntersectionObserver::new_with_options(
                callback.as_ref().unchecked_ref(),
                &options,
            ) {
                watch.observe(&element);
                observer.set_value(Some((watch, callback)));
            }
        }
    });
    on_cleanup(move || {
        observer.with_value(|v| {
            if let Some((watch, _)) = v {
                watch.disconnect();
            }
        });
    });
}
