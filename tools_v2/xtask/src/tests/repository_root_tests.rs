use super::*;
use crate::commands::platform::wave_execution::testcwd::CwdGuard;

#[test]
fn nested_tooling_directories_resolve_repository_and_fixtures() {
    let root = test_repo_root();
    for relative in [".", "tools_v2/xtask", "tools_v2/xtask/src"] {
        let _cwd = CwdGuard::enter(&root.join(relative));
        let resolved = find_repo_root().expect("repository root");
        assert_eq!(resolved, root);
        for fixture in [
            "contracts_v2/definitions/mission.schema.json",
            "tools_v2/developer-tools/test_fixtures/blueprint/FarmHouse_E_1L01_Wood_children.json",
            "tools_v2/xtask/src/commands/platform/wave_execution/schema.rs",
            "scripts/mod/package.json",
        ] {
            assert!(resolved.join(fixture).is_file(), "missing {fixture}");
        }
    }
}
