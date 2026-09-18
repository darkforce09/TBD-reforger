use super::cli::SchemaCmd;
use crate::*;
use anyhow::Result;

pub(crate) fn run(cmd: SchemaCmd) -> Result<u8> {
    {
        let code = match cmd {
            SchemaCmd::Codegen => crate::commands::generate::schema_types::codegen()?,
            SchemaCmd::ListGates => {
                u8::try_from(crate::commands::ci::task_runner::schema_list_gates()).unwrap_or(1)
            }
            SchemaCmd::Validate => crate::verifications::schemas::checks::validate_all()?,
            SchemaCmd::ValidateFile { target } => {
                crate::verifications::schemas::checks::validate_file(&target)?
            }
            SchemaCmd::Citations => crate::verifications::schemas::checks::citations()?,
            SchemaCmd::T090Specs => crate::verifications::schemas::checks::t090_specs()?,
            SchemaCmd::N6 => crate::verifications::schemas::checks::n6_sentence()?,
            SchemaCmd::N10 => crate::verifications::schemas::checks::n10_tile_budget()?,
            SchemaCmd::MapObjectGolden => verifications::map_assets::map_object_golden()?,
            SchemaCmd::HeightLabels { terrain } => {
                verifications::map_assets::height_labels(&terrain)?
            }
            SchemaCmd::TerrainAlignment { terrain, strict } => {
                verifications::map_assets::terrain_alignment(&terrain, strict)?
            }
            SchemaCmd::Locations { terrain } => verifications::map_assets::locations(&terrain)?,
            SchemaCmd::TownLabels { terrain, zoom } => {
                verifications::map_assets::town_labels(&terrain, zoom)?
            }
            SchemaCmd::RoadNames { terrain, zoom } => {
                verifications::map_assets::road_names(&terrain, zoom)?
            }
            SchemaCmd::MapGlyphs => crate::verifications::schemas::checks::map_glyphs()?,
            SchemaCmd::MapObjectEnums => crate::verifications::schemas::checks::map_object_enums()?,
            SchemaCmd::TypeInventory => crate::verifications::schemas::checks::type_inventory()?,
            SchemaCmd::TerrainManifest { terrain } => {
                verifications::map_assets::terrain_manifest(&terrain)?
            }
            SchemaCmd::FlattenOrbatSlots { path, in_place } => {
                super::mission_flattening::flatten_orbat_slots(&path, in_place)?
            }
        };
        Ok(code)
    }
}
