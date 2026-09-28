//! A small, self-consistent gameplay export and oracle output for the trim tests: one mortar, one
//! range card and one shell with two charges, plus a table at a coefficient that is not a charge.
//!
//! The charge tables refine their row spacing toward the vertical as the game's tables do (12.5,
//! 25, 100 and 200 mils), and the oracle's forward lattice step is [`FORWARD_STEP_MILS`], which
//! holds every row: the vertical first row and the maximum-range last row sit at the lattice ends
//! the engine answers with its sentinel, and every other row equals one forward sample. The
//! forward samples are the linear interpolation of the unskewed rows, exactly as the engine
//! answers.
use crate::commands::ballistics::catalog_extraction::WeaponSelection;
use crate::commands::ballistics::gameplay_export::sha256_hex;
use crate::commands::ballistics::trim_export::TrimLocations;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) const GENERATION_ID: &str = "0123456789ABCDEF";
const GAME_BUILD: &str = "1.8.0.13";
const PLUGIN_REVISION: &str = "tbd-ballistics-oracle/2";
const WEAPON: &str = "1111111111111111";
const RANGE_CARD: &str = "2222222222222222";
const SHELL: &str = "3333333333333333";
/// Every shell as `(prefab GUID, ammunition key)`; the first is the mortar's default ammunition.
const SHELLS: [(&str, &str); 2] = [(SHELL, "T1"), ("6666666666666666", "T2")];
const TABLES: &str = "4444444444444444";
const WIND: &str = "5555555555555555";

pub(super) const SELECTION: [WeaponSelection; 1] = [WeaponSelection {
    weapon_id: "test_mortar",
    prefab_guid: WEAPON,
    range_card_guid: RANGE_CARD,
}];

/// Charges as `(rings, coefficient, is_default)`.
const CHARGES: [(u32, f64, bool); 2] = [(0, 1.0, false), (1, 1.5, true)];
/// Table coefficients: both charges and one coefficient no charge uses.
const TABLE_COEFFICIENTS: [f64; 3] = [1.0, 1.5, 2.0];
/// Row elevations of every table, in 6400-mil units, in the game's order.
const ROW_ELEVATIONS: [f64; 10] = [
    1600.0, 1587.5, 1550.0, 1537.5, 1525.0, 1500.0, 1400.0, 1200.0, 1000.0, 800.0,
];

/// The oracle's forward lattice step in 6400-mil units: a divisor of every row spacing.
pub(super) const FORWARD_STEP_MILS: f64 = 12.5;

/// A row to skew: `(table coefficient, row index, metres added to its range)`.
pub(super) type RowSkew = (f64, usize, f64);

fn single(value: f64) -> f64 {
    f64::from(value as f32)
}

/// `[range, column_1, time_of_flight]` at `elevation` for coefficient `coefficient`.
fn row(elevation: f64, coefficient: f64) -> [f64; 3] {
    let angle = elevation / 6400.0 * std::f64::consts::TAU;
    let range = 400.0 * coefficient * coefficient * (2.0 * angle).sin().abs();
    let time = 14.0 * coefficient * angle.sin();
    [single(range), single(2.0 * range + 1.0), single(time)]
}

fn rows(coefficient: f64) -> Vec<[f64; 3]> {
    ROW_ELEVATIONS
        .iter()
        .map(|elevation| row(*elevation, coefficient))
        .collect()
}

fn forward_samples(coefficient: f64, rings: Option<u32>, step_mils: f64) -> Vec<Value> {
    let rows = rows(coefficient);
    let last = rows[rows.len() - 1];
    let steps = (800.0 / step_mils).round() as u32;
    (0..=steps)
        .map(|step| 1600.0 - step_mils * f64::from(step))
        .map(|elevation: f64| {
            let (range, time) = if elevation == 1600.0 || elevation == 800.0 {
                (last[0], -1.0)
            } else {
                let upper = ROW_ELEVATIONS
                    .iter()
                    .rposition(|row| *row >= elevation)
                    .unwrap_or(0);
                let (a, b) = (ROW_ELEVATIONS[upper], ROW_ELEVATIONS[upper + 1]);
                let fraction = (a - elevation) / (a - b);
                (
                    single(rows[upper][0] + (rows[upper + 1][0] - rows[upper][0]) * fraction),
                    single(rows[upper][2] + (rows[upper + 1][2] - rows[upper][2]) * fraction),
                )
            };
            json!({"init_speed_coef": coefficient, "rings": rings,
                   "inputs": {"elevation_mils_6400": elevation, "direct_fire": false},
                   "outputs": {"range_m": range, "time_of_flight_s": time}})
        })
        .collect()
}

fn node(id: &str, class: &str, properties: Value) -> Value {
    let properties: serde_json::Map<String, Value> = properties
        .as_object()
        .expect("properties object")
        .iter()
        .map(|(key, value)| (key.clone(), json!({"status": "present", "value": value})))
        .collect();
    json!({"node_id": id, "class_name": class, "properties": properties})
}

fn resource(guid: &str, path: &str, nodes: Vec<Value>, name: Option<&str>) -> Value {
    let names: Vec<Value> = name
        .map(|name| json!({"display_name_en": {"status": "present", "value": name}}))
        .into_iter()
        .collect();
    json!({"resource_name": format!("{{{guid}}}{path}"), "nodes": nodes, "names": names})
}

fn resources(skew: Option<RowSkew>) -> Vec<(String, String, Value)> {
    let charge_config: Vec<Value> = CHARGES
        .iter()
        .map(|(rings, coefficient, default)| {
            json!([
                f64::from(*rings),
                coefficient,
                if *default { 1.0 } else { 0.0 }
            ])
        })
        .collect();
    let shell = |guid: &str, ammunition: &str| {
        resource(
            guid,
            "Prefabs/Shell.et",
            vec![
                node(
                    "root/magazine",
                    "MagazineUIInfo",
                    json!({"m_sAmmoType": format!("#AR-AmmoType_{ammunition}"), "m_eAmmoTypeFlags": 40}),
                ),
                node(
                    "root/move",
                    "ShellMoveComponent",
                    json!({"InitSpeed": 66.0, "InitSpeedVariation": 3.0, "Mass": 4.0,
            "AirDrag": single(0.0005), "SideAirDragScale": 10.0, "WindInfluenceMultiplier": 1.0,
            "DispersionMultiplier": 1.0, "TimeToLive": 60.0, "Diameter": 81.0,
            "BallisticTableConfig": format!("{{{TABLES}}}Configs/Tables.conf"),
            "ProjectileWindTableConfig": format!("{{{WIND}}}Configs/Wind.conf")}),
                ),
                node(
                    "root/gadget",
                    "SCR_MortarShellGadgetComponent",
                    json!({"m_aChargeRingConfig": charge_config, "m_bIsUsingTimeFuze": false}),
                ),
            ],
            Some("Test shell"),
        )
    };
    let mut pages = Vec::new();
    for (guid, _) in SHELLS {
        for (rings, coefficient, _) in CHARGES {
            pages.push(node(&format!("root/page/{guid}/{rings}"), "SCR_VisualisedBallisticConfig", json!({
                "m_sProjectilePrefab": format!("{{{guid}}}Prefabs/Shell.et"), "m_fProjectileInitSpeedCoef": coefficient,
                "m_fStandardDispersion": 10.0 * f64::from(rings + 1), "m_sUnitType": "MILS_NATO"})));
        }
    }
    let weapon = resource(
        WEAPON,
        "Prefabs/Mortar.et",
        vec![
            node(
                "root/muzzle",
                "SCR_MortarMuzzleComponent",
                json!({"AmmoTemplate": format!("{{{SHELL}}}Prefabs/Shell.et"),
            "BulletInitSpeedCoef": 1.0, "DispersionDiameter": 1.0, "DispersionRange": 48.0}),
            ),
            node(
                "root/turret",
                "SCR_TurretControllerComponent",
                json!({"LimitsVert": [45.0, 85.0]}),
            ),
        ],
        Some("Test mortar"),
    );
    let card = resource(
        RANGE_CARD,
        "Prefabs/RangeCard.et",
        pages,
        Some("Test range card"),
    );
    let mut table_nodes = vec![node(
        "root",
        "BallisticTableArray",
        json!({"Indirect fire Table data":
        TABLE_COEFFICIENTS.iter().enumerate().map(|(index, _)| json!({"node_id": format!("root/table/{index}")})).collect::<Vec<_>>()}),
    )];
    for (index, coefficient) in TABLE_COEFFICIENTS.iter().enumerate() {
        let mut table_rows = rows(*coefficient);
        if let Some((skewed, row_index, offset)) = skew
            && skewed == *coefficient
        {
            table_rows[row_index][0] += offset;
        }
        table_nodes.push(node(
            &format!("root/table/{index}"),
            "BallisticTable",
            json!({"InitSpeedCoefficient": coefficient, "Table data": table_rows}),
        ));
    }
    let tables = resource(TABLES, "Configs/Tables.conf", table_nodes, None);
    let wind = resource(
        WIND,
        "Configs/Wind.conf",
        vec![
            node(
                "root",
                "SCR_ProjectileWindTable",
                json!({"m_aData": [{"node_id": "root/data/0"}]}),
            ),
            node(
                "root/data/0",
                "SCR_ProjectileWindData",
                json!({"m_fInitSpeedCoef": 1.0, "m_fWindSpeed": 10.0,
            "m_aFiringSolution": [[single(0.017), 15.0, single(0.06)], [single(0.035), 30.0, single(0.26)]],
            "m_aValues": [[single(0.1), 0.0, single(0.8)], [single(0.2), single(0.01), single(1.8)]]}),
            ),
        ],
        None,
    );
    let mut documents = vec![
        (WEAPON, "resources", weapon),
        (RANGE_CARD, "resources", card),
        (TABLES, "shared_configurations", tables),
        (WIND, "shared_configurations", wind),
    ];
    for (guid, ammunition) in SHELLS {
        documents.push((guid, "resources", shell(guid, ammunition)));
    }
    documents
        .into_iter()
        .map(|(guid, folder, document)| {
            (
                guid.to_owned(),
                format!("{folder}/arma_reforger/{}/{guid}.json", &guid[..2]),
                document,
            )
        })
        .collect()
}

fn write(path: &Path, value: &Value) -> Vec<u8> {
    fs::create_dir_all(path.parent().expect("parent")).expect("create folder");
    let bytes = serde_json::to_vec(value).expect("serialize");
    fs::write(path, &bytes).expect("write");
    bytes
}

fn write_oracle(directory: &Path, step_mils: f64) {
    let charge = |rings: u32| {
        CHARGES
            .iter()
            .find(|charge| charge.0 == rings)
            .expect("charge")
            .1
    };
    let forward: Vec<Value> = TABLE_COEFFICIENTS
        .iter()
        .flat_map(|coefficient| {
            let rings = CHARGES
                .iter()
                .find(|charge| charge.1 == *coefficient)
                .map(|charge| charge.0);
            forward_samples(*coefficient, rings, step_mils)
        })
        .collect();
    let altitude: Vec<Value> = CHARGES
        .iter()
        .map(|(rings, coefficient, _)| {
            json!({"rings": rings, "init_speed_coef": coefficient,
        "inputs": {"distance_m": 100.0, "altitude_difference_m": 50.0},
        "outputs": {"returned": true, "aim_height_m": 12.5, "time_of_flight_s": 10.25}})
        })
        .collect();
    let simulation: Vec<Value> = [0u32, 1]
        .iter()
        .map(|rings| {
            json!({"rings": rings, "init_speed_coef": charge(*rings),
        "inputs": {"elevation_deg": 45.0, "target_height_m": 0.0, "wind_speed_m_s": 0.0},
        "outputs": {"downrange_m": 400.0, "crossrange_m": 0.0, "time_of_flight_s": 9.5}})
        })
        .collect();
    let header = |kind: &str, run_at: &str| {
        json!({"document_type": kind, "export_generation_id": GENERATION_ID,
        "game_build": GAME_BUILD, "plugin_revision": PLUGIN_REVISION, "run_at": run_at, "status": "complete",
        "shell_error_count": 0, "errors": []})
    };
    let mut forward_document = header("forward", "2026-09-28T11:03:32Z");
    forward_document["elevation_lattice"] = json!({"mils_per_circle": 6400, "first_mils": 1600, "last_mils": 800, "step_mils": step_mils});
    forward_document["shells"] = SHELLS
        .iter()
        .map(|(guid, _)| {
            json!({"prefab_guid": guid, "forward_angle_samples": forward,
            "altitude_difference_samples": altitude, "errors": []})
        })
        .collect();
    let mut simulation_document = header("simulation", "2026-09-28T11:04:15Z");
    simulation_document["gravity"] =
        json!({"source": "PhysicsWorld.GetGravity", "magnitude_m_s2": f64::from(9.81_f32)});
    simulation_document["shells"] = SHELLS
        .iter()
        .map(|(guid, _)| json!({"prefab_guid": guid, "samples": simulation, "errors": [], "shell_errors": []}))
        .collect();
    for (file, document) in [
        ("forward_angles.json", forward_document),
        ("simulation.json", simulation_document),
    ] {
        let bytes = write(&directory.join(file), &document);
        let mut meta = header("meta", document["run_at"].as_str().unwrap_or_default());
        meta["file"] = json!(file);
        meta["bytes"] = json!(bytes.len());
        meta["sha256"] = json!(sha256_hex(&bytes));
        write(&directory.join(file.replace(".json", "_meta.json")), &meta);
    }
}

/// Writes the synthetic export and oracle output on the [`FORWARD_STEP_MILS`] lattice under a
/// fresh folder named for `tag`, with `skew` applied to the exported table rows only, and returns
/// where the trim reads and writes.
pub(super) fn synthetic_export(tag: &str, skew: Option<RowSkew>) -> (PathBuf, TrimLocations) {
    synthetic_export_on_lattice(tag, skew, FORWARD_STEP_MILS)
}

/// [`synthetic_export`] with the oracle's forward samples on a `step_mils` lattice.
pub(super) fn synthetic_export_on_lattice(
    tag: &str,
    skew: Option<RowSkew>,
    step_mils: f64,
) -> (PathBuf, TrimLocations) {
    let root = std::env::temp_dir().join(format!(
        "xtask-ballistics-trim-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let export_dir = root.join("export");
    let mut manifest = serde_json::Map::new();
    let mut index = Vec::new();
    for (guid, file, document) in resources(skew) {
        let bytes = write(&export_dir.join(&file), &document);
        manifest.insert(
            file.clone(),
            json!({"bytes": bytes.len(), "sha256": sha256_hex(&bytes)}),
        );
        index.push(json!({"resource_id": format!("guid:{guid}"), "resource_name": document["resource_name"], "resource_file": file}));
    }
    write(
        &export_dir.join("generation.json"),
        &json!({"generation_id": GENERATION_ID, "status": "completed",
        "environment": {"game_build": {"status": "present", "value": GAME_BUILD}}}),
    );
    write(
        &export_dir.join("manifest.json"),
        &json!({"generation_id": GENERATION_ID, "files": manifest}),
    );
    write(
        &export_dir.join("resource_index.json"),
        &json!({"resources": index}),
    );
    write_oracle(&root.join("oracle"), step_mils);
    let locations = TrimLocations {
        export_dir,
        oracle_dir: root.join("oracle"),
        catalog_path: root.join("out/catalogs/ballistics/test.catalog.json"),
        fixture_dir: root.join("out/fixtures/ballistics/test"),
    };
    (root, locations)
}
