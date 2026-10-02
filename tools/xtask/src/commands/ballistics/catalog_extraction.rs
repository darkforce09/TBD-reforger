//! The weapons and shells of a ballistics catalog, read from the gameplay export.
//!
//! **Role:** Turns each selected mortar (its prefab and its range card) into a
//! [`WeaponSystem`] and every shell its range card lists into a [`Shell`], reading only exported
//! component properties: `SCR_MortarMuzzleComponent`, `SCR_TurretControllerComponent`,
//! `ShellMoveComponent`, `SCR_MortarShellGadgetComponent`, `TimerTriggerComponent`,
//! `MagazineUIInfo` and the range card's `SCR_VisualisedBallisticConfig` pages.
//!
//! **Position:** Called by the trim with an opened [`GameplayExport`]; its [`ExtractedCatalog`]
//! feeds the catalog document and tells the table reader which configurations each shell names.
//!
//! **Signals & state:** none; each call reads the export afresh.
//!
//! **Invariants:** Every number passes through [`engine_number`]. The weapon's default ammunition
//! appears on its range card; the range card declares one mils unit; every shell has exactly one
//! default charge with a range card page; a shell listed by two weapons is extracted once. Any
//! missing component, property or page is an error, never a default.
use super::contract_documents::{Charge, ResourceRecord, Shell, TimeFuze, WeaponSystem};
use super::engine_numbers::engine_number;
use super::gameplay_export::{ExportedResource, GameplayExport, guid_of_resource_name};
use anyhow::{Context, Result, bail};
use serde_json::Value;

/// One mortar to trim: its catalog id, its prefab and the range card printed for it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WeaponSelection {
    /// Catalog `weapon_id`.
    pub(crate) weapon_id: &'static str,
    /// GUID of the mortar prefab.
    pub(crate) prefab_guid: &'static str,
    /// GUID of the range card (ballistic table item) that lists the mortar's shells.
    pub(crate) range_card_guid: &'static str,
}

/// The vanilla M252 and 2B14 with the US and USSR range cards.
pub(crate) const VANILLA_MORTARS: [WeaponSelection; 2] = [
    WeaponSelection {
        weapon_id: "m252",
        prefab_guid: "8094D99689ABE241",
        range_card_guid: "6113990D163E5249",
    },
    WeaponSelection {
        weapon_id: "2b14",
        prefab_guid: "D1FFE458E8AC4BDB",
        range_card_guid: "B41607EAB58A4252",
    },
];

/// Prefix of the localization key a magazine's `m_sAmmoType` carries before the shell name.
const AMMO_TYPE_KEY_PREFIX: &str = "#AR-AmmoType_";

/// A shell of the catalog with the configurations its tables live in.
pub(crate) struct ExtractedShell {
    pub(crate) shell: Shell,
    /// GUID of the shell's `BallisticTableArray` configuration.
    pub(crate) ballistic_table_guid: String,
    /// GUID of the shell's `SCR_ProjectileWindTable` configuration.
    pub(crate) wind_table_guid: String,
}

/// Weapons, shells and the resources they were read from.
pub(crate) struct ExtractedCatalog {
    pub(crate) weapons: Vec<WeaponSystem>,
    pub(crate) shells: Vec<ExtractedShell>,
    pub(crate) resources: Vec<ResourceRecord>,
}

/// One range card page: a shell at one charge coefficient.
struct RangeCardPage {
    shell_guid: String,
    init_speed_coef: f64,
    standard_dispersion_m: f64,
    unit_type: String,
}

fn record(resource: &ExportedResource) -> ResourceRecord {
    ResourceRecord {
        guid: resource.guid.clone(),
        resource_name: resource.resource_name.clone(),
        sha256: resource.sha256.clone(),
    }
}

fn string_property<'a>(
    resource: &ExportedResource,
    node: &'a Value,
    name: &str,
) -> Result<&'a str> {
    resource
        .property(node, name)?
        .as_str()
        .with_context(|| format!("{} property {name} is not a string", resource.resource_name))
}

fn number_property(resource: &ExportedResource, node: &Value, name: &str) -> Result<f64> {
    let value = resource.property(node, name)?;
    value
        .as_f64()
        .map(engine_number)
        .with_context(|| format!("{} property {name} is not a number", resource.resource_name))
}

/// The circle division an engine angle unit names.
fn mils_per_circle(unit_type: &str) -> Result<u32> {
    match unit_type {
        "MILS_NATO" => Ok(6400),
        "MILS_WP" => Ok(6000),
        other => bail!("range card unit {other:?} is not a mils unit"),
    }
}

/// The catalog role an `EAmmoType` flag set describes.
fn shell_role(ammo_type_flags: i64) -> &'static str {
    const HE: i64 = 8;
    const SMOKE: i64 = 64;
    const ILLUMINATION: i64 = 512;
    const TRAINING: i64 = 1024;
    if ammo_type_flags & ILLUMINATION != 0 {
        "illumination"
    } else if ammo_type_flags & SMOKE != 0 {
        "smoke"
    } else if ammo_type_flags & TRAINING != 0 {
        "practice"
    } else if ammo_type_flags & HE != 0 {
        "he"
    } else {
        "other"
    }
}

fn range_card_pages(card: &ExportedResource) -> Result<Vec<RangeCardPage>> {
    let pages = card.nodes_of_class("SCR_VisualisedBallisticConfig");
    if pages.is_empty() {
        bail!("range card {} has no ballistic pages", card.resource_name);
    }
    pages
        .into_iter()
        .map(|page| {
            Ok(RangeCardPage {
                shell_guid: guid_of_resource_name(string_property(
                    card,
                    page,
                    "m_sProjectilePrefab",
                )?)?
                .to_owned(),
                init_speed_coef: number_property(card, page, "m_fProjectileInitSpeedCoef")?,
                standard_dispersion_m: number_property(card, page, "m_fStandardDispersion")?,
                unit_type: string_property(card, page, "m_sUnitType")?.to_owned(),
            })
        })
        .collect()
}

fn charges(resource: &ExportedResource) -> Result<Vec<Charge>> {
    let gadget = resource.single_node("SCR_MortarShellGadgetComponent")?;
    let config = resource.property(gadget, "m_aChargeRingConfig")?;
    let mut charges = Vec::new();
    for entry in config.as_array().into_iter().flatten() {
        let parts: Vec<f64> = entry
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_f64)
            .collect();
        let [rings, coefficient, is_default] = parts.as_slice() else {
            bail!(
                "{} charge ring entry {entry} is not [rings, coefficient, default]",
                resource.resource_name
            );
        };
        if *rings < 0.0 || rings.fract() != 0.0 {
            bail!(
                "{} charge ring count {rings} is not a whole number",
                resource.resource_name
            );
        }
        charges.push(Charge {
            rings: *rings as u32,
            init_speed_coef: engine_number(*coefficient),
            is_default: *is_default != 0.0,
        });
    }
    if charges.iter().filter(|charge| charge.is_default).count() != 1 {
        bail!(
            "{} does not declare exactly one default charge",
            resource.resource_name
        );
    }
    Ok(charges)
}

fn time_fuze(resource: &ExportedResource) -> Result<Option<TimeFuze>> {
    let gadget = resource.single_node("SCR_MortarShellGadgetComponent")?;
    if resource.property(gadget, "m_bIsUsingTimeFuze")? != &Value::Bool(true) {
        return Ok(None);
    }
    Ok(Some(TimeFuze {
        min_s: number_property(resource, gadget, "m_fMinFuzeTime")?,
        max_s: number_property(resource, gadget, "m_fMaxFuzeTime")?,
        default_s: engine_number(resource.number("TimerTriggerComponent", "TIMER")?),
    }))
}

fn extract_shell(resource: &ExportedResource, pages: &[RangeCardPage]) -> Result<ExtractedShell> {
    let magazine = resource.single_node("MagazineUIInfo")?;
    let ammo_type = string_property(resource, magazine, "m_sAmmoType")?;
    let shell_id: String = ammo_type
        .strip_prefix(AMMO_TYPE_KEY_PREFIX)
        .with_context(|| {
            format!(
                "{} ammo type {ammo_type:?} has no {AMMO_TYPE_KEY_PREFIX} key",
                resource.resource_name
            )
        })?
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect();
    if shell_id.is_empty() {
        bail!(
            "{} ammo type {ammo_type:?} yields no shell id",
            resource.resource_name
        );
    }
    let flags = resource
        .property(magazine, "m_eAmmoTypeFlags")?
        .as_i64()
        .with_context(|| {
            format!(
                "{} m_eAmmoTypeFlags is not an integer",
                resource.resource_name
            )
        })?;
    let movement = resource.single_node("ShellMoveComponent")?;
    let move_number = |name: &str| number_property(resource, movement, name);
    let charges = charges(resource)?;
    let default_coefficient = charges
        .iter()
        .find(|charge| charge.is_default)
        .map(|charge| charge.init_speed_coef)
        .unwrap_or_default();
    let standard_dispersion_m = pages
        .iter()
        .find(|page| {
            page.shell_guid == resource.guid && page.init_speed_coef == default_coefficient
        })
        .map(|page| page.standard_dispersion_m)
        .with_context(|| {
            format!(
                "no range card page for {} at its default charge coefficient {default_coefficient}",
                resource.resource_name
            )
        })?;
    let shell = Shell {
        shell_id,
        display_name: resource.display_name()?,
        prefab_guid: resource.guid.clone(),
        role: shell_role(flags).to_owned(),
        init_speed_m_s: move_number("InitSpeed")?,
        init_speed_variation: move_number("InitSpeedVariation")?,
        mass_kg: move_number("Mass")?,
        air_drag: move_number("AirDrag")?,
        side_air_drag_scale: move_number("SideAirDragScale")?,
        wind_influence_multiplier: move_number("WindInfluenceMultiplier")?,
        dispersion_multiplier: move_number("DispersionMultiplier")?,
        time_to_live_s: move_number("TimeToLive")?,
        standard_dispersion_m,
        charges,
        time_fuze: time_fuze(resource)?,
    };
    let configuration_guid = |name: &str| -> Result<String> {
        Ok(guid_of_resource_name(string_property(resource, movement, name)?)?.to_owned())
    };
    Ok(ExtractedShell {
        shell,
        ballistic_table_guid: configuration_guid("BallisticTableConfig")?,
        wind_table_guid: configuration_guid("ProjectileWindTableConfig")?,
    })
}

/// Reads every selected weapon and every shell their range cards list.
pub(crate) fn extract_catalog(
    export: &GameplayExport,
    selections: &[WeaponSelection],
) -> Result<ExtractedCatalog> {
    let mut catalog = ExtractedCatalog {
        weapons: Vec::new(),
        shells: Vec::new(),
        resources: Vec::new(),
    };
    for selection in selections {
        let weapon = export.resource(selection.prefab_guid)?;
        let card = export.resource(selection.range_card_guid)?;
        let pages = range_card_pages(&card)?;
        let unit_type = &pages[0].unit_type;
        if pages.iter().any(|page| &page.unit_type != unit_type) {
            bail!("range card {} mixes mils units", card.resource_name);
        }
        let muzzle = weapon.single_node("SCR_MortarMuzzleComponent")?;
        let default_ammunition =
            guid_of_resource_name(string_property(&weapon, muzzle, "AmmoTemplate")?)?.to_owned();
        if !pages
            .iter()
            .any(|page| page.shell_guid == default_ammunition)
        {
            bail!(
                "range card {} does not list {}'s default ammunition {default_ammunition}",
                card.resource_name,
                weapon.resource_name
            );
        }
        let mut shell_ids = Vec::new();
        let mut caliber_mm = None;
        for page in &pages {
            let resource = export.resource(&page.shell_guid)?;
            if resource.guid == default_ammunition {
                caliber_mm = Some(engine_number(
                    resource.number("ShellMoveComponent", "Diameter")?,
                ));
            }
            if !catalog
                .shells
                .iter()
                .any(|shell| shell.shell.prefab_guid == resource.guid)
            {
                catalog.shells.push(extract_shell(&resource, &pages)?);
                catalog.resources.push(record(&resource));
            }
            let shell_id = catalog
                .shells
                .iter()
                .find(|shell| shell.shell.prefab_guid == resource.guid)
                .map(|shell| shell.shell.shell_id.clone())
                .unwrap_or_default();
            if !shell_ids.contains(&shell_id) {
                shell_ids.push(shell_id);
            }
        }
        let limits = weapon.property(
            weapon.single_node("SCR_TurretControllerComponent")?,
            "LimitsVert",
        )?;
        let (Some(low), Some(high)) = (limits[0].as_f64(), limits[1].as_f64()) else {
            bail!("{} LimitsVert is not [low, high]", weapon.resource_name);
        };
        catalog.weapons.push(WeaponSystem {
            weapon_id: selection.weapon_id.to_owned(),
            display_name: weapon.display_name()?,
            prefab_guid: weapon.guid.clone(),
            caliber_mm: caliber_mm.context("default ammunition caliber not read")?,
            mils_per_circle: mils_per_circle(unit_type)?,
            elevation_min_deg: engine_number(low),
            elevation_max_deg: engine_number(high),
            muzzle_init_speed_coef: number_property(&weapon, muzzle, "BulletInitSpeedCoef")?,
            dispersion_diameter_m: number_property(&weapon, muzzle, "DispersionDiameter")?,
            dispersion_range_m: number_property(&weapon, muzzle, "DispersionRange")?,
            shell_ids,
        });
        catalog.resources.push(record(&weapon));
        catalog.resources.push(record(&card));
    }
    catalog.resources.sort();
    Ok(catalog)
}
