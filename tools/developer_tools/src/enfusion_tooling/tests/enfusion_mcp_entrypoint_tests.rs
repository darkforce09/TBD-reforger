use super::*;
use crate::repository_paths::find_repo_root;

/// The pattern `pkill -f` receives must match a real command line and must not match a neighbour.
///
/// It is a path suffix rather than a whole path, so it finds this repository's server from any
/// checkout location and from an npm cache copy — and because a bare `.` in a regular expression
/// matches any character, the dot before the extension is escaped so `indexZjs` cannot match.
#[test]
fn the_process_pattern_is_the_escaped_installed_module_suffix() {
    assert_eq!(
        process_pattern(),
        r"node_modules/enfusion-mcp/dist/index\.js"
    );
}

/// A checkout with the package installed resolves to that copy, under `node`.
///
/// This is the tier every agent session and every gate takes, so it is asserted against the live
/// tree rather than a synthetic root.
#[test]
fn an_installed_package_resolves_to_the_pinned_module() {
    let root = find_repo_root().expect("active checkout");
    if !enfusion_mcp_entrypoint(&root).is_file() {
        return;
    }
    // SAFETY: single-threaded test process; the variable is restored before returning.
    let restore = std::env::var("ENFUSION_MCP_BIN").ok();
    unsafe { std::env::remove_var("ENFUSION_MCP_BIN") };
    let command = resolve(&root);
    if let Some(value) = restore {
        unsafe { std::env::set_var("ENFUSION_MCP_BIN", value) };
    }

    assert_eq!(command.source, EnfusionMcpSource::PinnedPackage);
    assert_eq!(command.program, "node");
    assert_eq!(
        command.entry_path.as_deref(),
        Some(enfusion_mcp_entrypoint(&root).to_string_lossy().as_ref())
    );
    assert_eq!(command.argv().len(), 2);
}

/// A root with nothing installed and no cache copy falls through to the download tier, which has
/// no file on disk to name.
#[test]
fn a_root_without_an_installed_package_falls_through_to_the_download_tier() {
    // SAFETY: single-threaded test process; both variables are restored before returning.
    let restore_bin = std::env::var("ENFUSION_MCP_BIN").ok();
    let restore_home = std::env::var("HOME").ok();
    unsafe {
        std::env::remove_var("ENFUSION_MCP_BIN");
        std::env::set_var("HOME", "/nonexistent-home-for-entrypoint-resolution");
    }
    let command = resolve(Path::new("/nonexistent-checkout-for-entrypoint-resolution"));
    unsafe {
        if let Some(value) = restore_bin {
            std::env::set_var("ENFUSION_MCP_BIN", value);
        }
        if let Some(value) = restore_home {
            std::env::set_var("HOME", value);
        }
    }

    assert_eq!(command.source, EnfusionMcpSource::NpxDownload);
    assert_eq!(command.program, "npx");
    assert_eq!(command.args, vec!["-y".to_string(), "enfusion-mcp".into()]);
    assert!(command.entry_path.is_none());
}

/// Every tier carries a label that names it without abbreviation, because the labels are what a
/// `MCP_DEBUG=1` run prints when a call reached the wrong server.
#[test]
fn every_source_has_a_self_describing_label() {
    for (source, label) in [
        (
            EnfusionMcpSource::EnvironmentOverride,
            "environment-override",
        ),
        (EnfusionMcpSource::PinnedPackage, "pinned-package"),
        (EnfusionMcpSource::NpxCache, "npx-cache"),
        (EnfusionMcpSource::NpxDownload, "npx-download"),
    ] {
        assert_eq!(source.label(), label);
    }
}
