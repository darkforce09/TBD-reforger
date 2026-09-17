//! Escape modal stack tests for the top command strip.

/// T-726 / T-814 — top-strip Esc yields when modal_stack consumed Escape (wave139 F3 / wave200 F4).
use crate::v2::core::test_support::class_r_scrub::{live_source, only_body};

#[test]
fn top_command_strip_escape_yields_when_modal_stack_consumed_escape() {
    // live_source (not live_code): Escape is a string literal; live_code blanks literals.
    let code = live_source(super::test_source::top_strip_source());
    let body = only_body(&code, "pub fn TopCommandStrip(");
    let esc = ["if ev.key() == \"", "Escape\""].concat();
    let esc_at = body
        .find(&esc)
        .unwrap_or_else(|| panic!("T-726: TopCommandStrip must own an Escape keydown arm"));
    let esc_region = &body[esc_at..];
    let guard = ["modal_stack", "::", "escape_consumed()"].concat();
    assert!(
        esc_region.contains(&guard),
        "T-814: TopCommandStrip Esc must consult modal_stack::escape_consumed() so an open \
         dialog/manager consumes Esc alone even when a peer listener already closed it \
         (wave200 F4). Hollow: delete the escape_consumed guard → RED."
    );
    let guard_at = esc_region
        .find(&guard)
        .expect("escape_consumed present in Esc arm");
    for needle in [
        "open_menu.set(None)",
        "export_open.set(false)",
        "save_open.set(false)",
    ] {
        let at = esc_region
            .find(needle)
            .unwrap_or_else(|| panic!("missing {needle} in Esc arm"));
        assert!(
            guard_at < at,
            "T-814: escape_consumed() must precede `{needle}` (yield before act)"
        );
    }
    // Must NOT regress to live any_open() — that is the F4 pile-up (dialog closes, then strip
    // sees any_open==false and clears the hint in the same keydown).
    let any = ["modal_stack", "::", "any_open()"].concat();
    assert!(
        !esc_region.contains(&any),
        "T-814: strip Esc must not consult live any_open() (wave200 F4 insertion-order trap)"
    );
}

#[test]
fn top_strip_escape_consumed_guard_is_load_bearing() {
    let code = live_source(super::test_source::top_strip_source());
    let body = only_body(&code, "pub fn TopCommandStrip(");
    let guard = ["modal_stack", "::", "escape_consumed()"].concat();
    assert!(
        body.contains(&guard),
        "canary: strip carries escape_consumed"
    );
    let perturbed = body.replacen(&guard, "false /* hollow */", 1);
    assert!(
        !perturbed.contains(&guard),
        "fired rule: deleting escape_consumed must break the T-814 top-strip Esc pin"
    );
}

#[test]
fn top_strip_registers_transient_closer_with_modal_stack() {
    let code = live_source(super::test_source::top_strip_source());
    let body = only_body(&code, "pub fn TopCommandStrip(");
    let reg = ["modal_stack", "::", "register_transient_closer"].concat();
    assert!(
        body.contains(&reg),
        "T-814: strip must register close_transients with modal_stack so non-strip dialog \
         opens clear menu/export/hint"
    );
    assert!(
        body.contains("unregister_transient_closer"),
        "T-814: strip must unregister the transient closer on cleanup"
    );
}
