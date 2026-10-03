use super::cli::SchemaCmd;
use anyhow::Result;
use ci_task_catalog::map_asset_checks;

pub(crate) fn run(cmd: SchemaCmd) -> Result<u8> {
    {
        let code = match cmd {
            SchemaCmd::Codegen => schema_tooling::codegen()?,
            SchemaCmd::ListGates => {
                u8::try_from(ci_task_catalog::task_runner::schema_list_gates()).unwrap_or(1)
            }
            SchemaCmd::Validate => schema_tooling::validate_all()?,
            SchemaCmd::ValidateFile { target } => schema_tooling::validate_file(&target)?,
            SchemaCmd::Citations => schema_tooling::citations()?,
            SchemaCmd::MapObjectGolden => map_asset_checks::map_object_golden()?,
            SchemaCmd::HeightLabels { terrain } => map_asset_checks::height_labels(&terrain)?,
            SchemaCmd::TerrainAlignment { terrain, strict } => {
                map_asset_checks::terrain_alignment(&terrain, strict)?
            }
            SchemaCmd::Locations { terrain } => map_asset_checks::locations(&terrain)?,
            SchemaCmd::TownLabels { terrain, zoom } => {
                map_asset_checks::town_labels(&terrain, zoom)?
            }
            SchemaCmd::RoadNames { terrain, zoom } => map_asset_checks::road_names(&terrain, zoom)?,
            SchemaCmd::MapGlyphs => schema_tooling::map_glyphs()?,
            SchemaCmd::MapObjectEnums => schema_tooling::map_object_enums()?,
            SchemaCmd::TypeInventory => schema_tooling::type_inventory()?,
            SchemaCmd::TerrainManifest { terrain } => map_asset_checks::terrain_manifest(&terrain)?,
            SchemaCmd::FlattenOrbatSlots { path, in_place } => {
                schema_tooling::flatten_orbat_slots(&path, in_place)?
            }
        };
        Ok(code)
    }
}
