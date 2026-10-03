use super::*;
use crate::browser_testing::mortar_offline::mission_plan::{
    COMMITTED_CATALOG, high_explosive_shell, read_catalog,
};
use crate::repository_layout::compiled_checkout_root;

fn committed_catalog() -> BallisticsCatalog {
    read_catalog(
        &compiled_checkout_root()
            .expect("repository root")
            .join(COMMITTED_CATALOG),
    )
    .unwrap()
}

fn mission_at(catalog: &BallisticsCatalog, distance_m: f64) -> TypedMission {
    let weapon_id = catalog.weapons[0].weapon_id.to_string();
    let shell_id = high_explosive_shell(catalog, &weapon_id).unwrap();
    TypedMission {
        weapon_id,
        shell_id,
        target: (6_400.5, 6_000.5, 42.0),
        gun: ("Gun 1".to_string(), 6_400.5, 6_000.5 - distance_m, 18.5),
        wind: (3.5, 250.0),
    }
}

#[test]
fn mortar_offline_expected_cells_are_the_shared_wording_of_the_native_solve() {
    let catalog = committed_catalog();
    let solution = solve_natively(&catalog, &mission_at(&catalog, 1_200.0)).unwrap();
    let gun = &solution.guns[0];
    let tables = expected_tables(&solution);

    let line = battery_line_words(gun, None);
    assert_eq!(
        tables.battery[0],
        vec![
            line.label,
            line.charge,
            line.elevation,
            line.aim_azimuth,
            line.time_of_flight
        ]
    );
    assert_eq!(tables.guns[0].heading, gun_heading(gun));
    for (cells, charge) in tables.guns[0].rows.iter().zip(&gun.charges) {
        let words = charge_row_words(charge);
        assert_eq!(
            *cells,
            vec![
                words.charge,
                words.elevation,
                words.aim_azimuth,
                words.deflection,
                words.range_correction,
                words.time_of_flight,
                words.apex
            ]
        );
    }
}

#[test]
fn mortar_offline_expected_tables_lay_the_recommended_charge() {
    let catalog = committed_catalog();
    let solution = solve_natively(&catalog, &mission_at(&catalog, 1_200.0)).unwrap();
    let gun = &solution.guns[0];
    let rings = gun.recommended_rings.expect("1200 m solves");
    let tables = expected_tables(&solution);

    assert_eq!(tables.battery.len(), 1);
    let battery = &tables.battery[0];
    assert_eq!(battery[0], "Gun 1");
    assert_eq!(battery[1], format!("Charge {rings}"));
    assert!(
        battery[2].ends_with('°') && battery[2].contains(" mils · "),
        "{battery:?}"
    );
    assert!(battery[4].ends_with(" s"), "{battery:?}");

    let table = &tables.guns[0];
    assert!(
        table.heading.starts_with("Gun 1 — 1200 m · line "),
        "{}",
        table.heading
    );
    assert!(table.heading.ends_with("Δh +23.5 m"), "{}", table.heading);
    assert_eq!(table.rows.len(), gun.charges.len());
    assert!(table.rows.iter().all(|row| row.len() == 7));
    assert_eq!(table.laid.iter().filter(|laid| **laid).count(), 1);
    let laid_row = table.laid.iter().position(|laid| *laid).unwrap();
    assert_eq!(table.rows[laid_row][0], format!("Charge {rings}"));
    assert_eq!(table.rows[laid_row][1], battery[2]);
    assert_eq!(table.rows[laid_row][2], battery[3]);
    assert_eq!(table.rows[laid_row][5], battery[4]);
}

#[test]
fn mortar_offline_refused_charges_show_why_and_no_figures() {
    let catalog = committed_catalog();
    let solution = solve_natively(&catalog, &mission_at(&catalog, 4_000.0)).unwrap();
    let tables = expected_tables(&solution);
    let refused: Vec<&Vec<String>> = tables.guns[0]
        .rows
        .iter()
        .filter(|row| row[2].is_empty())
        .collect();
    assert!(!refused.is_empty(), "4000 m refuses the low charges");
    for row in refused {
        assert!(
            row[1] == "out of range" || row[1] == "too close for this charge",
            "{row:?}"
        );
        assert!(row[2..].iter().all(String::is_empty), "{row:?}");
    }
}

#[test]
fn mortar_offline_table_differences_report_any_changed_cell() {
    let catalog = committed_catalog();
    let solution = solve_natively(&catalog, &mission_at(&catalog, 1_200.0)).unwrap();
    let expected = expected_tables(&solution);
    assert!(table_differences(&expected, &expected).is_empty());

    let mut elevation = expected.clone();
    elevation.guns[0].rows[0][1].push('0');
    assert_eq!(table_differences(&elevation, &expected).len(), 1);

    let mut laid = expected.clone();
    laid.guns[0].laid.iter_mut().for_each(|l| *l = !*l);
    assert_eq!(table_differences(&laid, &expected).len(), 1);

    let mut battery = expected.clone();
    battery.battery[0][4] = "0.0 s".to_string();
    assert_eq!(table_differences(&battery, &expected).len(), 1);

    let mut missing = expected.clone();
    missing.guns.clear();
    assert_eq!(table_differences(&missing, &expected).len(), 1);
}

#[test]
fn mortar_offline_page_tables_decode_from_the_page_script_shape() {
    let tables: SolutionTables = serde_json::from_value(serde_json::json!({
        "battery": [["Gun 1", "Charge 2", "a", "b", "c"]],
        "guns": [{ "heading": "h", "rows": [["Charge 2"]], "laid": [true] }],
    }))
    .unwrap();
    assert_eq!(tables.guns[0].laid, vec![true]);
}
