//! The frontend source walk: it reaches the shell layer's app and worker once each, and the
//! once-per-package check refuses a package read twice, under one spelling or two.

use super::{assert_each_package_read_once, frontend_source_roots};
use crate::repository_root::repository_root;
use std::path::PathBuf;

/// The `src/` folder of the shell crate `crate_folder`.
fn shell_source(crate_folder: &str) -> PathBuf {
    repository_root(env!("CARGO_MANIFEST_DIR"))
        .join("crates/frontend/shell")
        .join(crate_folder)
        .join("src")
}

/// The live walk holds the app, the worker and this crate once each, and one root per crate
/// folder of every layer.
#[test]
fn the_walk_reads_every_frontend_package_once() {
    let repository = repository_root(env!("CARGO_MANIFEST_DIR"));
    let roots = frontend_source_roots(&repository);
    for expected in [
        shell_source("frontend_application"),
        shell_source("offline_service_worker"),
        repository.join("crates/frontend/foundation/frontend_test_support/src"),
    ] {
        assert_eq!(
            roots.iter().filter(|root| **root == expected).count(),
            1,
            "{} is not read exactly once: {roots:#?}",
            expected.display()
        );
    }
    let crate_folders = std::fs::read_dir(repository.join("crates/frontend"))
        .expect("read crates/frontend")
        .map(|layer| layer.expect("read a layer entry").path())
        .filter(|layer| layer.is_dir())
        .map(|layer| {
            std::fs::read_dir(&layer)
                .expect("read a layer folder")
                .filter(|entry| entry.as_ref().expect("read a crate entry").path().is_dir())
                .count()
        })
        .sum::<usize>();
    assert_eq!(roots.len(), crate_folders, "{roots:#?}");
}

/// Two distinct packages pass the check.
#[test]
fn two_packages_pass_the_once_per_package_check() {
    assert_each_package_read_once(&[
        shell_source("frontend_application"),
        shell_source("offline_service_worker"),
    ]);
}

/// The app's source root listed twice is refused, naming the package.
#[test]
#[should_panic(expected = "the scan reads package `frontend_application` twice")]
fn a_package_listed_twice_is_refused() {
    let app = shell_source("frontend_application");
    assert_each_package_read_once(&[app.clone(), shell_source("offline_service_worker"), app]);
}

/// Two spellings of one folder are one package, so the second is refused.
#[test]
#[should_panic(expected = "the scan reads package `offline_service_worker` twice")]
fn two_spellings_of_one_root_are_one_package() {
    let worker = shell_source("offline_service_worker");
    let respelled = worker.join("..").join("src");
    assert_each_package_read_once(&[worker, respelled]);
}

/// A folder with no manifest beside it names no package and fails closed.
#[test]
#[should_panic(expected = "Cargo.toml")]
fn a_root_without_a_manifest_beside_it_is_refused() {
    assert_each_package_read_once(&[shell_source("frontend_application").join("shell")]);
}
