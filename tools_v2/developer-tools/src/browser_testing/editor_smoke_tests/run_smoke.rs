use super::*;
use crate::repository_layout::terrain_assets_dir;
use std::path::Path;

/// Dispatch one smoke by suite name. `dist`/`path` fall back to the Node defaults.
pub async fn run_smoke(name: &str, dist: Option<String>, path: Option<String>) -> Result<u8> {
    let dist = dist.unwrap_or_else(|| DIST_DEFAULT.to_string());
    let path = path.unwrap_or_else(|| EDIT_PATH.to_string());
    // The harness resolves a relative serving directory against the gate's working directory.
    let map_assets = terrain_assets_dir(Path::new(""))
        .to_string_lossy()
        .into_owned();
    match name {
        "editor" => smoke_editor(&dist, &path).await,
        "selfcheck" => smoke_selfcheck(&dist, &path).await,
        "fullmap" => smoke_fullmap(&dist, &map_assets).await,
        "hillshade" => smoke_hillshade(&dist, &map_assets).await,
        "doc" => smoke_doc(&dist, &path).await,
        "pan" => smoke_pan(&dist, &path).await,
        "persist" => smoke_persist(&dist, &path).await,
        "select" => smoke_select(&dist, &path).await,
        "t946-86" => outliner_drag::run(&dist).await,
        "save-export" => smoke_save_export(&dist, &path).await,
        "save-dialog-rect" => smoke_save_dialog_rect(&dist, &path).await,
        "entrance-motion-rect" => smoke_entrance_motion_rect(&dist, &path).await,
        "cur" => smoke_cur(&dist, &path).await,
        "attributes" => smoke_attributes(&dist, &path).await,
        "keyboard-settings" => smoke_keyboard_settings(&dist, &path).await,
        "arsenal" => smoke_arsenal(&dist, &path).await,
        "marquee-drag" => smoke_marquee_drag(&dist, &path).await,
        "undo" => smoke_undo(&dist, &path).await,
        "outliner-palette" => smoke_outliner_palette(&dist, &path).await,
        "virtual-outliner" => smoke_virtual_outliner(&dist, &path).await,
        "hydrate" => smoke_hydrate(&dist).await,
        "mutations" => smoke_mutations(&dist).await,
        "perf" => smoke_perf(&dist, false).await,
        "perf-strict" => smoke_perf(&dist, true).await,
        other => Err(anyhow!("unknown smoke '{other}' (see gate smoke --help)")),
    }
}

/// The `cargo xtask mk leptos-gates` smoke chain: every editor smoke in glob order, first failure stops
/// (the Makefile `set -e` semantics). Returns the first non-zero code, else 0.
pub async fn editor_suite(dist: Option<String>) -> Result<u8> {
    for name in EDITOR_SUITE {
        println!("== gate smoke {name}");
        let code = run_smoke(name, dist.clone(), None).await?;
        if code != 0 {
            return Ok(code);
        }
    }
    Ok(0)
}
