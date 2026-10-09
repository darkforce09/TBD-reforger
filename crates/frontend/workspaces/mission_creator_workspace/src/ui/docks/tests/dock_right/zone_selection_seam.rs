use super::{register_select_zone, route_select_zone};

/// The seam, end to end: with no panel mounted the route reports that it selected NOTHING (which
/// is what lets the router return `false` instead of centring on a phantom selection), and with
/// the panel mounted the zone id reaches the panel's selection hook verbatim.
#[test]
fn the_route_reports_honestly_and_delivers_the_id() {
    assert!(
        !route_select_zone(&mission_model::ids::ZoneId::new("z-circle")),
        "T-754: no Zones panel mounted ⇒ nothing was selected, and the router must be told so"
    );
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let sink = std::rc::Rc::clone(&seen);
    register_select_zone(std::rc::Rc::new(move |id: &str| {
        sink.borrow_mut().push(id.to_string());
    }));
    assert!(
        route_select_zone(&mission_model::ids::ZoneId::new("z-circle")),
        "T-754: a mounted panel selects the zone, so the routed click is a real selection"
    );
    assert_eq!(
        seen.borrow().as_slice(),
        ["z-circle".to_string()],
        "T-754: the id must reach the panel unchanged — no re-derivation on the way"
    );
}
