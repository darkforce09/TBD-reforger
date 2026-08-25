use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
use leptos::prelude::*;

#[test]
fn mark_registry_fetch_failed_writes_all_three_signals() {
    // Helper sits above the file's first `#[cfg(test)]` (inside registry_session), so
    // whole-file `live_code` keeps it. Body pin + behavioural flip.
    let src = live_code(include_str!("../mission_editor.rs"));
    let body = only_body(&src, "fn mark_registry_fetch_failed(");
    let failed_set = format!("{}{}", "registry_failed.", "set(true)");
    assert!(
        body.contains("CatalogState::Failed") && body.contains(&failed_set),
        "T-750: the helper must Fail both catalogs AND raise registry_failed"
    );
    let owner = Owner::new();
    owner.with(|| {
        let catalog = RwSignal::new(crate::editor::arsenal::asset_catalog::CatalogState::Loading);
        let vehicle = RwSignal::new(crate::editor::arsenal::asset_catalog::CatalogState::Loading);
        let failed = RwSignal::new(false);
        super::mark_registry_fetch_failed(catalog, vehicle, failed);
        assert!(
            matches!(
                catalog.get(),
                crate::editor::arsenal::asset_catalog::CatalogState::Failed
            ) && matches!(
                vehicle.get(),
                crate::editor::arsenal::asset_catalog::CatalogState::Failed
            ) && failed.get(),
            "T-750: calling the helper must leave every consumer in the terminal failure state"
        );
    });
}

#[test]
fn err_arm_and_retry_gen_are_wired_on_the_page() {
    let raw = include_str!("../mission_editor.rs");
    let call = format!("{}{}", "mark_registry_fetch_", "failed(");
    assert!(
        raw.contains(&call),
        "T-750: the wasm Err arm must call mark_registry_fetch_failed"
    );
    let fetch_at = raw
        .find("match fetch_registry_pages(auth).await")
        .expect("registry fetch match present");
    let fetch_window = &raw[fetch_at..fetch_at + 900];
    assert!(
        fetch_window.contains("Err(_) =>") && fetch_window.contains(&call),
        "T-750: the helper call must sit in the registry-fetch Err arm"
    );
    let gen = format!("{}{}", "registry_fetch_", "gen.get()");
    assert!(
        raw.contains(&gen),
        "T-750: Favourites Retry bumps registry_fetch_gen; the Effect must read it"
    );
    // Page body only: the early registry_session `#[cfg(test)]` would otherwise cut the page.
    let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
    assert_eq!(raw.matches(anchor.as_str()).count(), 1);
    let live = live_code(&raw[raw.find(anchor.as_str()).expect("anchor")..]);
    assert!(
        live.contains("let registry_failed = RwSignal::new(false)"),
        "T-750: registry_failed starts false so a slow load cannot look like a hard failure"
    );
}
