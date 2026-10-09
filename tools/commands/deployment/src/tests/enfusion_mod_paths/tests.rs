use super::*;
use repository_layout::enfusion_mod_folders::{
    EXPORT_ADDON_DIR, EXPORT_ADDON_FOLDER_NAME, MCP_BRIDGE_ADDON_DIR, MCP_BRIDGE_ADDON_FOLDER_NAME,
};

#[test]
fn every_path_lies_under_the_mod_folder() {
    assert_eq!(mod_path(LOCAL_TEST_PROFILE), "mod/.local-test-profile");
    assert_eq!(
        mod_folder_exclusion(EXPORT_ADDON_FOLDER_NAME),
        "--exclude=mod/tbd-export/"
    );
    assert_eq!(
        mod_folder_exclusion(MCP_BRIDGE_ADDON_FOLDER_NAME),
        "--exclude=mod/tbd-emcp/"
    );
    assert_eq!(
        mod_folder_exclusion(UNTRACKED_FRAMEWORK_FOLDER),
        "--exclude=mod/Tbd_framework/"
    );
}

/// An addon's exclusion names exactly the addon's checkout folder, so the rsync leaves out the
/// folder the workstation and the Workbench session use.
#[test]
fn an_addon_exclusion_names_the_addon_folder() {
    for (name, folder) in [
        (EXPORT_ADDON_FOLDER_NAME, EXPORT_ADDON_DIR),
        (MCP_BRIDGE_ADDON_FOLDER_NAME, MCP_BRIDGE_ADDON_DIR),
    ] {
        assert_eq!(mod_path(name), folder);
        assert_eq!(mod_folder_exclusion(name), format!("--exclude={folder}/"));
    }
}
