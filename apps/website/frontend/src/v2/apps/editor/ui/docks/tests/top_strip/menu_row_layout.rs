//! Menu row layout tests for the top command strip.

/// T-634 — two rows, and one action hierarchy.
///
/// Two defects, one cause. The strip held five menus, a title, a scrubber, a weather picker, three
/// history glyphs, three buttons, a status readout and a gear in ONE 48 px row, so (a) it read as
/// crowded and (b) the only signal left to separate `Save Version` from `Export JSON` and
/// `Export Compiled` — visual weight — was spent evenly across all three. Eden fits eight menus at
/// `y 0–22` and twenty-five tool icons at `y 22–40`, in forty pixels, by giving each kind of thing
/// its own row. That is what these pin: the split, its cost in pixels (zero), and the hierarchy the
/// space bought.
///
/// Source pins on scrubbed source — this is a Leptos view a native test cannot render — plus data
/// pins over `MENUS`, which is a `const` table and can be read directly.
use super::MENUS;
use crate::v2::core::test_support::class_r_scrub::{live_source, only_body};
// T-637 — the row recipes moved to `eden_layout` (the T-634 fold-back) and the dead one-row
// `STRIP` is deleted. `DOCK_L` stands in for it below: "the strip is the same glass as the docks
// it sits above" is what comparing the two shells always meant.
use crate::v2::apps::editor::shell::layout::{
    BTN_ICON, DOCK_L, ROW_MENUS, ROW_MENUS_PX, ROW_TOOLS, ROW_TOOLS_PX, STRIP_ROWS, STRIP_TOP_PX,
};

/// The strip's own view body, with comments blanked and class/aria literals kept — the literals
/// ARE the structure these pins read.
fn body() -> String {
    let src = live_source(include_str!("../../top_strip.rs"));
    only_body(&src, "pub fn TopCommandStrip(").to_string()
}

/// Where a needle first appears in the body. Panics rather than returning an `Option`: a missing
/// landmark is a renamed control, which is new information, not a silently-skipped ordering.
fn at(body: &str, needle: &str) -> usize {
    body.find(needle)
        .unwrap_or_else(|| panic!("T-634: the strip no longer contains `{needle}`"))
}

/// **THE HEIGHT CONTRACT.** `STRIP_TOP_PX` (48) is the top inset the four `eden_layout`
/// accessors and `mission_editor`'s `top-12`/`h-12` are written from. Two rows must therefore
/// SPLIT it, never add to it. The menu row states a fixed `h-6`; the tool row states no height at
/// all and takes the remainder (`flex-1`), so the sum is 48 by construction and no consumer of
/// `STRIP_TOP_PX` moves. This checks both halves: the arithmetic, and that the classes really
/// are "one fixed, one elastic" rather than two fixed heights that could drift apart.
#[test]
fn two_rows_split_the_strip_height_they_do_not_add_to_it() {
    assert!(
        (ROW_MENUS_PX + ROW_TOOLS_PX - STRIP_TOP_PX).abs() < f64::EPSILON,
        "T-634: the two rows must sum to eden_layout::STRIP_TOP_PX ({STRIP_TOP_PX}), not \
         {}. A taller strip is a layout change every inset reader downstream would have to \
         follow.",
        ROW_MENUS_PX + ROW_TOOLS_PX
    );
    assert!(
        ROW_MENUS.contains("h-6"),
        "T-634: the menu row is the FIXED one — `h-6` (= {ROW_MENUS_PX} px)"
    );
    assert!(
        ROW_TOOLS.contains("flex-1") && !ROW_TOOLS.contains("h-["),
        "T-634: the tool row must take the REMAINDER (`flex-1`) and state no height of its own \
         — two stated heights can drift apart from STRIP_TOP_PX; a remainder cannot"
    );
}

/// The shell is two stacked rows, and its MATERIAL is the docks', verbatim. Only the flow
/// changed: `items-center` (one centred row) → `flex-col` (two rows). If the surface halves ever
/// diverge the strip stops matching the docks it sits above, which is the "disjointed" complaint
/// `editor_chrome_direction.md` exists to answer.
///
/// T-637 — this used to compare `STRIP_ROWS` against `eden_layout::STRIP`, the one-row shell it
/// replaced. That shell had no consumer left after T-634 and is now deleted, so the comparison
/// runs against `DOCK_L` instead. This is not a weakening: "the same glass as the docks" was
/// always the property; `STRIP` was only ever a proxy for it.
#[test]
fn the_shell_is_two_rows_in_the_same_glass() {
    let surface = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl";
    assert!(
        DOCK_L.starts_with(surface) && STRIP_ROWS.starts_with(surface),
        "T-634/T-637: the two-row shell must wear the same docked-overlay glass as the docks"
    );
    assert!(
        STRIP_ROWS.contains("border-b") && STRIP_ROWS.contains("border-white/10"),
        "T-634: the strip's bottom edge is unchanged by the row split"
    );
    assert!(
        STRIP_ROWS.contains("flex-col") && STRIP_ROWS.contains("h-full"),
        "T-634: the shell stacks its two rows and still fills the 48 px it is given"
    );
    assert!(
        !STRIP_ROWS.contains("items-center"),
        "T-634: `items-center` in a column would centre the rows horizontally — the shell must \
         let each row stretch to full width"
    );
}

/// **Menus on row 1, the icon toolbar on row 2** — Eden's `y 0–22` / `y 22–40`. Proven as an
/// ORDERING over the rendered body: the menu table is iterated inside the menu row, and every
/// tool glyph appears only after the tool row opens. A re-layout that put a tool back in the
/// menu row would move one of these indices past another.
#[test]
fn the_menus_own_row_one_and_the_toolbar_owns_row_two() {
    let b = body();
    let row_menus = at(&b, "class=ROW_MENUS");
    let row_tools = at(&b, "class=ROW_TOOLS");
    assert!(
        row_menus < row_tools,
        "T-634: the menu row renders first — Eden puts the menus at y 0–22, above the tools"
    );
    // The menu bar is inside row 1.
    let menu_table = at(&b, "{MENUS");
    assert!(
        row_menus < menu_table && menu_table < row_tools,
        "T-634: the menu bar must render inside the MENU row, not the tool row"
    );
    // …and every tool-row citizen is inside row 2. `History` is the disabled version-list glyph,
    // `Undo`/`Redo` the two live ones; T-797 adds the widget-mode + snap-grid icon cluster and
    // T-795/T-799 makes it THREE widget buttons (`No widget` / `Translate widget` / `Rotate
    // widget`) beside `Toggle snap grid` and the two snap-step glyphs; `Mission settings` is the
    // gear and `Export` the demoted menu.
    for tool in [
        r#"aria-label="History""#,
        r#"aria-label="Undo""#,
        r#"aria-label="Redo""#,
        r#"aria-label="No widget""#,
        r#"aria-label="Translate widget""#,
        r#"aria-label="Rotate widget""#,
        r#"aria-label="Toggle snap grid""#,
        r#"aria-label="Decrease snap step""#,
        r#"aria-label="Increase snap step""#,
        r#"aria-label="Mission settings""#,
        r#"aria-label="Export""#,
    ] {
        assert!(
            at(&b, tool) > row_tools,
            "T-634: `{tool}` belongs to the TOOL row — a command in the menu row is the mixing \
             that made one 48 px strip unreadable"
        );
    }
    // The title stays with the menus: it is identity, not a tool.
    assert!(
        at(&b, r#"aria-label="Mission title""#) < row_tools,
        "T-634: the editable title is row-1 identity, beside the menus"
    );
    // T-797 (operator decision 2) — ORBAT Manager MOVED the other way: from a row-2 command to a
    // menu-row entry. It now renders in row 1 (after the menu bar, before the title divider), so
    // it must sit BEFORE the tool row opens — the inverse of the assertion it used to satisfy.
    assert!(
        at(&b, r#"aria-label="ORBAT Manager""#) < row_tools,
        "T-797: ORBAT Manager is now a menu-row entry (row 1), not a row-2 tool — it must render \
         before the toolbar row"
    );
}

/// **ONE primary.** `Save Version` is the only filled button in the strip; the exports are one
/// secondary trigger. Checked on the CONSTS (a filled recipe vs an outlined one) and on the body
/// (the primary recipe is used exactly once). Before this ticket three sibling buttons carried
/// `px-3 py-1 text-xs font-medium` and only their fill differed, so the routine save and the two
/// consequential exports read at near-equal weight.
#[test]
fn exactly_one_action_is_primary() {
    assert!(
        super::ACTION_PRIMARY.contains("bg-primary"),
        "T-634: the primary action is FILLED — that is what makes it the loudest thing"
    );
    assert!(
        !super::ACTION_SECONDARY.contains("bg-primary")
            && super::ACTION_SECONDARY.contains("border"),
        "T-634: the demoted tier is outlined and unfilled — a second fill is a second primary"
    );
    let b = body();
    assert_eq!(
        b.matches("class=ACTION_PRIMARY").count(),
        1,
        "T-634: exactly ONE control in the strip may wear the primary recipe"
    );
    assert!(
        b.contains("class=ACTION_SECONDARY") || b.contains("ACTION_SECONDARY, HOVER_FILL"),
        "T-634: the Export trigger must wear the demoted recipe"
    );
}

/// **…and the two exports are demoted BEHIND it, not merely shrunk.** Each export now has
/// exactly ONE dispatch site in the strip — the shared `run_action` — where before it had two
/// (the menu row and a top-level button of its own). The second copy was the near-equal weight.
/// The rows themselves reuse the T-668 menu vocabulary rather than inventing a second dropdown
/// language: `MENU_PANEL` / `MENU_ROW` / the unconditional `MENU_GUTTER`.
#[test]
fn the_exports_live_behind_one_secondary_trigger() {
    let b = body();
    for dispatch in [
        "crate::v2::apps::editor::shell::document_commands::export_now(",
        "crate::v2::apps::editor::shell::document_commands::export_compiled_now(",
    ] {
        assert_eq!(
            b.matches(dispatch).count(),
            1,
            "T-634: `{dispatch}` must have exactly one dispatch site in the strip (the shared \
             `run_action`); a second call site is the top-level button this ticket demoted"
        );
    }
    let gate = at(&b, "export_open");
    for row in [
        "run_action(MenuAction::Export)",
        "run_action(MenuAction::ExportCompiled)",
    ] {
        assert!(
            at(&b, row) > gate,
            "T-634: `{row}` must render inside the export dropdown, behind `export_open`"
        );
    }
    assert!(
        b.contains("MENU_PANEL, ") && b.matches("MENU_ROW, HOVER_FILL").count() >= 2,
        "T-634: the export menu reuses the T-668 menu recipes — the demotion lands inside the \
         vocabulary, not beside it"
    );
}

/// **The history glyphs are no longer too dim to find** — and, since T-637, neither is anything
/// else that wears the shared icon recipe.
///
/// **THIS PIN WENT RED ON PURPOSE.** T-634 wrote it with the premise `BTN_ICON.contains(
/// "text-on-surface-variant")` — an assertion that the SHARED recipe was still the muted, 36 px
/// one — because T-634 could not fix `eden_layout` (another slice owned it that wave) and had to
/// route around it with a local `TOOL_ICON` copy. The premise was a tripwire: it fires the moment
/// someone fixes the recipe at its source, which is exactly what T-637 did.
///
/// Resolved by INVERTING it, not by weakening it and not by leaving the recipe broken to keep
/// the pin green. The property never changed — "a live glyph rests bright, and dimming means
/// disabled" — only where it is enforced: `BTN_ICON` itself now carries it, so it holds for the
/// help panel's close button and every dock/toolbelt caller too, not just for these four.
#[test]
fn a_live_tool_glyph_rests_bright_and_only_a_dead_one_dims() {
    assert!(
        BTN_ICON.contains("text-on-surface") && !BTN_ICON.contains("text-on-surface-variant"),
        "T-637: the SHARED icon recipe rests at full strength — being hard to find was the \
         defect, and T-634's local TOOL_ICON copy only hid it from this one file"
    );
    assert!(
        !BTN_ICON.contains("hover:") && !BTN_ICON.contains("disabled:"),
        "T-637: the recipe carries geometry + rest weight only; its states come from the T-668 \
         vocabulary at the call site, so a disabled glyph dims, refuses the hover fill, and \
         keeps its title (rule 3)"
    );
    let b = body();
    assert_eq!(
        b.matches("cn(&[BTN_ICON, HOVER_FILL, DISABLED_GLYPH])")
            .count(),
        4,
        "T-634: all four tool glyphs (History · Undo · Redo · the settings gear) take the same \
         recipe and the same T-668 state pair"
    );
    assert!(
        !b.contains("TOOL_ICON"),
        "T-637: the local copy is gone — a second source of truth for the same geometry is what \
         let the defect survive everywhere except this file"
    );
    // Rule (3) survives the swap: the permanently-disabled History glyph still explains itself.
    assert!(
        b.contains("Version history (soon)"),
        "T-668 rule (3): a disabled glyph keeps its tooltip — it must not go silent"
    );
}

/// **The gear is not stranded any more.** It used to render LAST, past both export buttons, at
/// the far right of the strip, adjacent to nothing it had anything to do with. It now sits
/// immediately after the weather picker: the scrubber and the picker are two Mission Settings
/// fields rendered inline, and the gear opens the rest of them. Pinned as an ordering — gear
/// after the `Select`, and before the actions rather than after them.
#[test]
fn the_gear_sits_with_the_environment_it_opens() {
    let b = body();
    let weather = at(&b, "<Select");
    let gear = at(&b, r#"aria-label="Mission settings""#);
    let primary = at(&b, "class=ACTION_PRIMARY");
    assert!(
        weather < gear && gear < primary,
        "T-634: the gear belongs to the environment cluster (after the weather picker, before \
         the actions), not alone at the far right past every button"
    );
}

/// **T-668's `…` rule, applied to this ticket's menu.** `…` means "opens a dialog", everywhere.
/// `Save Version…` and the two `Mission Settings…` rows earn it. `Export Compiled Mission` did
/// not: `export_compiled_now` composes bytes, starts a browser download and reports through a
/// toast — no dialog ever appears. A suffix that promises one and delivers a download is the
/// convention leaking, and a leaking convention teaches the operator to ignore it.
///
/// T-690 gave that row a second effect (it publishes the compile's findings to the already-open
/// T-655 panel) and the verdict did not move: updating a surface the operator is already looking
/// at is not opening one. See the `Export Compiled Mission` row's own comment in `MENUS`.
#[test]
fn an_ellipsis_is_a_promise_of_a_dialog() {
    for (menu, items) in MENUS {
        for it in items {
            let promises_dialog = it.label.ends_with('…');
            let opens_dialog = matches!(
                it.action,
                Some(super::MenuAction::Save) | Some(super::MenuAction::Settings)
            );
            assert_eq!(
                promises_dialog, opens_dialog,
                "T-668/T-634: `{}` (menu `{menu}`) — the `…` suffix and \"a dialog follows\" \
                 must agree exactly. Save Version and Mission Settings put a dialog in front of \
                 the operator; every other row acts, downloads or toggles.",
                it.label
            );
        }
    }
}
