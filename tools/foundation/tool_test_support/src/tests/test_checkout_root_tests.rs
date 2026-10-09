use super::*;
use crate::CwdGuard;

#[test]
fn nested_tooling_directories_resolve_repository_and_fixtures() {
    let root = test_repo_root();
    for relative in [".", "tools/xtask", "tools/xtask/src"] {
        let _cwd = CwdGuard::enter(&root.join(relative));
        let resolved = repository_root::find_repository_root().expect("repository root");
        assert_eq!(resolved, root);
        for fixture in [
            "contracts/definitions/mission.schema.json",
            "tools/map_assets/blueprint_compiler/test_fixtures/blueprint/FarmHouse_E_1L01_Wood_children.json",
            "tools/commands/platform_execution/src/wave_execution/schema.rs",
            "tools/enfusion_mcp_node_package/package.json",
        ] {
            assert!(resolved.join(fixture).is_file(), "missing {fixture}");
        }
    }
}
