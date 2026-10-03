//! The committed vanilla mortar catalog and its calibration bundle, judged whole.
//!
//! Every document comes from `contracts` through `include_str!`, so a missing fixture fails
//! the build and a regenerated fixture is judged as committed.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use ballistics_model::ids::ShellId;

use super::charge_flight::{ChargeFlights, ONE_MIL_6400_RAD};
use super::native_tables::judge_native_table;
use super::provenance::check_provenance;
use super::*;

const CATALOG_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json"
));
const BUNDLE_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/fixtures/ballistics/vanilla_mortars.v1/calibration.json"
));
const SKEWED_NATIVE_ROW_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/fixtures/ballistics/vanilla_mortars.v1/negative/skewed_native_row.calibration.json"
));
const WRONG_GAME_BUILD_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/fixtures/ballistics/vanilla_mortars.v1/negative/wrong_game_build.calibration.json"
));
const MISSING_SHELL_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/fixtures/ballistics/vanilla_mortars.v1/negative/missing_shell.calibration.json"
));
const STALE_CATALOG_SHA_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../contracts/fixtures/ballistics/vanilla_mortars.v1/negative/stale_catalog_sha.calibration.json"
));

fn pinned() -> &'static PinnedCatalog {
    static PINNED: OnceLock<PinnedCatalog> = OnceLock::new();
    PINNED.get_or_init(|| {
        PinnedCatalog::from_json_slice(CATALOG_JSON.as_bytes()).expect("committed catalog decodes")
    })
}

fn bundle() -> &'static CalibrationBundle {
    static BUNDLE: OnceLock<CalibrationBundle> = OnceLock::new();
    BUNDLE.get_or_init(|| decode(BUNDLE_JSON))
}

fn decode(json: &str) -> CalibrationBundle {
    CalibrationBundle::from_json_slice(json.as_bytes()).expect("committed bundle decodes")
}

fn kinds(report: &CalibrationReport) -> Vec<FailureKind> {
    report.failures.iter().map(|failure| failure.kind).collect()
}

/// Failure counts per kind, then the first failures verbatim.
fn summary(report: &CalibrationReport) -> String {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for failure in &report.failures {
        *counts.entry(format!("{:?}", failure.kind)).or_default() += 1;
    }
    let listed: Vec<String> = report
        .failures
        .iter()
        .take(12)
        .map(|failure| format!("  {}: {}", failure.case_id, failure.reason))
        .collect();
    format!(
        "{} failures over {} cases {counts:?}\n{}",
        report.failures.len(),
        report.cases,
        listed.join("\n")
    )
}

/// Whether a forward-angle sample was fired at a native row of its shell and coefficient, read
/// from the oracle's own `elevation_mils_6400` input rather than the judge's radians.
fn forward_sample_is_at_a_native_row(bundle: &CalibrationBundle, sample: &OracleSample) -> bool {
    let mils = sample.inputs["elevation_mils_6400"]
        .as_f64()
        .expect("committed forward samples carry elevation_mils_6400");
    bundle.native_tables.iter().any(|table| {
        table.shell_id == sample.shell_id
            && table.init_speed_coef == sample.init_speed_coef
            && table.rows.iter().any(|row| row.elevation_mils_6400 == mils)
    })
}

/// The forward-angle samples of one shell: `(at a native row, between rows)`.
fn forward_sample_counts(bundle: &CalibrationBundle, shell_id: &ShellId) -> (usize, usize) {
    let forward: Vec<&OracleSample> = bundle
        .oracle_samples
        .iter()
        .filter(|sample| {
            sample.shell_id == *shell_id && sample.kind == OracleSampleKind::ForwardAngle
        })
        .collect();
    let at_rows = forward
        .iter()
        .filter(|sample| forward_sample_is_at_a_native_row(bundle, sample))
        .count();
    (at_rows, forward.len() - at_rows)
}

/// Native rows, forward-angle samples at native rows, simulation samples and wind rows of one
/// shell.
fn expected_case_count(bundle: &CalibrationBundle, shell_id: &ShellId) -> u32 {
    let native: usize = bundle
        .native_tables
        .iter()
        .filter(|table| table.shell_id == *shell_id)
        .map(|table| table.rows.len())
        .sum();
    let wind: usize = bundle
        .wind_tables
        .iter()
        .filter(|table| table.shell_id == *shell_id)
        .map(|table| table.rows.len())
        .sum();
    let simulation = bundle
        .oracle_samples
        .iter()
        .filter(|sample| {
            sample.shell_id == *shell_id && sample.kind == OracleSampleKind::Simulation
        })
        .count();
    let (forward_at_rows, _) = forward_sample_counts(bundle, shell_id);
    u32::try_from(native + wind + simulation + forward_at_rows).expect("case count fits u32")
}

fn assert_shell_passes(shell_id: &str) {
    let shell_id = &ShellId::new(shell_id);
    let bundle = bundle();
    let report = evaluate_shell(pinned(), bundle, shell_id);
    let expected_cases = expected_case_count(bundle, shell_id);
    let (forward_at_rows, forward_between_rows) = forward_sample_counts(bundle, shell_id);
    assert!(
        expected_cases > 0 && forward_at_rows > 0,
        "the bundle carries no case or no forward sample at a native row for shell {shell_id}"
    );
    assert_eq!(
        report.cases, expected_cases,
        "every case of {shell_id} is judged"
    );
    assert_eq!(
        report.interpolated_forward_samples as usize, forward_between_rows,
        "every forward sample of {shell_id} between native rows is counted, not judged"
    );
    assert!(report.accepted(), "shell {shell_id}: {}", summary(&report));
}

#[test]
fn committed_shell_m821_passes_every_case() {
    assert_shell_passes("m821");
}

#[test]
fn committed_shell_m819_passes_every_case() {
    assert_shell_passes("m819");
}

#[test]
fn committed_shell_m853a1_passes_every_case() {
    assert_shell_passes("m853a1");
}

#[test]
fn committed_shell_m879_passes_every_case() {
    assert_shell_passes("m879");
}

#[test]
fn committed_shell_o832du_passes_every_case() {
    assert_shell_passes("o832du");
}

#[test]
fn committed_shell_d832du_passes_every_case() {
    assert_shell_passes("d832du");
}

#[test]
fn committed_shell_s832s_passes_every_case() {
    assert_shell_passes("s832s");
}

#[test]
fn committed_shells_are_exactly_the_per_shell_tests() {
    let shell_ids: Vec<&str> = pinned()
        .catalog
        .shells
        .iter()
        .map(|shell| shell.shell_id.as_str())
        .collect();
    assert_eq!(
        shell_ids,
        [
            "m821", "m819", "m853a1", "m879", "o832du", "d832du", "s832s"
        ]
    );
}

#[test]
fn committed_bundle_provenance_and_coverage_hold() {
    let report = check_provenance(pinned(), bundle());
    assert!(report.accepted(), "{}", summary(&report));
    assert_eq!(report.cases, 0, "provenance checks are not cases");
}

#[test]
fn native_column_1_is_the_line_of_departure_height_above_the_point_of_fall() {
    let mut rows = 0;
    for table in &bundle().native_tables {
        for row in &table.rows {
            rows += 1;
            let elevation_mils = libm::atan2(row.column_1, row.range_m) / ONE_MIL_6400_RAD;
            assert!(
                (elevation_mils - row.elevation_mils_6400).abs() <= 0.01,
                "{} {} row {}: atan2(column_1, range) is {elevation_mils} mils, row elevation {}",
                table.shell_id,
                table.init_speed_coef,
                row.lattice_index,
                row.elevation_mils_6400
            );
        }
    }
    assert!(rows > 0);
}

#[test]
fn skewed_native_row_variant_fails_that_row_by_range() {
    let skewed = decode(SKEWED_NATIVE_ROW_JSON);
    assert!(check_provenance(pinned(), &skewed).accepted());
    let mut differing = Vec::new();
    for (good, bad) in bundle().native_tables.iter().zip(&skewed.native_tables) {
        for (good_row, bad_row) in good.rows.iter().zip(&bad.rows) {
            if good_row != bad_row {
                differing.push((bad.clone(), bad_row.clone()));
            }
        }
    }
    assert_eq!(differing.len(), 1, "the variant skews exactly one row");
    let (table, row) = &differing[0];
    let shell = pinned()
        .catalog
        .shells
        .iter()
        .find(|shell| shell.shell_id == table.shell_id)
        .expect("skewed table names a catalog shell");
    let mut flights = ChargeFlights::new(pinned().catalog.gravity_m_s2, shell);
    let mut report = CalibrationReport::default();
    judge_native_table(&mut report, &mut flights, table);
    let case_id = format!(
        "native/{}/{}/{}",
        table.shell_id, table.init_speed_coef, row.lattice_index
    );
    assert!(
        report
            .failures
            .iter()
            .any(|failure| failure.kind == FailureKind::NativeRowRange
                && failure.case_id == *case_id),
        "{case_id} is refused by range: {}",
        summary(&report)
    );
}

#[test]
fn wrong_game_build_variant_fails_only_by_game_build() {
    let report = check_provenance(pinned(), &decode(WRONG_GAME_BUILD_JSON));
    assert_eq!(kinds(&report), [FailureKind::GameBuildMismatch]);
}

#[test]
fn stale_catalog_sha_variant_fails_only_by_catalog_sha() {
    let report = check_provenance(pinned(), &decode(STALE_CATALOG_SHA_JSON));
    assert_eq!(kinds(&report), [FailureKind::CatalogShaMismatch]);
}

#[test]
fn missing_shell_variant_fails_every_charge_of_that_shell_by_coverage() {
    let missing = decode(MISSING_SHELL_JSON);
    let absent: Vec<_> = pinned()
        .catalog
        .shells
        .iter()
        .filter(|shell| {
            !missing
                .native_tables
                .iter()
                .any(|table| table.shell_id == shell.shell_id)
        })
        .collect();
    assert_eq!(absent.len(), 1, "the variant drops exactly one shell");
    let shell = absent[0];
    let mut expected = Vec::new();
    for charge in &shell.charges {
        let case_id = format!("coverage/{}/{}", shell.shell_id, charge.rings);
        expected.push((FailureKind::MissingNativeTable, case_id.clone()));
        expected.push((FailureKind::MissingSimulationSample, case_id));
    }
    let report = check_provenance(pinned(), &missing);
    let actual: Vec<_> = report
        .failures
        .iter()
        .map(|failure| (failure.kind, failure.case_id.to_string()))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn a_catalog_sha_mismatch_is_red() {
    let mut altered = bundle().clone();
    altered.catalog_sha256 = "0".repeat(64);
    assert_eq!(
        kinds(&check_provenance(pinned(), &altered)),
        [FailureKind::CatalogShaMismatch]
    );
    let respaced = format!("{CATALOG_JSON} ");
    let repinned = PinnedCatalog::from_json_slice(respaced.as_bytes()).expect("still decodes");
    assert_eq!(repinned.catalog, pinned().catalog);
    assert_eq!(
        kinds(&check_provenance(&repinned, bundle())),
        [FailureKind::CatalogShaMismatch],
        "the digest covers the bytes, not the decoded catalog"
    );
}

#[test]
fn a_game_build_mismatch_is_red() {
    let mut altered = bundle().clone();
    altered.game_build = format!("{}.1", altered.game_build);
    assert_eq!(
        kinds(&check_provenance(pinned(), &altered)),
        [FailureKind::GameBuildMismatch]
    );
}

#[test]
fn missing_coverage_is_red() {
    let shell = &pinned().catalog.shells[0];
    let charge = &shell.charges[0];
    let mut altered = bundle().clone();
    altered.oracle_samples.retain(|sample| {
        !(sample.kind == OracleSampleKind::Simulation
            && sample.shell_id == shell.shell_id
            && charge_flight::same_coefficient(sample.init_speed_coef, charge.init_speed_coef))
    });
    altered.native_tables.retain(|table| {
        !(table.shell_id == shell.shell_id
            && charge_flight::same_coefficient(table.init_speed_coef, charge.init_speed_coef))
    });
    assert_eq!(
        kinds(&check_provenance(pinned(), &altered)),
        [
            FailureKind::MissingNativeTable,
            FailureKind::MissingSimulationSample
        ]
    );
}

#[test]
fn a_resource_declared_with_two_digests_is_red() {
    let mut altered = bundle().clone();
    let mut duplicate = pinned().catalog.resources[0].clone();
    duplicate.sha256 = "f".repeat(64);
    altered.resources.push(duplicate);
    assert_eq!(
        kinds(&check_provenance(pinned(), &altered)),
        [FailureKind::ResourceShaMismatch]
    );
}

#[test]
fn an_unknown_shell_is_red() {
    let report = evaluate_shell(pinned(), bundle(), &ShellId::new("no_such_shell"));
    assert_eq!(kinds(&report), [FailureKind::UnknownShell]);
    let mut altered = bundle().clone();
    altered.wind_tables[0].shell_id = ShellId::new("no_such_shell");
    assert_eq!(
        kinds(&check_provenance(pinned(), &altered)),
        [FailureKind::UnknownShell]
    );
}

#[test]
fn an_unsupported_schema_version_is_refused() {
    let altered = BUNDLE_JSON.replacen("\"schema_version\": 1", "\"schema_version\": 2", 1);
    assert!(matches!(
        CalibrationBundle::from_json_slice(altered.as_bytes()),
        Err(CalibrationDecodeError::UnsupportedSchemaVersion(2))
    ));
}

fn first_charge_flights() -> (ChargeFlights<'static>, f64, ShellId) {
    let shell = &pinned().catalog.shells[0];
    let flights = ChargeFlights::new(pinned().catalog.gravity_m_s2, shell);
    (
        flights,
        shell.charges[0].init_speed_coef,
        shell.shell_id.clone(),
    )
}

fn simulation_sample(
    shell_id: &ShellId,
    coefficient: f64,
    target_height_m: f64,
    reached: bool,
) -> OracleSample {
    OracleSample {
        kind: OracleSampleKind::Simulation,
        shell_id: shell_id.clone(),
        init_speed_coef: coefficient,
        inputs: serde_json::json!({
            "elevation_deg": 60.0, "azimuth_deg": 0.0, "wind_speed_m_s": 0.0,
            "wind_from_deg": 0.0, "target_height_m": target_height_m
        }),
        outputs: serde_json::json!({
            "downrange_m": 0.0, "crossrange_m": 0.0, "time_of_flight_s": 0.0,
            "reached_target_height": reached
        }),
    }
}

#[test]
fn a_sample_missing_a_needed_value_is_red_not_skipped() {
    let (mut flights, coefficient, shell_id) = first_charge_flights();
    let mut report = CalibrationReport::default();
    let forward = OracleSample {
        kind: OracleSampleKind::ForwardAngle,
        shell_id: shell_id.clone(),
        init_speed_coef: coefficient,
        inputs: serde_json::json!({ "rings": 0 }),
        outputs: serde_json::json!({ "range_m": 1.0, "time_of_flight_s": 1.0 }),
    };
    oracle_samples::judge_oracle_sample(
        &mut report,
        &mut flights,
        &bundle().native_tables,
        0,
        &forward,
    );
    let mut simulation = simulation_sample(&shell_id, coefficient, 0.0, true);
    simulation.outputs = serde_json::json!({ "downrange_m": 1.0 });
    oracle_samples::judge_oracle_sample(
        &mut report,
        &mut flights,
        &bundle().native_tables,
        1,
        &simulation,
    );
    assert_eq!(report.cases, 2);
    assert_eq!(
        kinds(&report),
        [FailureKind::UnreadableCase, FailureKind::UnreadableCase]
    );
}

#[test]
fn a_simulation_the_engine_never_lands_passes_only_when_the_model_refuses_too() {
    let (mut flights, coefficient, shell_id) = first_charge_flights();
    let mut report = CalibrationReport::default();
    let unreachable = simulation_sample(&shell_id, coefficient, 100_000.0, false);
    oracle_samples::judge_oracle_sample(
        &mut report,
        &mut flights,
        &bundle().native_tables,
        0,
        &unreachable,
    );
    assert!(report.accepted(), "{}", summary(&report));
    let reachable = simulation_sample(&shell_id, coefficient, 0.0, false);
    oracle_samples::judge_oracle_sample(
        &mut report,
        &mut flights,
        &bundle().native_tables,
        1,
        &reachable,
    );
    assert_eq!(report.cases, 2);
    assert_eq!(kinds(&report), [FailureKind::SimulationImpact]);
}

#[test]
fn altitude_difference_samples_are_carried_but_not_judged() {
    let (mut flights, coefficient, shell_id) = first_charge_flights();
    let mut report = CalibrationReport::default();
    let sample = OracleSample {
        kind: OracleSampleKind::AltitudeDifference,
        shell_id,
        init_speed_coef: coefficient,
        inputs: serde_json::json!({}),
        outputs: serde_json::json!({}),
    };
    oracle_samples::judge_oracle_sample(
        &mut report,
        &mut flights,
        &bundle().native_tables,
        0,
        &sample,
    );
    assert_eq!(report, CalibrationReport::default());
}

/// The first shell's first native table and one of its rows away from the vertical.
fn first_native_row() -> (&'static NativeTable, &'static NativeTableRow) {
    let shell = &pinned().catalog.shells[0];
    let table = bundle()
        .native_tables
        .iter()
        .find(|table| {
            table.shell_id == shell.shell_id
                && table.init_speed_coef == shell.charges[0].init_speed_coef
        })
        .expect("the first charge has a native table");
    (table, &table.rows[3])
}

fn forward_sample(
    table: &NativeTable,
    elevation_rad: f64,
    range_m: f64,
    time_s: f64,
) -> OracleSample {
    OracleSample {
        kind: OracleSampleKind::ForwardAngle,
        shell_id: table.shell_id.clone(),
        init_speed_coef: table.init_speed_coef,
        inputs: serde_json::json!({ "elevation_rad": elevation_rad }),
        outputs: serde_json::json!({ "range_m": range_m, "time_of_flight_s": time_s }),
    }
}

#[test]
fn a_forward_sample_between_native_rows_is_not_judged_and_one_at_a_row_is() {
    let (table, row) = first_native_row();
    let mut flights =
        ChargeFlights::new(pinned().catalog.gravity_m_s2, &pinned().catalog.shells[0]);
    let native = &bundle().native_tables;
    let mut report = CalibrationReport::default();
    let at_row = forward_sample(
        table,
        row.elevation_rad(),
        row.range_m,
        row.time_of_flight_s,
    );
    oracle_samples::judge_oracle_sample(&mut report, &mut flights, native, 0, &at_row);
    assert_eq!((report.cases, report.interpolated_forward_samples), (1, 0));
    assert!(report.accepted(), "{}", summary(&report));

    let between = row.elevation_rad() + 0.75 * ONE_MIL_6400_RAD;
    let absurd_between = forward_sample(table, between, 1.0e6, 1.0e3);
    oracle_samples::judge_oracle_sample(&mut report, &mut flights, native, 1, &absurd_between);
    assert_eq!((report.cases, report.interpolated_forward_samples), (1, 1));
    assert!(report.accepted(), "{}", summary(&report));

    let absurd_at_row = forward_sample(table, row.elevation_rad(), 1.0e6, row.time_of_flight_s);
    oracle_samples::judge_oracle_sample(&mut report, &mut flights, native, 2, &absurd_at_row);
    assert_eq!((report.cases, report.interpolated_forward_samples), (2, 1));
    assert_eq!(kinds(&report), [FailureKind::ForwardSampleRange]);
}

#[test]
fn the_wind_crosswind_value_is_the_deflection_angle_over_the_crosswind_downrange() {
    let outcome = ballistics_model::flight_model::FlightOutcome {
        time_of_flight_s: 10.0,
        impact_position_m: [10.0, 1000.0, 0.0],
        impact_velocity_m_s: [0.0; 3],
        downrange_m: 1000.0,
        deflection_m: 10.0,
        apex_height_m: 100.0,
        apex_time_s: 5.0,
        path: Vec::new(),
    };
    assert_eq!(
        wind_tables::crosswind_deflection_angle_rad(&outcome),
        libm::atan(0.01)
    );

    let shell = &pinned().catalog.shells[0];
    let table = bundle()
        .wind_tables
        .iter()
        .find(|table| table.shell_id == shell.shell_id)
        .expect("the first shell has a wind table");
    let mut flights = ChargeFlights::new(pinned().catalog.gravity_m_s2, shell);
    let row = &table.rows[60];
    let crosswind = ballistics_model::wind::Wind {
        speed_m_s: table.wind_speed_m_s,
        from_deg: 270.0,
    };
    let flight = flights.at(table.init_speed_coef);
    let elevation_rad = flight
        .calm_elevation_for_range(row.elevation_rad, row.range_m)
        .expect("the row's calm range lies in its window");
    let flown = flight
        .fly(elevation_rad, 0.0, &crosswind, 0.0)
        .expect("the crosswind flight lands");
    let modelled_mrad = 1000.0 * wind_tables::crosswind_deflection_angle_rad(&flown);
    let mut judged = |offset_mrad: f64| {
        let mut shifted = table.clone();
        shifted.rows = vec![row.clone()];
        shifted.rows[0].values[0] = modelled_mrad + offset_mrad;
        let mut report = CalibrationReport::default();
        wind_tables::judge_wind_table(&mut report, &mut flights, &shifted);
        kinds(&report)
    };
    let one_mil_mrad = 1000.0 * ONE_MIL_6400_RAD;
    assert_eq!(judged(0.0), []);
    assert_eq!(judged(0.99 * one_mil_mrad), []);
    assert_eq!(judged(1.01 * one_mil_mrad), [FailureKind::WindRowCrosswind]);
    assert_eq!(
        judged(-1.01 * one_mil_mrad),
        [FailureKind::WindRowCrosswind]
    );
}

/// The flights of the shell named `shell_id`.
fn flights_of<'list>(
    flights: &'list mut [(ShellId, ChargeFlights<'static>)],
    shell_id: &ShellId,
) -> &'list mut ChargeFlights<'static> {
    let index = flights
        .iter()
        .position(|(id, _)| id == shell_id)
        .expect("every sample names a catalog shell");
    &mut flights[index].1
}

/// The single-precision flight reproduces the engine to its own rounding: over every committed
/// simulation sample the point of fall is within 0.010 m downrange and 0.001 m crossrange (a
/// double-precision flight of the same step misses by up to 0.016 m), and every native row's
/// range and time of flight are met at the row's elevation within the table's rounding.
#[test]
fn the_single_precision_flight_meets_every_simulation_sample_and_native_row() {
    let catalog = &pinned().catalog;
    let mut flights: Vec<(ShellId, ChargeFlights<'static>)> = catalog
        .shells
        .iter()
        .map(|shell| {
            (
                shell.shell_id.clone(),
                ChargeFlights::new(catalog.gravity_m_s2, shell),
            )
        })
        .collect();
    let (mut down_max, mut cross_max, mut landed) = (0.0_f64, 0.0_f64, 0_usize);
    for sample in &bundle().oracle_samples {
        if sample.kind != OracleSampleKind::Simulation
            || sample.outputs["reached_target_height"] != serde_json::json!(true)
        {
            continue;
        }
        let input = |name: &str| sample.inputs[name].as_f64().expect("simulation input");
        let output = |name: &str| sample.outputs[name].as_f64().expect("simulation output");
        let wind = ballistics_model::wind::Wind {
            speed_m_s: input("wind_speed_m_s"),
            from_deg: input("wind_from_deg"),
        };
        let outcome = flights_of(&mut flights, &sample.shell_id)
            .at(sample.init_speed_coef)
            .fly(
                input("elevation_deg").to_radians(),
                input("azimuth_deg").to_radians(),
                &wind,
                input("target_height_m"),
            )
            .expect("the model lands every sample the engine lands");
        down_max = down_max.max((outcome.downrange_m - output("downrange_m")).abs());
        cross_max = cross_max.max((outcome.deflection_m - output("crossrange_m")).abs());
        landed += 1;
    }
    let (mut range_max, mut time_max, mut rows) = (0.0_f64, 0.0_f64, 0_usize);
    for table in &bundle().native_tables {
        let flight = flights_of(&mut flights, &table.shell_id).at(table.init_speed_coef);
        for row in &table.rows {
            let outcome = flight
                .fly_calm(row.elevation_rad())
                .expect("every native row lands");
            range_max = range_max.max((outcome.downrange_m - row.range_m).abs());
            time_max = time_max.max((outcome.time_of_flight_s - row.time_of_flight_s).abs());
            rows += 1;
        }
    }
    eprintln!(
        "simulation samples landed {landed}: downrange max {down_max:.5} m, crossrange max \
         {cross_max:.5} m; native rows {rows}: range max {range_max:.5} m, time max {time_max:.5} s"
    );
    assert!(landed > 4000 && rows > 400, "{landed} samples, {rows} rows");
    assert!(down_max <= 0.010, "downrange residual {down_max} m");
    assert!(cross_max <= 0.001, "crossrange residual {cross_max} m");
    assert!(range_max <= 0.011, "native range residual {range_max} m");
    assert!(time_max <= 0.001, "native time residual {time_max} s");
}

/// The wind tables state elevations to three decimals; each row's calm range pins it back to
/// the export's one-degree lattice (row `i` at `i + 1` degrees). A range difference `δR` moves
/// the elevation by `δR / |dR/dθ|`, so the pinned elevation lies within 0.011 m (the native-row
/// range residual bound of the single-precision flight) over the local slope of the lattice
/// elevation; away from the maximum-range elevation that is a small fraction of a mil.
#[test]
fn a_rounded_wind_row_elevation_is_pinned_by_its_calm_range() {
    let catalog = &pinned().catalog;
    let (mut pinned_rows, mut sharp_rows) = (0, 0);
    for table in &bundle().wind_tables {
        let shell = catalog
            .shells
            .iter()
            .find(|shell| shell.shell_id == table.shell_id)
            .expect("every wind table names a catalog shell");
        let mut flights = ChargeFlights::new(catalog.gravity_m_s2, shell);
        let flight = flights.at(table.init_speed_coef);
        let calm_range = |elevation: f64| {
            flight
                .fly_calm(elevation)
                .expect("the calm flight lands")
                .downrange_m
        };
        for (index, row) in table.rows.iter().enumerate() {
            if row.range_m <= 0.0 {
                continue;
            }
            let lattice_rad = ((index + 1) as f64).to_radians();
            let pinned_rad = flight
                .calm_elevation_for_range(row.elevation_rad, row.range_m)
                .expect("the row's calm range lies in its window");
            let slope = (calm_range(lattice_rad + 1e-3) - calm_range(lattice_rad - 1e-3)) / 2e-3;
            let bound_rad = 0.011 / slope.abs();
            assert!(
                (pinned_rad - lattice_rad).abs() <= bound_rad,
                "wind/{}/{}/{index}: pinned {pinned_rad} rad, lattice {lattice_rad} rad, \
                 stated {}, bound {bound_rad}",
                table.shell_id,
                table.init_speed_coef,
                row.elevation_rad
            );
            pinned_rows += 1;
            if bound_rad <= 0.1 * ONE_MIL_6400_RAD {
                sharp_rows += 1;
            }
        }
    }
    assert!(
        pinned_rows > 2000 && sharp_rows * 10 >= pinned_rows * 9,
        "{pinned_rows} rows pinned, {sharp_rows} within a tenth of a mil"
    );
}
