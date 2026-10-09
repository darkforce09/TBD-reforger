use super::*;
use crate::workspace_folders::ENFUSION_MOD_DIR;
use repository_root::find_repository_root;

/// Each addon folder paired with its folder name.
const ADDONS: [(&str, &str); 3] = [
    (FRAMEWORK_ADDON_DIR, FRAMEWORK_ADDON_FOLDER_NAME),
    (EXPORT_ADDON_DIR, EXPORT_ADDON_FOLDER_NAME),
    (MCP_BRIDGE_ADDON_DIR, MCP_BRIDGE_ADDON_FOLDER_NAME),
];

/// Each addon folder is its folder name directly inside the mod folder.
#[test]
fn every_addon_folder_is_its_name_inside_the_mod_folder() {
    for (folder, name) in ADDONS {
        assert_eq!(folder, format!("{ENFUSION_MOD_DIR}/{name}"));
        assert!(!name.contains('/'), "{name}");
    }
}

/// The checkout holds each addon's project file where its folder points, so a mod folder or
/// addon rename that misses this module fails here.
#[test]
fn every_addon_folder_holds_its_project_file_in_the_checkout() {
    let root = find_repository_root().expect("active checkout");
    for (folder, _) in ADDONS {
        let project = root.join(folder).join("addon.gproj");
        assert!(project.is_file(), "missing {}", project.display());
    }
}
