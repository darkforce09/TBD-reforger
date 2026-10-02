use crate::v2::core::test_support::class_r_scrub::{live_code, only_body as fn_body};

/// The live production surface this pin examines spans two files since T-934.8 —
/// `ArsenalTab` (mod.rs) wires the commit, `cargo_panel` (this file) owns the mutations.
/// Each file is scrubbed separately (`cut_test_module` truncates a haystack at its first
/// cfg-test attribute, so a scrubbed concatenation would end at the first test tail) and
/// the live halves concatenate into one haystack.
fn live_production_src() -> String {
    [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/mod.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/arsenal/panels.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/ui/arsenal/panels/cargo_panel.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/arsenal/tab_content.rs"
        )),
    ]
    .into_iter()
    .map(live_code)
    .collect()
}

/// T-503 Class-R: every cargo mutation in the panel must commit through `on_change`, and the
/// commit must reach `loadout_commands::set_loadout`. Staging — a mutation that updates the local
/// signal and waits for a Save button — goes red here.
///
/// RED (staging): delete the `on_change(&items.get_value());` after the qty `+` handler in
/// `cargo_panel` → "every cargo mutation must commit: 4 `cargo.update(` vs 3 `on_change(`".
/// RED (decoy, `if true == false`): move `loadout_commands::set_loadout(…)` inside
/// `if true == false { … }` → "ArsenalTab must reach loadout_commands::set_loadout".
/// RED (decoy, `#[cfg(any())]`): park the call in an `#[cfg(any())] fn dead_persist() { … }`
/// → same failure.
/// RED (decoy, `loop { break; … }`): park the call after a bare `break;` → same failure.
#[test]
fn cargo_mutations_commit_without_a_staging_gate() {
    let live = live_production_src();
    let panel = fn_body(&live, "fn cargo_panel(");
    let mutations = panel.matches("cargo.update(").count();
    let commits = panel.matches("on_change(").count();
    assert!(
        mutations >= 4,
        "cargo_panel should still own the qty -/+, remove and add mutations; found {mutations}"
    );
    assert!(
        commits >= mutations,
        "every cargo mutation must commit: {mutations} `cargo.update(` vs {commits} `on_change(`"
    );

    let tab = fn_body(&live, "pub(super) fn loaded_catalog(");
    assert!(
        tab.contains("loadout_commands::set_loadout("),
        "ArsenalTab must reach loadout_commands::set_loadout on a live path"
    );
    assert!(
        tab.contains("persist(&picks.get_untracked(), items)"),
        "persist_cargo must forward to the same commit the pick path uses"
    );
}
