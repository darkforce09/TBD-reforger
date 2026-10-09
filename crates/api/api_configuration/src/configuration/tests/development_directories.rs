//! The development defaults of the directory settings: joined onto the checkout root found from a
//! folder anywhere inside it, refused when no root marker lies above, and never consulted for a
//! set value or outside development.

use super::*;
use std::fs;
use std::path::Path;

use repository_root::{ROOT_MARKER, find_repository_root_from};

/// Every directory setting with a development default.
const ALL: [CheckoutDirectory; 4] = [UPLOAD, EQUIPMENT_DATA, MAP_ASSETS, GLYPH_ASSETS];

/// A scratch folder of its own under the system temporary folder, emptied first.
fn scratch(name: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("api-configuration-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).expect("create the scratch folder");
    folder
}

/// Plants the root marker in `folder`, making it a checkout root.
fn plant_marker(folder: &Path) {
    let marker = folder.join(ROOT_MARKER);
    fs::create_dir_all(marker.parent().expect("the marker's folder")).expect("marker folder");
    fs::write(marker, "").expect("plant the root marker");
}

/// A walk the resolution must not run.
fn never_walks() -> repository_root::Result<PathBuf> {
    panic!("the checkout root was looked up for a value that needs no default")
}

#[test]
fn a_development_default_is_the_checkout_folder_from_any_folder_inside_the_checkout() {
    let root = scratch("anchored");
    plant_marker(&root);
    for start in [
        root.clone(),
        root.join("crates/api/api_server"),
        root.join("assets/terrains/everon"),
    ] {
        fs::create_dir_all(&start).expect("create the starting folder");
        for setting in ALL {
            let resolved = resolve(setting, "", true, || find_repository_root_from(&start))
                .unwrap_or_else(|error| panic!("{}: {error}", setting.variable));
            assert_eq!(
                PathBuf::from(&resolved),
                root.join(setting.checkout_relative),
                "{} from {}",
                setting.variable,
                start.display()
            );
            assert!(Path::new(&resolved).is_absolute(), "{resolved}");
        }
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_defaults_name_the_checkout_folders() {
    let defaults: Vec<(&str, &str)> = ALL
        .iter()
        .map(|setting| (setting.variable, setting.checkout_relative))
        .collect();
    assert_eq!(
        defaults,
        [
            ("UPLOAD_DIR", "assets/scratch/api/uploads"),
            ("EQUIPMENT_DATA_DIR", "assets/equipment"),
            ("MAP_ASSETS_DIR", "assets/terrains"),
            ("GLYPH_ASSETS_DIR", "assets/glyphs"),
        ]
    );
}

/// The Docker image and a process started outside the checkout have no root marker above them: an
/// unset directory there is refused by name instead of becoming a path relative to the working
/// directory.
#[test]
fn a_development_default_without_a_checkout_root_is_refused() {
    let root = scratch("unanchored");
    plant_marker(&root);
    let start = root.join("crates/api/api_server");
    fs::create_dir_all(&start).expect("create the starting folder");
    resolve(MAP_ASSETS, "", true, || find_repository_root_from(&start))
        .expect("the marker is planted");

    fs::remove_file(root.join(ROOT_MARKER)).expect("remove the root marker");
    for setting in ALL {
        match resolve(setting, "", true, || find_repository_root_from(&start)) {
            Err(ConfigError::CheckoutRootNotFound(variable, _)) => {
                assert_eq!(variable, setting.variable);
            }
            other => panic!(
                "{}: expected CheckoutRootNotFound, got {other:?}",
                setting.variable
            ),
        }
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_refusal_names_the_variable_and_the_folder_the_walk_started_from() {
    let start = scratch("refusal-message");
    let error = resolve(UPLOAD, "", true, || find_repository_root_from(&start))
        .expect_err("no marker above a temporary folder");
    let message = error.to_string();
    assert!(message.starts_with("UPLOAD_DIR is unset"), "{message}");
    assert!(message.contains(&start.display().to_string()), "{message}");
    let _ = fs::remove_dir_all(&start);
}

#[test]
fn a_set_value_is_kept_as_written_without_a_walk() {
    for development in [true, false] {
        for configured in ["/srv/tbd/assets/terrains", "relative/terrains"] {
            assert_eq!(
                resolve(MAP_ASSETS, configured, development, never_walks).expect("set value"),
                configured
            );
        }
    }
}

#[test]
fn outside_development_an_unset_value_stays_empty_without_a_walk() {
    for setting in ALL {
        assert_eq!(
            resolve(setting, "", false, never_walks).expect("unset value"),
            "",
            "{}",
            setting.variable
        );
    }
}
