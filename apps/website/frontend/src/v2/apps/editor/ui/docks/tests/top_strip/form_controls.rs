//! Form controls tests for the top command strip.

/// T-633 — the top strip's two native controls are gone.
///
/// The defect was narrow and visible: the time scrubber was a raw `<input type="range">` whose only
/// styling was `accent-[--color-primary]` — which tints the UA widget and nothing else, so it still
/// drew a browser-blue rail and a browser-shaped thumb against Aegis `#adc6ff` — and the weather
/// picker was a raw `<select>` wearing the platform's native arrow. Both are now `crate::v2::core::ui`
/// primitives (created by this ticket; see the pins in `ui.rs` for what they guarantee).
///
/// Source pins, on scrubbed source: this is a Leptos view a native test cannot render. Absence
/// needles are assembled from fragments so this module's own prose cannot satisfy them.
use super::WEATHER_OPTIONS;
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};

/// THE FIX, stated as an absence. No raw range input and no raw select may remain in the strip.
/// Checked on the string-KEPT source, because `type="range"` is a literal and that is exactly
/// where the defect lived.
#[test]
fn no_raw_browser_control_remains_in_the_strip() {
    let src = live_source(super::test_source::top_strip_source());
    let raw_range = [r#"type=""#, r#"range""#].concat();
    assert!(
        !src.contains(&raw_range),
        "T-633: the time scrubber must be the ui::Slider primitive, not a raw range input"
    );
    let raw_select = ["<sel", "ect"].concat();
    assert!(
        !src.contains(&raw_select),
        "T-633: the weather picker must be the ui::Select primitive, not a raw select element"
    );
    // The accent-colour escape hatch is the thing that LOOKED like a fix and was not: it tints
    // the UA widget and leaves its geometry alone. It must be gone, not merely supplemented.
    let accent = ["accent-[--color-", "primary]"].concat();
    assert!(
        !src.contains(&accent),
        "T-633: `accent-color` only tints browser chrome — the control must paint its own parts"
    );
}

/// …and the primitives are actually WIRED, in the strip's own subtree. An absence pin alone is
/// satisfied by deleting the controls, which is not the fix.
#[test]
fn the_strip_renders_the_aegis_primitives() {
    let code = live_code(super::test_source::top_strip_source());
    let body = only_body(&code, "pub fn TopCommandStrip(");
    for needle in ["<Slider", "<Select", "options=WEATHER_OPTIONS"] {
        assert!(
            body.contains(needle),
            "T-633: the top strip must render `{needle}` — the raw controls were replaced, not \
             removed"
        );
    }
    // The import is by name, so a stale `ui::` glob cannot make the pin above pass on nothing.
    assert!(
        code.contains("use crate::v2::core::ui::{cn, MaterialIcon, Select, Slider}"),
        "T-633: the strip must import the two primitives by name from the shared ui module"
    );
}

/// **Settle commit path is not regressed.** T-192's whole point is that a held scrubber
/// emits ~30 values/second and the `missions` row gets ONE PATCH per settle. This pin locks
/// that path only: `on_change` → `author_env` → `row_mirror.set_time`, plus the HH:MM span
/// wired to settled `env` time. It does **not** claim strip-local `on:input` absence
/// (Save-dialog handlers elsewhere in `TopCommandStrip` would false-fail a whole-body scan;
/// `ui.rs` already pins the Slider primitive itself).
#[test]
fn the_scrubber_settle_commit_path() {
    let code = live_code(super::test_source::top_strip_source());
    let body = only_body(&code, "pub fn TopCommandStrip(");
    assert!(
        body.contains("on_change=Callback::new(move |mins: i32|"),
        "T-633: the scrubber commits through the primitive's settle callback"
    );
    for step in ["author_env(", "row_mirror.set_time(&hhmm)"] {
        assert!(
            body.contains(step),
            "T-633: the T-192 mirror path (`{step}`) must survive the control swap"
        );
    }
    // HH:MM span is wired to settled authored time via `env` (doc_tick). That is display of
    // the committed value, not mid-drag preview — preview would need a local drag signal.
    assert!(
        body.contains("{move || env.get().time}"),
        "T-633: the HH:MM readout must stay wired to the settled env time"
    );
}

/// The option table is data, and it is the wire enum. A picker whose values drifted from the
/// schema's weather strings would author a document the mod cannot read, and `MIRROR_WEATHER`
/// would mirror the drift onto the `missions` row.
#[test]
fn the_weather_options_are_the_wire_enum() {
    let values: Vec<&str> = WEATHER_OPTIONS.iter().map(|(v, _)| *v).collect();
    assert_eq!(
        values,
        vec!["clear", "overcast", "heavy_rain", "dense_fog"],
        "T-633: the picker's values are the schema's snake_case weather enum, in order"
    );
    assert!(
        WEATHER_OPTIONS.iter().all(|(_, label)| !label.is_empty()),
        "every option needs a human label — a blank row is an unpickable option"
    );
}
