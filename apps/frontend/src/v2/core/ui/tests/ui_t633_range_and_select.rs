//! The range and select primitives are painted, not tinted, and stay uncontrolled.

use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

fn code() -> String {
    live_code(&crate::v2::core::test_support::pins::ui_source())
}

fn source() -> String {
    live_source(&crate::v2::core::test_support::pins::ui_source())
}

/// The primitives EXIST here. That is half the ticket: `ui.rs` is where the suite's shared
/// components live (MaterialIcon / PageHeader / AuthGate / Dialog / Sheet / AdminGate) and it
/// had no slider and no select, which is why two raw browser controls were sitting in the
/// editor's top strip with nothing to replace them with.
#[test]
fn the_suite_has_a_slider_and_a_select_primitive() {
    let code = code();
    for component in ["pub fn Slider(", "pub fn Select("] {
        assert!(
            code.contains(component),
            "`{component}` must be defined in ui.rs — the shared primitives home"
        );
    }
}

/// **The performance contract, stated as a test.** The time scrubber is dragged, and a drag
/// emits ~30 values/second into `eden_top_strip`'s RowMirror (dedupe → debounce →
/// single-flight). A primitive that listened on `input`, or that owned a signal it wrote per
/// event and re-rendered from, would put a Leptos render on every one of those values and
/// defeat the sequencing. So: `change` only, `prop:value` straight to the DOM property, and no
/// signal of the component's own.
#[test]
fn neither_control_re_renders_per_event() {
    let code = code();
    // Assembled so this file's prose cannot satisfy the absence checks.
    let per_pixel = ["on:", "input"].concat();
    let owned_signal = ["RwSignal::", "new"].concat();
    for component in ["pub fn Slider(", "pub fn Select("] {
        let body = only_body(&code, component);
        assert!(
            body.contains("on:change"),
            "{component} must commit on the native `change` (the settle), not on drag"
        );
        assert!(
            !body.contains(&per_pixel),
            "{component} must NOT listen on `input` — that is an event per pixel, and it \
             defeats the RowMirror debounce the scrubber is sequenced by"
        );
        assert!(
            !body.contains(&owned_signal),
            "{component} must stay uncontrolled — an internal signal written per event \
             is a re-render per event"
        );
        assert!(
            body.contains("prop:value"),
            "{component} must write the DOM value PROPERTY (uncontrolled), not re-render to it"
        );
    }
}

/// Off browser chrome and onto the Aegis palette. `accent-color` alone was the old scrubber's
/// approach and it only tints the UA widget — the track geometry, the thumb and the select's
/// arrow stayed the browser's. Both controls must therefore be `appearance-none` with the parts
/// painted explicitly: the `::-*-track` / `::-*-thumb` pseudo-elements for the slider (both
/// engine prefixes — a WebKit-only recipe leaves Firefox on browser chrome), and a Material
/// chevron for the select.
#[test]
fn both_controls_paint_their_own_parts() {
    let src = source();
    for part in [
        "::-webkit-slider-runnable-track",
        "::-webkit-slider-thumb",
        "::-moz-range-track",
        "::-moz-range-thumb",
    ] {
        assert!(
            src.contains(part),
            "the slider must paint `{part}` itself — accent-color does not reshape it"
        );
    }
    assert!(
        src.matches("appearance-none").count() >= 2,
        "both the slider and the select must be `appearance-none` — otherwise the UA \
         still draws the widget underneath"
    );
    let select_body = only_body(&src, "pub fn Select(");
    assert!(
        select_body.contains("expand_more"),
        "the select's native arrow is replaced by a Material chevron, not merely hidden"
    );
    assert!(
        select_body.contains("pointer-events-none"),
        "the chevron overlays the control, so it must not eat the click that opens it"
    );
}

/// a disabled Select dims via DISABLED_GLYPH on the <select>, but the chevron is a
/// *sibling* span, so `disabled:` variants never reach it. The select must be a Tailwind
/// `peer` and the chevron must carry `peer-disabled:opacity-30` (matching DISABLED_GLYPH's
/// opacity) or a disabled control is half-lit. Needles are fragment-assembled so this module
/// is not its own haystack.
#[test]
fn disabled_select_chevron_dims_with_peer_disabled() {
    let src = source();
    let select_body = only_body(&src, "pub fn Select(");
    let peer = format!("{}{}", "peer ", "appearance-none");
    let dim = format!("{}{}", "peer-disabled:", "opacity-30");
    assert!(
        select_body.contains(&peer),
        "the <select> must be a Tailwind `peer` so a sibling can react to :disabled"
    );
    assert!(
        select_body.contains(&dim),
        "the Material chevron must carry peer-disabled:opacity-30 — DISABLED_GLYPH cannot reach a sibling"
    );
}

/// Built on the SHIPPED state vocabulary, not on new state classes. Both controls take
/// their hover and disabled treatment from `eden_layout`'s named recipes, so the chrome keeps
/// ONE state language; a hand-rolled `hover:bg-…` here would be a second definition of a rule
/// the chrome files are already pinned against.
#[test]
fn both_controls_consume_the_t668_recipes() {
    let code = code();
    for component in ["pub fn Slider(", "pub fn Select("] {
        let body = only_body(&code, component);
        for recipe in ["HOVER_FILL", "DISABLED_GLYPH"] {
            assert!(
                body.contains(recipe),
                "/: {component} must consume {recipe} rather than invent a state class"
            );
        }
    }
    // …and no local hover fill anywhere in the file's own class strings (assembled needle).
    let hand_rolled = ["hover:bg-", "white/"].concat();
    assert!(
        !source().contains(&hand_rolled),
        "/: the neutral hover fill has one definition (eden_layout::HOVER_FILL); \
         re-typing it here is the duplication the vocabulary exists to remove"
    );
}

/// Rule (3)'s tooltip half: the `title=` is emitted unconditionally, NOT gated on `!disabled`.
/// A control that cannot act must still explain itself.
#[test]
fn a_disabled_control_keeps_its_tooltip() {
    let code = code();
    for component in ["pub fn Slider(", "pub fn Select("] {
        let body = only_body(&code, component);
        assert!(
            body.contains("title=label") && body.contains("disabled=disabled"),
            "{component} must carry `title=label` beside `disabled=disabled` — the tooltip is \
             not gated on the control being enabled ( rule 3)"
        );
    }
}
