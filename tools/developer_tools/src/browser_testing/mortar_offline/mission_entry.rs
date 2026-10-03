//! The fire mission the gate types into the calculator, and its comparison with the native solve.
//!
//! **Role:** types the mission (the weapon's high-explosive shell, the recommended charge, a map
//! click for the target, the gun [`super::mission_plan::GUN_OFFSET_M`] south, manual heights and
//! the wind), presses Calculate, and holds the page's tables to the native solution of what the
//! page now holds.
//! **Position:** called by the steps of `super` that solve on the page: the visit with the API
//! down behind the proxy and the visit with the listener stopped.
//! **Signals & state:** none; drives the page it is given.
//! **Invariants:** the comparison is exact at every shown digit (both sides word the solution
//! with `fire_mission_planning`'s `solution_wording`); a difference names the page's input values.

use anyhow::{Result, anyhow, bail};
use ballistics_model::catalog::BallisticsCatalog;
use map_coordinates::grid_reference::{GridFigures, format_grid, parse_grid};

use super::expected_solution::{TypedMission, expected_tables, solve_natively, table_differences};
use super::mission_plan::{
    GUN_HEIGHT_M, TARGET_HEIGHT_M, WIND, gun_position, high_explosive_shell,
};
use super::page_driver::{
    click_at, field_value, map_canvas_rect, press_calculate, read_solution_tables, set_field,
    wait_true,
};
use chrome_devtools_protocol::Page;

/// The page script that lists every input and picker value of the calculator.
const INPUT_VALUES: &str = "Array.from(document.querySelectorAll('[data-mortar-inputs] input, \
     [data-mortar-inputs] select')).map((e) => e.value).join(' | ')";

/// Types the mission, places the target with a map click and presses Calculate; returns what
/// the page now holds.
pub async fn enter_mission(page: &Page, catalog: &BallisticsCatalog) -> Result<TypedMission> {
    let weapon_id = field_value(page, "[data-mortar-input=\"weapon\"]", 0).await?;
    let shell_id = high_explosive_shell(catalog, &weapon_id)?;
    set_field(page, "[data-mortar-input=\"shell\"]", 0, &shell_id).await?;
    set_field(page, "[data-mortar-input=\"charge\"]", 0, "recommended").await?;
    set_field(page, "[data-mortar-input=\"placement\"]", 0, "target").await?;

    let rect = map_canvas_rect(page).await?;
    click_at(page, rect.x + rect.width / 2.0, rect.y + rect.height / 2.0).await?;
    let target_grid = "[data-mortar-position=\"Target\"] input[type=\"text\"]";
    wait_true(
        page,
        &format!("document.querySelector('{target_grid}').value.replace(/\\s/g, '').length === 10"),
        10,
        "the map click placing the target",
    )
    .await?;
    let typed_target = field_value(page, target_grid, 0).await?;
    let (tx, ty) = parse_grid(&typed_target).map_err(|e| anyhow!("target {typed_target}: {e}"))?;
    let (gx, gy) = gun_position(tx, ty);
    let gun_grid = format_grid(gx, gy, GridFigures::Ten);

    for (position, grid, height) in [
        ("Target", None, TARGET_HEIGHT_M),
        ("Gun", Some(gun_grid.as_str()), GUN_HEIGHT_M),
    ] {
        let scope = format!("[data-mortar-position=\"{position}\"]");
        if let Some(grid) = grid {
            set_field(page, &format!("{scope} input[type=\"text\"]"), 0, grid).await?;
        }
        set_field(page, &format!("{scope} select"), 0, "manual").await?;
        set_field(
            page,
            &format!("{scope} input[type=\"number\"]"),
            0,
            &format!("{height:.1}"),
        )
        .await?;
    }
    set_field(
        page,
        "[data-mortar-input=\"wind-speed\"]",
        0,
        &WIND.0.to_string(),
    )
    .await?;
    set_field(
        page,
        "[data-mortar-input=\"wind-from\"]",
        0,
        &WIND.1.to_string(),
    )
    .await?;
    press_calculate(page).await?;

    let read = |selector: &'static str| field_value(page, selector, 0);
    let (gun_x, gun_y) =
        parse_grid(&read("[data-mortar-position=\"Gun\"] input[type=\"text\"]").await?)
            .map_err(|e| anyhow!("gun grid: {e}"))?;
    let (target_x, target_y) =
        parse_grid(&read("[data-mortar-position=\"Target\"] input[type=\"text\"]").await?)
            .map_err(|e| anyhow!("target grid: {e}"))?;
    let gun_label = field_value(
        page,
        "[data-mortar-input=\"battery\"] input[type=\"text\"]",
        0,
    )
    .await?;
    Ok(TypedMission {
        weapon_id,
        shell_id,
        target: (target_x, target_y, TARGET_HEIGHT_M),
        gun: (gun_label, gun_x, gun_y, GUN_HEIGHT_M),
        wind: WIND,
    })
}

/// Holds the page's solution tables for `mission` to the native solve on `catalog`.
///
/// # Errors
///
/// Every differing cell, with the page's input values, or a failure to read the tables.
pub async fn solution_matches_native(
    page: &Page,
    catalog: &BallisticsCatalog,
    mission: &TypedMission,
) -> Result<()> {
    let page_tables = read_solution_tables(page).await?;
    let native = expected_tables(&solve_natively(catalog, mission)?);
    let differences = table_differences(&page_tables, &native);
    if !differences.is_empty() {
        let fields = page.evaluate(INPUT_VALUES, false).await?;
        bail!(
            "page fields {fields}; the page's solution differs from the native one for {mission:?}: {}",
            differences.join("; ")
        );
    }
    Ok(())
}
