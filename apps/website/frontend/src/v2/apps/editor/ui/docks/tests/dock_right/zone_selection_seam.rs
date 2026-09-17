use super::tests::DOCK_RIGHT_PRODUCTION_SOURCE;
use super::{register_select_zone, route_select_zone, ZONES_TAB};

/// The production half of this file — everything above the first test module, so a needle here
/// cannot satisfy itself (the T-759 hollow-pin trap).
fn production() -> &'static str {
    DOCK_RIGHT_PRODUCTION_SOURCE
        .split("#[cfg(test)]")
        .next()
        .expect("the production half precedes the test modules")
}

/// The seam, end to end: with no panel mounted the route reports that it selected NOTHING (which
/// is what lets the router return `false` instead of centring on a phantom selection), and with
/// the panel mounted the zone id reaches the panel's selection hook verbatim.
#[test]
fn the_route_reports_honestly_and_delivers_the_id() {
    assert!(
        !route_select_zone("z-circle"),
        "T-754: no Zones panel mounted ⇒ nothing was selected, and the router must be told so"
    );
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let sink = std::rc::Rc::clone(&seen);
    register_select_zone(std::rc::Rc::new(move |id: &str| {
        sink.borrow_mut().push(id.to_string());
    }));
    assert!(
        route_select_zone("z-circle"),
        "T-754: a mounted panel selects the zone, so the routed click is a real selection"
    );
    assert_eq!(
        seen.borrow().as_slice(),
        ["z-circle".to_string()],
        "T-754: the id must reach the panel unchanged — no re-derivation on the way"
    );
}

/// The hook drives the panel's OWN selection signal (not `select_tool`'s — a zone id there reads
/// `SEL 1` with nothing highlighted) and raises the tab it is visible on, un-collapsing the dock.
/// A selection the author cannot see is the same dead click wearing a different costume.
///
/// wave-129 F2 moved the mount-time call from `register_*` to `install_*` (register + an
/// `on_cleanup` unregister); the needle follows it, because the thing being pinned is what the
/// hook DOES, and the hook is the same hook.
#[test]
fn the_hook_selects_the_zone_and_shows_it() {
    let src = production();
    let at = src
        .find(&format!("install{}", "_select_zone(std::rc::Rc::new"))
        .expect("T-754: DockRight must install the zone-selection hook at mount");
    // Taken by CHARS, not bytes: the window runs into em-dashed prose, and a byte slice through
    // one of those is a panic that has nothing to do with what this test is asserting.
    let body: String = src[at..].chars().take(320).collect();
    assert!(
        body.contains(&format!("zone{}", "_selected.set(Some(")),
        "T-754: the hook must set the Zones panel's own selection signal"
    );
    assert!(
        body.contains(&format!("tab.set(ZONES{}", "_TAB)")),
        "T-754: and raise the Zones tab, so the selection is visible"
    );
    assert!(
        body.contains(&format!("collapsed.set({}", "false)")),
        "T-754: and un-collapse the dock (T-638), for the same reason"
    );
}

/// One index, three consumers. The tab button, the panel under it and the routed click all read
/// [`ZONES_TAB`]; a literal in any of them is a silent way to select a zone on a tab nobody is
/// looking at.
#[test]
fn the_zones_tab_index_is_stated_once() {
    let src = production();
    assert_eq!(ZONES_TAB, 3, "T-754: the Zones tab's index, stated once");
    assert!(
        src.contains(&format!("tab_btn(ZONES{}", "_TAB,")),
        "T-754: the tab button must read the constant"
    );
    assert!(
        src.contains(&format!("ZONES_TAB => zones{}", "_panel(")),
        "T-754: the panel arm must read the constant too"
    );
    assert!(
        !src.contains(&format!("tab_btn(3,{}", "")),
        "T-754: no literal 3 may address the Zones tab"
    );
}
