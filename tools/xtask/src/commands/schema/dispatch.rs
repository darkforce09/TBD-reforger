use super::cli::SchemaCmd;
use anyhow::Result;
use map_asset_verification::{labels, object_goldens, terrain_manifest};
use repository_layout::prelude::find_repository_root;

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
            SchemaCmd::MapObjectGolden => {
                object_goldens::map_object_golden(&find_repository_root()?)?
            }
            SchemaCmd::HeightLabels { terrain } => {
                labels::height_labels(&find_repository_root()?, &terrain)?
            }
            SchemaCmd::TerrainAlignment { terrain, strict } => {
                labels::terrain_alignment(&find_repository_root()?, &terrain, strict)?
            }
            SchemaCmd::Locations { terrain } => {
                labels::locations(&find_repository_root()?, &terrain)?
            }
            SchemaCmd::TownLabels { terrain, zoom } => {
                labels::town_labels(&find_repository_root()?, &terrain, zoom)?
            }
            SchemaCmd::RoadNames { terrain, zoom } => {
                labels::road_names(&find_repository_root()?, &terrain, zoom)?
            }
            SchemaCmd::MapGlyphs => schema_tooling::map_glyphs()?,
            SchemaCmd::MapObjectEnums => schema_tooling::map_object_enums()?,
            SchemaCmd::TypeInventory => schema_tooling::type_inventory()?,
            SchemaCmd::TerrainManifest { terrain } => {
                terrain_manifest::terrain_manifest(&find_repository_root()?, &terrain)?
            }
            SchemaCmd::FlattenOrbatSlots { path, in_place } => {
                schema_tooling::flatten_orbat_slots(&path, in_place)?
            }
        };
        Ok(code)
    }
}
