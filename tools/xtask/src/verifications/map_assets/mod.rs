//! Delegate map verification without importing map-engine types.
use anyhow::Result;
use repository_layout::find_repository_root;
use std::path::Path;

pub fn verify_blas_manifest(root: &Path) -> Result<u8> {
    developer_tools::map_verification::blas_manifest::verify_blas_manifest(root)
}

pub fn map_object_golden() -> Result<u8> {
    developer_tools::map_verification::object_goldens::map_object_golden(&find_repository_root()?)
}

pub fn terrain_manifest(terrain: &str) -> Result<u8> {
    developer_tools::map_verification::terrain_manifest::terrain_manifest(
        &find_repository_root()?,
        terrain,
    )
}

pub fn height_labels(terrain: &str) -> Result<u8> {
    developer_tools::map_verification::labels::height_labels(&find_repository_root()?, terrain)
}

pub fn locations(terrain: &str) -> Result<u8> {
    developer_tools::map_verification::labels::locations(&find_repository_root()?, terrain)
}

pub fn terrain_alignment(terrain: &str, strict: bool) -> Result<u8> {
    developer_tools::map_verification::labels::terrain_alignment(
        &find_repository_root()?,
        terrain,
        strict,
    )
}

pub fn town_labels(terrain: &str, zoom: f64) -> Result<u8> {
    developer_tools::map_verification::labels::town_labels(&find_repository_root()?, terrain, zoom)
}

pub fn road_names(terrain: &str, zoom: f64) -> Result<u8> {
    developer_tools::map_verification::labels::road_names(&find_repository_root()?, terrain, zoom)
}
